#![feature(exit_status_error)]

use std::env::{var, var_os};
use std::path::PathBuf;
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
        root_dir.join("agent/build.zig")
    );
    println!(
        "cargo::rerun-if-changed={:?}",
        root_dir.join("agent/build.zig.zon")
    );
    println!("cargo::rerun-if-changed={:?}", root_dir.join("agent/src"));
    // if the output file changes, re-run to make sure we use the right version
    println!("cargo::rerun-if-changed={:?}", root_dir.join("agent/zig-out/libmanderrow_agent"));
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

    build_agent(&agent_dir, env, false);
    if os == "linux" {
        build_agent(&agent_dir, env, true);
    }

    let mut zig_out_dir = agent_dir;
    zig_out_dir.push("zig-out");

    let to_path = zig_out_dir.join("libmanderrow_agent");

    let mut from_path = zig_out_dir;
    from_path.push("lib");
    from_path.push(match os {
        "linux" => "libmanderrow_agent.so",
        "darwin" => "libmanderrow_agent.dylib",
        "windows" => "manderrow_agent.dll",
        _ => panic!("Unsupported target: {:?}", target),
    });
    std::fs::copy(from_path, to_path).unwrap();

    tauri_build::build()
}

fn build_agent(
    agent_dir: &PathBuf,
    Env {
        arch,
        os,
        abi,
        debug,
        ..
    }: Env,
    proton: bool,
) {
    let mut command = Command::new("zig");
    command.current_dir(agent_dir);
    command.args([
        "build",
        &format!("-Doptimize={}", if debug { "Debug" } else { "ReleaseSafe" }),
    ]);

    command.arg(format!(
        "-Dtarget={arch}-{}{}{}",
        match (os, proton) {
            (_, true) => "windows",
            ("darwin", _) => "macos",
            _ => os,
        },
        match (abi, proton) {
            (_, true) | (Some(_), false) => "-",
            (None, false) => "",
        },
        match (os, proton) {
            (_, true) | ("windows", _) => "gnu",
            _ => abi.unwrap_or(""),
        }
    ));
    command.status().unwrap().exit_ok().unwrap();
}
