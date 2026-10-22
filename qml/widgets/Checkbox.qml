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
    implicitHeight: size + 2
    radius: 2
    color: checked ? Theme.primary40 : Theme.alpha(Theme.primary40, 0)
    border.color: Theme.neutral60
    border.width: 1

    // the icon sits at the start of the box, like in the web UI's flex container
    CheckMark {
        x: 1
        y: 1
        checked: box.checked
        size: box.size
    }

    TapHandler {
        onTapped: box.toggled(!box.checked)
    }
    HoverHandler {
        cursorShape: Qt.PointingHandCursor
    }
}
