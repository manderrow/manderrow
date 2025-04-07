#![deny(unused_must_use)]
#![feature(os_str_slice)]
#![feature(panic_backtrace_config)]
#![feature(slice_split_once)]

mod crash;
mod init;

use std::ops::ControlFlow;
use std::sync::OnceLock;

use init::MaybeArgs;
use manderrow_ipc::ipc_channel::ipc::IpcSender;
use manderrow_ipc::{C2SMessage, Ipc, LogLevel, S2CMessage};
use manderrow_types::agent::Instruction;
use slog::info;

static IPC: OnceLock<Ipc> = OnceLock::new();

fn send_ipc_sync(log: &slog::Logger, message: impl FnOnce() -> C2SMessage) {
    if let Some(ipc) = IPC.get() {
        let msg = message();
        _ = ipc.send(msg);
    } else {
        info!(log, "{:?}", message());
    }
}

#[derive(Debug, thiserror::Error)]
enum SendError {
    #[error(transparent)]
    Ipc(#[from] manderrow_ipc::SendError),
    #[error(transparent)]
    Tokio(#[from] tokio::task::JoinError),
}

async fn send_ipc(log: &slog::Logger, message: impl FnOnce() -> C2SMessage) {
    if let Some(ipc) = IPC.get() {
        let msg = message();
        _ = tokio::task::spawn_blocking(move || ipc.send(msg))
            .await
            .unwrap();
    } else {
        info!(log, "{:?}", message());
    }
}

#[cfg(target_os = "windows")]
#[no_mangle]
pub unsafe extern "stdcall" fn DllMain(
    _module: HMODULE,
    reason: isize,
    _res: *const std::ffi::c_void,
) -> i32 {
    if reason == 1 {
        main();
    }

    1
}

#[cfg(not(target_os = "windows"))]
#[ctor::ctor]
fn init() {
    main();
}

fn main() {
    std::panic::set_backtrace_style(std::panic::BacktraceStyle::Full);
    std::panic::set_hook(Box::new(|info| {
        crash::report_crash(
            if let Some(&s) = info.payload().downcast_ref::<&'static str>() {
                s
            } else if let Some(s) = info.payload().downcast_ref::<String>() {
                s.as_str()
            } else {
                "Box<dyn Any>"
            },
        )
    }));

    std::fs::write(
        "manderrow-agent-args.txt",
        format!("{:?}", std::env::args_os().collect::<Vec<_>>()),
    )
    .unwrap();

    let MaybeArgs::Enabled(mut args) = init::init(std::env::args_os()).unwrap() else {
        return;
    };

    let log = slog_scope::logger();

    // now we need to run some async code, so do the rest in a tokio runtime
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            send_ipc(&log, || C2SMessage::Started {
                pid: std::process::id(),
            })
            .await;

            interpret_instructions(args.instructions.drain(..));

            // TODO: replace stdout and stderr with in-process pipes and spawn a thread to listen to them and forward over IPC
            // let tasks = if let Some(ipc) = ipc {
            //     fn spawn_output_pipe_task<const TRY_PARSE_LOGS: bool>(
            //         c2s_tx: &IpcSender<C2SMessage>,
            //         rdr: impl tokio::io::AsyncRead + Unpin + Send + 'static,
            //         channel: crate::ipc::StandardOutputChannel,
            //     ) -> tokio::task::JoinHandle<Result<(), anyhow::Error>> {
            //         let c2s_tx = c2s_tx.clone();
            //         tokio::task::spawn(async move {
            //             let mut rdr = tokio::io::BufReader::new(rdr);
            //             let mut buf = Vec::new();
            //             loop {
            //                 rdr.read_until(b'\n', &mut buf).await?;
            //                 if buf.is_empty() {
            //                     break Ok(());
            //                 }
            //                 if matches!(buf.last(), Some(b'\n')) {
            //                     buf.pop();
            //                     if matches!(buf.last(), Some(b'\r')) {
            //                         buf.pop();
            //                     }
            //                 }
            //                 if TRY_PARSE_LOGS {
            //                     if let ControlFlow::Break(()) = try_handle_log_record(&c2s_tx, &buf)
            //                     {
            //                         buf.clear();
            //                         continue;
            //                     }
            //                 }
            //                 let line = OutputLine::new(std::mem::take(&mut buf));
            //                 let c2s_tx = &c2s_tx;
            //                 _ = tokio::task::block_in_place(move || {
            //                     c2s_tx.send(C2SMessage::Output { channel, line })
            //                 });
            //             }
            //         })
            //     }
            //     Some((
            //         spawn_output_pipe_task::<false>(
            //             &ipc.c2s_tx,
            //             child.stdout.take().unwrap(),
            //             crate::ipc::StandardOutputChannel::Out,
            //         ),
            //         spawn_output_pipe_task::<true>(
            //             &ipc.c2s_tx,
            //             child.stderr.take().unwrap(),
            //             crate::ipc::StandardOutputChannel::Err,
            //         ),
            //     ))
            // } else {
            //     None
            // };

            // TODO: intercept process exit to send exit message
            // send_ipc(log, ipc, || {
            //     Ok(C2SMessage::Exit {
            //         code: status.code(),
            //     })
            // })
            // .await?;

            if let Some(ipc) = init::ipc() {
                std::thread::Builder::new()
                    .name("manderrow-killer".into())
                    .spawn(move || {
                        while let Ok(msg) = ipc.recv() {
                            match msg {
                                S2CMessage::Connect => {}
                                S2CMessage::PatientResponse { .. } => {}
                                S2CMessage::Kill => std::process::exit(1),
                            }
                        }
                    })
                    .unwrap();
            }
        });
}

fn interpret_instructions(instructions: impl IntoIterator<Item = Instruction>) {
    for insn in instructions {
        match insn {
            Instruction::LoadLibrary { path } => {
                let lib = unsafe { libloading::Library::new(&path) }
                    .unwrap_or_else(|e| panic!("Failed to load library {:?}: {}", path, e));
                // forget the lib so it won't be unloaded
                std::mem::forget(lib);
            }
            Instruction::SetVar { kv, eq_sign } => {
                let key = kv.slice_encoded_bytes(..eq_sign);
                let value = kv.slice_encoded_bytes(eq_sign + 1..);
                // SAFETY: this is technically unsafe except on Windows, but it seems extremely
                //         unlikely to be an issue for our use case.
                unsafe { std::env::set_var(key, value) };
            }
            Instruction::PrependArg { arg } => {
                todo!()
            }
            Instruction::AppendArg { arg } => {
                todo!()
            }
        }
    }
}

fn try_handle_log_record(c2s_tx: &IpcSender<C2SMessage>, buf: &[u8]) -> ControlFlow<()> {
    if let Some((level, rem)) = buf.split_once(|b| *b == b' ') {
        if let Some((scope, msg)) = rem.split_once(|b| *b == b' ') {
            let level = match level {
                b"fatal" => Some(LogLevel::Critical),
                b"err" => Some(LogLevel::Error),
                b"warn" => Some(LogLevel::Warning),
                b"msg" | b"info" => Some(LogLevel::Info),
                b"debug" => Some(LogLevel::Debug),
                _ => None,
            };
            if let Some(level) = level {
                if let Ok(scope) = std::str::from_utf8(scope) {
                    if scope.chars().all(|c| c.is_ascii_graphic()) {
                        if let Ok(msg) = std::str::from_utf8(msg) {
                            _ = c2s_tx.send(C2SMessage::Log {
                                level,
                                scope: scope.into(),
                                message: msg.to_owned(),
                            });
                            return ControlFlow::Break(());
                        }
                    }
                }
            }
        }
    }
    ControlFlow::Continue(())
}
