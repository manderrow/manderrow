import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Manderrow

// Describes an error and lets the user dismiss it, ported from ErrorDialog.tsx.
Dialog {
    id: dialog

    property var err
    /// Called when the user chooses to ignore the error. May return a promise.
    property var reset: function () {}

    readonly property bool _native: !!err && err.isNative === true

    hideCloseBtn: true
    padding: 0
    closePolicy: Popup.NoAutoClose

    component Disclosure: ColumnLayout {
        id: disclosure

        property string title
        property string body
        property bool expanded: false

        Layout.fillWidth: true
        spacing: 0

        // a <summary>
        Item {
            implicitWidth: summary.implicitWidth
            implicitHeight: summary.implicitHeight

            ClickArea {
                onActivated: disclosure.expanded = !disclosure.expanded
            }

            Row {
                id: summary
                spacing: 0.4 * Theme.rem

                Text {
                    text: disclosure.expanded ? "▼" : "▶"
                    color: Theme.neutral05
                    font.pixelSize: 0.7 * Theme.rem
                    anchors.verticalCenter: parent.verticalCenter
                }
                Text {
                    text: disclosure.title
                    color: Theme.neutral05
                    font.family: Theme.font
                    font.pixelSize: 1.17 * Theme.rem
                    font.weight: Font.Bold
                    font.variableAxes: ({ "wght": Font.Bold })
                }
            }
        }

        Rectangle {
            visible: disclosure.expanded
            Layout.fillWidth: true
            Layout.preferredHeight: Math.min(pre.implicitHeight + 2 * 0.5 * Theme.rem, (Ui.window ? Ui.window.height : 600) * 0.7)
            color: "transparent"
            border.color: Theme.neutral70
            radius: 0.5 * Theme.rem

            Flickable {
                anchors.fill: parent
                anchors.margins: 0.5 * Theme.rem
                contentWidth: pre.implicitWidth
                contentHeight: pre.implicitHeight
                boundsBehavior: Flickable.StopAtBounds
                clip: true
                ScrollBar.vertical: AppScrollBar {}
                ScrollBar.horizontal: AppScrollBar {}
                WheelScroll {}

                TextEdit {
                    id: pre
                    text: disclosure.body
                    readOnly: true
                    selectByMouse: true
                    color: Theme.neutral05
                    selectionColor: Theme.primary50
                    font.family: Theme.fontMonospace
                    font.pixelSize: 0.8125 * Theme.rem
                }
            }
        }
    }

    ColumnLayout {
        width: parent.width
        spacing: 0

        ColumnLayout {
            Layout.fillWidth: true
            Layout.margins: 2 * Theme.rem
            spacing: 8

            // centered when it fits on one line, like the web UI's flex column
            component Paragraph: Text {
                Layout.fillWidth: true
                horizontalAlignment: implicitWidth <= width ? Text.AlignHCenter : Text.AlignLeft
                wrapMode: Text.Wrap
                color: Theme.neutral05
                font.family: Theme.font
                font.pixelSize: Theme.rem
            }

            Paragraph {
                text: I18n.t("error.title")
                font.pixelSize: 1.5 * Theme.rem
                font.weight: Font.Bold
                font.variableAxes: ({ "wght": Font.Bold })
            }
            Paragraph {
                text: I18n.t("error.deescalation_msg")
            }

            ColumnLayout {
                Layout.fillWidth: true
                Layout.topMargin: 16 - 8
                Layout.bottomMargin: 16 - 8
                spacing: 0

                Repeater {
                    model: dialog._native ? dialog.err.messages : [dialog.err ? String(dialog.err) : ""]
                    delegate: Paragraph {
                        required property var modelData
                        horizontalAlignment: Text.AlignLeft
                        text: modelData
                        textFormat: Text.PlainText
                    }
                }

                Disclosure {
                    visible: dialog._native
                    title: I18n.t("error.native_stack_trace") + ":"
                    body: dialog._native ? dialog.err.backtrace : ""
                }

                Disclosure {
                    visible: !!dialog.err && !!dialog.err.stack
                    title: I18n.t("error.js_stack_trace") + ":"
                    body: dialog.err && dialog.err.stack ? dialog.err.stack : ""
                }
            }

            Paragraph {
                text: I18n.t("error.report_msg")
            }
            Paragraph {
                text: I18n.t("error.ignore_msg")
            }
        }

        RowLayout {
            Layout.margins: Theme.rem
            spacing: 0

            Btn {
                id: ignoreBtn
                property bool busy: false
                enabled: !busy
                text: I18n.t("error.ignore_btn")
                onClicked: {
                    busy = true;
                    Promise.resolve(dialog.reset()).then(() => {
                        ignoreBtn.busy = false;
                        dialog.close();
                    }, e => {
                        ignoreBtn.busy = false;
                        console.error(e);
                    });
                }
            }

            // TODO: Add link to report button
            Btn {
                text: I18n.t("error.report_btn")
            }
        }
    }
}
