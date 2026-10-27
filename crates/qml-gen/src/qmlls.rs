use std::fmt::Write as _;
use std::path::Path;

use crate::QmlFile;

/// Writes a resource file next to the exported QML module, mapping the copies of the QML files in
/// the module back to their sources. qmlls finds it by scanning its import paths, and then opens
/// the sources rather than the copies, as it does for CMake builds.
pub fn write_qmlls_qrc(
    root: &Path,
    modules_dir: &Path,
    qml_files: &[QmlFile],
) -> std::io::Result<()> {
    let mut qrc = format!(
        "<RCC>\n  <qresource prefix=\"/qt/qml/{}\">\n",
        crate::MODULE
    );
    let qmldir = modules_dir.join(crate::MODULE).join("qmldir");
    writeln!(
        qrc,
        "    <file alias=\"qmldir\">{}</file>",
        qmldir.display()
    )
    .unwrap();
    for file in qml_files {
        let alias = file.get_path().to_str().unwrap().replace('\\', "/");
        let source = root.join(file.get_path());
        writeln!(
            qrc,
            "    <file alias=\"{alias}\">{}</file>",
            source.display()
        )
        .unwrap();
    }
    qrc.push_str("  </qresource>\n</RCC>\n");
    crate::write_if_changed(&modules_dir.join(format!("{}.qrc", crate::MODULE)), qrc)?;
    Ok(())
}

/// Points the exported QML module's type information about the types defined in Rust at their
/// declarations in the bridge files, rather than in the C++ headers that cxx-qt generates from
/// them. qmlls's go to definition, as on `Games.search`, then opens the bridge at `fn search`.
/// qmlls opens a type's file directly when its path is absolute, and otherwise searches the
/// workspace for a file of that name, which it gives up on in a target directory.
pub fn point_qmltypes_at_bridges(
    root: &Path,
    modules_dir: &Path,
    bridges: &[&str],
) -> std::io::Result<()> {
    let path = modules_dir.join(crate::MODULE).join("plugin.qmltypes");
    let qmltypes = std::fs::read_to_string(&path)?;
    let mut lines = qmltypes.lines().map(str::to_owned).collect::<Vec<_>>();

    let mut blocks = Vec::<Block>::new();
    // the lines of the bridge that declares the component being read
    let mut bridge = None::<Vec<String>>;
    for i in 0..lines.len() {
        let line = lines[i].trim();
        if let Some(kind) = line.strip_suffix(" {") {
            blocks.push(Block {
                kind: kind.to_owned(),
                name: None,
                line_number: None,
            });
        } else if line == "}" {
            let block = blocks.pop().unwrap();
            if let (Some(source), Some(name), Some(at)) = (&bridge, &block.name, block.line_number)
            {
                let component = component_name(&blocks).unwrap_or(name);
                if let Some(line) = declaration_line(source, &block.kind, name, component) {
                    lines[at] = with_line_number(&lines[at], line + 1);
                }
            }
            if block.kind == "Component" {
                bridge = None;
            }
        } else if let Some((kind, fields)) =
            line.strip_suffix(" }").and_then(|l| l.split_once(" { "))
        {
            // a block on one line, like `Method { name: "relaunch"; lineNumber: 54 }`
            let name = fields
                .split("; ")
                .find_map(|field| field.strip_prefix("name: "))
                .map(|name| name.trim_matches('"'));
            if let (Some(source), Some(name), Some(component)) =
                (&bridge, name, component_name(&blocks))
                && let Some(line) = declaration_line(source, kind, name, component)
            {
                lines[i] = with_line_number(&lines[i], line + 1);
            }
        } else if let Some(block) = blocks.last_mut() {
            if let Some(value) = line.strip_prefix("name: ") {
                block.name = Some(value.trim_matches('"').to_owned());
            } else if line.starts_with("lineNumber: ") {
                block.line_number = Some(i);
            } else if let Some(stem) = line
                .strip_prefix("file: \"")
                .and_then(|file| file.strip_suffix(".cxxqt.h\""))
            {
                let Some(file) = bridges
                    .iter()
                    .map(|bridge| root.join(bridge))
                    .find(|bridge| bridge.file_stem().is_some_and(|s| s == stem))
                else {
                    continue;
                };
                let indent = &lines[i][..lines[i].len() - lines[i].trim_start().len()];
                let file_name = file.display().to_string().replace('\\', "/");
                lines[i] = format!("{indent}file: \"{file_name}\"");
                bridge = std::fs::read_to_string(&file)
                    .ok()
                    .map(|source| source.lines().map(str::to_owned).collect());
            }
        }
    }

    lines.push(String::new());
    std::fs::write(&path, lines.join("\n"))
}

fn component_name(blocks: &[Block]) -> Option<&str> {
    blocks
        .iter()
        .find(|block| block.kind == "Component")
        .and_then(|component| component.name.as_deref())
}

/// Replaces the number of a line's `lineNumber` field, if it has one.
fn with_line_number(line: &str, number: usize) -> String {
    let Some((before, after)) = line.split_once("lineNumber: ") else {
        return line.to_owned();
    };
    let rest = after.trim_start_matches(|c: char| c.is_ascii_digit());
    format!("{before}lineNumber: {number}{rest}")
}

/// A `Component`, `Property`, `Method`, `Signal`, `Enum`, or another block of a qmltypes file.
struct Block {
    kind: String,
    name: Option<String>,
    /// The index of its `lineNumber` line.
    line_number: Option<usize>,
}

/// Finds the index of the line of a bridge that declares a component (its QObject), or one of a
/// component's properties, methods, signals, or enums, by its name in QML. It falls back to the
/// component's declaration.
fn declaration_line(source: &[String], kind: &str, name: &str, component: &str) -> Option<usize> {
    let find =
        |predicate: &dyn Fn(&str) -> bool| source.iter().position(|line| predicate(line.trim()));
    let component_line = find(&|line| line.starts_with(&format!("type {component} =")));
    // QML's names are the camelCase of the bridge's snake_case ones
    let snake = name.chars().fold(String::new(), |mut snake, c| {
        if c.is_ascii_uppercase() {
            snake.push('_');
        }
        snake.push(c.to_ascii_lowercase());
        snake
    });
    let property = |snake: &str| {
        // a component's properties are declared in the attributes before it
        let end = component_line?;
        (0..end)
            .rev()
            .take_while(|&i| {
                source[i].trim().starts_with("#[") || source[i].trim().starts_with("//")
            })
            .find(|&i| {
                source[i]
                    .trim()
                    .strip_prefix("#[qproperty(")
                    .and_then(|args| args.split(',').nth(1))
                    .is_some_and(|property| property.trim().trim_end_matches(")]") == snake)
            })
    };
    let line = match kind {
        "Component" => component_line,
        "Enum" => find(&|line| line.starts_with(&format!("enum {name} "))),
        "Property" => property(&snake),
        "Method" | "Signal" => (0..source.len())
            .find(|&i| {
                source[i].contains(&format!("fn {snake}("))
                    && source[i..(i + 4).min(source.len())].iter().any(|line| {
                        line.contains(&format!("self: &{component}"))
                            || line.contains(&format!("self: Pin<&mut {component}>"))
                    })
            })
            // the change signals of properties are declared by the properties
            .or_else(|| property(snake.strip_suffix("_changed")?)),
        _ => None,
    };
    line.or(component_line)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BRIDGE: &str = r#"
        #[qobject]
        #[qml_element]
        #[qproperty(QString, cover_url, READ, CONSTANT)]
        #[qproperty(bool, is_open)]
        type Game = super::GameRust;

        #[qenum(Game)]
        enum SortColumn {
            Name,
        }

        #[qinvokable]
        fn by_id(self: &Game, id: &QString) -> *mut Game;

        #[qsignal]
        fn opened(
            self: Pin<&mut Game>,
        );
    "#;

    fn line(kind: &str, name: &str) -> Option<usize> {
        let source = BRIDGE.lines().map(str::to_owned).collect::<Vec<_>>();
        declaration_line(&source, kind, name, "Game")
    }

    #[test]
    fn finds_declarations() {
        assert_eq!(line("Component", "Game"), Some(5));
        assert_eq!(line("Property", "coverUrl"), Some(3));
        assert_eq!(line("Enum", "SortColumn"), Some(8));
        assert_eq!(line("Method", "byId"), Some(13));
        assert_eq!(line("Signal", "opened"), Some(16));
        assert_eq!(line("Signal", "isOpenChanged"), Some(4));
        assert_eq!(line("Method", "unknown"), Some(5));
    }

    #[test]
    fn replaces_line_numbers() {
        assert_eq!(
            with_line_number("    lineNumber: 54", 7),
            "    lineNumber: 7"
        );
        assert_eq!(
            with_line_number(r#"Method { name: "relaunch"; lineNumber: 54 }"#, 7),
            r#"Method { name: "relaunch"; lineNumber: 7 }"#
        );
    }
}
