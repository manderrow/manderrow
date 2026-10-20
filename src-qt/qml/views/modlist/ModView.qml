import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Manderrow

// The details of the focused mod, ported from ModView.tsx.
Item {
    id: view

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

    component MetaLink: Row {
        id: link
        property string text
        property string url
        property real fontSize: Theme.rem
        property int fontWeight: Font.Normal
        spacing: 0.35 * Theme.rem

        Text {
            text: link.text
            color: linkHover.hovered ? Theme.primary70 : Theme.primary60
            font.family: Theme.font
            font.pixelSize: link.fontSize
            font.weight: link.fontWeight
            anchors.verticalCenter: parent.verticalCenter
        }
        FaIcon {
            name: "externalLink"
            color: linkHover.hovered ? Theme.primary70 : Theme.primary60
            anchors.verticalCenter: parent.verticalCenter
        }
        HoverHandler {
            id: linkHover
            cursorShape: Qt.PointingHandCursor
        }
        TapHandler {
            onTapped: Util.openUrl(link.url)
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

            GridLayout {
                Layout.fillWidth: true
                Layout.bottomMargin: 0.75 * Theme.rem
                columns: 2
                rowSpacing: 0.25 * Theme.rem
                columnSpacing: 0.5 * Theme.rem

                MetaLink {
                    Layout.fillWidth: true
                    text: view.mod.name
                    url: Util.getModUrl(view.gameId, view.mod.owner, view.mod.name)
                    fontSize: 1.5 * Theme.rem
                    fontWeight: Font.Bold
                }

                Btn {
                    Layout.alignment: Qt.AlignRight | Qt.AlignTop
                    variant: "plain"
                    iconName: "xmark"
                    fontSize: 1.25 * Theme.rem
                    horizontalPadding: 0.5 * fontSize
                    verticalPadding: 0.1 * fontSize
                    onClicked: view.closeRequested()
                }

                ColumnLayout {
                    Layout.fillWidth: true
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
                            FaIcon {
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
                            icon: "hardDrive"
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

                        FaIcon {
                            anchors.horizontalCenter: parent.horizontalCenter
                            name: "regular:heart"
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

                    HoverHandler {
                        id: donateHover
                        cursorShape: Qt.PointingHandCursor
                    }
                    TapHandler {
                        onTapped: Util.openUrl(view.mod.donation_link)
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

        Flickable {
            id: scroll
            objectName: "modViewScroll"
            Layout.fillWidth: true
            Layout.fillHeight: true
            contentWidth: width
            contentHeight: scrollContent.implicitHeight
            boundsBehavior: Flickable.StopAtBounds
            clip: true
            ScrollBar.vertical: AppScrollBar {}
            WheelScroll {}

            Item {
                id: scrollContent
                width: scroll.width
                implicitHeight: tabContent.implicitHeight + 2 * Theme.rem

                Loader {
                    id: tabContent
                    y: Theme.rem
                    width: parent.width - 0.75 * Theme.rem
                    sourceComponent: tabs.currentId === "dependencies" ? dependenciesTab : markdownTab
                }
            }
        }

        Component {
            id: markdownTab
            ModMarkdown {
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

        // the install, version, and uninstall controls
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
                    Layout.fillWidth: true
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

                Dropdown {
                    id: versionDropdown
                    Layout.fillWidth: true
                    Layout.preferredWidth: 1
                    label: I18n.t("modlist.installed.change_version_btn")
                    fillToTriggerWidth: true
                    panelPadding: 0.5 * Theme.rem
                    contentWidth: trigger.width
                    contentHeight: versionPanel.implicitHeight

                    ColumnLayout {
                        id: versionPanel
                        x: 0.5 * Theme.rem
                        width: versionDropdown.panel.availableWidth - Theme.rem
                        spacing: 0

                        Field {
                            Layout.fillWidth: true
                            Layout.bottomMargin: 0.5 * Theme.rem
                            placeholderText: I18n.t("modlist.installed.search_version_placeholder")
                            onTextChanged: view.versionSearch = text
                        }

                        SelectList {
                            Layout.fillWidth: true
                            Layout.leftMargin: -0.5 * Theme.rem
                            Layout.rightMargin: -0.5 * Theme.rem
                            Layout.preferredHeight: Math.min(contentHeight, 20 * Theme.rem)
                            options: view.versionOptions(view.versionSearch)
                            fontSize: 0.9 * Theme.rem
                            itemPadding: 0.25 * 0.9 * Theme.rem
                            onChanged: value => view.selectedVersion = value
                        }

                        InstallButton {
                            Layout.fillWidth: true
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
                    Layout.preferredWidth: 1
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
