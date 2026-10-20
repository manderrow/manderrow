import QtQuick
import Manderrow

// The default contents of a Btn: an optional icon and a label.
Item {
    id: content

    required property Btn btn

    implicitWidth: row.implicitWidth
    // at least a line of text tall, like the line box around a web button's icon
    implicitHeight: Math.max(row.implicitHeight, metrics.height)

    FontMetrics {
        id: metrics
        font: content.btn.font
    }

    Row {
        id: row
        spacing: content.btn.iconName !== "" && content.btn.text !== "" ? content.btn.spacing : 0
        layoutDirection: content.btn.iconTrailing ? Qt.RightToLeft : Qt.LeftToRight
        x: content.btn.horizontalAlignment === Qt.AlignLeft ? 0 : (parent.width - width) / 2
        anchors.verticalCenter: parent.verticalCenter

        Icon {
            name: content.btn.iconName
            filled: content.btn.iconFilled
            visible: content.btn.iconName !== ""
            size: content.btn.iconSize
            color: content.btn.currentTextColor
            rotation: content.btn.iconRotation
            anchors.verticalCenter: parent.verticalCenter

            Behavior on rotation {
                SmoothNumberAnimation {
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
                SmoothColorAnimation {
                    duration: 100
                }
            }
        }
    }
}
