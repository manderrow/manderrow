import QtQuick
import Manderrow

// A check mark revealed from left to right, like the web UI's
// `clip-path: inset(0 100% 0 0)` to `inset(0)` transition. The icon itself doesn't move.
Item {
    id: mark

    property bool checked: false
    property real size: Theme.rem
    property color color: Theme.neutral05
    /// Hides the mark at once when unchecked instead of wiping it out, for
    /// `visibility: hidden` on unchecked.
    property bool hideWhenUnchecked: false

    implicitWidth: icon.implicitWidth
    implicitHeight: icon.implicitHeight

    Item {
        width: parent.width * (mark.checked ? 1 : 0)
        height: parent.height
        clip: true
        visible: mark.checked || !mark.hideWhenUnchecked

        Behavior on width {
            // always enabled, since toggling it with `checked` would race the width's binding
            SmoothNumberAnimation {
                duration: 200
            }
        }

        Icon {
            id: icon
            name: "check"
            size: mark.size
            color: mark.color
        }
    }
}
