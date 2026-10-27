//! Renders mod READMEs and changelogs for display in the frontend.
//!
//! Qt's rich text engine only supports a subset of HTML and CSS, so documents are converted into a
//! list of blocks that the frontend lays out itself. The inline content of each block is rendered
//! to sanitized HTML, which rich text handles well.

use std::borrow::Cow;
use std::sync::LazyLock;

use comrak::nodes::{AlertType, ListType, Node, NodeValue, TableAlignment};
use comrak::Anchorizer;

/// Images are only loaded from these schemes, which are all that the `web` image provider and
/// Image load. They come from any origin, as badges like shields.io's do, which the web frontend
/// this replaced also loaded.
const ALLOWED_IMAGE_PREFIXES: &[&str] = &["https://", "data:"];

fn is_allowed_image(src: &str) -> bool {
    ALLOWED_IMAGE_PREFIXES
        .iter()
        .any(|prefix| src.starts_with(prefix))
}

static OPTIONS: LazyLock<comrak::Options<'static>> = LazyLock::new(|| {
    let mut options = comrak::Options::default();
    // GitHub-flavoured Markdown, like marked's defaults
    options.extension.table = true;
    options.extension.strikethrough = true;
    options.extension.autolink = true;
    options.extension.tasklist = true;
    options.extension.footnotes = true;
    // GitHub alerts, like marked-alert
    options.extension.alerts = true;
    // :emoji: shortcodes, like node-emoji
    options.extension.shortcodes = true;
    // raw HTML is passed through and then sanitized, like DOMPurify did
    options.render.r#unsafe = true;
    options
});

static SANITIZER: LazyLock<ammonia::Builder<'static>> = LazyLock::new(|| {
    let mut builder = ammonia::Builder::default();
    builder
        .add_generic_attributes(["class", "align"])
        .attribute_filter(|element, attribute, value| {
            if element == "img" && attribute == "src" && !is_allowed_image(value) {
                return None;
            }
            Some(Cow::Borrowed(value))
        })
        .add_url_schemes(["data"]);
    builder
});

#[derive(Debug)]
pub enum Block {
    Heading {
        level: u8,
        html: String,
        /// The fragment that links to it, as GitHub gives it, like `getting-started`.
        anchor: String,
    },
    Paragraph {
        html: String,
    },
    /// An image on its own line, which is scaled down to fit.
    Image {
        src: String,
        alt: String,
        href: Option<String>,
        width: Option<u32>,
        centered: bool,
    },
    Code {
        text: String,
    },
    Quote {
        children: Vec<Block>,
    },
    Alert {
        kind: &'static str,
        title: String,
        children: Vec<Block>,
    },
    List {
        ordered: bool,
        start: usize,
        /// Tight lists don't separate their items' paragraphs.
        tight: bool,
        items: Vec<ListItem>,
    },
    Table {
        alignments: Vec<&'static str>,
        rows: Vec<TableRow>,
    },
    Rule,
    Html {
        html: String,
    },
}

#[derive(Debug)]
pub struct ListItem {
    /// Whether a task list item is checked, or `None` for an ordinary item.
    pub checked: Option<bool>,
    pub children: Vec<Block>,
}

#[derive(Debug)]
pub struct TableRow {
    pub header: bool,
    pub cells: Vec<String>,
}

/// Renders a node and its descendants to sanitized HTML.
fn node_html<'a>(node: Node<'a>) -> String {
    let mut html = String::new();
    comrak::format_html(node, &OPTIONS, &mut html).expect("formatting into a String cannot fail");
    SANITIZER.clean(&html).to_string()
}

/// Renders the inline children of a node to sanitized HTML.
fn inline_html<'a>(node: Node<'a>) -> String {
    let mut html = String::new();
    for child in node.children() {
        comrak::format_html(child, &OPTIONS, &mut html)
            .expect("formatting into a String cannot fail");
    }
    SANITIZER.clean(&html).to_string()
}

fn is_blank_inline<'a>(node: Node<'a>) -> bool {
    match &node.data.borrow().value {
        NodeValue::SoftBreak | NodeValue::LineBreak => true,
        NodeValue::Text(text) => text.trim().is_empty(),
        _ => false,
    }
}

/// Recognizes a paragraph consisting of a single image, optionally wrapped in a link.
fn lone_image<'a>(paragraph: Node<'a>) -> Option<Block> {
    let mut content = paragraph.children().filter(|child| !is_blank_inline(child));
    let node = content.next()?;
    if content.next().is_some() {
        return None;
    }
    let (image, href) = match &node.data.borrow().value {
        NodeValue::Image(_) => (node, None),
        NodeValue::Link(link) => {
            let mut inner = node.children().filter(|child| !is_blank_inline(child));
            let image = inner.next()?;
            if inner.next().is_some() || !matches!(image.data.borrow().value, NodeValue::Image(_)) {
                return None;
            }
            (image, Some(link.url.clone()))
        }
        _ => return None,
    };
    let NodeValue::Image(link) = &image.data.borrow().value else {
        unreachable!()
    };
    let alt = image
        .descendants()
        .filter_map(|n| match &n.data.borrow().value {
            NodeValue::Text(text) => Some(text.to_string()),
            _ => None,
        })
        .collect::<String>();
    Some(Block::Image {
        src: link.url.clone(),
        alt,
        href,
        width: None,
        centered: false,
    })
}

/// Recognizes sanitized HTML that only wraps images, like `<p align="center"><img src="..."></p>`,
/// which is common in READMEs.
fn html_images(html: &str) -> Option<Vec<Block>> {
    let centered = html.contains("align=\"center\"");
    let mut images = Vec::new();
    let mut text = String::new();
    let mut rest = html;
    while let Some(start) = rest.find('<') {
        text.push_str(&rest[..start]);
        let end = rest[start..].find('>')? + start;
        let tag = &rest[start + 1..end];
        if let Some(attrs) = tag.strip_prefix("img ") {
            let attr = |name: &str| {
                let key = format!("{name}=\"");
                let at = attrs.find(&key)? + key.len();
                let len = attrs[at..].find('"')?;
                Some(attrs[at..at + len].to_owned())
            };
            let src = attr("src")?;
            images.push(Block::Image {
                src: html_escape_decode(&src),
                alt: attr("alt").unwrap_or_default(),
                href: None,
                width: attr("width").and_then(|w| w.trim_end_matches("px").parse().ok()),
                centered,
            });
        } else if !matches!(
            tag.split_whitespace().next().unwrap_or_default().trim_start_matches('/'),
            "p" | "div" | "br" | "center" | "a"
        ) {
            return None;
        }
        rest = &rest[end + 1..];
    }
    text.push_str(rest);
    (!images.is_empty() && text.trim().is_empty()).then_some(images)
}

fn html_escape_decode(s: &str) -> String {
    s.replace("&amp;", "&")
}

fn blocks<'a>(parent: Node<'a>, anchors: &mut Anchorizer, out: &mut Vec<Block>) {
    for node in parent.children() {
        let value = node.data.borrow().value.clone();
        match value {
            NodeValue::Paragraph => match lone_image(node) {
                Some(Block::Image { src, .. }) if !is_allowed_image(&src) => {}
                Some(image) => out.push(image),
                None => {
                    let html = inline_html(node);
                    if !html.trim().is_empty() {
                        out.push(Block::Paragraph { html });
                    }
                }
            },
            NodeValue::Heading(heading) => out.push(Block::Heading {
                level: heading.level,
                html: inline_html(node),
                anchor: anchors.anchorize(&node.collect_text()),
            }),
            NodeValue::CodeBlock(code) => out.push(Block::Code {
                text: code.literal.trim_end_matches('\n').to_owned(),
            }),
            NodeValue::BlockQuote | NodeValue::MultilineBlockQuote(_) => {
                let mut children = Vec::new();
                blocks(node, anchors, &mut children);
                out.push(Block::Quote { children });
            }
            NodeValue::Alert(alert) => {
                let mut children = Vec::new();
                blocks(node, anchors, &mut children);
                out.push(Block::Alert {
                    kind: match alert.alert_type {
                        AlertType::Note => "note",
                        AlertType::Tip => "tip",
                        AlertType::Important => "important",
                        AlertType::Warning => "warning",
                        AlertType::Caution => "caution",
                    },
                    title: alert
                        .title
                        .clone()
                        .unwrap_or_else(|| alert.alert_type.default_title().to_owned()),
                    children,
                });
            }
            NodeValue::List(list) => {
                let items = node
                    .children()
                    .map(|item| {
                        let checked = match &item.data.borrow().value {
                            NodeValue::TaskItem(task) => Some(task.symbol.is_some()),
                            _ => None,
                        };
                        let mut children = Vec::new();
                        blocks(item, anchors, &mut children);
                        ListItem { checked, children }
                    })
                    .collect();
                out.push(Block::List {
                    ordered: list.list_type == ListType::Ordered,
                    start: list.start,
                    tight: list.tight,
                    items,
                });
            }
            NodeValue::Table(table) => {
                let alignments = table
                    .alignments
                    .iter()
                    .map(|alignment| match alignment {
                        TableAlignment::None | TableAlignment::Left => "left",
                        TableAlignment::Center => "center",
                        TableAlignment::Right => "right",
                    })
                    .collect();
                let rows = node
                    .children()
                    .map(|row| TableRow {
                        header: matches!(row.data.borrow().value, NodeValue::TableRow(true)),
                        cells: row.children().map(inline_html).collect(),
                    })
                    .collect();
                out.push(Block::Table { alignments, rows });
            }
            NodeValue::ThematicBreak => out.push(Block::Rule),
            NodeValue::HtmlBlock(_) => {
                let html = node_html(node);
                match html_images(&html) {
                    Some(images) => out.extend(images),
                    None if html.trim().is_empty() => {}
                    None => out.push(Block::Html { html }),
                }
            }
            NodeValue::FrontMatter(_) => {}
            // footnotes, description lists, and anything else with block children
            _ => blocks(node, anchors, out),
        }
    }
}

/// Converts GitHub-flavoured Markdown into blocks for the frontend to lay out.
pub fn to_blocks(markdown: &str) -> Vec<Block> {
    let arena = comrak::Arena::new();
    let root = comrak::parse_document(&arena, markdown, &OPTIONS);
    let mut out = Vec::new();
    blocks(root, &mut Anchorizer::new(), &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::{to_blocks, Block};

    fn debug(markdown: &str) -> String {
        format!("{:?}", to_blocks(markdown))
    }

    #[test]
    fn renders_gfm() {
        let out = debug("# Title\n\n| a |\n|---|\n| 1 |\n\n> [!NOTE]\n> hi\n\n:smile: **bold**\n\n- [x] done\n- todo");
        assert!(out.contains("Heading { level: 1"), "{out}");
        assert!(out.contains("Table {"), "{out}");
        assert!(out.contains(r#"kind: "note""#), "{out}");
        assert!(out.contains('😄'), "{out}");
        assert!(out.contains("<strong>bold</strong>"), "{out}");
        assert!(out.contains("checked: Some(true)"), "{out}");
    }

    #[test]
    fn strips_scripts_and_insecure_images() {
        let out = debug(
            "<script>alert(1)</script>\n\n![x](http://insecure.example/x.png)\n\n\
             ![y](https://gcdn.thunderstore.io/live/y.png)\n\ntext <img src=\"file:///etc/z.png\"> \
             [![badge](https://img.shields.io/badge/a-b-green)](https://example.com)",
        );
        assert!(!out.contains("script"), "{out}");
        assert!(!out.contains("insecure.example"), "{out}");
        assert!(!out.contains("file:"), "{out}");
        assert!(out.contains(r#"Image { src: "https://gcdn.thunderstore.io/live/y.png""#), "{out}");
        assert!(out.contains(r#"src=\"https://img.shields.io/badge/a-b-green\""#), "{out}");
    }

    #[test]
    fn anchors_headings_like_github() {
        let blocks =
            to_blocks("# Getting Started\n\n## `config.toml` & you\n\n> ## Getting started");
        let anchors: Vec<_> = blocks
            .iter()
            .flat_map(|block| match block {
                Block::Quote { children } => &children[..],
                _ => std::slice::from_ref(block),
            })
            .filter_map(|block| match block {
                Block::Heading { anchor, .. } => Some(anchor.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(
            anchors,
            ["getting-started", "configtoml--you", "getting-started-1"]
        );
    }

    #[test]
    fn extracts_images_from_html() {
        let blocks = to_blocks("<p align=\"center\">\n<img src=\"https://gcdn.thunderstore.io/a.png\" width=\"300\">\n</p>");
        assert!(
            matches!(&blocks[..], [Block::Image { width: Some(300), centered: true, .. }]),
            "{blocks:?}"
        );
    }
}
