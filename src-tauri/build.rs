#![feature(exit_status_error)]

use std::env::{var, var_os};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Copy)]
struct Env<'a> {
    arch: &'a str,
    os: &'a str,
    abi: Option<&'a str>,
    debug: bool,
}

fn main() {
    let mut root_dir = PathBuf::from(var_os("CARGO_MANIFEST_DIR").unwrap());
    assert!(root_dir.pop());
    let crates_dir = root_dir.join("crates");

    println!(
        "cargo::rerun-if-changed={:?}",
        crates_dir.join("Cargo.toml")
    );
    println!(
        "cargo::rerun-if-changed={:?}",
        crates_dir.join("Cargo.lock")
    );
    println!(
        "cargo::rerun-if-changed={:?}",
        crates_dir.join("agent/Cargo.toml")
    );
    println!("cargo::rerun-if-changed={:?}", crates_dir.join("agent/src"));
    println!(
        "cargo::rerun-if-changed={:?}",
        crates_dir.join("ipc/Cargo.toml")
    );
    println!("cargo::rerun-if-changed={:?}", crates_dir.join("ipc/src"));

    let target = var("TARGET").unwrap();
    let (arch, rem) = target.split_once('-').unwrap();
    let (_device, rem) = rem.split_once('-').unwrap();
    let (os, abi) = match rem.split_once('-') {
        Some((os, abi)) => (os, Some(abi)),
        None => (rem, None),
    };
    let env = Env {
        debug: var("DEBUG").unwrap().parse().expect("Invalid debug value"),
        arch,
        os,
        abi,
    };

    let agent_dir = root_dir.join("agent");

    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());

    let (native_out_dir, host_out_dir) = std::thread::scope(|scope| {
        let native_out_dir = scope.spawn(|| build_agent(&agent_dir, &out_dir, env, false, false));

        let host_out_dir = if os == "linux" {
            scope.spawn(|| build_agent(&agent_dir, &out_dir, env, true, false));
            Some(scope.spawn(|| build_agent(&agent_dir, &out_dir, env, false, true)))
        } else {
            None
        };

        (
            native_out_dir.join().unwrap(),
            host_out_dir.map(|h| h.join().unwrap()),
        )
    });

    let mut to_path = agent_dir;
    to_path.push("zig-out");
    std::fs::create_dir_all(&to_path).unwrap();
    to_path.push("libmanderrow_agent");
    copy(
        &native_out_dir.join("lib").join(match env.os {
            "linux" => "libmanderrow_agent.so",
            "darwin" => "libmanderrow_agent.dylib",
            "windows" => "manderrow_agent.dll",
            os => panic!("Unsupported OS: {os:?}"),
        }),
        // This is kinda weird. We need Tauri to have access to it, so can't use anything under OUT_DIR (based on profile).
        &to_path,
    );

    tauri_build::build()
}

fn copy(from: &Path, to: &Path) {
    match std::fs::copy(from, to) {
        Ok(_) => {}
        Err(e) => panic!("Failed to copy from {from:?} to {to:?}: {e}"),
    }
}

fn build_agent(
    agent_dir: &Path,
    out_dir: &Path,
    env: Env,
    proton: bool,
    host_lib: bool,
) -> PathBuf {
    assert!(!proton || !host_lib);
    let mut out_dir = out_dir.join("agent");
    if proton {
        out_dir.as_mut_os_string().push("-proton");
    }
    if host_lib {
        out_dir.as_mut_os_string().push("-host_lib");
    }
    zig_build(
        agent_dir,
        &out_dir,
        Env {
            os: if proton { "windows" } else { env.os },
            abi: if proton { None } else { env.abi },
            ..env
        },
        if proton {
            &["-Dipc-mode=winelib"]
        } else if host_lib {
            &["-Dhost-lib=true"]
        } else {
            &[]
        },
    )
}

fn zig_build(
    dir: &Path,
    out_dir: &Path,
    Env {
        arch,
        os,
        abi,
        debug,
        ..
    }: Env,
    args: &[&str],
) -> PathBuf {
    println!("cargo::rerun-if-changed={:?}", dir.join("build.zig"));
    println!("cargo::rerun-if-changed={:?}", dir.join("build.zig.zon"));
    println!("cargo::rerun-if-changed={:?}", dir.join("src"));

    let mut command = Command::new("zig");
    command.current_dir(dir);
    command.arg("build");

    command.arg("--cache-dir");
    command.arg(out_dir.join("cache"));

    let out_dir = out_dir.join("out");
    command.arg("-p");
    command.arg(&out_dir);

    if !debug {
        command.arg("-Drelease");
    }

    command.arg(format!(
        "-Dtarget={arch}-{}{}{}",
        match os {
            "darwin" => "macos",
            _ => os,
        },
        match (abi, os) {
            (Some(_), _) | (_, "windows") => "-",
            (None, _) => "",
        },
        match os {
            "windows" => "gnu",
            _ => abi.unwrap_or(""),
        }
    ));
    command.args(args);
    command.status().unwrap().exit_ok().unwrap();

    out_dir
}
