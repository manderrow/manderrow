import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Manderrow

// Lets the user choose mods to update, ported from Updater.tsx.
Dialog {
    id: dialog

    /// `[{ newMod, oldVersionNumber }]`
    property var updates: []
    /// Indices into `updates` of the mods selected for updating.
    property var selected: ({})

    readonly property int selectedCount: Object.keys(selected).length

    maxWidth: 600
    padding: 0.75 * Theme.rem
    onUpdatesChanged: selectAll(true)
    Component.onCompleted: selectAll(true)

    function selectAll(on) {
        const out = {};
        if (on) {
            for (let i = 0; i < updates.length; i++)
                out[i] = true;
        }
        selected = out;
    }

    ColumnLayout {
        width: parent.width
        spacing: Theme.rem

        SText {
            Layout.fillWidth: true
            Layout.bottomMargin: 0.5 * 1.5 * Theme.rem
            horizontalAlignment: TextEdit.AlignHCenter
            text: Strings.modlist.installed.updater_title
            color: Theme.neutral10
            font.family: Theme.font
            font.pixelSize: 1.5 * Theme.rem
            font.weight: Font.Medium
            font.variableAxes: ({ "wght": Font.Medium })
        }

        Item {
            Layout.fillWidth: true
            implicitHeight: listColumn.implicitHeight + 2 * Ui.hairline

            BoxBackground {
                radius: 0.5 * Theme.rem
                border.color: Theme.neutral70
            }

            ColumnLayout {
                id: listColumn
                x: Ui.hairline
                y: Ui.hairline
                width: parent.width - 2 * Ui.hairline
                spacing: 0

                RowLayout {
                    Layout.fillWidth: true
                    Layout.margins: 0.5 * Theme.rem
                    spacing: 0.5 * Theme.rem

                    Row {
                        spacing: 0.5 * Theme.rem
                        Checkbox {
                            checked: dialog.selectedCount === dialog.updates.length
                            onToggled: checked => dialog.selectAll(checked)
                            anchors.verticalCenter: parent.verticalCenter
                        }
                        Text {
                            text: Strings.global.phrases.select_all
                            color: Theme.neutral05
                            font.family: Theme.font
                            font.pixelSize: Theme.rem
                            anchors.verticalCenter: parent.verticalCenter
                        }
                    }
                    Item {
                        Layout.fillWidth: true
                    }
                    Field {
                        placeholderText: Strings.global.phrases.search
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: Ui.hairline
                    color: Theme.neutral70
                }

                ListView {
                    id: list
                    Layout.fillWidth: true
                    Layout.preferredHeight: Math.min(contentHeight, (Ui.window ? Ui.window.height : 600) * 0.75)
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
                    model: dialog.updates

                    delegate: Rectangle {
                        id: entry

                        required property var modelData
                        required property int index
                        readonly property bool isLast: index === dialog.updates.length - 1

                        width: list.width - bar.gutter
                        implicitHeight: entryRow.implicitHeight + Theme.rem
                        bottomLeftRadius: isLast ? 4 : 0
                        bottomRightRadius: isLast ? 4 : 0
                        color: entryHover.hovered ? Theme.neutral70 : index % 2 === 1 ? Theme.neutral85 : "transparent"

                        ClickArea {
                            id: entryHover
                            cursorShape: Qt.ArrowCursor
                            onActivated: {
                                const selected = Object.assign({}, dialog.selected);
                                if (selected[entry.index])
                                    delete selected[entry.index];
                                else
                                    selected[entry.index] = true;
                                dialog.selected = selected;
                            }
                        }

                        RowLayout {
                            id: entryRow
                            anchors.fill: parent
                            anchors.margins: 0.5 * Theme.rem
                            spacing: 0.5 * Theme.rem

                            Checkbox {
                                checked: !!dialog.selected[entry.index]
                                enabled: false
                            }

                            // img { width: 3rem; border-radius: 4px }
                            RoundedImage {
                                Layout.preferredWidth: 3 * Theme.rem
                                Layout.preferredHeight: 3 * Theme.rem
                                radius: 4
                                source: Util.getIconUrl(Util.getQualifiedModName(entry.modelData.newMod.owner, entry.modelData.newMod.name, entry.modelData.newMod.versions[0].version_number))
                            }

                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 0

                                Text {
                                    Layout.fillWidth: true
                                    text: entry.modelData.newMod.name
                                    elide: Text.ElideRight
                                    color: Theme.neutral10
                                    font.family: Theme.font
                                    font.pixelSize: 0.9 * Theme.rem
                                    font.weight: Font.Medium
                                    font.variableAxes: ({ "wght": Font.Medium })
                                }
                                Text {
                                    Layout.fillWidth: true
                                    text: entry.modelData.newMod.owner
                                    elide: Text.ElideRight
                                    color: Theme.neutral30
                                    font.family: Theme.font
                                    font.pixelSize: 0.9 * Theme.rem
                                }
                            }

                            // old version → new version
                            Row {
                                spacing: 0.3 * Theme.rem

                                Text {
                                    width: 7 * Ui.ch * 0.9
                                    horizontalAlignment: Text.AlignRight
                                    text: entry.modelData.oldVersionNumber
                                    color: Theme.neutral20
                                    font.family: Theme.font
                                    font.pixelSize: 0.9 * Theme.rem
                                }
                                Icon {
                                    name: "move-right"
                                    size: 0.9 * Theme.rem
                                    color: Theme.neutral40
                                    anchors.verticalCenter: parent.verticalCenter
                                }
                                Text {
                                    width: 7 * Ui.ch * 0.9
                                    text: entry.modelData.newMod.versions[0].version_number
                                    color: Theme.neutral20
                                    font.family: Theme.font
                                    font.pixelSize: 0.9 * Theme.rem
                                    font.weight: Font.DemiBold
                                    font.variableAxes: ({ "wght": Font.DemiBold })
                                }
                            }
                        }
                    }
                }
            }
        }

        RowLayout {
            Layout.alignment: Qt.AlignRight
            spacing: 0.5 * Theme.rem

            Btn {
                variant: "ghost"
                text: Strings.global.phrases.cancel
                onClicked: dialog.close()
            }
            Btn {
                variant: "primary"
                text: dialog.selectedCount === dialog.updates.length ? Strings.modlist.installed.update_all_btn : Strings.modlist.installed.update_selected_btn
            }
        }
    }
}
