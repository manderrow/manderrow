//! Generates `src/qml/core/Strings.qml` from the translations, as the build does, but without
//! building the app.

use std::path::Path;
use std::process::ExitCode;

use anyhow::Result;
use manderrow_qml_gen::{STRINGS, generate_strings};

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

    if generate_strings(root)? {
        println!("Updated {STRINGS}");
    } else {
        println!("The QML's generated files are up to date.");
    }
    Ok(())
}
