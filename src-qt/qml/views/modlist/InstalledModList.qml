import QtQuick
import QtQuick.Layouts
import Manderrow

// The mods installed in the profile, ported from InstalledModList in ModList.tsx.
Item {
    id: root

    property string game
    property var installContext

    // available updates: `[{ newMod, oldVersionNumber }]`
    property var updates: []
    property bool updatesLoading: false
    property var checkUpdatesProgress: Tasks.initProgress()
    property int _updatesGeneration: 0

    /// Looks for updates to the installed mods. With `refresh`, the mod index is refetched first.
    function checkUpdates(refresh) {
        const generation = ++_updatesGeneration;
        const installed = installContext.installed;
        if (installed.length === 0) {
            updates = [];
            updatesLoading = false;
            return Promise.resolve();
        }
        updatesLoading = true;
        return Api.fetchModIndex(game, refresh, event => {
            // the component may have been destroyed by the time this runs
            if (root && event.event === "progress")
                root.checkUpdatesProgress = event.progress;
        }).then(() => Api.getFromModIndex(game, installed.map(mod => ({ owner: mod.owner, name: mod.name })))).then(latest => {
            if (!root || generation !== root._updatesGeneration)
                return;
            root.updates = latest.map((mod, i) => ({ newMod: mod, oldVersionNumber: installed[i].version.version_number })).filter(update => update.newMod.versions[0].version_number !== update.oldVersionNumber);
            root.updatesLoading = false;
        }, e => {
            if (!root)
                return;
            if (generation === root._updatesGeneration)
                root.updatesLoading = false;
            throw e;
        });
    }

    Connections {
        target: root.installContext
        function onInstalledChanged() {
            root.checkUpdates(false).catch(e => Ui.window.reportError(e));
        }
    }
    Component.onCompleted: checkUpdates(false).catch(e => Ui.window.reportError(e))

    ColumnLayout {
        visible: root.installContext.installed.length === 0
        anchors.centerIn: parent
        // centred within the page's padding, which the list leaves beside it
        anchors.horizontalCenterOffset: -Theme.rem / 2
        spacing: 0.5 * Theme.rem

        Text {
            Layout.alignment: Qt.AlignHCenter
            text: I18n.t("modlist.installed.no_mods_title")
            color: Theme.neutral05
            font.family: Theme.font
            font.pixelSize: 1.5 * Theme.rem
            font.weight: Font.Bold
            font.variableAxes: ({ "wght": Font.Bold })
        }
        Text {
            Layout.alignment: Qt.AlignHCenter
            text: I18n.t("modlist.installed.no_mods_msg")
            color: Theme.neutral20
            font.family: Theme.font
            font.pixelSize: Theme.rem
        }
        Btn {
            Layout.alignment: Qt.AlignHCenter
            Layout.topMargin: 0.5 * Theme.rem
            variant: "primary"
            text: I18n.t("modlist.installed.browse_btn")
            onClicked: Router.setSearch({ "profile-tab": "mod-search" })
        }
    }

    ModListView {
        visible: root.installContext.installed.length !== 0
        anchors.fill: parent
        game: root.game
        installContext: root.installContext
        multiselect: true
        progress: Tasks.initProgress()
        refresh: () => root.installContext.refetchInstalled()
        // TODO: implement filter and sort
        fetcher: {
            const data = root.installContext.installed;
            return (game, query, sort) => ({
                    count: data.length,
                    mods: page => page === 0 ? data : [],
                    get: id => data.find(mod => Util.modIdEquals(mod, id))
                });
        }

        trailingControls: Component {
            Item {
                implicitWidth: root.updates.length > 0 ? updatesBtn.implicitWidth : checkBtn.implicitWidth
                implicitHeight: root.updates.length > 0 ? updatesBtn.implicitHeight : checkBtn.implicitHeight

                AsyncButton {
                    id: checkBtn
                    visible: root.updates.length === 0
                    variant: "ghost"
                    iconName: "refresh-cw"
                    text: I18n.t("modlist.installed.check_updates_btn")
                    externalBusy: root.updatesLoading
                    progressMode: "external"
                    externalProgress: root.checkUpdatesProgress
                    action: () => root.checkUpdates(true)
                }

                Btn {
                    id: updatesBtn
                    visible: root.updates.length > 0
                    variant: "primary"
                    iconName: "circle-arrow-up"
                    iconFilled: true
                    text: I18n.t("modlist.installed.updates_available_btn")
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
