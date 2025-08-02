pub mod commands;

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use indexmap::IndexMap;
use smol_str::SmolStr;
use uuid::Uuid;

use crate::profiles::{profile_path, CONFIG_FOLDER};

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Patch {
    /// The path to the key the patch applies to.
    pub path: Vec<String>,
    pub change: Change,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
#[serde(untagged)]
pub enum PathComponent {
    Key(SmolStr),
    Index(usize),
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Change {
    /// Sets the referenced key-value pair or array element, or inserts a new key-value pair if necessary.
    Set(Value),
    /// Appends a value to the referenced array.
    Append(Value),
    /// Removes the referenced key-value pair or array element.
    Remove,
}

/// Returns the config root path and the full path to each config file.
pub async fn scan_configs(profile: Uuid) -> Result<(PathBuf, Vec<PathBuf>)> {
    let mut configs_path = profile_path(profile);
    configs_path.push(CONFIG_FOLDER);
    tokio::task::spawn_blocking(move || {
        let mut iter = walkdir::WalkDir::new(&configs_path).into_iter();
        match iter.next().expect("root entry") {
            Ok(e) => {}
            Err(e) => match e.io_error() {
                Some(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                _ => return Err(e).context(format!("Failed to walk {:?}", configs_path)),
            },
        }
        let paths = iter
            .filter_map(|r| match r {
                Ok(e) if !e.file_type().is_dir() => Some(Ok(e.into_path())),
                Ok(_) => None,
                Err(e) => Some(Err(e).context(format!("Failed to walk {:?}", configs_path))),
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok((configs_path, paths))
    })
    .await?
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DocumentSection {
    title: String,
    id: String,
    children: Vec<DocumentSection>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type")]
pub enum File {
    Config {
        sections: Vec<Section>,
    },
    Document {
        html: String,
        sections: Vec<DocumentSection>,
    },
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Section {
    path: Vec<PathComponent>,
    value: Value,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(untagged)]
pub enum Value {
    Null,
    Bool(bool),
    Number(serde_json::Number),
    String(String),
    Array(Vec<Value>),
    Object(IndexMap<String, Value>),
}

pub fn build_config_path(profile: Uuid, path: &Path) -> PathBuf {
    let mut buf = profile_path(profile);
    buf.push(CONFIG_FOLDER);
    buf.push(path);
    buf
}

pub async fn read_config(profile: Uuid, path: &Path) -> Result<File> {
    read_config_at(&build_config_path(profile, path)).await
}

async fn read_config_at(path: &Path) -> Result<File> {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some(s) if s.eq_ignore_ascii_case("txt") => {
            let content = tokio::fs::read_to_string(&path).await?;
            Ok(File::Document {
                html: ammonia::clean_text(&content),
                sections: Vec::new(),
            })
        }
        Some(s) if s.eq_ignore_ascii_case("md") => {
            let content = tokio::fs::read_to_string(&path).await?;
            // let mut stack = Vec::new();
            // let mut current_sections = Vec::new();
            // let mut current_section = None::<DocumentSection>;
            let html = crate::util::markdown::render(&content, |event| {
                // match event {
                //     pulldown_cmark::Event::Start(pulldown_cmark::Tag::Heading { level, id, .. }) => {
                //         if let Some(section) = current_section {
                //             stack.push(current_section);
                //         }
                //         current_section = Some(DocumentSection { title: String::new(), id: id, children: () });
                //     }
                // }
                event
            })?;
            Ok(File::Document {
                html,
                sections: Vec::new(),
            })
        }
        Some(s) if s.eq_ignore_ascii_case("json") => {
            let content = tokio::fs::read_to_string(&path).await?;
            let content = serde_json::from_str::<IndexMap<String, Value>>(&content)?;
            Ok(File::Config {
                sections: content
                    .into_iter()
                    .map(|(k, v)| Section {
                        path: vec![PathComponent::Key(k.into())],
                        value: v,
                    })
                    .collect(),
            })
        }
        Some(s) if s.eq_ignore_ascii_case("cfg") || s.eq_ignore_ascii_case("ini") => {
            let content = tokio::fs::read_to_string(&path).await?;
            let content = ini::Ini::load_from_str(&content)?;
            Ok(File::Config {
                sections: content
                    .into_iter()
                    .map(|(k, p)| Section {
                        path: k
                            .map(|k| PathComponent::Key(k.into()))
                            .into_iter()
                            .collect::<Vec<_>>(),
                        value: Value::Object(
                            p.into_iter()
                                .map(|(k, v)| (k.to_owned(), Value::String(v.to_owned())))
                                .collect(),
                        ),
                    })
                    .collect(),
            })
        }
        format => bail!("Unsupported config format {:?}", format),
    }
}

/// Updates and returns the config.
pub async fn update_config(profile: Uuid, path: &Path, patches: &[Patch]) -> Result<File> {
    let path = build_config_path(profile, path);
    match path.extension() {
        _ => bail!("Unsupported config format"),
    }
    read_config_at(&path).await
}
