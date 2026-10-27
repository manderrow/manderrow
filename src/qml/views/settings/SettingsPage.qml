pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Manderrow

// The /settings route, ported from Settings.tsx.
Item {
    id: page

    readonly property string currentSection: {
        const id = Router.search["settings-tab"];
        return id !== undefined ? id : Settings.sections.length > 0 ? (Settings.sections[0] as SettingsSection).id : "";
    }
    readonly property list<QtObject> currentSettings: {
        const section = Settings.sections.find(s => (s as SettingsSection).id === currentSection) as SettingsSection;
        return section ? section.settings : [];
    }

    readonly property real sidebarWidth: Math.min(19 * Theme.rem, width)
    readonly property real paddingBlock: 2 * Theme.rem

    function update(call: Call) {
        Async.promise(call).catch(e => Ui.window.reportError(e));
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
        Item {
            id: navbar
            x: Theme.rem
            y: page.paddingBlock
            width: page.sidebarWidth
            // the content and the border around it
            height: Math.min(navContent.implicitHeight + 2 * 1.25 * Theme.rem + 2 * Ui.hairline, parent.height - 2 * page.paddingBlock)

            BoxBackground {
                radius: 6
                border.color: Theme.neutral80
            }

            Flickable {
                anchors.fill: parent
                anchors.margins: Ui.hairline
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
                            iconName: "chevron-left"
                            horizontalPadding: fontSize
                            verticalPadding: 0.4 * fontSize
                            onClicked: Router.navigate(-1)
                        }
                        SText {
                            Layout.fillWidth: true
                            text: Strings.settings.title
                            color: Theme.neutral05
                            font.family: Theme.font
                            font.pixelSize: 2 * Theme.rem
                            font.weight: Font.Bold
                            font.variableAxes: ({ "wght": Font.Bold })
                        }
                    }

                    Field {
                        Layout.fillWidth: true
                        Layout.topMargin: 0.5 * Theme.rem
                        Layout.bottomMargin: Theme.rem
                        placeholderText: Strings.settings.search_input_placeholder
                    }

                    Repeater {
                        model: Settings.sections

                        delegate: Btn {
                            required property SettingsSection modelData
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

                    required property Setting modelData
                    required property int index
                    readonly property bool isFirst: index === 0
                    readonly property bool isLast: index === page.currentSettings.length - 1

                    Layout.fillWidth: true
                    implicitHeight: optionRow.implicitHeight + ((isFirst ? 0.5 : 1) + (isLast ? 0.5 : 1)) * Theme.rem

                    Rectangle {
                        visible: !option.isLast
                        anchors.bottom: parent.bottom
                        width: parent.width
                        height: Ui.hairline
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
                            Layout.fillHeight: option.modelData.input === Setting.GameSelect || option.modelData.input === Setting.Select
                            sourceComponent: {
                                switch (option.modelData.input) {
                                case Setting.Toggle:
                                    return toggleInput;
                                case Setting.GameSelect:
                                    return gameSelectInput;
                                case Setting.Select:
                                    return selectInput;
                                case Setting.Percent:
                                    return percentInput;
                                default:
                                    return null;
                                }
                            }

                            Component {
                                id: toggleInput
                                Checkbox {
                                    checked: option.modelData.value
                                    onToggled: checked => page.update(option.modelData.set(checked))
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
                                        value: option.modelData.value
                                        // applies as it moves, without saving every step
                                        onMoved: commit.restart()

                                        Timer {
                                            id: commit
                                            interval: 100
                                            onTriggered: page.update(option.modelData.set(Math.round(slider.value)))
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
                                id: selectInput
                                SelectDropdown {
                                    options: option.modelData.options.map(value => ({
                                                label: I18n.t(`settings.options.${option.modelData.key}.${value}`),
                                                value: value,
                                                selected: option.modelData.value === value
                                            }))
                                    onChanged: (value, selected) => {
                                        if (selected)
                                            page.update(option.modelData.set(value));
                                    }
                                }
                            }
                            Component {
                                id: gameSelectInput
                                SelectDropdown {
                                    options: Games.all.map(game => ({
                                                label: game.name,
                                                value: game.id,
                                                selected: option.modelData.value === game.id
                                            })).sort((a, b) => a.label.localeCompare(b.label))
                                    onChanged: (value, selected) => {
                                        if (selected)
                                            page.update(option.modelData.set(value));
                                    }
                                }
                            }
                        }

                        Btn {
                            variant: "plain"
                            iconName: "history"
                            padAll: 0.5 * fontSize
                            enabled: !option.modelData.isDefault
                            onClicked: page.update(option.modelData.reset())
                        }
                    }
                }
            }
        }
    }
}
