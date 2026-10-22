import QtQuick
import QtQuick.Controls.Basic
import Manderrow

// The default contents of a Btn: an optional icon and a label.
Item {
    id: content

    /// The Btn, typed as its base: Btn instantiates this, and Qt 6.10 can't load a type with a
    /// property of a type that instantiates it (6.11 can).
    required property AbstractButton btn

    /// The width taken by the icon and the space after it.
    readonly property real _iconWidth: icon.visible ? icon.width + row.spacing : 0
    /// Its width with the label wrapped as narrow as it goes, if the Btn wraps it.
    readonly property real minimumWidth: !btn.wrapText ? implicitWidth : _iconWidth + Util.longestWordWidth(metrics, btn.text)

    // unwrapped, so that the label's wrapping doesn't narrow it, and rounded up, so that layouts,
    // which round widths to whole pixels, don't wrap it for want of a fraction of one
    implicitWidth: btn.wrapText ? Math.ceil(_iconWidth + label.implicitWidth) : row.implicitWidth
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
            id: icon
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
            id: label
            // the label wraps to the space beside the icon, but no narrower than its longest word
            width: content.btn.wrapText ? Math.max(Math.min(Math.ceil(implicitWidth), content.width - content._iconWidth), content.minimumWidth - content._iconWidth) : implicitWidth
            wrapMode: content.btn.wrapText ? Text.WordWrap : Text.NoWrap
            horizontalAlignment: Text.AlignHCenter
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
