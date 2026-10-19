import QtQuick
import QtQuick.Layouts
import Manderrow

// Offers fixes for a problem a game reported, ported from DoctorDialog in Console.tsx.
Dialog {
    id: dialog

    /// A DoctorReport C2S message with its `connId`.
    property var report

    readonly property string _key: report ? report.translation_key : ""

    ColumnLayout {
        width: parent.width
        spacing: 0

        Text {
            Layout.fillWidth: true
            Layout.bottomMargin: 0.25 * 1.5 * Theme.rem
            horizontalAlignment: Text.AlignHCenter
            text: "Uh oh!"
            color: Theme.neutral05
            font.family: Theme.font
            font.pixelSize: 1.5 * Theme.rem
            font.weight: Font.Bold
        }

        Text {
            Layout.fillWidth: true
            wrapMode: Text.Wrap
            text: dialog.report ? I18n.t(dialog.report.message || `doctor.${dialog._key}.message`, dialog.report.message_args) : ""
            color: Theme.neutral05
            font.family: Theme.font
            font.pixelSize: Theme.rem
        }

        Repeater {
            model: dialog.report ? dialog.report.fixes : []

            delegate: ColumnLayout {
                id: fix

                required property var modelData
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
                    text: I18n.t(`${fix.prefix}.confirm_label`, fix.modelData.confirm_label)
                    onClicked: {
                        Api.sendS2CMessage(dialog.report.connId, {
                            type: "PatientResponse",
                            id: dialog.report.id,
                            choice: fix.modelData.id
                        }).catch(e => Ui.window.reportError(e)).then(() => dialog.close());
                    }
                }
            }
        }
    }
}
