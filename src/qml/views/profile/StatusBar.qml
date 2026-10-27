import QtQuick
import Manderrow

// The bar along the bottom of the profile view, ported from StatusBar.tsx.
Item {
    id: bar
    objectName: "statusBar"

    /// Where the bar's background starts, after what covers the bar's start. The background isn't
    /// drawn under it, so that it doesn't show through when the window is translucent. The
    /// contents stay centered in the whole bar.
    property real leftInset: 0

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

    implicitHeight: row.implicitHeight + 2 * 0.2 * Theme.rem + Ui.hairline

    Rectangle {
        x: bar.leftInset
        width: parent.width - x
        height: parent.height
        color: Theme.surface(Theme.neutral90)

        Rectangle {
            width: parent.width
            height: Ui.hairline
            color: Theme.neutral80
        }
    }

    ClickArea {
        id: tasksHover
        anchors.fill: row
        onActivated: tasksDialog.open()
    }

    Row {
        id: row
        anchors.centerIn: parent
        anchors.verticalCenterOffset: Ui.hairline / 2

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
                width: Ui.hairline
                height: parent.height
                color: Theme.neutral70
            }
        }

        Chunk {
            first: true
            text: Strings.status_bar.downloads_tracker(bar.counts.downloads)
        }
        Chunk {
            last: true
            text: Strings.status_bar.other_tasks_tracker(bar.counts.other)
        }
    }

    PopupLoader {
        id: tasksDialog
        sourceComponent: TasksDialog {}
    }
}
