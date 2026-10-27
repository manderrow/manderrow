//! The QML that's generated from the rest of the app: `src/qml/core/Strings.qml` from the
//! translations, and the type information of the QML module that cxx-qt exports for qmlls and
//! qmllint. Both the build script and the `qml-gen` tool generate them, the tool without building
//! the app.

use std::path::{Path, PathBuf};

pub use qt_build_utils::QmlFile;

mod qmlls;
mod strings;

pub use qmlls::{point_qmltypes_at_bridges, write_qmlls_qrc};
pub use strings::generate_strings;

/// The URI of the app's QML module.
pub const MODULE: &str = "Manderrow";

/// The translations that `Strings.qml` is generated from, relative to the root of the repository.
pub const TRANSLATIONS: &str = "src/locales/en-CA.json";

/// The generated singleton of the translations, relative to the root of the repository.
pub const STRINGS: &str = "src/qml/core/Strings.qml";

/// The platform that the app is built for, which decides which of Native's bridges it has.
#[derive(Clone, Copy)]
pub enum Platform {
    Windows,
    Apple,
    Unix,
    Other,
}

impl Platform {
    /// The platform that this is running on.
    pub fn host() -> Self {
        if cfg!(windows) {
            Self::Windows
        } else if cfg!(target_vendor = "apple") {
            Self::Apple
        } else if cfg!(unix) {
            Self::Unix
        } else {
            Self::Other
        }
    }
}

/// The files that bridge Rust to QML, relative to the root of the repository.
pub fn bridges(platform: Platform) -> Vec<&'static str> {
    let mut bridges = vec![
        "src/qt/app.rs",
        "src/qt/call.rs",
        "src/qt/console.rs",
        "src/qt/games.rs",
        "src/qt/markdown.rs",
        "src/qt/mod_index.rs",
        "src/qt/modpacks.rs",
        "src/qt/mods.rs",
        "src/qt/native.rs",
        "src/qt/profiles.rs",
        "src/qt/rich_text.rs",
        "src/qt/settings.rs",
        "src/qt/setup.rs",
        "src/qt/tasks.rs",
        "src/qt/web_images.rs",
    ];
    // Native's platform services, which only build for their platforms
    match platform {
        Platform::Windows => bridges.push("src/qt/native_win32.rs"),
        Platform::Apple => bridges.push("src/qt/native_macos.rs"),
        Platform::Unix => bridges.push("src/qt/native_unix.rs"),
        Platform::Other => {}
    }
    bridges
}

/// The QML files of the app's QML module, in order, relative to the root of the repository.
pub fn qml_files(root: &Path) -> anyhow::Result<Vec<QmlFile>> {
    let mut paths = Vec::new();
    collect_qml_files(&root.join("src").join("qml"), &mut paths)?;
    paths.sort();
    paths
        .iter()
        .map(|path| {
            let source = std::fs::read_to_string(path)?;
            Ok(QmlFile::from(path.strip_prefix(root)?)
                .singleton(source.lines().any(|line| line.trim() == "pragma Singleton")))
        })
        .collect()
}

fn collect_qml_files(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_qml_files(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "qml") {
            out.push(path);
        }
    }
    Ok(())
}

/// Writes a file, unless it already has the contents, so that its modification time only changes
/// when it does. Returns whether it changed.
pub fn write_if_changed(path: &Path, contents: impl AsRef<[u8]>) -> std::io::Result<bool> {
    let contents = contents.as_ref();
    if std::fs::read(path).ok().as_deref() == Some(contents) {
        return Ok(false);
    }
    std::fs::write(path, contents)?;
    Ok(true)
}
