import QtQuick
import Manderrow

// The web UI's keyboard focus outline, `*:focus-visible { outline: 2px solid warn-5;
// outline-offset: -2px }`, drawn over its parent while that is focused from the keyboard. Set
// the radii to follow the parent's corners.
Rectangle {
    id: ring

    /// The focused item, by default the parent.
    property Item target: parent
    /// Whether the outline shows.
    readonly property bool shown: !!target && Window.activeFocusItem === target && Ui.focusVisible

    anchors.fill: parent
    color: "transparent"
    border.color: Theme.warn5
    border.width: 2
    visible: shown
    z: 100
}
