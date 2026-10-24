//! The plain text of rich text documents, as browsers copy it.

#[cxx_qt::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!(<QtGui/QTextDocument>);
        type QTextDocument;
        include!(<QtGui/QTextCursor>);
        type QTextCursor;
        include!(<QtGui/QTextFormat>);
        type QTextCharFormat;
        include!(<QtGui/QTextTable>);
        type QTextTableCell;
    }

    unsafe extern "C++Qt" {
        include!(<QtQuick/QQuickTextDocument>);
        #[qobject]
        #[base = QObject]
        type QQuickTextDocument;

        include!(<QtGui/QTextFrame>);
        #[qobject]
        #[base = QObject]
        type QTextFrame;

        #[qobject]
        #[base = QTextFrame]
        type QTextTable;
    }

    #[namespace = "manderrow"]
    #[repr(u32)]
    enum QTextCursorMoveMode {
        MoveAnchor = 0,
    }

    #[namespace = "manderrow"]
    unsafe extern "C++" {
        include!("manderrow/src/cpp/qt.h");
        type QTextCursorMoveMode;

        #[rust_name = "cell_at"]
        fn cellAt(table: &QTextTable, row: i32, column: i32) -> UniquePtr<QTextTableCell>;
        #[rust_name = "char_format"]
        fn charFormat(cursor: &QTextCursor) -> UniquePtr<QTextCharFormat>;
    }

    unsafe extern "C++" {
        #[rust_name = "text_document"]
        fn textDocument(self: &QQuickTextDocument) -> *mut QTextDocument;

        #[rust_name = "frame_at"]
        fn frameAt(self: &QTextDocument, position: i32) -> *mut QTextFrame;
        #[rust_name = "root_frame"]
        fn rootFrame(self: &QTextDocument) -> *mut QTextFrame;
        #[rust_name = "to_raw_text"]
        fn toRawText(self: &QTextDocument) -> QString;

        #[rust_name = "first_position"]
        fn firstPosition(self: &QTextFrame) -> i32;
        #[rust_name = "last_position"]
        fn lastPosition(self: &QTextFrame) -> i32;
        #[rust_name = "parent_frame"]
        fn parentFrame(self: &QTextFrame) -> *mut QTextFrame;

        fn rows(self: &QTextTable) -> i32;
        fn columns(self: &QTextTable) -> i32;

        fn row(self: &QTextTableCell) -> i32;
        fn column(self: &QTextTableCell) -> i32;
        #[rust_name = "first_position"]
        fn firstPosition(self: &QTextTableCell) -> i32;
        #[rust_name = "last_position"]
        fn lastPosition(self: &QTextTableCell) -> i32;

        #[rust_name = "set_position"]
        fn setPosition(self: Pin<&mut QTextCursor>, position: i32, mode: QTextCursorMoveMode);

        #[rust_name = "is_image_format"]
        fn isImageFormat(self: &QTextCharFormat) -> bool;
        #[rust_name = "string_property"]
        fn stringProperty(self: &QTextCharFormat, property_id: i32) -> QString;
    }

    #[namespace = "rust::cxxqtlib1"]
    unsafe extern "C++" {
        include!("cxx-qt-lib/common.h");

        #[rust_name = "qtextcursor_new"]
        unsafe fn make_unique(document: *mut QTextDocument) -> UniquePtr<QTextCursor>;
    }
}

use cxx::UniquePtr;
use cxx_qt::casting::Downcast;
use cxx_qt::QObject;
use cxx_qt_lib::QString;

use ffi::{
    QQuickTextDocument, QTextCursor, QTextCursorMoveMode, QTextDocument, QTextFrame, QTextTable,
};

/// `QTextFormat::ImageAltText`
const IMAGE_ALT_TEXT: i32 = 0x5002;

// the characters of a document's raw text that aren't text
const PARAGRAPH_SEPARATOR: u16 = 0x2029;
const LINE_SEPARATOR: u16 = 0x2028;
const BEGINNING_OF_FRAME: u16 = 0xfdd0;
const END_OF_FRAME: u16 = 0xfdd1;
const OBJECT_REPLACEMENT: u16 = 0xfffc;
const NO_BREAK_SPACE: u16 = 0xa0;

const NEWLINE: u16 = b'\n' as u16;
const SPACE: u16 = b' ' as u16;
const TAB: u16 = b'\t' as u16;

/// Text built as a browser's `innerText` is: line breaks between blocks are asked for, and only put
/// between text, as many in a row as one.
#[derive(Default)]
struct PlainText {
    text: Vec<u16>,
    line_break: bool,
}

impl PlainText {
    fn break_line(&mut self) {
        self.line_break = true;
    }

    fn push(&mut self, text: &[u16]) {
        if text.is_empty() {
            return;
        }
        if self.line_break && !self.text.is_empty() {
            self.text.push(NEWLINE);
        }
        self.line_break = false;
        self.text.extend_from_slice(text);
    }
}

/// Reads the text of a selection of a document.
struct Selection<'a> {
    document: &'a QTextDocument,
    /// The document's text, where each character is at its position in the document, with
    /// separators between its blocks and around its frames, and a character for each image.
    raw: &'a [u16],
    start: i32,
    end: i32,
    /// For reading the formats of images, made from `document_ptr` once one is needed.
    document_ptr: *mut QTextDocument,
    cursor: Option<UniquePtr<QTextCursor>>,
}

impl<'a> Selection<'a> {
    fn is_selected(&self, position: i32) -> bool {
        self.start <= position && position < self.end
    }

    /// Adds the selected text of the content of `frame` between `from` and `to`, which is either
    /// all of a frame that is not a table, or a cell of a table.
    fn add_content(&mut self, out: &mut PlainText, frame: &QTextFrame, from: i32, to: i32) {
        let mut line = Vec::new();
        let mut position = from;
        while position < to {
            let at = position;
            position += 1;
            match self.raw[at as usize] {
                BEGINNING_OF_FRAME => {
                    out.push(&line);
                    line.clear();
                    out.break_line();
                    // the frame in this one whose content starts after its beginning
                    if let Some(child) = self.child_at(frame, position) {
                        self.add_frame(out, child);
                        out.break_line();
                        // past its end
                        position = child.last_position() + 1;
                    }
                }
                PARAGRAPH_SEPARATOR | END_OF_FRAME => {
                    out.push(&line);
                    line.clear();
                    out.break_line();
                }
                _ if !self.is_selected(at) => {}
                LINE_SEPARATOR => line.push(NEWLINE),
                NO_BREAK_SPACE => line.push(SPACE),
                OBJECT_REPLACEMENT => line.extend_from_slice(self.alt_text(at).as_slice()),
                c => line.push(c),
            }
        }
        out.push(&line);
    }

    /// The frame in `parent` whose content starts at `position`.
    fn child_at(&self, parent: &QTextFrame, position: i32) -> Option<&'a QTextFrame> {
        // SAFETY: the frames are the document's, which outlives the selection, and isn't changed
        // while it is read
        let mut frame = unsafe { self.document.frame_at(position).as_ref() }?;
        loop {
            // SAFETY: as above
            let up = unsafe { frame.parent_frame().as_ref() }?;
            if std::ptr::eq(up, parent) {
                return (frame.first_position() == position).then_some(frame);
            }
            frame = up;
        }
    }

    /// Adds the selected text of a frame: a table's rows on lines of their own, with their cells
    /// separated by tabs, as browsers copy them, or the content of any other frame.
    fn add_frame(&mut self, out: &mut PlainText, frame: &QTextFrame) {
        if frame.first_position() > self.end || frame.last_position() < self.start {
            return;
        }
        let Some(table) = frame.downcast::<QTextTable>() else {
            self.add_content(out, frame, frame.first_position(), frame.last_position());
            return;
        };
        for row in 0..table.rows() {
            let mut cells = Vec::new();
            for column in 0..table.columns() {
                let cell = ffi::cell_at(table, row, column);
                // a cell that spans several is only copied once, where it starts
                if cell.row() != row || cell.column() != column {
                    continue;
                }
                // only the selected cells, but each of those, empty or not, to keep the columns
                let (first, last) = (cell.first_position(), cell.last_position());
                if first >= self.end || (last <= self.start && first < self.start) {
                    continue;
                }
                let mut text = PlainText::default();
                self.add_content(&mut text, frame, first, last);
                cells.push(text.text);
            }
            if !cells.is_empty() {
                out.break_line();
                out.push(&cells.join(&TAB));
            }
        }
    }

    /// The alternative text of the image at a position, if it has any.
    fn alt_text(&mut self, position: i32) -> QString {
        let document = self.document_ptr;
        // SAFETY: the document is alive, and the cursor only reads it
        let cursor = self
            .cursor
            .get_or_insert_with(|| unsafe { ffi::qtextcursor_new(document) });
        // a cursor has the format of the character before it
        cursor
            .pin_mut()
            .set_position(position + 1, QTextCursorMoveMode::MoveAnchor);
        let format = ffi::char_format(cursor);
        if format.is_image_format() {
            format.string_property(IMAGE_ALT_TEXT)
        } else {
            QString::default()
        }
    }
}

/// The text between `start` and `end` of a TextEdit's `textDocument`, as a browser copies it: a
/// table's rows go on lines of their own, with their cells separated by tabs, its lines and
/// paragraphs are separated by newlines, and its images are copied as their alternative text. The
/// TextEdit's own `selectedText` runs a table's cells together, with only invisible characters
/// between them.
pub fn selected_text(document: &QObject, start: i32, end: i32) -> QString {
    let Some(document) = document.downcast::<QQuickTextDocument>() else {
        return QString::default();
    };
    let document_ptr = document.text_document();
    // SAFETY: a QQuickTextDocument's document lives as long as it does
    let Some(document) = (unsafe { document_ptr.as_ref() }) else {
        return QString::default();
    };
    // SAFETY: a document's root frame lives as long as it does
    let Some(root) = (unsafe { document.root_frame().as_ref() }) else {
        return QString::default();
    };
    let raw = document.to_raw_text();
    let raw = raw.as_slice();
    let mut selection = Selection {
        document,
        raw,
        start,
        end,
        document_ptr,
        cursor: None,
    };
    let mut text = PlainText::default();
    selection.add_content(&mut text, root, 0, raw.len() as i32);
    QString::from(String::from_utf16_lossy(&text.text))
}
