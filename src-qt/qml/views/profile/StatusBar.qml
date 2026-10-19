import QtQuick
import Manderrow

// The bar along the bottom of the profile view, ported from StatusBar.tsx.
Rectangle {
    id: bar
    objectName: "statusBar"

    function openTasks() {
        tasksDialog.open();
    }

    readonly property var counts: {
        let downloads = 0;
        let other = 0;
        for (const task of Tasks.tasksArray) {
            if (task.isComplete || task.status.status === "Unstarted")
                continue;
            if (task.metadata.kind === "Download")
                downloads++;
            else
                other++;
        }
        return { downloads, other };
    }

    implicitHeight: row.implicitHeight + 2 * 0.2 * Theme.rem + 1
    color: Theme.neutral90

    Rectangle {
        width: parent.width
        height: 1
        color: Theme.neutral80
    }

    Row {
        id: row
        anchors.centerIn: parent
        anchors.verticalCenterOffset: 0.5

        component Chunk: Item {
            property alias text: label.text
            property bool last: false
            property bool first: false
            implicitWidth: label.implicitWidth + ((first ? 0.3 : 1) + (last ? 0.3 : 1)) * Theme.rem
            implicitHeight: label.implicitHeight + 2 * 0.1 * Theme.rem

            Text {
                id: label
                x: (parent.first ? 0.3 : 1) * Theme.rem
                anchors.verticalCenter: parent.verticalCenter
                color: tasksHover.hovered ? Theme.neutral10 : Theme.neutral20
                font.family: Theme.font
                font.pixelSize: 0.8 * Theme.rem
            }

            Rectangle {
                visible: !parent.last
                anchors.right: parent.right
                width: 1
                height: parent.height
                color: Theme.neutral70
            }
        }

        Chunk {
            first: true
            text: I18n.t("status_bar.downloads_tracker", { count: bar.counts.downloads })
        }
        Chunk {
            last: true
            text: I18n.t("status_bar.other_tasks_tracker", { count: bar.counts.other })
        }

        HoverHandler {
            id: tasksHover
            cursorShape: Qt.PointingHandCursor
        }
        TapHandler {
            onTapped: tasksDialog.open()
        }
    }

    TasksDialog {
        id: tasksDialog
    }
}
