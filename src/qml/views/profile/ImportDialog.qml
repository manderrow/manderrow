pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Manderrow

// Imports a modpack from a Thunderstore profile code, ported from ImportDialog.tsx.
MultistageModal {
    id: dialog

    property string gameId
    property var profileId

    estimatedStages: 2
    initialStage: ({
            title: Strings.profile.import_model.import_title,
            component: choosePage
        })

    component ButtonRow: RowLayout {
        Layout.fillWidth: true
        Layout.topMargin: Theme.rem
    }

    Component {
        id: choosePage

        ColumnLayout {
            id: page
            property var modal
            spacing: 0

            SText {
                Layout.bottomMargin: 0.25 * Theme.rem
                text: Strings.profile.import_model.thunderstore_code_label
                color: Theme.neutral05
                font.family: Theme.font
                font.pixelSize: Theme.rem
            }
            Field {
                id: code
                Layout.fillWidth: true
                variant: "pill"
                leftPadding: Theme.rem
                rightPadding: Theme.rem
                topPadding: 0.5 * Theme.rem
                bottomPadding: 0.5 * Theme.rem
                Component.onCompleted: forceActiveFocus()
                onAccepted: next.run()
            }

            ButtonRow {
                Btn {
                    text: Strings.global.phrases.cancel
                    onClicked: dialog.close()
                }
                Item {
                    Layout.fillWidth: true
                }
                AsyncButton {
                    id: next
                    text: Strings.global.phrases.next
                    progressMode: "task"
                    // the field is required, so an empty code focuses it instead
                    action: showTask => {
                        if (code.text === "")
                            return code.forceActiveFocus();
                        const preview = Profiles.previewModpack(code.text, dialog.gameId);
                        showTask(Tasks.get(preview.taskId));
                        return Async.promise(preview).then(modpack => {
                            dialog.pushStage({
                                title: Strings.profile.import_model.preview_title,
                                component: previewPage,
                                props: { modpack }
                            });
                        });
                    }
                }
            }
        }
    }

    Component {
        id: previewPage

        ColumnLayout {
            id: page

            property var modal
            property Modpack modpack: null

            spacing: 0

            RowLayout {
                spacing: 0.5 * Theme.rem

                Btn {
                    iconName: "chevron-left"
                    onClicked: dialog.popStage()
                }
                Text {
                    Layout.fillWidth: true
                    text: page.modpack ? page.modpack.name : ""
                    elide: Text.ElideRight
                    color: Theme.neutral05
                    font.family: Theme.font
                    font.pixelSize: 1.5 * Theme.rem
                    font.weight: Font.Bold
                    font.variableAxes: ({ "wght": Font.Bold })
                }
            }

            Flickable {
                id: preview
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.preferredHeight: previewColumn.implicitHeight
                contentWidth: width
                contentHeight: previewColumn.implicitHeight
                boundsBehavior: Flickable.StopAtBounds
                clip: true
                // overlayscrollbars, which hide when the pointer leaves
                ScrollBar.vertical: AppScrollBar {
                    autoHide: true
                }
                WheelScroll {}

                ColumnLayout {
                    id: previewColumn
                    width: preview.width
                    spacing: 0

                    component Heading: Text {
                        Layout.topMargin: 0.5 * Theme.rem
                        color: Theme.neutral05
                        font.family: Theme.font
                        font.pixelSize: 1.17 * Theme.rem
                        font.weight: Font.Bold
                        font.variableAxes: ({ "wght": Font.Bold })
                    }
                    component Item_: Text {
                        Layout.fillWidth: true
                        wrapMode: Text.WrapAnywhere
                        color: Theme.neutral05
                        font.family: Theme.font
                        font.pixelSize: Theme.rem
                    }

                    Heading {
                        text: "Mods"
                    }
                    Repeater {
                        model: page.modpack ? page.modpack.mods : []

                        delegate: RowLayout {
                            id: modEntry

                            required property ModpackMod modelData
                            readonly property Task task: modelData.taskId >= 0 ? Tasks.get(modelData.taskId) : null

                            Layout.fillWidth: true

                            Item_ {
                                textFormat: Text.PlainText
                                text: `${modEntry.modelData.name} v${modEntry.modelData.version} by ${modEntry.modelData.owner}`
                            }
                            ProgressBar {
                                visible: modEntry.task !== null
                                Layout.alignment: Qt.AlignRight
                                completed: modEntry.task ? modEntry.task.completed : 0
                                total: modEntry.task ? modEntry.task.total : 0
                            }
                        }
                    }

                    Heading {
                        text: "Files"
                    }
                    Repeater {
                        model: page.modpack ? page.modpack.files : []
                        delegate: Item_ {
                            required property string modelData
                            textFormat: Text.StyledText
                            text: `<b>[Created]</b> ${Util.escapeHtml(modelData)}`
                        }
                    }
                }
            }

            ButtonRow {
                Btn {
                    text: Strings.global.phrases.cancel
                    onClicked: dialog.close()
                }
                Item {
                    Layout.fillWidth: true
                }
                AsyncButton {
                    text: "Import"
                    progressMode: "task"
                    action: showTask => {
                        const importing = page.modpack.importInto(dialog.profileId ?? "");
                        showTask(Tasks.get(importing.taskId));
                        return Async.promise(importing).then(id => {
                            console.log(`Imported to profile ${id}`);
                            dialog.close();
                        });
                    }
                }
            }
        }
    }
}
