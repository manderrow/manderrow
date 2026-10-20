import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Effects
import Manderrow

// A tooltip styled like the web UI's corvu tooltips. Place it inside the item it describes.
ToolTip {
    id: tip

    property bool hideArrow: false
    readonly property real _offset: 6
    readonly property real _maxWidth: 200
    property bool _below: false

    delay: 50
    padding: 0.25 * Theme.rem
    margins: 4
    x: (parent ? parent.width - width : 0) / 2
    y: _below ? (parent ? parent.height : 0) + _offset : -height - _offset

    onAboutToShow: {
        // flip below the target when there isn't room above it
        _below = parent ? parent.mapToItem(null, 0, 0).y - height - _offset < Theme.titlebarHeight : false;
    }

    enter: Transition {
        NumberAnimation {
            property: "opacity"
            from: 0
            to: 1
            duration: 100
            easing.type: Easing.Bezier
            easing.bezierCurve: Theme.ease
        }
    }
    exit: null

    contentItem: Text {
        text: tip.text
        color: Theme.neutral10
        font.family: Theme.font
        font.pixelSize: 0.9 * Theme.rem
        font.weight: 320
        // Qt rounds weights to the font's named instances otherwise
        font.variableAxes: ({ "wght": 320 })
        wrapMode: Text.Wrap
        width: Math.min(implicitWidth, tip._maxWidth - 2 * tip.padding - 2)
    }

    background: Item {
        RectangularShadow {
            anchors.fill: bg
            radius: bg.radius
            blur: 0.5 * Theme.rem
            color: Qt.rgba(0, 0, 0, 0.3)
        }

        Rectangle {
            id: bg
            anchors.fill: parent
            color: Theme.neutral80
            border.color: Theme.neutral70
            radius: 4
        }

        // the arrow, pointing at the target
        Rectangle {
            visible: !tip.hideArrow
            width: 8
            height: 8
            rotation: 45
            color: Theme.neutral80
            x: (parent.width - width) / 2
            y: tip._below ? -height / 2 : parent.height - height / 2
            z: -1
        }
    }
}
