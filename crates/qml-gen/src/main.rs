//! Generates what qmlls and qmllint read about the QML, without building the app, which compiles
//! all of the C++ that cxx-qt and qmlcachegen generate: `src/qml/core/Strings.qml` from the
//! translations, and the QML module that the build exports to `target/cxxqt/qml_modules`, with the
//! types that the bridges in `src/qt` define. It runs the same code generation and Qt tools on the
//! bridges as cxx-qt-build does, so that the module comes out as the build exports it.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use anyhow::{Context as _, Result, bail};
use cxx_qt_gen::{
    CppFragment, CxxQtItem, GeneratedCppBlocks, GeneratedOpt, GeneratedRustBlocks, Parser,
    parse_qt_file, self_inlining::qualify_self_types, write_cpp, write_rust,
};
use manderrow_qml_gen::{
    MODULE, Platform, QmlFile, STRINGS, bridges, generate_strings, point_qmltypes_at_bridges,
    qml_files, write_if_changed, write_qmlls_qrc,
};
use qt_build_utils::{QmlDirBuilder, QmlUri};
use quote::ToTokens as _;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap();
    let target_dir = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target"));
    // where cxx-qt-build exports the module
    let modules_dir = std::env::var_os("CXX_QT_EXPORT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| target_dir.join("cxxqt"))
        .join("qml_modules");

    let mut changed = Vec::new();
    if generate_strings(root)? {
        changed.push(STRINGS.to_owned());
    }
    // what qmlls and qmllint read of the module, as they read the QML from the sources
    let module_files =
        ["qmldir", "plugin.qmltypes"].map(|file| modules_dir.join(MODULE).join(file));
    let before = module_files.each_ref().map(|file| std::fs::read(file).ok());
    generate_module(root, &modules_dir, &target_dir.join("qml-gen"))?;
    for (file, before) in module_files.iter().zip(before) {
        if std::fs::read(file).ok() != before {
            changed.push(
                file.strip_prefix(root)
                    .unwrap_or(file)
                    .display()
                    .to_string(),
            );
        }
    }

    if changed.is_empty() {
        println!("The QML's generated files are up to date.");
    } else {
        for file in changed {
            println!("Updated {file}");
        }
        // it doesn't reload them
        println!("Run Qt: Restart QML language server in VS Code to see the changes.");
    }
    Ok(())
}

/// Generates the app's QML module into `modules_dir`, as cxx-qt-build does for a QmlModule, but for
/// the C++ it would compile: the type information that qmltyperegistrar generates from the
/// bridges' QObjects, the qmldir, and the copies of the QML files.
fn generate_module(root: &Path, modules_dir: &Path, work_dir: &Path) -> Result<()> {
    let qt = Qt::find()?;
    let include_dir = work_dir.join("include");
    let moc_dir = work_dir.join("moc");
    std::fs::create_dir_all(&moc_dir)?;

    let bridges = bridges(Platform::host());
    let mut metatypes = Vec::new();
    for bridge in &bridges {
        if let Some(header) = generate_bridge_headers(root, bridge, &include_dir)? {
            metatypes.push(qt.moc(&header, &include_dir, Some(MODULE), &moc_dir)?);
        }
    }
    // cxx-qt's types for Rust's integers
    let builtins = work_dir.join("builtins.h");
    write_if_changed(&builtins, include_str!("cxx-qt-builtins.h"))?;
    metatypes.push(qt.moc(&builtins, &include_dir, None, &moc_dir)?);

    let module_dir = modules_dir.join(MODULE);
    std::fs::create_dir_all(&module_dir)?;
    let mut qmltyperegistrar = qt.tool("qmltyperegistrar");
    qmltyperegistrar
        .arg("--generate-qmltypes")
        .arg(module_dir.join("plugin.qmltypes"))
        .args([
            "--major-version",
            "1",
            "--minor-version",
            "0",
            "--import-name",
            MODULE,
            "-o",
        ])
        .arg(work_dir.join(format!("{MODULE}_qmltyperegistration.cpp")))
        .args(
            metatypes
                .iter()
                .filter(|json| std::fs::metadata(json).is_ok_and(|m| m.len() > 0)),
        );
    run_tool(qmltyperegistrar)?;
    point_qmltypes_at_bridges(root, modules_dir, &bridges)?;

    let qml_files = qml_files(root)?
        .into_iter()
        .map(|file| file.version(1, 0))
        .collect::<Vec<_>>();
    // only the files whose names start with a capital are QML types
    let qml_types = qml_files.iter().filter(|file| {
        let name = file.get_path().file_name().and_then(|name| name.to_str());
        name.is_some_and(|name| name.starts_with(char::is_uppercase))
    });
    let mut qmldir = Vec::new();
    QmlDirBuilder::new(QmlUri::from(MODULE))
        .plugin(MODULE, true)
        .class_name(format!("{MODULE}_plugin"))
        .type_info("plugin.qmltypes")
        .qml_files(qml_types.cloned())
        .write(&mut qmldir)?;
    write_if_changed(&module_dir.join("qmldir"), qmldir)?;
    copy_qml_files(root, &module_dir, &qml_files)?;
    write_qmlls_qrc(root, modules_dir, &qml_files)?;
    Ok(())
}

/// Generates the C++ headers of a bridge into `include_dir` as cxx-qt-build does: its QObjects'
/// header, which moc reads, and the header of the CXX bridge that it includes, which includes the
/// headers for the QML registration macros. Returns the QObjects' header, if it declares any.
fn generate_bridge_headers(
    root: &Path,
    bridge: &str,
    include_dir: &Path,
) -> Result<Option<PathBuf>> {
    let path = root.join(bridge);
    let file = parse_qt_file(&path).with_context(|| format!("Unable to parse {bridge}"))?;
    // like cxx-qt-build, which names them after the crate and the bridge's path
    let ident = format!("manderrow/{}", bridge.strip_suffix(".rs").unwrap());

    let mut qobjects = None;
    let mut tokens = proc_macro2::TokenStream::new();
    for attr in &file.attrs {
        tokens.extend(attr.into_token_stream());
    }
    for item in &file.items {
        match item {
            CxxQtItem::Cxx(module) => tokens.extend(module.into_token_stream()),
            CxxQtItem::CxxQt(module) => {
                let generated = (|| {
                    let mut parser = Parser::from(*module.clone())?;
                    qualify_self_types(&mut parser)?;
                    let cpp = GeneratedCppBlocks::from(&parser, &GeneratedOpt::default())?;
                    let rust = GeneratedRustBlocks::from(&parser)?;
                    cxx_qt_gen::Result::Ok((
                        write_cpp(&cpp, &ident),
                        write_rust(&rust, Some(&ident)),
                    ))
                })();
                let (cpp, rust) = generated.with_context(|| format!("Invalid bridge {bridge}"))?;
                qobjects = Some(match cpp {
                    CppFragment::Pair { header, .. } | CppFragment::Header(header) => header,
                    CppFragment::Source(_) => bail!("cxx-qt generated no header for {bridge}"),
                });
                tokens.extend(rust);
            }
            CxxQtItem::Item(item) => tokens.extend(item.into_token_stream()),
        }
    }
    let cxx = cxx_gen::generate_header_and_cc(tokens, &cxx_gen::Opt::default())
        .with_context(|| format!("Invalid bridge {bridge}"))?;

    let header = include_dir.join(format!("{ident}.cxx.h"));
    std::fs::create_dir_all(header.parent().unwrap())?;
    write_if_changed(&header, cxx.header)?;
    let Some(qobjects) = qobjects else {
        return Ok(None);
    };
    let header = include_dir.join(format!("{ident}.cxxqt.h"));
    write_if_changed(&header, qobjects)?;
    Ok(Some(header))
}

/// Copies the QML files into the module, as cxx-qt-build does, for the paths in the qmldir.
fn copy_qml_files(root: &Path, module_dir: &Path, qml_files: &[QmlFile]) -> Result<()> {
    for file in qml_files {
        let copy = module_dir.join(file.get_path());
        std::fs::create_dir_all(copy.parent().unwrap())?;
        write_if_changed(&copy, std::fs::read(root.join(file.get_path()))?)?;
    }
    Ok(())
}

/// The Qt that the app builds with, which `QMAKE` or the qmake on the path is the qmake of.
struct Qt {
    headers: PathBuf,
    libexecs: PathBuf,
}

impl Qt {
    fn find() -> Result<Self> {
        let qmake = match std::env::var_os("QMAKE") {
            Some(qmake) => vec![PathBuf::from(qmake)],
            None => vec!["qmake6".into(), "qmake".into()],
        };
        let query = |qmake: &Path, key: &str| -> Option<PathBuf> {
            let output = Command::new(qmake).args(["-query", key]).output().ok()?;
            let value = String::from_utf8(output.stdout).ok()?;
            output.status.success().then(|| PathBuf::from(value.trim()))
        };
        for qmake in qmake {
            if let (Some(headers), Some(libexecs)) = (
                query(&qmake, "QT_INSTALL_HEADERS"),
                query(&qmake, "QT_HOST_LIBEXECS"),
            ) {
                return Ok(Self { headers, libexecs });
            }
        }
        bail!("Unable to locate Qt. Set the QMAKE environment variable to the path of qmake.")
    }

    fn tool(&self, name: &str) -> Command {
        let mut command = Command::new(self.libexecs.join(name));
        // as cxx-qt-build runs them, so that another Qt's libraries on LD_LIBRARY_PATH can't break
        // them
        command.env_clear();
        command
    }

    /// Runs moc on a header, with the include paths that cxx-qt-build gives it, and returns the
    /// path of the JSON of the metatypes it found, which qmltyperegistrar reads.
    fn moc(
        &self,
        header: &Path,
        include_dir: &Path,
        uri: Option<&str>,
        out_dir: &Path,
    ) -> Result<PathBuf> {
        let mut file_name = header.file_name().unwrap().to_owned();
        file_name.push(".cpp");
        let output = out_dir.join(format!("moc_{}", file_name.to_str().unwrap()));
        let mut moc = self.tool("moc");
        if let Some(uri) = uri {
            moc.arg(format!("-Muri={uri}"));
        }
        moc.arg(format!("-I{}", self.headers.display()));
        for module in ["QtCore", "QtGui", "QtQml", "QtQuick", "QtQuickControls2"] {
            moc.arg(format!("-I{}", self.headers.join(module).display()));
        }
        moc.arg(format!("-I{}", include_dir.display()))
            .arg(header)
            .arg("-o")
            .arg(&output)
            .arg("--output-json");
        run_tool(moc)?;
        let mut json = output.into_os_string();
        json.push(".json");
        Ok(json.into())
    }
}

fn run_tool(mut command: Command) -> Result<()> {
    let program = command.get_program().to_owned();
    let output = command
        .output()
        .with_context(|| format!("Unable to run {}", program.display()))?;
    if !output.status.success() {
        bail!(
            "{} failed:\n{}",
            program.display(),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}
