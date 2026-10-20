import QtQuick
import QtQuick.Controls.Basic
import Manderrow

// A list of selectable options with check marks, ported from SelectDropdownList.
//
// Each option is `{ label, value, selected }`, with an optional `secondary` line of text shown
// below the label.
ListView {
    id: list

    property var options: []
    property real fontSize: Theme.rem
    property real itemPadding: 0.3 * fontSize
    property real maxHeight: Infinity

    signal changed(var value, bool selected)

    implicitWidth: {
        let widest = 0;
        for (const option of options) {
            widest = Math.max(widest, (option.secondary !== undefined ? boldMetrics : metrics).advanceWidth(option.label));
            if (option.secondary !== undefined)
                widest = Math.max(widest, secondaryMetrics.advanceWidth(option.secondary));
        }
        // padding, check icon, and spacing
        return Math.ceil(widest + 2 * 0.5 * fontSize + 0.875 * fontSize + 0.5 * fontSize) + 1;
    }
    implicitHeight: Math.min(contentHeight, maxHeight)
    model: options
    clip: true
    boundsBehavior: Flickable.StopAtBounds
    keyNavigationEnabled: true
    ScrollBar.vertical: AppScrollBar {}
    WheelScroll {}

    FontMetrics {
        id: metrics
        font.family: Theme.font
        font.pixelSize: list.fontSize
    }
    FontMetrics {
        id: boldMetrics
        font.family: Theme.font
        font.pixelSize: list.fontSize
        font.weight: 600
    }
    FontMetrics {
        id: secondaryMetrics
        font.family: Theme.font
        font.pixelSize: 0.8 * Theme.rem
    }

    function scrollToSelected() {
        for (let i = 0; i < options.length; i++) {
            if (options[i].selected) {
                positionViewAtIndex(i, ListView.Center);
                return;
            }
        }
    }

    delegate: Rectangle {
        id: option

        required property var modelData
        required property int index

        width: list.width
        implicitHeight: row.implicitHeight + 2 * list.itemPadding
        color: hover.hovered || activeFocus ? Theme.neutral70 : "transparent"

        function select() {
            list.changed(modelData.value, !modelData.selected);
        }

        HoverHandler {
            id: hover
            cursorShape: Qt.PointingHandCursor
        }
        TapHandler {
            onTapped: option.select()
        }
        Keys.onReturnPressed: select()
        Keys.onSpacePressed: select()

        Row {
            id: row
            x: 0.5 * list.fontSize
            anchors.verticalCenter: parent.verticalCenter
            spacing: 0.5 * list.fontSize

            FaIcon {
                name: "check"
                size: list.fontSize
                opacity: option.modelData.selected ? 1 : 0
                anchors.verticalCenter: parent.verticalCenter
            }

            Column {
                anchors.verticalCenter: parent.verticalCenter

                Text {
                    text: option.modelData.label
                    color: Theme.neutral05
                    font.family: Theme.font
                    font.pixelSize: list.fontSize
                    font.weight: option.modelData.secondary !== undefined ? 600 : Font.Normal
                }

                Text {
                    visible: option.modelData.secondary !== undefined
                    text: option.modelData.secondary || ""
                    color: Theme.neutral30
                    font.family: Theme.font
                    font.pixelSize: 0.8 * Theme.rem
                    topPadding: 0.25 * list.fontSize
                }
            }
        }
    }
}
