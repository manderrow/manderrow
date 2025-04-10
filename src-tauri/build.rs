#![feature(exit_status_error)]

use std::env::{var, var_os};
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let mut crates_dir = PathBuf::from(var_os("CARGO_MANIFEST_DIR").unwrap());
    assert!(crates_dir.pop());
    crates_dir.push("crates");

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

    match os {
        "windows" => {
            build_agent(&crates_dir, false);
        }
        "linux" => {
            build_agent(&crates_dir, false);
            build_agent(&crates_dir, true);
        }
        "darwin" => {
            build_agent(&crates_dir, false);
        }
        _ => panic!("Unsupported target triple: {:?}", native_target),
    }

    let mut target_dir = crates_dir;
    target_dir.push("target");
    let mut to_path = target_dir.join("release");
    to_path.push(format!(
        "libmanderrow_agent.dynamic_library-{}{}",
        native_target,
        if os == "windows" { ".exe" } else { "" }
    ));

    let mut from_path = target_dir.clone();
    from_path.push("release");
    from_path.push(match os {
        "linux" => "libmanderrow_agent.so",
        "darwin" => "libmanderrow_agent.dylib",
        "windows" => "manderrow_agent.dll",
        _ => panic!("Unsupported target triple: {:?}", native_target),
    });
    std::fs::copy(from_path, to_path).unwrap();

    tauri_build::build()
}

fn build_agent(crates_dir: &PathBuf, proton: bool) {
    let mut command = Command::new(var_os("CARGO").unwrap());
    command.args([
        "build",
        "--package",
        "manderrow-agent",
        "--release",
        "--manifest-path",
    ]);

    command.arg(crates_dir.join("Cargo.toml"));

    if proton {
        command.args(["--target", "x86_64-pc-windows-gnu"]);
    }
    command.status().unwrap().exit_ok().unwrap();
}
