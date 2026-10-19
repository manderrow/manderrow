import QtQuick
import Manderrow

// The "moving-bg" tab list of TabRenderer.tsx: pill tabs whose highlight slides to the selected
// tab. The selection is stored in the route's search parameters under `<id>-tab`.
Item {
    id: tabs

    /// The tab list id, used for the search parameter.
    property string tabListId
    /// `[{ id, name }]`. The first tab is the default.
    property var model: []
    /// The inset style of the task manager's tabs, without separators.
    property bool inset: false

    readonly property string currentId: {
        const id = Router.search[tabListId + "-tab"];
        return id !== undefined ? id : model.length > 0 ? model[0].id : "";
    }
    readonly property int currentIndex: model.findIndex(tab => tab.id === currentId)

    function select(id) {
        const params = {};
        params[tabListId + "-tab"] = id;
        Router.setSearch(params);
    }

    implicitWidth: row.implicitWidth + 2 * row.x
    implicitHeight: row.implicitHeight + 2 * row.y

    // the inset well: `box-shadow: 0 0 8px -2px inset rgb(0 0 0 / 0.6)`
    Rectangle {
        visible: tabs.inset
        anchors.fill: parent
        radius: 8
        color: Theme.lighten(Theme.neutral80, -2)
        border.color: Qt.rgba(0, 0, 0, 0.35)
        border.width: 1
    }

    Rectangle {
        id: highlight
        // depends on the count, since itemAt doesn't notify when delegates are created
        readonly property Item target: repeater.count > tabs.currentIndex && tabs.currentIndex >= 0 ? repeater.itemAt(tabs.currentIndex) : null
        visible: target !== null
        x: target ? row.x + target.x : 0
        y: row.y
        width: target ? target.width : 0
        height: row.height
        radius: 6
        color: Theme.primary40

        Behavior on x {
            NumberAnimation {
                duration: 200
            }
        }
        Behavior on width {
            NumberAnimation {
                duration: 200
            }
        }
    }

    Row {
        id: row
        x: tabs.inset ? 0.2 * Theme.rem : 0
        y: x

        Repeater {
            id: repeater
            model: tabs.model

            delegate: Btn {
                id: tab

                required property var modelData
                required property int index
                readonly property bool isCurrent: index === tabs.currentIndex
                // the separator is hidden next to the selected tab
                readonly property bool separated: !tabs.inset && !isCurrent && index + 1 !== tabs.currentIndex

                variant: "plain"
                radius: 6
                text: modelData.name
                fontWeight: Font.Medium
                horizontalPadding: fontSize
                verticalPadding: 0.5 * fontSize
                width: Math.max(implicitWidth, 8 * Ui.ch)
                textColor: isCurrent ? "white" : Theme.neutral40
                activeTextColor: isCurrent ? "white" : Theme.neutral20
                activeBgColor: "transparent"
                onClicked: tabs.select(modelData.id)

                Rectangle {
                    visible: tab.separated
                    anchors.right: parent.right
                    width: 1
                    height: parent.height
                    color: Theme.neutral70
                }
            }
        }
    }
}
