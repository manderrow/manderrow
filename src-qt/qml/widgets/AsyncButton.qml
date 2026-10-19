import QtQuick
import Manderrow

// A button that runs an asynchronous action, ported from SimpleAsyncButton in AsyncButton.tsx.
//
// While the action runs, the button is disabled and can show its progress. If the action fails,
// the button is replaced by an ErrorIndicator until the error is dismissed.
Item {
    id: root

    /// The action. When `progressMode` is "listener", it receives a task listener that it should
    /// pass to the API call whose progress is shown. It may return a promise.
    property var action: function () {}
    /// "" for no progress, "listener" to track a task, or "external" to show `externalProgress`.
    property string progressMode: ""
    property var externalProgress
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
    readonly property var progress: progressMode === "external" ? externalProgress : _task ? _task.progress : Tasks.initProgress()
    readonly property bool showProgress: progressMode !== "" && busy
    readonly property real percent: !progress ? 0 : progress.total == null ? 0 : progress.completed / (progress.total === 0 ? 1 : progress.total) * 100
    readonly property bool _inPlace: showProgress && progressStyle === "in-place"

    property bool _busy: false
    property var _task: null
    property var _err: null

    implicitWidth: _err !== null ? errorIndicator.implicitWidth : button.implicitWidth
    implicitHeight: _err !== null ? errorIndicator.implicitHeight : button.implicitHeight

    function run() {
        _busy = true;
        _task = null;
        let result;
        try {
            result = progressMode === "listener" ? action(event => {
                // the button may have been destroyed by the time this runs
                if (root && event.event === "created")
                    root._task = event.task;
            }) : action();
        } catch (e) {
            result = Promise.reject(e);
        }
        Promise.resolve(result).then(() => {
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
            implicitWidth: root.showProgress && root.progressStyle === "circular" ? circular.implicitWidth : root.showProgress && root.progressStyle === "simple" ? bar.implicitWidth : label.implicitWidth
            implicitHeight: root.showProgress && root.progressStyle === "circular" ? circular.implicitHeight : label.implicitHeight

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

            CircularProgress {
                id: circular
                visible: root.showProgress && root.progressStyle === "circular"
                anchors.centerIn: parent
                progress: root.progress
                size: button.fontSize
            }

            ProgressBar {
                id: bar
                visible: root.showProgress && root.progressStyle === "simple"
                anchors.verticalCenter: parent.verticalCenter
                width: parent.width
                progress: root.progress
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
                    NumberAnimation {
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

    ErrorIndicator {
        id: errorIndicator
        visible: root._err !== null
        anchors.fill: parent
        err: root._err
        reset: () => root._err = null
    }
}
