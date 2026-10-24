import QtQuick
import Manderrow

// The background and border of a box with contents, filling its parent and snapped to device
// pixels. The box's contents can't be snapped with it (see PixelSnap), so the box is drawn by this
// rather than by the box itself.
Rectangle {
    anchors.fill: parent
    color: "transparent"
    border.width: Ui.hairline

    PixelSnap {}
}
