import QtQuick
import QtQuick.Controls.Basic
import Manderrow

// A list of selectable options with check marks, ported from SelectDropdownList.
//
// Each option is `{ label, value, selected }`, with an optional `secondary` line of text shown
// below the label.
//
// It is one stop in the Tab order, entered at the selected option, and the arrow keys move between
// its options, as in a listbox.
ListView {
    id: list

    property var options: []
    property real fontSize: Theme.rem
    property real itemPadding: 0.3 * fontSize
    property real maxHeight: Infinity
    /// Where the list was scrolled to show the selected option in its middle.
    property real _centeredY: NaN

    signal changed(var value, bool selected)

    implicitWidth: {
        let widest = 0;
        for (const option of options) {
            widest = Math.max(widest, (option.secondary !== undefined ? boldMetrics : metrics).advanceWidth(option.label));
            if (option.secondary !== undefined)
                widest = Math.max(widest, secondaryMetrics.advanceWidth(option.secondary));
        }
        // padding, check icon, and spacing, and the scroll bar's gutter beside the options
        return Math.ceil(widest + 2 * 0.5 * fontSize + 0.875 * fontSize + 0.5 * fontSize) + 1 + bar.gutter;
    }
    implicitHeight: Math.min(contentHeight, maxHeight)
    model: options
    clip: true
    boundsBehavior: Flickable.StopAtBounds
    // FocusNavigator moves the focus between the options
    keyNavigationEnabled: false
    Accessible.role: Accessible.List
    ScrollBar.vertical: AppScrollBar {
        id: bar
    }
    WheelScroll {}
    FocusGroup {
        id: group
        view: list
    }
    // The selected option shows in the middle of the list, which keeps it there as it takes its
    // size, unless it has scrolled since.
    Component.onCompleted: Qt.callLater(scrollToSelected)
    onHeightChanged: {
        if (contentY === _centeredY)
            scrollToSelected();
    }

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
        font.variableAxes: ({ "wght": 600 })
    }
    FontMetrics {
        id: secondaryMetrics
        font.family: Theme.font
        font.pixelSize: 0.8 * Theme.rem
    }

    /// Scrolls the selected option into the middle of the list, and makes it where Tab enters
    /// the list. Returns it, or the first option if none is selected.
    function scrollToSelected() {
        const index = options.findIndex(option => option.selected);
        if (index >= 0)
            positionViewAtIndex(index, ListView.Center);
        forceLayout();
        _centeredY = contentY;
        const item = itemAtIndex(Math.max(index, 0));
        if (item)
            group.current = item;
        return item;
    }

    /// Where the focus goes when a Dropdown showing the list opens: the selected option.
    function initialFocusItem() {
        return scrollToSelected();
    }

    delegate: Rectangle {
        id: option

        required property var modelData
        required property int index

        width: list.width - bar.gutter
        implicitHeight: row.implicitHeight + 2 * list.itemPadding
        // only while the focus shows, as with the pointer it would also highlight the selected
        // option, which a list takes the focus at
        color: hover.hovered || (activeFocus && Ui.focusVisible) ? Theme.neutral70 : "transparent"
        activeFocusOnTab: true
        Accessible.role: Accessible.ListItem
        Accessible.name: modelData.label
        Accessible.description: modelData.secondary || ""
        Accessible.selectable: true
        Accessible.selected: !!modelData.selected
        Accessible.onPressAction: select()

        function select() {
            list.changed(modelData.value, !modelData.selected);
        }

        FocusRing {}
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

            Icon {
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
                    font.variableAxes: ({ "wght": option.modelData.secondary !== undefined ? 600 : Font.Normal })
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
