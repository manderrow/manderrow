import QtQuick
import Manderrow

// The "base" tab list of TabRenderer.tsx, as styled by the mod view: tabs underlined when
// selected. The selection is stored in the route's search parameters under `<id>-tab`.
Item {
    id: tabs

    property string tabListId
    /// `[{ id, name }]`. The first tab is the default.
    property var model: []

    readonly property string currentId: {
        const id = Router.search[tabListId + "-tab"];
        return id !== undefined ? id : model.length > 0 ? model[0].id : "";
    }

    implicitWidth: row.implicitWidth
    implicitHeight: row.implicitHeight + 1

    Rectangle {
        anchors.bottom: parent.bottom
        width: parent.width
        height: 1
        color: Theme.neutral70
    }

    Row {
        id: row

        Repeater {
            model: tabs.model

            delegate: Btn {
                required property var modelData
                readonly property bool isCurrent: modelData.id === tabs.currentId

                variant: "plain"
                radius: 0
                text: modelData.name
                horizontalPadding: fontSize
                topPadding: 0.4 * fontSize
                bottomPadding: 0.4 * fontSize + 2
                textColor: isCurrent ? "white" : Theme.neutral40
                activeTextColor: isCurrent ? "white" : Theme.neutral20
                activeBgColor: "transparent"
                onClicked: {
                    const params = {};
                    params[tabs.tabListId + "-tab"] = modelData.id;
                    Router.setSearch(params);
                }

                Rectangle {
                    anchors.bottom: parent.bottom
                    width: parent.width
                    height: 2
                    color: parent.isCurrent ? Theme.neutral20 : "transparent"
                }
            }
        }
    }
}
