import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Manderrow

// A dialog that steps through stages, ported from MultistageModel.tsx. Push a stage with
// `pushStage({ title, component, props })`, where the component's root may declare `modal` to
// receive this dialog.
Dialog {
    id: modal

    /// The first stage.
    property var initialStage
    property int estimatedStages: 1
    property var stack: []

    readonly property var currentStage: stack.length > 0 ? stack[stack.length - 1] : null

    function pushStage(stage) {
        stack = stack.concat([stage]);
    }
    function popStage() {
        stack = stack.slice(0, -1);
    }

    hideCloseBtn: true
    padding: 0
    maxWidth: 36 * Theme.rem
    cornerRadius: 4
    height: Math.min(container.implicitHeight, (parent ? parent.height : 600) * 0.8)
    onAboutToShow: stack = [initialStage]
    onClosed: stack = []

    Behavior on height {
        NumberAnimation {
            duration: 200
            easing.type: Easing.InOutQuad
        }
    }

    ColumnLayout {
        id: container
        width: parent.width
        height: parent.height
        spacing: 0.75 * Theme.rem

        // the step indicators
        RowLayout {
            Layout.fillWidth: true
            Layout.margins: Theme.rem
            Layout.bottomMargin: 0
            spacing: 0.5 * Theme.rem

            Repeater {
                model: Math.max(modal.estimatedStages, modal.stack.length)

                delegate: ColumnLayout {
                    id: step

                    required property int index
                    readonly property bool current: index === modal.stack.length - 1

                    Layout.fillWidth: true
                    Layout.preferredWidth: 1
                    spacing: 0.25 * Theme.rem

                    Rectangle {
                        Layout.fillWidth: true
                        implicitHeight: 0.5 * Theme.rem
                        radius: 0.25 * Theme.rem
                        color: step.current ? Theme.primary50 : Theme.neutral50
                    }
                    Text {
                        Layout.fillWidth: true
                        text: `Step ${step.index + 1}` + (step.index < modal.stack.length ? ` - ${modal.stack[step.index].title}` : "")
                        elide: Text.ElideRight
                        color: Theme.neutral20
                        font.family: Theme.font
                        font.pixelSize: 0.9 * Theme.rem
                    }
                }
            }
        }

        Text {
            Layout.fillWidth: true
            Layout.leftMargin: Theme.rem
            Layout.rightMargin: Theme.rem
            Layout.bottomMargin: 0.5 * Theme.rem
            horizontalAlignment: Text.AlignHCenter
            text: modal.currentStage ? modal.currentStage.title : ""
            color: Theme.neutral05
            font.family: Theme.font
            font.pixelSize: 1.5 * Theme.rem
            font.weight: Font.Bold
        }

        Loader {
            id: content
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.leftMargin: Theme.rem
            Layout.rightMargin: Theme.rem
            Layout.bottomMargin: Theme.rem
            Layout.preferredHeight: item ? item.implicitHeight : 0
            sourceComponent: modal.currentStage ? modal.currentStage.component : null
            onLoaded: {
                if ("modal" in item)
                    item.modal = modal;
                const props = modal.currentStage.props || {};
                for (const key of Object.keys(props))
                    item[key] = props[key];
            }
        }
    }
}
