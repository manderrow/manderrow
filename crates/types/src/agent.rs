use std::ffi::OsString;
use std::path::PathBuf;

/// An injection instruction.
pub enum Instruction {
    LoadLibrary {
        path: PathBuf,
    },
    SetVar {
        kv: OsString,
        eq_sign: usize,
    },
    PrependArg {
        arg: OsString,
    },
    AppendArg {
        arg: OsString,
    },
}
