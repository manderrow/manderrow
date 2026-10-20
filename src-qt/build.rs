#![feature(exit_status_error)]

use std::env::{var, var_os};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use cxx_qt_build::{CxxQtBuilder, QmlFile, QmlModule};

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

    let (native_out_dir, _host_out_dir) = std::thread::scope(|scope| {
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
        // This is kinda weird. The app needs to find it at runtime, so can't use anything under OUT_DIR (based on profile).
        &to_path,
    );

    let crate_dir = PathBuf::from(var_os("CARGO_MANIFEST_DIR").unwrap());
    build_qt(&crate_dir);
    compile_assets(&crate_dir, &out_dir);
}

fn build_qt(crate_dir: &Path) {
    println!("cargo::rerun-if-changed={}", crate_dir.join("qml").display());
    let mut qml_files = Vec::new();
    collect_files(&crate_dir.join("qml"), "qml", &mut qml_files);
    qml_files.sort();

    let module = QmlModule::new("Manderrow").qml_files(qml_files.iter().map(|path| {
        let source = std::fs::read_to_string(path).unwrap();
        QmlFile::from(path.strip_prefix(crate_dir).unwrap())
            .singleton(source.lines().any(|line| line.trim() == "pragma Singleton"))
    }));

    CxxQtBuilder::new_qml_module(module)
        .qt_module("Gui")
        .qt_module("Quick")
        .qt_module("QuickControls2")
        .files(["src/qt/bridge.rs"])
        .cpp_files(["cpp/native.h", "cpp/native.cpp"])
        .build();
}

/// Compiles the static assets into a binary Qt resource bundle, which is embedded in the
/// executable and registered at startup. This is much faster to build than having rcc generate
/// C++ source for 25MB of images.
fn compile_assets(crate_dir: &Path, out_dir: &Path) {
    let assets_dir = crate_dir.join("assets");
    println!("cargo::rerun-if-changed={}", assets_dir.display());
    let mut files = Vec::new();
    collect_files(&assets_dir, "", &mut files);
    files.sort();

    let mut qrc = String::from("<RCC>\n  <qresource prefix=\"/assets\">\n");
    for file in &files {
        let alias = file.strip_prefix(&assets_dir).unwrap().to_str().unwrap();
        writeln!(
            qrc,
            "    <file alias=\"{}\">{}</file>",
            alias.replace('\\', "/"),
            file.display()
        )
        .unwrap();
    }
    for shader in compile_shaders(crate_dir, out_dir) {
        writeln!(
            qrc,
            "    <file alias=\"shaders/{}\">{}</file>",
            shader.file_name().unwrap().to_str().unwrap(),
            shader.display()
        )
        .unwrap();
    }
    let icon = crate_dir.join("icons/128x128@2x.png");
    writeln!(qrc, "    <file alias=\"icon.png\">{}</file>", icon.display()).unwrap();
    qrc.push_str("  </qresource>\n</RCC>\n");

    let qrc_path = out_dir.join("assets.qrc");
    std::fs::write(&qrc_path, qrc).unwrap();

    let mut command = Command::new(qt_dir("QT_HOST_LIBEXECS").join("rcc"));
    command
        .arg("--binary")
        .arg("--no-compress")
        .arg("-o")
        .arg(out_dir.join("assets.rcc"))
        .arg(&qrc_path);
    command.status().unwrap().exit_ok().unwrap();
}

/// Compiles the shaders for every graphics API Qt Quick may render with, as qt_add_shaders does.
fn compile_shaders(crate_dir: &Path, out_dir: &Path) -> Vec<PathBuf> {
    let shaders_dir = crate_dir.join("shaders");
    println!("cargo::rerun-if-changed={}", shaders_dir.display());
    let mut shaders = Vec::new();
    collect_files(&shaders_dir, "frag", &mut shaders);
    shaders.sort();

    let qsb = qt_dir("QT_HOST_BINS").join("qsb");
    let compiled_dir = out_dir.join("shaders");
    std::fs::create_dir_all(&compiled_dir).unwrap();
    shaders
        .iter()
        .map(|shader| {
            let mut compiled = compiled_dir.join(shader.file_name().unwrap());
            compiled.as_mut_os_string().push(".qsb");
            Command::new(&qsb)
                .args(["--glsl", "100 es,120,150", "--hlsl", "50", "--msl", "12", "-o"])
                .arg(&compiled)
                .arg(shader)
                .status()
                .unwrap()
                .exit_ok()
                .unwrap();
            compiled
        })
        .collect()
}

fn qt_dir(key: &str) -> PathBuf {
    println!("cargo::rerun-if-env-changed=QMAKE");
    let qmake = match var_os("QMAKE") {
        Some(qmake) => vec![PathBuf::from(qmake)],
        None => vec!["qmake6".into(), "qmake".into()],
    };
    for qmake in qmake {
        if let Ok(output) = Command::new(&qmake)
            .args(["-query", key])
            .output()
        {
            if output.status.success() {
                return PathBuf::from(String::from_utf8(output.stdout).unwrap().trim());
            }
        }
    }
    panic!("Unable to locate Qt. Set the QMAKE environment variable to the path of qmake.");
}

/// Recursively collects the files under `dir` with the given extension, or all files if the
/// extension is empty.
fn collect_files(dir: &Path, extension: &str, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_files(&path, extension, out);
        } else if extension.is_empty() || path.extension().is_some_and(|ext| ext == extension) {
            out.push(path);
        }
    }
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
