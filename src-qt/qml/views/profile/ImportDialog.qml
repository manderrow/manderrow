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
            title: I18n.t("profile.import_model.import_title"),
            component: choosePage
        })

    // Parses the mod source from a Thunderstore download URL.
    function detectModSource(url) {
        const match = url.match(/https:\/\/thunderstore\.io\/package\/download\/([a-zA-Z0-9_]+)\/([a-zA-Z0-9_]+)\/(\d+\.\d+\.\d+)\//) || url.match(/https:\/\/gcdn\.thunderstore\.io\/live\/repository\/packages\/([a-zA-Z0-9_]+)-([a-zA-Z0-9_]+)-(\d+\.\d+\.\d+).zip/);
        if (match === null)
            return undefined;
        return { author: match[1], name: match[2], version: match[3], source: "Thunderstore" };
    }

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
                text: I18n.t("profile.import_model.thunderstore_code_label")
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
                    text: I18n.t("global.phrases.cancel")
                    onClicked: dialog.close()
                }
                Item {
                    Layout.fillWidth: true
                }
                AsyncButton {
                    id: next
                    text: I18n.t("global.phrases.next")
                    progressMode: "listener"
                    // the field is required, so an empty code focuses it instead
                    action: listener => code.text === "" ? code.forceActiveFocus() : Api.previewImportModpackFromThunderstoreCode(code.text, dialog.gameId, dialog.profileId, listener).then(modpack => {
                        dialog.pushStage({
                            title: I18n.t("profile.import_model.preview_title"),
                            component: previewPage,
                            props: { thunderstoreCode: code.text, modpack }
                        });
                    })
                }
            }
        }
    }

    Component {
        id: previewPage

        ColumnLayout {
            id: page

            property var modal
            property string thunderstoreCode
            property var modpack: ({ name: "", mods: [], diff: [] })
            /// The task installing each mod, by URL.
            property var modProgress: ({})

            spacing: 0

            RowLayout {
                spacing: 0.5 * Theme.rem

                Btn {
                    iconName: "chevron-left"
                    onClicked: dialog.popStage()
                }
                Text {
                    Layout.fillWidth: true
                    text: page.modpack.name
                    elide: Text.ElideRight
                    color: Theme.neutral05
                    font.family: Theme.font
                    font.pixelSize: 1.5 * Theme.rem
                    font.weight: Font.Bold
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
                        model: page.modpack.mods

                        delegate: RowLayout {
                            id: modEntry

                            required property var modelData
                            readonly property var metadata: dialog.detectModSource(modelData.url)
                            readonly property var task: page.modProgress[modelData.url] !== undefined ? Tasks.get(page.modProgress[modelData.url]) : undefined

                            Layout.fillWidth: true

                            Item_ {
                                textFormat: Text.PlainText
                                text: modEntry.metadata ? `${modEntry.metadata.name} v${modEntry.metadata.version}` + (modEntry.metadata.author ? ` by ${modEntry.metadata.author}` : "") : modEntry.modelData.url
                            }
                            ProgressBar {
                                visible: modEntry.task !== undefined
                                Layout.alignment: Qt.AlignRight
                                progress: modEntry.task ? modEntry.task.progress : Tasks.initProgress()
                            }
                        }
                    }

                    Heading {
                        text: "Files"
                    }
                    Repeater {
                        model: page.modpack.diff
                        delegate: Item_ {
                            required property var modelData
                            textFormat: Text.StyledText
                            text: `<b>[${modelData.diff}]</b> ${Util.escapeHtml(modelData.path)}`
                        }
                    }
                }
            }

            ButtonRow {
                Btn {
                    text: I18n.t("global.phrases.cancel")
                    onClicked: dialog.close()
                }
                Item {
                    Layout.fillWidth: true
                }
                AsyncButton {
                    text: "Import"
                    progressMode: "listener"
                    action: listener => Api.importModpackFromThunderstoreCode(page.thunderstoreCode, dialog.gameId, dialog.profileId, info => {
                        const progress = Object.assign({}, page.modProgress);
                        progress[info.url] = info.task;
                        page.modProgress = progress;
                    }, listener).then(id => {
                        console.log(`Imported to profile ${id}`);
                        dialog.close();
                        return Globals.refetchProfiles();
                    })
                }
            }
        }
    }
}
