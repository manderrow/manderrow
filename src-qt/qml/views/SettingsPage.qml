import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Manderrow

// The /settings route, ported from Settings.tsx.
Item {
    id: page

    readonly property var sections: Globals.settingsUI ? Globals.settingsUI.sections : []
    readonly property string currentSection: {
        const id = Router.search["settings-tab"];
        return id !== undefined ? id : sections.length > 0 ? sections[0].id : "";
    }
    readonly property var currentSettings: {
        const section = sections.find(s => s.id === currentSection);
        return section ? section.settings : [];
    }

    readonly property real sidebarWidth: Math.min(19 * Theme.rem, width)
    readonly property real paddingBlock: 2 * Theme.rem

    function update(key, change) {
        const patch = {};
        patch[key] = change;
        Api.updateSettings(patch).catch(e => Ui.window.reportError(e));
    }

    Rectangle {
        anchors.fill: parent
        color: Theme.surface(Theme.neutral85)
    }

    Item {
        id: main
        anchors.horizontalCenter: parent.horizontalCenter
        width: Math.min(parent.width, 1150)
        height: parent.height

        // the sidebar
        Rectangle {
            id: navbar
            x: Theme.rem
            y: page.paddingBlock
            width: page.sidebarWidth
            // the content and the border around it
            height: Math.min(navContent.implicitHeight + 2 * 1.25 * Theme.rem + 2, parent.height - 2 * page.paddingBlock)
            radius: 6
            color: "transparent"
            border.color: Theme.neutral80

            Flickable {
                anchors.fill: parent
                anchors.margins: 1
                contentHeight: navContent.implicitHeight + 2 * 1.25 * Theme.rem
                clip: true
                ScrollBar.vertical: AppScrollBar {}
                WheelScroll {}

                ColumnLayout {
                    id: navContent
                    x: Theme.rem
                    y: 1.25 * Theme.rem
                    width: parent.width - 2 * Theme.rem
                    spacing: 0

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 0

                        Btn {
                            variant: "plain"
                            iconName: "chevronLeft"
                            horizontalPadding: fontSize
                            verticalPadding: 0.4 * fontSize
                            onClicked: Router.navigate(-1)
                        }
                        SText {
                            Layout.fillWidth: true
                            text: I18n.t("settings.title")
                            color: Theme.neutral05
                            font.family: Theme.font
                            font.pixelSize: 2 * Theme.rem
                            font.weight: Font.Bold
                        }
                    }

                    Field {
                        Layout.fillWidth: true
                        Layout.topMargin: 0.5 * Theme.rem
                        Layout.bottomMargin: Theme.rem
                        placeholderText: I18n.t("settings.search_input_placeholder")
                    }

                    Repeater {
                        model: page.sections

                        delegate: Btn {
                            required property var modelData
                            readonly property bool isCurrent: modelData.id === page.currentSection

                            Layout.fillWidth: true
                            Layout.bottomMargin: 0.2 * Theme.rem
                            variant: "plain"
                            radius: 4
                            horizontalPadding: 0.75 * fontSize
                            horizontalAlignment: Qt.AlignLeft
                            bgColor: isCurrent ? Theme.primary40 : "transparent"
                            activeBgColor: isCurrent ? Theme.primary40 : Theme.neutral80
                            text: I18n.t(`settings.section.${modelData.id}`)
                            onClicked: Router.setSearch({ "settings-tab": modelData.id })
                        }
                    }
                }
            }
        }

        // the options of the current section
        ColumnLayout {
            // the sidebar is absolutely positioned, so the flex gap doesn't separate them
            x: Theme.rem + page.sidebarWidth + Theme.rem
            y: page.paddingBlock
            width: parent.width - x - Theme.rem
            spacing: 0

            Repeater {
                model: page.currentSettings

                delegate: Item {
                    id: option

                    required property var modelData
                    required property int index
                    readonly property var value: Globals.settings[modelData.key]
                    readonly property bool isFirst: index === 0
                    readonly property bool isLast: index === page.currentSettings.length - 1

                    Layout.fillWidth: true
                    implicitHeight: optionRow.implicitHeight + ((isFirst ? 0.5 : 1) + (isLast ? 0.5 : 1)) * Theme.rem

                    Rectangle {
                        visible: !option.isLast
                        anchors.bottom: parent.bottom
                        width: parent.width
                        height: 1
                        color: Theme.neutral80
                    }

                    RowLayout {
                        id: optionRow
                        x: 0.5 * Theme.rem
                        y: (option.isFirst ? 0.5 : 1) * Theme.rem
                        width: parent.width - Theme.rem
                        spacing: 0

                        Text {
                            Layout.fillWidth: true
                            text: I18n.t(`settings.settings.${option.modelData.key}`)
                            color: Theme.neutral05
                            font.family: Theme.font
                            font.pixelSize: Theme.rem
                        }

                        Loader {
                            // .option__input stretches the dropdown to the reset button's height
                            Layout.fillHeight: option.modelData.input === "game_select"
                            sourceComponent: {
                                switch (option.modelData.input) {
                                case "toggle":
                                    return toggleInput;
                                case "text":
                                    return textInput;
                                case "game_select":
                                    return gameSelectInput;
                                case "percent":
                                    return percentInput;
                                default:
                                    return null;
                                }
                            }

                            Component {
                                id: toggleInput
                                Checkbox {
                                    checked: option.value.value
                                    onToggled: checked => page.update(option.modelData.key, { override: checked })
                                }
                            }
                            Component {
                                id: textInput
                                Field {
                                    text: option.value.value || ""
                                    onEditingFinished: page.update(option.modelData.key, { override: text })
                                }
                            }
                            Component {
                                id: percentInput
                                Row {
                                    spacing: 0.75 * Theme.rem

                                    AppSlider {
                                        id: slider
                                        anchors.verticalCenter: parent.verticalCenter
                                        from: 0
                                        to: 100
                                        stepSize: 1
                                        value: option.value.value
                                        // applies as it moves, without saving every step
                                        onMoved: commit.restart()

                                        Timer {
                                            id: commit
                                            interval: 100
                                            onTriggered: page.update(option.modelData.key, { override: Math.round(slider.value) })
                                        }
                                    }
                                    Text {
                                        anchors.verticalCenter: parent.verticalCenter
                                        // the widest value, so the slider doesn't move as it changes
                                        width: percentMetrics.advanceWidth
                                        horizontalAlignment: Text.AlignRight
                                        text: `${Math.round(slider.value)}%`
                                        color: Theme.neutral05
                                        font.family: Theme.font
                                        font.pixelSize: Theme.rem
                                        font.features: ({ "tnum": 1 })

                                        TextMetrics {
                                            id: percentMetrics
                                            font.family: Theme.font
                                            font.pixelSize: Theme.rem
                                            text: "100%"
                                        }
                                    }
                                }
                            }
                            Component {
                                id: gameSelectInput
                                SelectDropdown {
                                    options: Globals.games.map(game => ({
                                                label: game.name,
                                                value: game.id,
                                                selected: option.value.value === game.id
                                            })).sort((a, b) => a.label.localeCompare(b.label))
                                    onChanged: (value, selected) => {
                                        if (selected)
                                            page.update(option.modelData.key, { override: value });
                                    }
                                }
                            }
                        }

                        Btn {
                            variant: "plain"
                            iconName: "clockRotateLeft"
                            padAll: 0.5 * fontSize
                            enabled: !option.value.isDefault
                            onClicked: page.update(option.modelData.key, "default")
                        }
                    }
                }
            }
        }
    }
}
