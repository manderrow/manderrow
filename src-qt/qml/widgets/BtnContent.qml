import QtQuick
import Manderrow

// The default contents of a Btn: an optional icon and a label.
Item {
    id: content

    required property Btn btn

    implicitWidth: row.implicitWidth
    implicitHeight: row.implicitHeight

    Row {
        id: row
        spacing: content.btn.iconName !== "" && content.btn.text !== "" ? content.btn.spacing : 0
        layoutDirection: content.btn.iconTrailing ? Qt.RightToLeft : Qt.LeftToRight
        x: content.btn.horizontalAlignment === Qt.AlignLeft ? 0 : (parent.width - width) / 2
        anchors.verticalCenter: parent.verticalCenter

        FaIcon {
            name: content.btn.iconName
            visible: content.btn.iconName !== ""
            size: content.btn.iconSize
            color: content.btn.currentTextColor
            rotation: content.btn.iconRotation
            anchors.verticalCenter: parent.verticalCenter

            Behavior on rotation {
                NumberAnimation {
                    duration: 200
                }
            }
        }

        Text {
            text: content.btn.text
            visible: content.btn.text !== ""
            font: content.btn.font
            color: content.btn.currentTextColor
            anchors.verticalCenter: parent.verticalCenter

            Behavior on color {
                ColorAnimation {
                    duration: 100
                }
            }
        }
    }
}
