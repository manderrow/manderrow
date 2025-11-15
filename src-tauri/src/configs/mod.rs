pub mod commands;

use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::{bail, Context, Result};
use indexmap::IndexMap;
use serde_json::Number;
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
    #[serde(flatten)]
    annotations: Annotations,
    value: Value,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct Annotations {
    docs: Option<String>,
    default_value: Option<Value>,
    type_hint: Option<TypeHint>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type")]
pub enum TypeHint {
    Bool,
    Int {
        min: Option<i64>,
        max: Option<Int>,
    },
    Float,
    Enum {
        values: Vec<Value>,
        multiple: bool,
    },
    String,
    Array {
        /// The type of items in the array.
        item: Option<Box<TypeHint>>,
    },
    Object {
        /// The types of the values of the object.
        entries: IndexMap<SmolStr, Annotations>,
    },
}

#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(untagged)]
pub enum Int {
    PosOrZero(u64),
    Neg(i64),
}

impl Int {
    pub fn from_signed<T: Into<i64>>(value: T) -> Self {
        match value.into() {
            i @ 0.. => Self::PosOrZero(i as u64),
            i => Self::Neg(i),
        }
    }

    pub fn from_unsigned<T: Into<u64>>(value: T) -> Self {
        Self::PosOrZero(value.into())
    }
}

impl FromStr for Int {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        s.parse::<u64>()
            .map(Self::PosOrZero)
            .or_else(|_| s.parse::<i64>().map(Self::Neg))
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum Value {
    Null,
    Bool(bool),
    Integer(SmolStr),
    Float(SmolStr),
    String(SmolStr),
    Array(Vec<Value>),
    Object(IndexMap<SmolStr, Value>),
}

pub fn build_config_path(profile: Uuid, path: &Path) -> PathBuf {
    let mut buf = profile_path(profile);
    buf.push(CONFIG_FOLDER);
    buf.push(path);
    buf
}

#[derive(Debug, Clone, Copy, Default, serde::Deserialize, serde::Serialize)]
pub struct ConfigOptions {
    pub special_format: Option<SpecialConfigFormat>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum SpecialConfigFormat {
    BepInEx,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
    Text,
    Markdown,
    Json,
    BepInEx,
    // TODO: decide if we really want to attempt to support these generically, seeing as the format\
    //       is not standardized.
    Ini,
}

impl Format {
    pub fn guess(path: &Path, options: ConfigOptions) -> Result<Format> {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some(s) if s.eq_ignore_ascii_case("txt") => Ok(Format::Text),
            Some(s) if s.eq_ignore_ascii_case("md") => Ok(Format::Markdown),
            Some(s) if s.eq_ignore_ascii_case("json") => Ok(Format::Json),
            Some(s)
                if options.special_format == Some(SpecialConfigFormat::BepInEx) && s.eq_ignore_ascii_case("cfg") => Ok(Format::BepInEx),
            Some(s) if s.eq_ignore_ascii_case("cfg") || s.eq_ignore_ascii_case("ini") => Ok(Format::Ini),
            format => bail!("Unsupported config format {:?}", format),
        }
    }
}

pub async fn read_config(profile: Uuid, path: &Path, options: ConfigOptions) -> Result<File> {
    read_config_at(&build_config_path(profile, path), options).await
}

async fn read_config_at(path: &Path, options: ConfigOptions) -> Result<File> {
    match Format::guess(path, options)? {
        Format::Text => {
            let content = tokio::fs::read_to_string(&path).await?;
            Ok(File::Document {
                html: ammonia::clean_text(&content),
                sections: Vec::new(),
            })
        }
        Format::Markdown => {
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
        Format::Json => {
            // TODO: lossless parser
            // FIXME: parse numbers
            let content = tokio::fs::read_to_string(&path).await?;
            let content = serde_json::from_str::<IndexMap<String, Value>>(&content)?;
            Ok(File::Config {
                sections: content
                    .into_iter()
                    .map(|(k, v)| Section {
                        path: vec![PathComponent::Key(k.into())],
                        value: v,
                        // TODO: consider building type hints from existing structure or spec if it can be detected
                        annotations: Annotations {
                            docs: None,
                            default_value: None,
                            type_hint: None,
                        },
                    })
                    .collect(),
            })
        }
        Format::BepInEx =>
        {
            tokio::task::block_in_place(|| {
                read_config_from_bepinex_cfg(std::fs::File::open(&path)?)
            })
        }
        Format::Ini => {
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
                                .map(|(k, v)| (k.into(), Value::String(v.into())))
                                .collect(),
                        ),
                        annotations: Annotations::default(),
                    })
                    .collect(),
            })
        }
    }
}

// TODO: tests
fn read_config_from_bepinex_cfg(
    rdr: impl std::io::Read,
) -> std::result::Result<File, anyhow::Error> {
    use bepinex_cfg::Event;
    let mut rdr = bepinex_cfg::Reader::new(rdr);
    type SectionMap<V> = IndexMap<SmolStr, V>;
    let mut sections = IndexMap::<SmolStr, SectionMap<Value>>::default();
    let mut section = None::<&mut SectionMap<Value>>;
    let mut sections_annos = IndexMap::<SmolStr, SectionMap<Annotations>>::default();
    let mut section_annos = None::<&mut SectionMap<Annotations>>;
    let mut entry_anno = Annotations::default();
    while let Some(event) = rdr.next()? {
        match event {
            Event::SectionStart { name, .. } => {
                section = None;
                section = Some(sections.entry(name.into()).or_default());
                section_annos = None;
                section_annos = Some(sections_annos.entry(name.into()).or_default());
            }
            Event::DocComment {
                pre_whitespace,
                text,
            } => {
                if let Some(ref mut s) = entry_anno.docs {
                    s.push('\n');
                    s.push_str(text);
                } else {
                    entry_anno.docs = Some(text.to_owned());
                }
            }
            Event::TypeAnnotation {
                pre_whitespace,
                literal_prefix,
                type_name,
            } => {
                entry_anno.type_hint = match type_name {
                    "Boolean" => Some(TypeHint::Bool),
                    // keep existing type hint
                    "Int16" | "Int32" | "Int64" | "UInt16" | "UInt32" | "UInt64"
                        if matches!(entry_anno.type_hint, Some(TypeHint::Int { .. })) =>
                    {
                        entry_anno.type_hint
                    }
                    "Int16" => Some(TypeHint::Int {
                        min: Some(i16::MIN.into()),
                        max: Some(Int::from_signed(i16::MAX)),
                    }),
                    "Int32" => Some(TypeHint::Int {
                        min: Some(i32::MIN.into()),
                        max: Some(Int::from_signed(i32::MAX)),
                    }),
                    "Int64" => Some(TypeHint::Int {
                        min: Some(i64::MIN),
                        max: Some(Int::from_signed(i64::MAX)),
                    }),
                    "UInt16" => Some(TypeHint::Int {
                        min: Some(0),
                        max: Some(Int::from_unsigned(u16::MAX)),
                    }),
                    "UInt32" => Some(TypeHint::Int {
                        min: Some(0),
                        max: Some(Int::from_unsigned(u32::MAX)),
                    }),
                    "UInt64" => Some(TypeHint::Int {
                        min: Some(0),
                        max: Some(Int::from_unsigned(u64::MAX)),
                    }),
                    "Single" | "Double" => Some(TypeHint::Float),
                    "String" => Some(TypeHint::String),
                    // keep existing type hint
                    _ => entry_anno.type_hint,
                };
            }
            Event::ValueRange {
                pre_whitespace,
                literal_prefix,
                from,
                literal_delimiter,
                to,
            } => {
                if entry_anno.type_hint.is_none() {
                    entry_anno.type_hint = Some(TypeHint::Int {
                        min: None,
                        max: None,
                    });
                }
                if let Some(TypeHint::Int { min, max }) = &mut entry_anno.type_hint {
                    *min = from.parse::<i64>().ok();
                    *max = to.parse::<Int>().ok();
                }
            }
            Event::EnumValues {
                pre_whitespace,
                literal_prefix,
                values,
            } => {
                if matches!(entry_anno.type_hint, None | Some(TypeHint::Enum { .. })) {
                    entry_anno.type_hint = Some(TypeHint::Enum {
                        values: values
                            .split(", ")
                            .map(|s| Value::String(s.into()))
                            .collect(),
                        multiple: match entry_anno.type_hint {
                            Some(TypeHint::Enum { multiple, .. }) => multiple,
                            _ => false,
                        },
                    });
                }
            }
            Event::EnumMultiValue {
                pre_whitespace,
                literal,
                post_whitespace,
            } => {
                if matches!(entry_anno.type_hint, None | Some(TypeHint::Enum { .. })) {
                    entry_anno.type_hint = Some(TypeHint::Enum {
                        values: match entry_anno.type_hint {
                            Some(TypeHint::Enum { values, .. }) => values,
                            _ => Vec::new(),
                        },
                        multiple: true,
                    });
                }
            }
            Event::DefaultValue {
                pre_whitespace,
                literal_prefix,
                value,
            } => entry_anno.default_value = Some(parse_bepinex_value(value)),
            Event::Comment {
                pre_whitespace,
                text,
            } => {}
            Event::Entry {
                pre_whitespace,
                key,
                post_key_whitespace,
                pre_value_whitespace,
                value,
            } => {
                if let Some(section) = &mut section {
                    let section_annos = section_annos.as_mut().unwrap();
                    section.insert(key.into(), parse_bepinex_value(value));
                    section_annos.insert(key.into(), std::mem::take(&mut entry_anno));
                }
            }
            Event::FileEnd { whitespace } => todo!(),
        }
    }
    Ok(File::Config {
        sections: sections
            .into_iter()
            .zip(sections_annos.into_values())
            .map(|((name, entries), annos)| Section {
                path: vec![PathComponent::Key(name)],
                annotations: Annotations {
                    docs: None,
                    default_value: None,
                    type_hint: Some(TypeHint::Object { entries: annos }),
                },
                value: Value::Object(entries),
            })
            .collect(),
    })
}

fn parse_bepinex_value(value: &str) -> Value {
    if let Ok(b) = value.parse() {
        return Value::Bool(b);
    }
    if value.parse::<u64>().is_ok() || value.parse::<i64>().is_ok() {
        return Value::Integer(value.into());
    }
    if let Ok(f) = value.parse::<f64>() {
        if f.is_finite() {
            return Value::Float(value.into());
        }
    }
    Value::String(value.into())
}

/// Updates and returns the config.
pub async fn update_config(
    profile: Uuid,
    path: &Path,
    options: ConfigOptions,
    patches: &[Patch],
) -> Result<File> {
    let path = build_config_path(profile, path);
    match Format::guess(&path, options)? {
        Format::BepInEx => {
            use bepinex_cfg::Event;
            let mut rdr = bepinex_cfg::Reader::new(std::fs::File::open(&path)?);
            let mut section = None::<SmolStr>;
            let (mut tmp_file, tmp_path) = tempfile::NamedTempFile::new_in(
                path.parent().context("path must have a parent")?,
            )?;
            while let Some(event) = rdr.next()? {
                match event {
                    Event::SectionStart { name, .. } => {
                        bepinex_cfg::ser::write_io(event, &mut tmp_file)?;
                        section = Some(name.into());
                    }
                    Event::Entry {
                        pre_whitespace,
                        key,
                        post_key_whitespace,
                        pre_value_whitespace,
                        value,
                    } => {
                        // TODO: check if section matches one of the patches... will need to scan over the file ahead of time to find last occurrance of each entry (also, confirm that BepInEx pays attention to last and not first)
                    }
                    _ => {
                        bepinex_cfg::ser::write_io(event, &mut tmp_file)?;
                    }
                }
            }
        },
        _ => bail!("Unsupported config format"),
    }
    read_config_at(&path, options).await
}
