import QtQuick
import Manderrow

// A checkbox whose check mark wipes in, ported from Checkbox.tsx.
Rectangle {
    id: box

    property bool checked: false
    /// The font size, which sets the box's size.
    property real size: Theme.rem

    signal toggled(bool checked)

    // .switchDefault: 1em wide with border-box sizing; its height is the icon's plus the border
    implicitWidth: size
    implicitHeight: size + 2 * Ui.hairline
    radius: 2
    color: checked ? Theme.primary40 : Theme.alpha(Theme.primary40, 0)
    border.color: Theme.neutral60
    border.width: Ui.hairline

    PixelSnap {}

    // the check fills the box inside its border
    CheckMark {
        x: box.border.width
        y: box.border.width
        width: box.width - 2 * box.border.width
        height: box.height - 2 * box.border.width
        checked: box.checked
        fill: true
    }

    TapHandler {
        onTapped: box.toggled(!box.checked)
    }
    HoverHandler {
        cursorShape: Qt.PointingHandCursor
    }
}
