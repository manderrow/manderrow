import QtQuick
import QtQuick.Layouts
import Manderrow

// The details of the focused mod, ported from ModView.tsx.
//
// When it is narrow, its parts overflow as the web UI's do: they keep the widths of what they
// can't wrap, like the mod's name, and what overflows the view is cut off at its edge (overflow:
// hidden), except where the web UI scrolls instead, in the tabs and the content below them.
Item {
    id: view

    clip: true

    /// A mod listing or an installed mod package.
    required property var mod
    required property string gameId
    property var installContext: null

    signal closeRequested

    readonly property bool isPackage: mod.version !== undefined
    readonly property var installed: installContext ? installContext.installedOf(mod) : undefined

    /// All versions of the mod. A package's listing is looked up in the mod index.
    property var modListing: isPackage ? Object.assign(Util.removeProperty(mod, "version"), { versions: [mod.version] }) : mod
    property var selectedVersion: isPackage ? mod.version : undefined
    property string versionSearch: ""

    onModListingChanged: {
        if (selectedVersion === undefined && modListing)
            selectedVersion = modListing.versions[0];
    }

    Component.onCompleted: {
        if (!isPackage) {
            selectedVersion = mod.versions[0];
            return;
        }
        Api.fetchModIndex(gameId, false, () => {}).then(() => Api.getFromModIndex(gameId, [{ owner: mod.owner, name: mod.name }])).then(listings => {
            if (view && listings[0])
                view.modListing = listings[0];
        }).catch(e => Ui.window.reportError(e));
    }

    function isSelectedVersion(version) {
        return !!selectedVersion && selectedVersion.version_number === version.version_number;
    }

    function versionOptions(filter) {
        if (!modListing)
            return [];
        return modListing.versions.map((version, i) => ({
                    label: version.version_number,
                    secondary: Util.formatDateMed(version.date_created),
                    value: version,
                    selected: selectedVersion === undefined && i === 0 ? true : isSelectedVersion(version)
                })).filter(option => !filter || option.label.toLowerCase().includes(filter.toLowerCase()));
    }

    component MetaLink: Item {
        id: link
        property string text
        property string url
        property real fontSize: Theme.rem
        property int fontWeight: Font.Normal

        implicitWidth: linkRow.implicitWidth
        implicitHeight: linkRow.implicitHeight

        ClickArea {
            id: linkArea
            activatesOnSpace: false
            onActivated: Util.openUrl(link.url)
        }

        Row {
            id: linkRow
            spacing: 0.35 * Theme.rem

            Text {
                text: link.text
                color: linkArea.hovered ? Theme.primary70 : Theme.primary60
                font.family: Theme.font
                font.pixelSize: link.fontSize
                font.weight: link.fontWeight
                font.variableAxes: ({ "wght": link.fontWeight })
                anchors.verticalCenter: parent.verticalCenter
            }
            Icon {
                name: "external-link"
                color: linkArea.hovered ? Theme.primary70 : Theme.primary60
                anchors.verticalCenter: parent.verticalCenter
            }
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // the name, owner, metadata, and tabs
        ColumnLayout {
            Layout.fillWidth: true
            Layout.rightMargin: Theme.rem
            spacing: 0

            // grid-template-columns: minmax(min-content, 100%) 1fr. Its columns are no narrower than
            // what is in them, which overflows the view when it is narrower than that, rather than
            // widening the rest of it.
            GridLayout {
                Layout.fillWidth: true
                Layout.minimumWidth: 0
                Layout.bottomMargin: 0.75 * Theme.rem
                columns: 2
                rowSpacing: 0.25 * Theme.rem
                columnSpacing: 0.5 * Theme.rem

                // width: min-content
                MetaLink {
                    text: view.mod.name
                    url: Util.getModUrl(view.gameId, view.mod.owner, view.mod.name)
                    fontSize: 1.5 * Theme.rem
                    fontWeight: Font.Bold
                }

                Btn {
                    Layout.alignment: Qt.AlignRight | Qt.AlignTop
                    variant: "plain"
                    iconName: "x"
                    fontSize: 1.25 * Theme.rem
                    horizontalPadding: 0.5 * fontSize
                    verticalPadding: 0.1 * fontSize
                    onClicked: view.closeRequested()
                }

                // takes the column's free space, which a layout of items that don't fill it
                // otherwise wouldn't, leaving the grid narrower than the view
                ColumnLayout {
                    Layout.fillWidth: true
                    Layout.maximumWidth: Number.POSITIVE_INFINITY
                    spacing: 0.25 * Theme.rem

                    MetaLink {
                        text: view.mod.owner
                        url: Util.getModAuthorUrl(view.gameId, view.mod.owner)
                    }

                    Row {
                        // margin-top on top of the row gap
                        Layout.topMargin: 0.5 * Theme.rem
                        spacing: Theme.rem

                        component MetaField: Row {
                            property string icon
                            property string value
                            spacing: 0.3 * Theme.rem
                            Icon {
                                visible: parent.icon !== ""
                                name: parent.icon
                                color: Theme.neutral20
                                anchors.verticalCenter: parent.verticalCenter
                            }
                            SText {
                                text: parent.value
                                color: Theme.neutral20
                                font.family: Theme.font
                                font.pixelSize: Theme.rem
                            }
                        }

                        MetaField {
                            value: "v" + (view.selectedVersion ? view.selectedVersion.version_number : "...")
                        }
                        MetaField {
                            icon: "download"
                            value: view.selectedVersion ? Util.formatRounded(view.selectedVersion.downloads) : "..."
                        }
                        MetaField {
                            icon: "hard-drive"
                            value: view.selectedVersion ? Util.humanizeFileSize(view.selectedVersion.file_size) : "..."
                        }
                    }
                }

                // the donation link
                Rectangle {
                    visible: !!view.mod.donation_link
                    Layout.alignment: Qt.AlignRight | Qt.AlignVCenter
                    implicitWidth: donate.implicitWidth + 2 * Theme.rem
                    implicitHeight: donate.implicitHeight + Theme.rem
                    radius: 4
                    color: "transparent"
                    border.color: Theme.neutral70

                    Column {
                        id: donate
                        anchors.centerIn: parent

                        Icon {
                            anchors.horizontalCenter: parent.horizontalCenter
                            name: "heart"
                            size: 1.25 * 0.9 * Theme.rem
                            color: donateHover.hovered ? Theme.primary70 : Theme.primary60
                        }
                        Text {
                            anchors.horizontalCenter: parent.horizontalCenter
                            text: I18n.t("modlist.modview.donate_btn")
                            color: donateHover.hovered ? Theme.primary70 : Theme.primary60
                            font.family: Theme.font
                            font.pixelSize: 0.9 * Theme.rem
                        }
                    }

                    ClickArea {
                        id: donateHover
                        radius: parent.radius
                        activatesOnSpace: false
                        onActivated: Util.openUrl(view.mod.donation_link)
                    }
                }
            }

            UnderlineTabs {
                id: tabs
                Layout.fillWidth: true
                tabListId: "mod-view"
                model: [
                    {
                        id: "overview",
                        name: "Overview"
                    },
                    {
                        id: "dependencies",
                        name: "Dependencies"
                    },
                    {
                        id: "changelog",
                        name: "Changelog"
                    }
                ]
            }
        }

        // overflow: auto: the content wraps to the view's width, and scrolls sideways to reach what
        // doesn't fit in it, like a long word or a dependency's name
        ScrollArea {
            Layout.fillWidth: true
            Layout.fillHeight: true
            flickable: scroll

            Flickable {
                id: scroll
                objectName: "modViewScroll"
                contentWidth: width + overflow
                contentHeight: scrollContent.implicitHeight
                boundsBehavior: Flickable.StopAtBounds
                clip: true

                /// How far the content reaches past the view.
                property real overflow: 0
                // measured once the content has been laid out at a new width, so that where it
                // reached at the old one doesn't show a horizontal scroll bar for a moment
                Binding on overflow {
                    value: tabContent.item ? Math.max(0, tabContent.item.overflowWidth - scrollContent.width) : 0
                    delayed: true
                }

                Item {
                    id: scrollContent
                    width: scroll.width
                    implicitHeight: tabContent.implicitHeight + 2 * Theme.rem

                    Loader {
                        id: tabContent
                        y: Theme.rem
                        width: parent.width
                        sourceComponent: tabs.currentId === "dependencies" ? dependenciesTab : markdownTab
                    }
                }
            }
        }

        Component {
            id: markdownTab
            ModMarkdown {
                // .markdown { padding-right: 0.75em }
                rightPadding: 0.75 * Theme.rem
                mod: view.mod
                selectedVersion: view.selectedVersion ? view.selectedVersion.version_number : undefined
                endpoint: tabs.currentId === "changelog" ? "changelog" : "readme"
            }
        }
        Component {
            id: dependenciesTab
            ModDependencies {
                gameId: view.gameId
                dependencies: view.selectedVersion ? view.selectedVersion.dependencies : undefined
            }
        }

        // the install, version, and uninstall controls, which keep their min-content widths, as
        // flex items do, and are cut off at the view's edge when they don't fit
        Item {
            Layout.fillWidth: true
            Layout.rightMargin: Theme.rem
            implicitHeight: (view.installed === undefined ? onlineActions.implicitHeight : installedActions.implicitHeight) + 0.5 * Theme.rem + 0.75 * Theme.rem + 1

            Rectangle {
                width: parent.width
                height: 1
                color: Theme.neutral70
            }

            RowLayout {
                id: onlineActions
                visible: view.installed === undefined && view.installContext !== null
                y: 0.5 * Theme.rem + 1
                width: parent.width
                spacing: 0.25 * Theme.rem

                SelectDropdown {
                    objectName: "versionSelect"
                    Layout.fillWidth: true
                    Layout.minimumWidth: implicitWidth
                    Layout.maximumWidth: 8 * Theme.rem
                    Layout.fillHeight: true
                    options: view.versionOptions("")
                    optionFontSize: 0.9 * Theme.rem
                    optionPadding: 0.25 * 0.9 * Theme.rem
                    trigger.bgColor: Theme.neutral80
                    trigger.borderColor: Theme.neutral60
                    trigger.activeBorderColor: Theme.neutral60
                    trigger.borderWidth: 1
                    // border-radius: 100vmax 0 0 100vmax
                    trigger.rightRadius: 0
                    onChanged: value => view.selectedVersion = value
                }

                // installs the latest version, as in the web UI
                InstallButton {
                    Layout.fillWidth: true
                    Layout.minimumWidth: implicitWidth
                    mod: view.mod
                    installContext: view.installContext
                    text: I18n.t("modlist.online.install_btn")
                    fontSize: 1.15 * Theme.rem
                    btn.fontWeight: Font.Medium
                    // border-radius: 0 100vmax 100vmax 0
                    btn.leftRadius: 0
                    btn.horizontalPadding: 0.25 * fontSize
                    btn.verticalPadding: 0.5 * fontSize
                }
            }

            RowLayout {
                id: installedActions
                visible: view.installed !== undefined
                y: 0.5 * Theme.rem + 1
                width: parent.width
                spacing: 0.75 * Theme.rem

                // width: 100%, shrinking alike, with the label wrapped once it doesn't fit, and
                // stretched to the row's height
                Dropdown {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    Layout.preferredWidth: 1
                    Layout.minimumWidth: trigger.minimumWidth
                    label: I18n.t("modlist.installed.change_version_btn")
                    trigger.wrapText: true
                    fillToTriggerWidth: true
                    panelPadding: 0.5 * Theme.rem

                    ColumnLayout {
                        spacing: 0

                        Field {
                            Layout.fillWidth: true
                            Layout.leftMargin: 0.5 * Theme.rem
                            Layout.rightMargin: 0.5 * Theme.rem
                            Layout.bottomMargin: 0.5 * Theme.rem
                            placeholderText: I18n.t("modlist.installed.search_version_placeholder")
                            text: view.versionSearch
                            onTextEdited: view.versionSearch = text
                        }

                        SelectList {
                            Layout.fillWidth: true
                            Layout.preferredHeight: Math.min(contentHeight, 20 * Theme.rem)
                            options: view.versionOptions(view.versionSearch)
                            fontSize: 0.9 * Theme.rem
                            itemPadding: 0.25 * 0.9 * Theme.rem
                            onChanged: value => view.selectedVersion = value
                        }

                        InstallButton {
                            Layout.fillWidth: true
                            Layout.leftMargin: 0.5 * Theme.rem
                            Layout.rightMargin: 0.5 * Theme.rem
                            Layout.topMargin: 0.5 * Theme.rem
                            mod: Object.assign(Util.removeProperty(view.modListing, "versions"), {
                                version: view.modListing.versions.find(v => view.isSelectedVersion(v)) || view.modListing.versions[0]
                            })
                            installContext: view.installContext
                            text: I18n.t("global.phrases.apply")
                            btn.enabled: !busy && !(view.isPackage && view.isSelectedVersion(view.mod.version))
                        }
                    }
                }

                UninstallButton {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    Layout.preferredWidth: 1
                    Layout.minimumWidth: implicitWidth
                    mod: view.installed || view.mod
                    installContext: view.installContext
                    text: I18n.t("modlist.installed.uninstall_btn")
                    btn.bgColor: "transparent"
                    btn.activeBgColor: Theme.danger5
                    btn.borderColor: Theme.danger4
                    btn.activeBorderColor: Theme.danger5
                    btn.borderWidth: 1
                    progressFromColor: Theme.danger4
                    progressToColor: Theme.danger6
                    progressBorderColor: Theme.danger4
                }
            }
        }
    }
}
