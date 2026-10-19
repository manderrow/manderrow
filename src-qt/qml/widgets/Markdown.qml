import QtQuick
import Manderrow

// Renders GitHub-flavoured Markdown, styled like the web UI's Markdown.css.
MarkdownBlocks {
    property string source

    blocks: JSON.parse(Backend.markdownBlocks(source))
    first: true
}
