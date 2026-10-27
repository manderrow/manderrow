pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import Manderrow

// Offers fixes for a problem a game reported, ported from DoctorDialog in Console.tsx.
Dialog {
    id: dialog

    property DoctorReport report: null

    readonly property string _key: report ? report.translationKey : ""

    ColumnLayout {
        width: parent.width
        spacing: 0

        SText {
            Layout.fillWidth: true
            Layout.bottomMargin: 0.25 * 1.5 * Theme.rem
            horizontalAlignment: TextEdit.AlignHCenter
            text: "Uh oh!"
            color: Theme.neutral05
            font.family: Theme.font
            font.pixelSize: 1.5 * Theme.rem
            font.weight: Font.Bold
            font.variableAxes: ({ "wght": Font.Bold })
        }

        SText {
            Layout.fillWidth: true
            wrapMode: TextEdit.Wrap
            text: dialog.report ? I18n.t(dialog.report.message || `doctor.${dialog._key}.message`, dialog.report.messageArgs) : ""
            color: Theme.neutral05
            font.family: Theme.font
            font.pixelSize: Theme.rem
        }

        Repeater {
            model: dialog.report ? dialog.report.fixes : []

            delegate: ColumnLayout {
                id: fix

                required property DoctorFix modelData
                readonly property string prefix: `doctor.${dialog._key}.fixes.${modelData.id}`

                Layout.fillWidth: true
                spacing: 0

                component Line: Text {
                    Layout.fillWidth: true
                    wrapMode: Text.Wrap
                    color: Theme.neutral05
                    font.family: Theme.font
                    font.pixelSize: Theme.rem
                }

                Line {
                    text: I18n.t(`${fix.prefix}.label`, fix.modelData.label)
                }
                Line {
                    text: I18n.t(`${fix.prefix}.description`, fix.modelData.description)
                }
                Btn {
                    text: I18n.t(`${fix.prefix}.confirm_label`, fix.modelData.confirmLabel)
                    onClicked: Async.promise(dialog.report.choose(fix.modelData.id)).catch(e => Ui.window.reportError(e)).then(() => dialog.close())
                }
            }
        }
    }
}
