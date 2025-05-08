#![deny(unused_must_use)]

use std::ffi::OsString;

pub const ARG_START_DELIMITER: &str = "{manderrow";
pub const ARG_END_DELIMITER: &str = "manderrow}";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Found unbalanced argument delimiters")]
    UnbalancedArgumentDelimiters,
}

pub fn extract(
    args: impl IntoIterator<Item = OsString>,
) -> Result<(Vec<OsString>, Vec<OsString>), Error> {
    let mut buf = Vec::new();
    let mut remaining = Vec::new();

    let mut capturing = false;
    for arg in args {
        if arg == ARG_START_DELIMITER {
            if capturing {
                return Err(Error::UnbalancedArgumentDelimiters);
            }
            capturing = true;
        } else if arg == ARG_END_DELIMITER {
            if !capturing {
                return Err(Error::UnbalancedArgumentDelimiters);
            }
            capturing = false;
        } else {
            if capturing {
                buf.push(arg);
            } else {
                remaining.push(arg);
            }
        }
    }

    if capturing {
        return Err(Error::UnbalancedArgumentDelimiters);
    }

    Ok((buf, remaining))
}
