import QtQuick
import QtQuick.Layouts
import Manderrow

// A dialog asking the user to confirm or cancel, ported from PromptDialog in Dialog.tsx.
Dialog {
    id: prompt

    /// Undefined hides the title; null shows the default title.
    property var title
    property string question
    /// "danger", "ok", "info", or "" for the confirm button's color.
    property string okType: ""
    property string okText: Strings.global.phrases.confirm
    property string cancelType: ""
    property string cancelText: Strings.global.phrases.cancel
    /// When false, the dialog only has a cancel button, as in InfoDialog.
    property bool showOk: true

    signal accepted
    signal cancelled

    function _btnColor(type) {
        switch (type) {
        case "danger":
            return Theme.danger4;
        case "ok":
            return Theme.ok4;
        case "info":
            return Theme.primary40;
        default:
            return Theme.neutral70;
        }
    }

    ColumnLayout {
        width: parent.width
        spacing: 0

        SText {
            visible: prompt.title !== undefined
            Layout.fillWidth: true
            Layout.bottomMargin: 0.25 * 1.5 * Theme.rem
            text: prompt.title === null || prompt.title === undefined ? Strings.global.dialogue_default_title : prompt.title
            horizontalAlignment: TextEdit.AlignHCenter
            wrapMode: TextEdit.Wrap
            color: Theme.neutral05
            font.family: Theme.font
            font.pixelSize: 1.5 * Theme.rem
            font.weight: Font.Bold
            font.variableAxes: ({ "wght": Font.Bold })
        }

        SText {
            Layout.fillWidth: true
            text: prompt.question
            wrapMode: TextEdit.Wrap
            color: Theme.neutral20
            font.family: Theme.font
            font.pixelSize: Theme.rem
        }

        RowLayout {
            Layout.alignment: Qt.AlignRight
            Layout.topMargin: 2 * Theme.rem
            spacing: 0.25 * Theme.rem

            Btn {
                visible: prompt.showOk
                text: prompt.okText
                radius: 4
                horizontalPadding: fontSize
                verticalPadding: 0.5 * fontSize
                bgColor: prompt._btnColor(prompt.okType)
                activeBgColor: prompt.okType === "" ? Theme.neutral70 : Theme.lighten(bgColor, 5)
                onClicked: prompt.accepted()
            }

            Btn {
                text: prompt.cancelText
                radius: 4
                horizontalPadding: fontSize
                verticalPadding: 0.5 * fontSize
                bgColor: prompt._btnColor(prompt.cancelType)
                activeBgColor: prompt.cancelType === "" ? Theme.neutral70 : Theme.lighten(bgColor, 5)
                onClicked: {
                    prompt.cancelled();
                    prompt.close();
                }
            }
        }
    }
}
