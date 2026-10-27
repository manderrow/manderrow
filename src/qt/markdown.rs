//! The `MarkdownDocument`s that `App.markdown` gives QML: Markdown, as blocks that QML lays out
//! itself, as Qt's rich text only supports a subset of HTML and CSS.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!(<QtCore/QObject>);
        include!("cxx-qt-lib/qlist.h");
        type QObjectList = crate::qt::QObjectList;
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;
    }

    #[qenum(MarkdownBlock)]
    enum MarkdownBlockKind {
        Heading,
        Paragraph,
        /// An image on its own line, which is scaled down to fit.
        Image,
        Code,
        Quote,
        /// A GitHub alert, like a note or a warning.
        Alert,
        List,
        Table,
        Rule,
        Html,
    }

    #[auto_cxx_name]
    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_uncreatable]
        #[qproperty(MarkdownBlockKind, kind, READ, CONSTANT)]
        /// A heading's level, from 1.
        #[qproperty(i32, level, READ, CONSTANT)]
        /// The sanitized HTML of a heading, paragraph, or HTML block.
        #[qproperty(QString, html, READ, CONSTANT)]
        /// The fragment that links to a heading, without its `#`.
        #[qproperty(QString, anchor, READ, CONSTANT)]
        /// An image's source, alternative text, and link.
        #[qproperty(QString, src, READ, CONSTANT)]
        #[qproperty(QString, alt, READ, CONSTANT)]
        #[qproperty(QString, href, READ, CONSTANT)]
        /// An image's width, or 0 if it has none of its own.
        #[qproperty(i32, width, READ, CONSTANT)]
        #[qproperty(bool, centered, READ, CONSTANT)]
        /// The text of a code block.
        #[qproperty(QString, text, READ, CONSTANT)]
        /// An alert's kind, like `note` or `warning`, and title.
        #[qproperty(QString, alert_kind, READ, CONSTANT)]
        #[qproperty(QString, title, READ, CONSTANT)]
        /// The blocks of a quote or alert.
        #[qproperty(QObjectList, children, READ, CONSTANT)]
        #[qproperty(bool, ordered, READ, CONSTANT)]
        /// The number of an ordered list's first item.
        #[qproperty(i32, start, READ, CONSTANT)]
        /// Whether a list doesn't separate its items' paragraphs.
        #[qproperty(bool, tight, READ, CONSTANT)]
        /// A list's `MarkdownListItem`s.
        #[qproperty(QObjectList, items, READ, CONSTANT)]
        /// The alignment of each of a table's columns: `left`, `center`, `right`, or empty.
        #[qproperty(QStringList, alignments, READ, CONSTANT)]
        /// A table's `MarkdownTableRow`s.
        #[qproperty(QObjectList, rows, READ, CONSTANT)]
        type MarkdownBlock = super::MarkdownBlockRust;

        #[qobject]
        #[qml_element]
        #[qml_uncreatable]
        /// Whether it's a task, with a checkbox.
        #[qproperty(bool, is_task, READ, CONSTANT)]
        #[qproperty(bool, checked, READ, CONSTANT)]
        #[qproperty(QObjectList, children, READ, CONSTANT)]
        type MarkdownListItem = super::MarkdownListItemRust;

        #[qobject]
        #[qml_element]
        #[qml_uncreatable]
        #[qproperty(bool, header, READ, CONSTANT)]
        /// The sanitized HTML of its cells.
        #[qproperty(QStringList, cells, READ, CONSTANT)]
        type MarkdownTableRow = super::MarkdownTableRowRust;

        #[qobject]
        #[qml_element]
        #[qml_uncreatable]
        /// Its `MarkdownBlock`s, which it owns.
        #[qproperty(QObjectList, blocks, READ, CONSTANT)]
        type MarkdownDocument = super::MarkdownDocumentRust;
    }

    #[namespace = "rust::cxxqtlib1"]
    unsafe extern "C++" {
        include!("cxx-qt-lib/common.h");
        #[rust_name = "new_markdown_block"]
        fn make_unique() -> UniquePtr<MarkdownBlock>;
        #[rust_name = "new_markdown_list_item"]
        fn make_unique() -> UniquePtr<MarkdownListItem>;
        #[rust_name = "new_markdown_table_row"]
        fn make_unique() -> UniquePtr<MarkdownTableRow>;
        #[rust_name = "new_markdown_document"]
        fn make_unique() -> UniquePtr<MarkdownDocument>;
    }
}

use core::pin::Pin;

use cxx::UniquePtr;
use cxx_qt::casting::Upcast;
use cxx_qt::{CxxQtType, QObject};
use cxx_qt_lib::{QString, QStringList};

use super::child::Child;
use super::QObjectList;
use crate::markdown::{self, Block};
use qobject::{MarkdownBlockKind, MarkdownDocument};

pub struct MarkdownBlockRust {
    kind: MarkdownBlockKind,
    level: i32,
    html: QString,
    anchor: QString,
    src: QString,
    alt: QString,
    href: QString,
    width: i32,
    centered: bool,
    text: QString,
    alert_kind: QString,
    title: QString,
    children: QObjectList,
    ordered: bool,
    start: i32,
    tight: bool,
    items: QObjectList,
    alignments: QStringList,
    rows: QObjectList,
}

impl Default for MarkdownBlockRust {
    fn default() -> Self {
        Self {
            kind: MarkdownBlockKind::Paragraph,
            level: 0,
            html: QString::default(),
            anchor: QString::default(),
            src: QString::default(),
            alt: QString::default(),
            href: QString::default(),
            width: 0,
            centered: false,
            text: QString::default(),
            alert_kind: QString::default(),
            title: QString::default(),
            children: QObjectList::default(),
            ordered: false,
            start: 0,
            tight: false,
            items: QObjectList::default(),
            alignments: QStringList::default(),
            rows: QObjectList::default(),
        }
    }
}

#[derive(Default)]
pub struct MarkdownListItemRust {
    is_task: bool,
    checked: bool,
    children: QObjectList,
}

#[derive(Default)]
pub struct MarkdownTableRowRust {
    header: bool,
    cells: QStringList,
}

#[derive(Default)]
pub struct MarkdownDocumentRust {
    blocks: QObjectList,
}

/// Lays out Markdown as blocks, for QML to show. QML owns the document, which owns the rest.
pub fn new_document(source: &str) -> UniquePtr<MarkdownDocument> {
    let mut document = qobject::new_markdown_document();
    let blocks = new_blocks(document.pin_mut(), markdown::to_blocks(source));
    document.pin_mut().rust_mut().blocks = blocks;
    document
}

/// Makes the objects of blocks as children of an object of the document.
fn new_blocks<P: Upcast<QObject>>(mut parent: Pin<&mut P>, blocks: Vec<Block>) -> QObjectList {
    blocks
        .into_iter()
        .map(|block| {
            let object = new_block(block);
            Child::adopt(parent.as_mut(), object).as_qobject()
        })
        .collect()
}

fn new_block(block: Block) -> UniquePtr<qobject::MarkdownBlock> {
    let mut object = qobject::new_markdown_block();
    match block {
        Block::Heading {
            level,
            html,
            anchor,
        } => {
            let mut rust = object.pin_mut().rust_mut();
            rust.kind = MarkdownBlockKind::Heading;
            rust.level = i32::from(level);
            rust.html = QString::from(&html);
            rust.anchor = QString::from(&anchor);
        }
        Block::Paragraph { html } => {
            let mut rust = object.pin_mut().rust_mut();
            rust.kind = MarkdownBlockKind::Paragraph;
            rust.html = QString::from(&html);
        }
        Block::Image {
            src,
            alt,
            href,
            width,
            centered,
        } => {
            let mut rust = object.pin_mut().rust_mut();
            rust.kind = MarkdownBlockKind::Image;
            rust.src = QString::from(&src);
            rust.alt = QString::from(&alt);
            rust.href = href.as_deref().map(QString::from).unwrap_or_default();
            rust.width = width.map_or(0, |width| i32::try_from(width).unwrap_or(i32::MAX));
            rust.centered = centered;
        }
        Block::Code { text } => {
            let mut rust = object.pin_mut().rust_mut();
            rust.kind = MarkdownBlockKind::Code;
            rust.text = QString::from(&text);
        }
        Block::Quote { children } => {
            let children = new_blocks(object.pin_mut(), children);
            let mut rust = object.pin_mut().rust_mut();
            rust.kind = MarkdownBlockKind::Quote;
            rust.children = children;
        }
        Block::Alert {
            kind,
            title,
            children,
        } => {
            let children = new_blocks(object.pin_mut(), children);
            let mut rust = object.pin_mut().rust_mut();
            rust.kind = MarkdownBlockKind::Alert;
            rust.alert_kind = QString::from(kind);
            rust.title = QString::from(&title);
            rust.children = children;
        }
        Block::List {
            ordered,
            start,
            tight,
            items,
        } => {
            let items = items
                .into_iter()
                .map(|item| {
                    let mut item_object = qobject::new_markdown_list_item();
                    let children = new_blocks(item_object.pin_mut(), item.children);
                    {
                        let mut rust = item_object.pin_mut().rust_mut();
                        rust.is_task = item.checked.is_some();
                        rust.checked = item.checked == Some(true);
                        rust.children = children;
                    }
                    Child::adopt(object.pin_mut(), item_object).as_qobject()
                })
                .collect();
            let mut rust = object.pin_mut().rust_mut();
            rust.kind = MarkdownBlockKind::List;
            rust.ordered = ordered;
            rust.start = i32::try_from(start).unwrap_or(i32::MAX);
            rust.tight = tight;
            rust.items = items;
        }
        Block::Table { alignments, rows } => {
            let rows = rows
                .into_iter()
                .map(|row| {
                    let mut row_object = qobject::new_markdown_table_row();
                    {
                        let mut rust = row_object.pin_mut().rust_mut();
                        rust.header = row.header;
                        rust.cells = row.cells.iter().map(QString::from).collect();
                    }
                    Child::adopt(object.pin_mut(), row_object).as_qobject()
                })
                .collect();
            let mut rust = object.pin_mut().rust_mut();
            rust.kind = MarkdownBlockKind::Table;
            rust.alignments = alignments.into_iter().map(QString::from).collect();
            rust.rows = rows;
        }
        Block::Rule => object.pin_mut().rust_mut().kind = MarkdownBlockKind::Rule,
        Block::Html { html } => {
            let mut rust = object.pin_mut().rust_mut();
            rust.kind = MarkdownBlockKind::Html;
            rust.html = QString::from(&html);
        }
    }
    object
}
