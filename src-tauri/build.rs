#![feature(exit_status_error)]

use std::env::{var, var_os};
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let mut root_dir = PathBuf::from(var_os("CARGO_MANIFEST_DIR").unwrap());
    assert!(root_dir.pop());
    let crates_dir = root_dir.join("crates");

    println!("cargo::rerun-if-changed={:?}", root_dir.join("agent"));
    println!("cargo::rerun-if-changed={:?}", crates_dir.join("agent"));
    println!("cargo::rerun-if-changed={:?}", crates_dir.join("args"));
    println!("cargo::rerun-if-changed={:?}", crates_dir.join("ipc"));
    println!(
        "cargo::rerun-if-changed={:?}",
        crates_dir.join("Cargo.toml")
    );
    println!(
        "cargo::rerun-if-changed={:?}",
        crates_dir.join("Cargo.lock")
    );

    let native_target = var("TARGET").unwrap();
    let (_arch, rem) = native_target.split_once('-').unwrap();
    let (_device, rem) = rem.split_once('-').unwrap();
    let (os, _abi) = match rem.split_once('-') {
        Some((os, abi)) => (os, Some(abi)),
        None => (rem, None),
    };

    let agent_dir = root_dir.join("agent");

    build_agent(&agent_dir, false);
    if os == "linux" {
        build_agent(&agent_dir, true);
    }

    let mut target_dir = agent_dir;
    target_dir.push("zig-out");
    let to_path = target_dir.join("libmanderrow_agent");

    let mut from_path = target_dir;
    from_path.push("lib");
    from_path.push(match os {
        "linux" => "libmanderrow_agent.so",
        "darwin" => "libmanderrow_agent.dylib",
        "windows" => "manderrow_agent.dll",
        _ => panic!("Unsupported target: {:?}", native_target),
    });
    std::fs::copy(from_path, to_path).unwrap();

    tauri_build::build()
}

fn build_agent(agent_dir: &PathBuf, proton: bool) {
    let mut command = Command::new("zig");
    command.current_dir(agent_dir);
    command.args(["build", "-Doptimize=ReleaseSafe"]);

    if proton {
        command.args(["-Dtarget=x86_64-windows-gnu"]);
    }
    command.status().unwrap().exit_ok().unwrap();
}
