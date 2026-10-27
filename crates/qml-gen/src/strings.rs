use std::collections::HashSet;
use std::fmt::Write as _;
use std::path::Path;

use anyhow::{Context as _, bail};

/// Generates `src/qml/core/Strings.qml` from the default locale's translations: a singleton with a
/// typed property for each of them, so that the editor completes the translations and qmllint
/// checks them, as TypeScript did the keys of `t`. A translation with placeholders is a function of
/// them. Like the dictionary, it's checked in, and only rewritten when it changes. Returns whether
/// it changed.
pub fn generate_strings(root: &Path) -> anyhow::Result<bool> {
    let source = std::fs::read_to_string(root.join(crate::TRANSLATIONS))
        .with_context(|| format!("Unable to read {}", crate::TRANSLATIONS))?;
    let dict = serde_json::from_str(&source)
        .with_context(|| format!("Invalid {}", crate::TRANSLATIONS))?;
    let qml = strings_qml(&dict)?;
    Ok(crate::write_if_changed(&root.join(crate::STRINGS), qml)?)
}

fn strings_qml(dict: &serde_json::Map<String, serde_json::Value>) -> anyhow::Result<String> {
    let mut qml = String::from(
        "// Generated from src/locales/en-CA.json by `just gen` and the build. Edit that instead.\n\
         pragma Singleton\n\
         import QtQuick\n\
         import Manderrow\n\
         \n\
         // The translations, as typed properties: `Strings.game_select.title` is\n\
         // `I18n.t(\"game_select.title\")`, and a translation with placeholders is a function of them,\n\
         // like `Strings.status_bar.downloads_tracker(count)`. Like `I18n.t`, they follow the locale.\n\
         QtObject {\n",
    );
    let mut components = String::new();
    let mut component_names = HashSet::new();
    write_strings_group(&mut qml, &mut components, &mut component_names, &[], dict)?;
    qml.push_str(&components);
    qml.push_str("}\n");
    Ok(qml)
}

/// Writes the members of a group of translations into `out`, and the components for its nested
/// groups, which QML requires to be at the root of the file, into `components`.
fn write_strings_group(
    out: &mut String,
    components: &mut String,
    component_names: &mut HashSet<String>,
    path: &[&str],
    group: &serde_json::Map<String, serde_json::Value>,
) -> anyhow::Result<()> {
    let indent = if path.is_empty() { "    " } else { "        " };
    for (key, value) in group {
        let mut key_path = path.to_vec();
        key_path.push(key);
        let name = qml_identifier(key);
        match value {
            serde_json::Value::Object(nested) => {
                let component = format!(
                    "Strings{}",
                    key_path
                        .iter()
                        .flat_map(|segment| segment.split('_'))
                        .map(|word| word[..1].to_uppercase() + &word[1..])
                        .collect::<String>()
                );
                if !component_names.insert(component.clone()) {
                    bail!("Two groups of translations are both named {component}");
                }
                writeln!(
                    out,
                    "{indent}readonly property {component} {name}: {component} {{}}"
                )?;
                let mut members = String::new();
                write_strings_group(&mut members, components, component_names, &key_path, nested)?;
                write!(
                    components,
                    "\n    component {component}: QtObject {{\n{members}    }}\n"
                )?;
            }
            serde_json::Value::String(text) => {
                let key = key_path.join(".");
                // the English, as a reminder of what it says
                writeln!(out, "{indent}/// {}", serde_json::to_string(text)?)?;
                let placeholders = template_placeholders(text);
                if placeholders.is_empty() {
                    writeln!(
                        out,
                        "{indent}readonly property string {name}: I18n.t(\"{key}\")"
                    )?;
                } else {
                    let params = placeholders
                        .iter()
                        .map(|p| format!("{}: var", qml_identifier(p)))
                        .collect::<Vec<_>>()
                        .join(", ");
                    let args = placeholders
                        .iter()
                        .map(|p| format!("{p}: {}", qml_identifier(p)))
                        .collect::<Vec<_>>()
                        .join(", ");
                    writeln!(out, "{indent}function {name}({params}): string {{")?;
                    writeln!(out, "{indent}    return I18n.t(\"{key}\", {{ {args} }});")?;
                    writeln!(out, "{indent}}}")?;
                }
            }
            _ => bail!("The translation {} is not a string", key_path.join(".")),
        }
    }
    Ok(())
}

/// The names of the `{{ name }}` placeholders of a translation, in their first order.
fn template_placeholders(text: &str) -> Vec<&str> {
    let mut names = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        let Some(end) = rest[start..].find("}}") else {
            break;
        };
        let name = rest[start + 2..start + end].trim();
        if !names.contains(&name) {
            names.push(name);
        }
        rest = &rest[start + end + 2..];
    }
    names
}

/// A translation's key as a QML identifier: one that's an ECMAScript or QML keyword, or the name
/// of one of the JavaScript globals that QML doesn't allow a property to have, gets a `_` after it,
/// like `delete_` or `console_`, and one that starts with a digit gets one before it, like
/// `_404_page`.
fn qml_identifier(key: &str) -> String {
    #[rustfmt::skip]
    const RESERVED: &[&str] = &[
        "alias", "arguments", "as", "await", "break", "case", "catch", "class", "component",
        "console", "const", "continue", "debugger", "decodeURI", "decodeURIComponent", "default",
        "delete", "do", "else", "encodeURI", "encodeURIComponent", "enum", "escape", "eval",
        "export", "extends", "false", "finally", "for", "function", "gc", "globalThis", "if",
        "implements", "import", "in", "instanceof", "interface", "isFinite", "isNaN", "let", "new",
        "null", "on", "package", "parseFloat", "parseInt", "pragma", "print", "private", "property",
        "protected", "public", "qsTr", "qsTrId", "qsTranslate", "readonly", "required", "return",
        "signal", "static", "super", "switch", "this", "throw", "true", "try", "typeof", "undefined",
        "unescape", "var", "void", "while", "with", "yield",
    ];
    if RESERVED.contains(&key) {
        format!("{key}_")
    } else if key.starts_with(|c: char| c.is_ascii_digit()) {
        format!("_{key}")
    } else {
        key.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_groups_and_functions() {
        let dict = serde_json::from_str(
            r#"{
                "title": "Manderrow",
                "game_select": { "search": { "delete": "Delete", "found": "{{ count }} of {{ total }}" } },
                "404_page": "Not found"
            }"#,
        )
        .unwrap();
        let qml = strings_qml(&dict).unwrap();
        assert!(qml.contains(
            "    /// \"Manderrow\"\n    readonly property string title: I18n.t(\"title\")\n"
        ));
        assert!(qml.contains(
            "    readonly property StringsGameSelect game_select: StringsGameSelect {}\n"
        ));
        assert!(qml.contains("    component StringsGameSelectSearch: QtObject {\n"));
        assert!(qml.contains(
            "        readonly property string delete_: I18n.t(\"game_select.search.delete\")\n"
        ));
        assert!(qml.contains(
            "        function found(count: var, total: var): string {\n            return I18n.t(\"game_select.search.found\", { count: count, total: total });\n        }\n"
        ));
        assert!(qml.contains("    readonly property string _404_page: I18n.t(\"404_page\")\n"));
    }

    #[test]
    fn rejects_groups_with_the_same_component() {
        let dict =
            serde_json::from_str(r#"{ "a_b": { "x": "" }, "a": { "b": { "y": "" } } }"#).unwrap();
        assert!(strings_qml(&dict).is_err());
    }

    #[test]
    fn finds_placeholders_once() {
        assert_eq!(
            template_placeholders("{{a}} and {{ b }} and {{ a }}"),
            ["a", "b"]
        );
        assert_eq!(template_placeholders("no {{ end"), Vec::<&str>::new());
    }
}
