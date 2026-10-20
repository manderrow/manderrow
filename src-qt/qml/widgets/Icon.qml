import QtQuick
import Manderrow

// A Lucide icon, drawn as a character of the icon font so that it is as sharp as the text beside
// it. It is a square a little larger than the font size it is drawn at, as Lucide draws within a
// margin of its grid: at 1em, its icons look smaller than the text around them.
Text {
    id: root

    /// The icon's Lucide name, such as "x".
    property string name
    /// Draws the icon solid, for the icons that have a filled variant: their shape filled in, with
    /// the details inside it knocked out.
    property bool filled: false
    /// The font size the icon is drawn at.
    property real size: Theme.rem

    text: (filled && Icons.filled[name]) || Icons.icons[name] || ""
    color: Theme.neutral05
    font.family: Icons.family
    // a whole number of pixels, which a font's size is in Qt Quick
    font.pixelSize: Math.round(1.15 * size)
}
