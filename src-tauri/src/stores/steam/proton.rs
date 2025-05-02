use std::ops::Range;

use anyhow::{bail, Result};
use slog::{debug, trace};

use super::paths::{resolve_app_install_directory, resolve_steam_app_compat_data_directory};

/// The `game_id` is Steam's numerical id for the game.
pub async fn uses_proton(log: &slog::Logger, game_id: &str) -> Result<bool> {
    if cfg!(target_os = "linux") {
        let install_dir = resolve_app_install_directory(log, game_id).await?;
        let mut iter = tokio::fs::read_dir(&install_dir).await?;
        while let Some(e) = iter.next_entry().await? {
            let name = e.file_name();
            if name.as_encoded_bytes().ends_with(b".exe") {
                debug!(
                    log,
                    "Guessing that {game_id:?} uses proton because it has file {name:?}"
                );
                return Ok(true);
            }
        }
        Ok(false)
    } else {
        Ok(false)
    }
}

pub async fn ensure_wine_will_load_dll_override(
    log: &slog::Logger,
    game_id: &str,
    proxy_dll: &str,
) -> Result<()> {
    let compat_data_dir = resolve_steam_app_compat_data_directory(log, game_id).await?;

    let mut user_reg = compat_data_dir;
    user_reg.push("pfx");
    user_reg.push("user.reg");

    let mut user_reg_data = tokio::fs::read_to_string(&user_reg).await?;

    trace!(log, "user.reg:\n{user_reg_data}");

    if reg_add_in_section(
        &mut user_reg_data,
        "[Software\\\\Wine\\\\DllOverrides]",
        proxy_dll,
        "native,builtin",
    )? {
        trace!(log, "replacement user.reg:\n{user_reg_data}");
        let mut backup_file = user_reg.clone();
        loop {
            backup_file.add_extension("bak");
            if !tokio::fs::try_exists(&backup_file).await? {
                break;
            }
        }
        tokio::fs::copy(&user_reg, &backup_file).await?;
        tokio::fs::write(&user_reg, &user_reg_data).await?;
    }
    Ok(())
}

fn find_line_starting_with(haystack: &str, needle: &str) -> Option<Range<usize>> {
    let start = if haystack.starts_with(needle) {
        0
    } else {
        haystack
            .as_bytes()
            .windows(1 + needle.len())
            .position(|window| window[0] == b'\n' && window[1..] == *needle.as_bytes())?
            + 1
    };
    let end = match haystack[start + needle.len()..].find('\n') {
        Some(i) => start + needle.len() + i,
        None => haystack.len(),
    };
    Some(start..end)
}

fn reg_add_in_section(
    reg: &mut String,
    section_header: &str,
    key: &str,
    value: &str,
) -> Result<bool> {
    let section_start = find_line_starting_with(&reg, section_header).map(|range| range.end);
    let Some(section_start) = section_start else {
        if !reg.is_empty() && !reg.ends_with('\n') {
            reg.push('\n');
        }
        reg.push_str(section_header);
        reg.push_str("\n\"");
        reg.push_str(key);
        reg.push_str("\"=\"");
        reg.push_str(value);
        reg.push('"');
        return Ok(true);
    };

    let mut replaced = false;
    let mut found = false;

    let mut line_start = section_start + 1;
    while line_start < reg.len() {
        if reg.len() < line_start + 1 + key.len() + 4 {
            bail!("Invalid reg file");
        }

        if reg[line_start..].starts_with("[") {
            break;
        }

        let mut end_i = reg[line_start..]
            .find('\n')
            .map(|j| line_start + j)
            .unwrap_or(reg.len());

        let mut i = line_start;
        if reg[i..].starts_with('"')
            && reg[i + 1..].starts_with(key)
            && reg[i + 1 + key.len()..].starts_with("\"=\"")
        {
            i += 1 + key.len() + 3;

            found = true;
            if reg[i..end_i - 1] != *value {
                reg.replace_range(i..end_i - 1, value);
                replaced = true;
                end_i = i + value.len() + 1;
            }
        }

        line_start = end_i + 1;
    }

    if !found {
        // fallback to adding new assignment
        reg.insert_str(section_start + 1, &format!("\"{key}\"=\"{value}\"\n"));
        Ok(true)
    } else {
        Ok(replaced)
    }
}

#[cfg(test)]
mod tests {
    use super::reg_add_in_section;

    #[test]
    fn test_reg_add_in_section() {
        const SAMPLES: &[(&str, Option<&str>)] = &[
            (
                include_str!("reg_mod_samples/01-in.reg"),
                Some(include_str!("reg_mod_samples/01-out.reg")),
            ),
            (include_str!("reg_mod_samples/02-in.reg"), None),
            (include_str!("reg_mod_samples/03-in.reg"), None),
        ];
        for &(in_data, out_data) in SAMPLES {
            let mut buf = in_data.to_owned();
            assert_eq!(
                reg_add_in_section(
                    &mut buf,
                    "[Software\\\\Wine\\\\DllOverrides]",
                    "winhttp",
                    "native,builtin",
                )
                .unwrap(),
                out_data.is_some(),
                "Changed status does not match expected"
            );
            if let Some(out_data) = out_data {
                assert_eq!(
                    buf, out_data,
                    "Output should be changed, but does not match expected output"
                );
            } else {
                assert_eq!(
                    buf, in_data,
                    "Output should be unchanged, but does not match input"
                );
            }
        }
    }
}
