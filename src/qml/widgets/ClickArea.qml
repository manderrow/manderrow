import QtQuick
import Manderrow

// Makes its parent clickable like a web link or button: it can be tapped, or focused with Tab and
// activated with Enter (and Space, for buttons), showing a FocusRing while focused from the
// keyboard.
//
// It fills its parent. Declare it before the parent's other children: it is then under them, so
// controls among them still get their own hovering and presses.
Item {
    id: area

    /// Tapped, or activated from the keyboard.
    signal activated

    readonly property alias hovered: hover.hovered
    readonly property alias pressed: tap.pressed
    property int cursorShape: Qt.PointingHandCursor
    /// Whether Space activates it, as it does buttons but not links.
    property bool activatesOnSpace: true
    /// The focus ring's corner radius.
    property alias radius: ring.radius

    anchors.fill: parent
    activeFocusOnTab: enabled

    HoverHandler {
        id: hover
        cursorShape: area.enabled ? area.cursorShape : Qt.ArrowCursor
    }
    TapHandler {
        id: tap
        onTapped: area.activated()
    }

    Keys.onReturnPressed: area.activated()
    Keys.onEnterPressed: area.activated()
    Keys.onSpacePressed: event => {
        if (area.activatesOnSpace)
            area.activated();
        else
            event.accepted = false;
    }

    FocusRing {
        id: ring
    }
}
