#![deny(unused_must_use)]
#![feature(os_str_slice)]
#![feature(panic_backtrace_config)]
#![feature(slice_split_once)]

mod crash;
mod init;

use std::ops::ControlFlow;
use std::sync::Once;

use manderrow_ipc::ipc_channel::ipc::IpcSender;
use manderrow_ipc::{C2SMessage, LogLevel, S2CMessage};
use slog::info;

use init::{Instruction, MaybeArgs, ipc};

fn send_ipc(log: &slog::Logger, message: impl FnOnce() -> C2SMessage) {
    if let Some(ipc) = ipc() {
        _ = ipc.send(message());
    } else {
        info!(log, "{:?}", message());
    }
}

#[cfg(target_os = "windows")]
#[repr(transparent)]
pub struct HMODULE(*mut std::ffi::c_void);

#[cfg(target_os = "windows")]
#[unsafe(no_mangle)]
pub unsafe extern "stdcall" fn DllMain(
    module: HMODULE,
    reason: isize,
    _res: *const std::ffi::c_void,
) -> i32 {
    if reason == 1 {
        init();

        unsafe {
            loadProxy(module);
        }
    }

    1
}

#[cfg(not(target_os = "windows"))]
#[ctor::ctor]
fn ctor() {
    init();
}

const DEINIT: Once = Once::new();

#[cfg(not(target_os = "windows"))]
unsafe extern "C" {
    fn atexit(f: extern "C" fn());

    fn at_quick_exit(f: extern "C" fn());
}

#[cfg(not(target_os = "windows"))]
#[ctor::dtor]
fn dtor() {
    // TODO: implement our own IPC that doesn't rely on thread locals so that this won't panic
    deinit(false);
}

#[cfg(target_os = "windows")]
// +whole-archive is necessary to keep the exported proxy functions
#[cfg_attr(target_env = "msvc", link(name = "dll_proxy_msvc", kind = "static", modifiers = "+whole-archive"))]
#[cfg_attr(target_env = "gnu", link(name = "dll_proxy_gnu", kind = "static", modifiers = "+whole-archive"))]
unsafe extern "C" {
    fn loadProxy(module: HMODULE);
}

fn init() {
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

    #[cfg(not(target_os = "windows"))]
    {
        extern "C" fn deinit_c() {
            // TODO: see note in dtor
            deinit(false);
        }

        unsafe { atexit(deinit_c) };
        unsafe { at_quick_exit(deinit_c) };
    }

    std::fs::write(
        "manderrow-agent-args.txt",
        format!("{:?}", std::env::args_os().collect::<Vec<_>>()),
    )
    .unwrap();

    let MaybeArgs::Enabled(mut args) = init::init(std::env::args_os()).unwrap() else {
        return;
    };

    let log = slog_scope::logger();

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
    //                 rdr.read_until(b'\n', &mut buf)?;
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

    if let Some(ipc) = ipc() {
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
}

fn deinit(send_exit: bool) {
    DEINIT.call_once(|| {
        if send_exit {
            if let Some(ipc) = ipc() {
                // TODO: send the correct exit code
                _ = ipc.send(C2SMessage::Exit { code: None });
            }
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
            Instruction::PrependArg { arg: _ } => {
                todo!()
            }
            Instruction::AppendArg { arg: _ } => {
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
