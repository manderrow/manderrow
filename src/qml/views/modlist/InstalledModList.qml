import QtQuick
import QtQuick.Layouts
import Manderrow

// The mods installed in the profile, ported from InstalledModList in ModList.tsx.
Item {
    id: root

    property string game
    property Profile profile: null
    readonly property list<QtObject> installed: profile ? profile.mods : []

    /// The available updates: `[{ newMod, oldVersionNumber }]`, where `newMod` is the summary of a
    /// mod whose latest version isn't the one installed.
    property var updates: []
    property bool updatesLoading: false
    property Task checkUpdatesTask: null
    property int _updatesGeneration: 0

    /// Looks for updates to the installed mods. With `refresh`, the mod index is refetched first.
    function checkUpdates(refresh: bool): var {
        const generation = ++_updatesGeneration;
        const profile = root.profile;
        if (!profile || profile.mods.length === 0) {
            updates = [];
            updatesLoading = false;
            return Promise.resolve();
        }
        updatesLoading = true;
        const fetch = ModIndex.fetch(game, refresh);
        checkUpdatesTask = Tasks.get(fetch.taskId);
        return Async.co(function* () {
            yield fetch;
            const newMods = yield profile.findUpdates();
            if (!root || generation !== root._updatesGeneration)
                return;
            root.updates = newMods.map(newMod => ({
                        newMod,
                        oldVersionNumber: profile.installedMod(newMod.owner, newMod.name).version.versionNumber
                    }));
            root.updatesLoading = false;
        }).catch(e => {
            if (!root)
                return;
            if (generation === root._updatesGeneration)
                root.updatesLoading = false;
            throw e;
        });
    }

    Connections {
        target: root.profile
        function onModsChanged() {
            root.checkUpdates(false).catch(e => Ui.window.reportError(e));
        }
    }
    Component.onCompleted: checkUpdates(false).catch(e => Ui.window.reportError(e))

    ColumnLayout {
        visible: root.installed.length === 0
        anchors.centerIn: parent
        // centred within the page's padding, which the list leaves beside it
        anchors.horizontalCenterOffset: -Theme.rem / 2
        spacing: 0.5 * Theme.rem

        Text {
            Layout.alignment: Qt.AlignHCenter
            text: Strings.modlist.installed.no_mods_title
            color: Theme.neutral05
            font.family: Theme.font
            font.pixelSize: 1.5 * Theme.rem
            font.weight: Font.Bold
            font.variableAxes: ({ "wght": Font.Bold })
        }
        Text {
            Layout.alignment: Qt.AlignHCenter
            text: Strings.modlist.installed.no_mods_msg
            color: Theme.neutral20
            font.family: Theme.font
            font.pixelSize: Theme.rem
        }
        Btn {
            Layout.alignment: Qt.AlignHCenter
            Layout.topMargin: 0.5 * Theme.rem
            variant: "primary"
            text: Strings.modlist.installed.browse_btn
            onClicked: Router.setSearch({ "profile-tab": "mod-search" })
        }
    }

    ModListView {
        visible: root.installed.length !== 0
        anchors.fill: parent
        game: root.game
        profile: root.profile
        multiselect: true
        refresh: () => root.profile ? root.profile.refreshMods() : undefined
        // TODO: implement filter and sort
        fetcher: {
            const data = root.installed;
            return (game, query, sort) => ({
                    count: data.length,
                    mods: page => page === 0 ? data : [],
                    get: id => data.find(mod => Util.modIdEquals(mod, id))
                });
        }

        // their labels wrap when the line is short of space, as on the web
        trailingControls: Component {
            Item {
                readonly property real minimumWidth: root.updates.length > 0 ? updatesBtn.minimumWidth : checkBtn.minimumWidth

                implicitWidth: root.updates.length > 0 ? updatesBtn.implicitWidth : checkBtn.implicitWidth
                implicitHeight: root.updates.length > 0 ? updatesBtn.implicitHeight : checkBtn.implicitHeight

                AsyncButton {
                    id: checkBtn
                    visible: root.updates.length === 0
                    width: parent.width
                    btn.wrapText: true
                    variant: "ghost"
                    iconName: "refresh-cw"
                    text: Strings.modlist.installed.check_updates_btn
                    externalBusy: root.updatesLoading
                    progressMode: "external"
                    externalTask: root.checkUpdatesTask
                    action: () => root.checkUpdates(true)
                }

                Btn {
                    id: updatesBtn
                    visible: root.updates.length > 0
                    width: parent.width
                    wrapText: true
                    variant: "primary"
                    iconName: "circle-arrow-up"
                    iconFilled: true
                    text: Strings.modlist.installed.updates_available_btn
                    onClicked: updater.open()
                }

                PopupLoader {
                    id: updater
                    sourceComponent: UpdaterDialog {
                        objectName: "updaterDialog"
                        updates: root.updates
                    }
                }
            }
        }
    }
}
