import QtQuick
import QtQuick.Layouts
import Manderrow

// The profile view of a game, ported from ProfileWithGame in Profile.tsx.
Item {
    id: view

    required property string gameId
    property var profileId
    property bool gameSelectShowing: false

    signal showGameSelect

    readonly property var profiles: {
        const out = {};
        for (const profile of Globals.profiles) {
            if (profile.game === gameId)
                out[profile.id] = profile;
        }
        return out;
    }
    readonly property var currentProfile: profileId === undefined ? undefined : profiles[profileId]

    // Update the title bar with the current profile's name
    readonly property string titleName: currentProfile === undefined || gameSelectShowing ? "" : currentProfile.name
    onTitleNameChanged: Ui.currentProfileName = titleName
    Component.onCompleted: Ui.currentProfileName = titleName
    Component.onDestruction: Ui.currentProfileName = ""

    onCurrentProfileChanged: {
        if (profileId !== undefined && currentProfile === undefined)
            Ui.window.reportError(new Error(`Unknown profile ${profileId}`));
    }

    /// The width of the sidebar where it doesn't expand over the content.
    readonly property real sidebarWidth: Theme.windowWidth <= 1200 ? sidebar.miniWidth : sidebar.fullWidth
    // the height of the profile's tab bar, while it shows
    property real _tabBarHeight: 0

    // launch errors are tracked here instead of being reported globally
    property var launchError: null

    function launch(modded) {
        ConsoleStore.allocate(profileId).then(conn => {
            ConsoleStore.focusedConnection = conn;
            if (Globals.settings.openConsoleOnLaunch.value && Router.search["profile-tab"] !== "logs")
                Router.setSearch({ "profile-tab": "logs" });
            return Api.launchProfile(conn.connId, profileId !== undefined ? { profile: profileId } : { vanilla: gameId }, modded).catch(error => {
                conn.handleEvent({ type: "Error", error });
                view.launchError = error;
            });
        }).catch(e => view.launchError = e);
    }

    function killGame() {
        const conn = ConsoleStore.focusedConnection;
        if (conn === null)
            return;
        Api.killIpcClient(conn.connId).catch(error => {
            conn.handleEvent({ type: "Error", error });
            view.launchError = error;
        });
    }

    InstallContext {
        id: profileInstallContext
        profileId: view.profileId === undefined ? "" : view.profileId
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            // the content's background, beside the sidebar's, and below the tab bar's, so that
            // they don't overlap where the window is translucent
            Rectangle {
                x: view.sidebarWidth
                y: view._tabBarHeight
                width: parent.width - x
                height: parent.height - y
                color: Theme.surface(Theme.neutral85)
            }

            // the content, to the right of the sidebar. Its padding is left by each part of it that
            // has any, so that what reaches its edges, as the tab bar does, stays within it: Qt
            // can skip delivering the pointer to items outside their parent.
            Item {
                id: content
                anchors.fill: parent
                anchors.leftMargin: view.sidebarWidth

                Loader {
                    anchors.fill: parent
                    anchors.margins: Theme.rem
                    anchors.bottomMargin: 0
                    active: view.profileId === undefined
                    sourceComponent: Item {
                        ColumnLayout {
                            anchors.centerIn: parent
                            spacing: 0.5 * Theme.rem

                            Text {
                                Layout.alignment: Qt.AlignHCenter
                                text: I18n.t("profile.blank.no_profiles_title")
                                color: Theme.neutral05
                                font.family: Theme.font
                                font.pixelSize: 1.5 * Theme.rem
                                font.weight: Font.Bold
                                font.variableAxes: ({ "wght": Font.Bold })
                            }
                            Text {
                                Layout.alignment: Qt.AlignHCenter
                                text: I18n.t("profile.blank.no_profiles_subtitle")
                                color: Theme.neutral20
                                font.family: Theme.font
                                font.pixelSize: Theme.rem
                            }
                            Btn {
                                Layout.alignment: Qt.AlignHCenter
                                Layout.topMargin: 0.5 * Theme.rem
                                variant: "primary"
                                text: I18n.t("profile.blank.create_profile_btn")
                                onClicked: sidebar.createProfile()
                            }
                        }
                    }
                }

                Loader {
                    anchors.fill: parent
                    active: view.profileId !== undefined
                    sourceComponent: ColumnLayout {
                        spacing: 0

                        Rectangle {
                            id: tabBar
                            Layout.fillWidth: true
                            Layout.bottomMargin: 0.75 * Theme.rem
                            implicitHeight: tabs.implicitHeight + 2 * 0.3 * Theme.rem + 1
                            color: Theme.surface(Theme.neutral90)

                            PillTabs {
                                id: tabs
                                x: 0.3 * Theme.rem
                                y: 0.3 * Theme.rem
                                tabListId: "profile"
                                model: [
                                    {
                                        id: "mod-list",
                                        name: I18n.t("profile.tabs.installed")
                                    },
                                    {
                                        id: "mod-search",
                                        name: I18n.t("profile.tabs.online")
                                    },
                                    {
                                        id: "logs",
                                        name: "Logs"
                                    },
                                    {
                                        id: "config",
                                        name: I18n.t("profile.tabs.config")
                                    }
                                ]
                            }

                            Rectangle {
                                anchors.bottom: parent.bottom
                                width: parent.width
                                height: 1
                                color: Theme.neutral80
                            }
                        }
                        Binding {
                            target: view
                            property: "_tabBarHeight"
                            value: tabBar.height
                        }

                        Loader {
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            Layout.leftMargin: Theme.rem
                            // the mod lists and the log reach the window's edge, and leave its
                            // padding themselves
                            Layout.rightMargin: ["mod-list", "mod-search", "logs"].includes(tabs.currentId) ? 0 : Theme.rem
                            sourceComponent: {
                                switch (tabs.currentId) {
                                case "mod-list":
                                    return installedTab;
                                case "mod-search":
                                    return onlineTab;
                                case "logs":
                                    return logsTab;
                                default:
                                    return configTab;
                                }
                            }
                        }

                        Component {
                            id: installedTab
                            InstalledModList {
                                game: view.gameId
                                installContext: profileInstallContext
                            }
                        }
                        Component {
                            id: onlineTab
                            OnlineModList {
                                game: view.gameId
                                installContext: profileInstallContext
                            }
                        }
                        Component {
                            id: logsTab
                            Console {}
                        }
                        Component {
                            id: configTab
                            Item {}
                        }
                    }
                }
            }
        }

        StatusBar {
            Layout.fillWidth: true
            leftInset: view.sidebarWidth
        }
    }

    // the sidebar spans the full height, over the start of the status bar
    Sidebar {
        id: sidebar
        height: parent.height
        z: 2
        gameId: view.gameId
        profileId: view.profileId
        profiles: view.profiles
        focusedConnection: ConsoleStore.focusedConnection
        onBackClicked: view.showGameSelect()
        onLaunchClicked: view.launch(true)
        onKillClicked: view.killGame()
        onCancelClicked: ConsoleStore.focusedConnection = null
    }

    Repeater {
        model: ConsoleStore.doctorReports
        delegate: DoctorDialog {
            required property var modelData
            required property int index
            report: modelData
            Component.onCompleted: open()
            onDismissed: ConsoleStore.dismissDoctorReport(index)
        }
    }

    Loader {
        active: view.launchError !== null
        sourceComponent: ErrorDialog {
            visible: true
            err: view.launchError
            reset: () => view.launchError = null
        }
    }
}
