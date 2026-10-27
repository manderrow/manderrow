//! The QML that's generated from the rest of the app: `src/qml/core/Strings.qml` from the
//! translations. Both the build script and the `qml-gen` tool generate it, the tool without
//! building the app.

use std::path::Path;

mod strings;

pub use strings::generate_strings;

/// The translations that `Strings.qml` is generated from, relative to the root of the repository.
pub const TRANSLATIONS: &str = "src/locales/en-CA.json";

/// The generated singleton of the translations, relative to the root of the repository.
pub const STRINGS: &str = "src/qml/core/Strings.qml";

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
