pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Manderrow

// The task manager, ported from TasksDialog.tsx.
Dialog {
    id: dialog

    maxWidth: 70 * Ui.ch

    /// The names of the tasks' statuses, as they're shown.
    readonly property var statusNames: ({
            [Task.Unstarted]: "Unstarted",
            [Task.Running]: "Running",
            [Task.Succeeded]: "Success",
            [Task.Failed]: "Failed",
            [Task.Cancelled]: "Cancelled"
        })
    height: (Ui.window ? Ui.window.height : 600) * 0.75

    ColumnLayout {
        // not anchored to the body, whose implicit size is that of its children
        width: parent.width
        height: dialog.availableHeight
        spacing: 0

        SText {
            Layout.fillWidth: true
            horizontalAlignment: TextEdit.AlignHCenter
            text: Strings.task_manager.title
            color: Theme.neutral05
            font.family: Theme.font
            font.pixelSize: 1.5 * Theme.rem
            font.weight: Font.Bold
            font.variableAxes: ({ "wght": Font.Bold })
            bottomPadding: 0.5 * Theme.rem
        }
        Rectangle {
            Layout.fillWidth: true
            Layout.bottomMargin: 0.5 * Theme.rem
            implicitHeight: Ui.hairline
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
                        name: Strings.task_manager.active_tab_name
                    },
                    {
                        id: "completed",
                        name: Strings.task_manager.completed_tab_name
                    }
                ]
            }

            Item {
                Layout.fillWidth: true
            }

            AsyncButton {
                text: "Clear Cache"
                action: () => App.clearCache()
            }
        }

        ListView {
            id: list
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            ScrollBar.vertical: AppScrollBar {
                id: bar
            }
            WheelScroll {}
            // one stop in the Tab order, whose items the arrow keys move between
            FocusGroup {
                view: list
            }
            model: Tasks.all.filter(task => (task as Task).isComplete !== (tabs.currentId === "active") && (task as Task).status !== Task.Unstarted)

            SText {
                visible: list.count === 0
                topPadding: 8
                text: Strings.task_manager.no_tasks_yet_msg
                color: Theme.neutral05
                font.family: Theme.font
                font.pixelSize: Theme.rem
            }

            delegate: ColumnLayout {
                id: entry

                required property Task modelData
                required property int index
                readonly property Task task: modelData
                readonly property bool bytes: task.progressUnit === Task.Bytes

                width: list.width - bar.gutter
                spacing: 0

                // a 2px white separator between tasks
                Rectangle {
                    visible: entry.index > 0
                    Layout.fillWidth: true
                    implicitHeight: Ui.snapBorder(2)
                    color: "white"
                }

                Item {
                    Layout.preferredHeight: 8
                }

                ProgressBar {
                    visible: !entry.task.isComplete
                    Layout.fillWidth: true
                    Layout.maximumWidth: 12 * Theme.rem
                    completed: entry.task.completed
                    total: entry.task.total
                }

                Row {
                    spacing: 0.3 * Theme.rem

                    Icon {
                        visible: entry.task.kind !== Task.Other
                        name: entry.task.kind === Task.Aggregate ? "list-checks" : "download"
                        anchors.verticalCenter: parent.verticalCenter
                    }
                    SText {
                        text: entry.task.title
                        color: Theme.neutral05
                        font.family: Theme.font
                        font.pixelSize: Theme.rem
                        font.weight: Font.Bold
                        font.variableAxes: ({ "wght": Font.Bold })
                        width: Math.min(implicitWidth, list.width - 2 * Theme.rem)
                        wrapMode: TextEdit.WrapAnywhere
                    }
                }

                // the status, percentage, size, and success info, separated by dots
                SText {
                    Layout.fillWidth: true
                    wrapMode: TextEdit.Wrap
                    color: Theme.neutral05
                    font.family: Theme.font
                    font.pixelSize: Theme.rem
                    text: {
                        const parts = [];
                        const task = entry.task;
                        if (task.status !== Task.Running || task.total === 0)
                            parts.push(dialog.statusNames[task.status]);
                        if (!task.isComplete && task.total !== 0)
                            parts.push(Util.formatRounded(task.completed / task.total * 100) + "%");
                        if (entry.bytes) {
                            if (task.isComplete && task.completed === task.total)
                                parts.push(Util.humanizeFileSize(task.completed));
                            else
                                parts.push(Util.humanizeFileSize(task.completed) + " / " + Util.humanizeFileSize(task.total));
                        }
                        if (task.cached)
                            parts.push("Cached");
                        return parts.join(" · ");
                    }
                }

                Row {
                    visible: entry.task.kind === Task.Download
                    spacing: 8

                    HoverHandler {
                        id: urlHover
                    }

                    SText {
                        anchors.verticalCenter: parent.verticalCenter
                        text: Strings.task_manager.source_label
                        color: sourceHover.hovered ? Theme.primary70 : Theme.primary60
                        font.family: Theme.font
                        font.pixelSize: Theme.rem

                        ClickArea {
                            id: sourceHover
                            activatesOnSpace: false
                            onActivated: Util.openUrl(entry.task.url)
                        }
                        Tip {
                            text: entry.task.url
                            shown: sourceHover.hovered
                        }
                    }

                    Btn {
                        opacity: urlHover.hovered ? 1 : 0
                        width: 2 * Theme.rem
                        height: width
                        padAll: 0
                        iconName: "copy"
                        onClicked: Native.copyText(entry.task.url)
                    }
                }

                Item {
                    Layout.preferredHeight: 8
                }
            }
        }
    }
}
