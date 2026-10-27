pragma ComponentBehavior: Bound
import QtQuick
import Manderrow

// A button that runs an asynchronous action, ported from SimpleAsyncButton in AsyncButton.tsx.
//
// While the action runs, the button is disabled and can show its progress. If the action fails,
// the button is replaced by an ErrorIndicator until the error is dismissed.
Item {
    id: root

    /// The action, which may return a call or a promise. It receives a function to give the task
    /// whose progress to show, for a call that doesn't return its task.
    property var action: function () {}
    /// "" for no progress, "task" for the progress of the action's task, or "external" for
    /// `externalTask`'s.
    property string progressMode: ""
    property Task externalTask: null
    /// Shows the progress of the task's download rather than its own, as an installation's.
    property bool downloadProgress: false
    /// "in-place" (fill the button), "circular", or "simple" (a progress bar).
    property string progressStyle: "in-place"
    /// Marks the button busy regardless of the action.
    property bool externalBusy: false
    /// Removes the padding and background while busy, as the busy download buttons do.
    property bool busyPlain: false
    property color progressFromColor: Theme.primary40
    property color progressToColor: Theme.primary70
    property color progressBorderColor: Theme.primary40

    property alias btn: button
    property alias text: button.text
    property alias iconName: button.iconName
    property alias variant: button.variant
    property alias tooltip: button.tooltip
    property alias fontSize: button.fontSize

    readonly property bool busy: externalBusy || _busy
    /// The task whose progress is shown.
    readonly property Task task: progressMode === "external" ? externalTask : _task && downloadProgress ? _task.download : _task
    readonly property real completed: task ? task.completed : 0
    readonly property real total: task ? task.total : 0
    readonly property bool showProgress: progressMode !== "" && busy
    readonly property real percent: total === 0 ? 0 : completed / total * 100
    readonly property bool _inPlace: showProgress && progressStyle === "in-place"

    property bool _busy: false
    property Task _task: null
    property var _err: null

    implicitWidth: errorIndicator.item ? (errorIndicator.item as Item).implicitWidth : button.implicitWidth
    implicitHeight: errorIndicator.item ? (errorIndicator.item as Item).implicitHeight : button.implicitHeight
    /// Its min-content width, with the button's label wrapped if it wraps. See Btn.minimumWidth.
    readonly property real minimumWidth: errorIndicator.item ? implicitWidth : button.minimumWidth

    function run() {
        _busy = true;
        _task = null;
        let result;
        try {
            result = action(task => {
                // the button may have been destroyed by the time this runs
                if (root)
                    root._task = task;
            });
        } catch (e) {
            result = Promise.reject(e);
        }
        if (Async.isCall(result) && (result as Call).taskId >= 0)
            _task = Tasks.get((result as Call).taskId);
        Async.resolve(result).then(() => {
            if (root)
                root._busy = false;
        }, e => {
            console.error(e, e && e.stack);
            if (!root)
                return;
            root._err = e;
            root._busy = false;
        });
    }

    Btn {
        id: button
        visible: root._err === null
        anchors.fill: parent
        enabled: !root.busy
        // busy buttons are not dimmed when they show progress in place of their label
        opacity: enabled || (root.busy && (root.busyPlain || root.showProgress)) ? 1 : 0.5
        bgColor: root._inPlace || (root.busyPlain && root.busy) ? "transparent" : variant === "primary" ? Theme.primary40 : variant === "default" ? Theme.neutral80 : "transparent"
        borderColor: root._inPlace ? root.progressBorderColor : variant === "ghost" ? Theme.neutral70 : "transparent"
        borderWidth: root._inPlace || variant === "ghost" ? 1 : 0
        // undefined resets each side to the horizontal/vertical padding
        leftPadding: root.busyPlain && root.busy ? 0 : undefined
        rightPadding: root.busyPlain && root.busy ? 0 : undefined
        topPadding: root.busyPlain && root.busy ? 0 : undefined
        bottomPadding: root.busyPlain && root.busy ? 0 : undefined
        onClicked: root.run()

        contentItem: Item {
            implicitWidth: circular.item ? circular.implicitWidth : bar.item ? bar.implicitWidth : label.implicitWidth
            implicitHeight: circular.item ? circular.implicitHeight : label.implicitHeight
            readonly property real minimumWidth: circular.item || bar.item ? implicitWidth : label.minimumWidth

            BtnContent {
                id: label
                btn: button
                anchors.fill: parent
                // keeps its width while hidden, like the web UI's width retainer
                opacity: root.showProgress ? 0 : 1
            }

            Text {
                visible: root._inPlace
                anchors.centerIn: parent
                text: Util.formatRounded(root.percent) + "%"
                color: Theme.neutral05
                font: button.font
                style: Text.Raised
                styleColor: Qt.rgba(0, 0, 0, 0.3)
                z: 1
            }

            Loader {
                id: circular
                anchors.centerIn: parent
                active: root.showProgress && root.progressStyle === "circular"
                sourceComponent: CircularProgress {
                    completed: root.completed
                    total: root.total
                    size: button.fontSize
                }
            }

            Loader {
                id: bar
                anchors.verticalCenter: parent.verticalCenter
                width: parent.width
                active: root.showProgress && root.progressStyle === "simple"
                sourceComponent: ProgressBar {
                    completed: root.completed
                    total: root.total
                }
            }
        }

        // the in-place progress fill
        Rectangle {
            visible: root._inPlace
            parent: button.background
            anchors.fill: parent
            color: "transparent"
            clip: false

            Item {
                width: parent.width * Math.min(1, root.percent / 100)
                height: parent.height
                clip: true

                Behavior on width {
                    SmoothNumberAnimation {
                        duration: 150
                    }
                }

                Rectangle {
                    width: button.width
                    height: button.height
                    topLeftRadius: button.leftRadius
                    bottomLeftRadius: button.leftRadius
                    topRightRadius: button.rightRadius
                    bottomRightRadius: button.rightRadius
                    gradient: Gradient {
                        orientation: Gradient.Horizontal
                        GradientStop {
                            position: 0
                            color: root.progressFromColor
                        }
                        GradientStop {
                            position: 1
                            color: root.progressToColor
                        }
                    }
                }
            }
        }
    }

    Loader {
        id: errorIndicator
        anchors.fill: parent
        active: root._err !== null
        sourceComponent: ErrorIndicator {
            err: root._err
            reset: () => root._err = null
        }
    }
}
