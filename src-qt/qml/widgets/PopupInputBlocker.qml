import QtQuick

// Keeps pointer input over a popup from reaching the items behind it. Qt Quick delivers input to
// each item under the pointer until it is accepted, but popups only get it as far as their pointer
// handlers, which mostly grab it passively, so a press on a popup would otherwise also act on what
// is behind it. Placed in a popup's background, this is below the popup's own controls, which still
// get the input first.
Item {
    id: blocker

    /// Whether presses are blocked, as well as hovering and scrolling.
    property bool blockPresses: true

    anchors.fill: parent

    TapHandler {
        enabled: blocker.blockPresses
        acceptedButtons: Qt.AllButtons
        // takes the exclusive grab on press, which stops the press there
        gesturePolicy: TapHandler.WithinBounds
        grabPermissions: PointerHandler.TakeOverForbidden
    }
    HoverHandler {
        blocking: true
    }
    WheelHandler {
        blocking: true
    }
}
