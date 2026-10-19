import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Manderrow

// The task manager, ported from TasksDialog.tsx.
Dialog {
    id: dialog

    maxWidth: 70 * Ui.ch
    height: (Ui.window ? Ui.window.height : 600) * 0.75

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        Text {
            Layout.fillWidth: true
            horizontalAlignment: Text.AlignHCenter
            text: I18n.t("task_manager.title")
            color: Theme.neutral05
            font.family: Theme.font
            font.pixelSize: 1.5 * Theme.rem
            font.weight: Font.Bold
            bottomPadding: 0.5 * Theme.rem
        }
        Rectangle {
            Layout.fillWidth: true
            Layout.bottomMargin: 0.5 * Theme.rem
            implicitHeight: 1
            color: Theme.neutral70
        }

        RowLayout {
            Layout.fillWidth: true

            PillTabs {
                id: tabs
                tabListId: "tasks"
                inset: true
                model: [
                    {
                        id: "active",
                        name: I18n.t("task_manager.active_tab_name")
                    },
                    {
                        id: "completed",
                        name: I18n.t("task_manager.completed_tab_name")
                    }
                ]
            }

            Item {
                Layout.fillWidth: true
            }

            AsyncButton {
                text: "Clear Cache"
                progressMode: "listener"
                action: () => Api.clearCache()
            }
        }

        ListView {
            id: list
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            ScrollBar.vertical: AppScrollBar {}
            model: Tasks.tasksArray.filter(task => task.isComplete !== (tabs.currentId === "active") && task.status.status !== "Unstarted")

            Text {
                visible: list.count === 0
                topPadding: 8
                text: I18n.t("task_manager.no_tasks_yet_msg")
                color: Theme.neutral05
                font.family: Theme.font
                font.pixelSize: Theme.rem
            }

            delegate: ColumnLayout {
                id: entry

                required property var modelData
                required property int index
                readonly property var task: modelData
                readonly property var progress: task.progress
                readonly property bool bytes: task.metadata.progress_unit === "Bytes"

                width: list.width
                spacing: 0

                // a 2px white separator between tasks
                Rectangle {
                    visible: entry.index > 0
                    Layout.fillWidth: true
                    implicitHeight: 2
                    color: "white"
                }

                Item {
                    Layout.preferredHeight: 8
                }

                ProgressBar {
                    visible: !entry.task.isComplete
                    Layout.fillWidth: true
                    Layout.maximumWidth: 12 * Theme.rem
                    progress: entry.progress
                }

                Row {
                    spacing: 0.3 * Theme.rem

                    FaIcon {
                        visible: entry.task.metadata.kind !== "Other"
                        name: entry.task.metadata.kind === "Aggregate" ? "listCheck" : "download"
                        anchors.verticalCenter: parent.verticalCenter
                    }
                    Text {
                        text: entry.task.metadata.title
                        color: Theme.neutral05
                        font.family: Theme.font
                        font.pixelSize: Theme.rem
                        font.weight: Font.Bold
                        width: Math.min(implicitWidth, list.width - 2 * Theme.rem)
                        wrapMode: Text.WrapAnywhere
                    }
                }

                // the status, percentage, size, and success info, separated by dots
                Text {
                    Layout.fillWidth: true
                    wrapMode: Text.Wrap
                    color: Theme.neutral05
                    font.family: Theme.font
                    font.pixelSize: Theme.rem
                    text: {
                        const parts = [];
                        const task = entry.task;
                        const progress = entry.progress;
                        if (task.status.status !== "Running" || progress.total === 0)
                            parts.push(task.status.status);
                        if (!task.isComplete && progress.total !== 0)
                            parts.push(Util.formatRounded(progress.completed / progress.total * 100) + "%");
                        if (entry.bytes) {
                            if (task.isComplete && progress.completed === progress.total)
                                parts.push(Util.humanizeFileSize(progress.completed));
                            else
                                parts.push(Util.humanizeFileSize(progress.completed) + " / " + Util.humanizeFileSize(progress.total));
                        }
                        if (task.status.status === "Success" && task.status.success)
                            parts.push(task.status.success);
                        return parts.join(" · ");
                    }
                }

                Row {
                    visible: entry.task.metadata.kind === "Download"
                    spacing: 8

                    HoverHandler {
                        id: urlHover
                    }

                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        text: I18n.t("task_manager.source_label")
                        color: sourceHover.hovered ? Theme.primary70 : Theme.primary60
                        font.family: Theme.font
                        font.pixelSize: Theme.rem

                        HoverHandler {
                            id: sourceHover
                            cursorShape: Qt.PointingHandCursor
                        }
                        TapHandler {
                            onTapped: Util.openUrl(entry.task.metadata.url)
                        }
                        Tip {
                            text: entry.task.metadata.url || ""
                            visible: sourceHover.hovered
                        }
                    }

                    Btn {
                        opacity: urlHover.hovered ? 1 : 0
                        width: 2 * Theme.rem
                        height: width
                        padAll: 0
                        iconName: "copy"
                        onClicked: Native.copyText(entry.task.metadata.url)
                    }
                }

                Item {
                    Layout.preferredHeight: 8
                }
            }
        }
    }
}
