import QtQuick
import QtQuick.Controls.Basic
import Manderrow

// A profile in the sidebar, ported from SidebarProfileComponent.
Rectangle {
    id: item

    required property var profile
    required property string gameId
    /// Whether this is the profile being viewed.
    property bool selected: false
    /// Whether this profile is part of the multi-selection.
    property bool highlighted: false
    property bool isPivot: false
    /// Whether the list containing this item has focus, which shows the pivot.
    property bool listFocused: false

    signal ctrlClicked
    signal shiftClicked

    property bool renaming: false
    property bool _deleting: false

    readonly property bool _menuOpen: menu.opened
    readonly property bool _hot: hover.hovered || _menuOpen || options.activeFocus
    readonly property var _activeConnection: {
        ConsoleStore.connectionsUpdate; // track
        return ConsoleStore.connections[ConsoleStore.connectionsUpdate];
    }
    readonly property bool _running: !!_activeConnection && _activeConnection.status !== "disconnected" && _activeConnection.profileId === profile.id

    implicitHeight: renaming ? editorLoader.implicitHeight : name.implicitHeight
    color: highlighted ? Theme.primary25 : selected ? Theme.neutral80 : _hot ? Theme.neutral85 : "transparent"

    function _metadata(changes) {
        return Object.assign({ name: profile.name, game: profile.game, pinned: profile.pinned }, changes);
    }

    function _delete() {
        if (selected)
            Router.navigate(`/profile/${gameId}`, { replace: true });
        if (_deleting)
            return Promise.resolve();
        _deleting = true;
        const refetch = () => {
            item._deleting = false;
            return Globals.refetchProfiles();
        };
        return Api.deleteProfile(profile.id).then(refetch, e => refetch().then(() => {
            throw e;
        }));
    }

    HoverHandler {
        id: hover
    }

    // the pivot of a shift-click range selection
    Rectangle {
        anchors.fill: parent
        color: "transparent"
        border.color: Theme.primary45
        border.width: 1
        visible: item.isPivot && (item.listFocused || item._menuOpen)
        z: 2
    }

    Loader {
        id: editorLoader
        active: item.renaming
        width: parent.width
        sourceComponent: ProfileNameEditor {
            initialValue: item.profile.name
            submit: name => Api.overwriteProfileMetadata(item.profile.id, item._metadata({ name })).then(() => Globals.refetchProfiles(), e => Globals.refetchProfiles().then(() => {
                    throw e;
                }))
            onCancelled: item.renaming = false
        }
    }

    Item {
        id: name
        visible: !item.renaming
        width: parent.width
        implicitHeight: label.implicitHeight + 2 * 0.35 * Theme.rem

        FaIcon {
            visible: item._running
            name: "playCircle"
            size: 0.6 * Theme.rem
            color: Theme.primary60
            x: 0.2 * Theme.rem
            anchors.verticalCenter: parent.verticalCenter

            HoverHandler {
                id: playingHover
            }
            Tip {
                text: I18n.t("profile.sidebar.profile_running_icon")
                visible: playingHover.hovered
            }
        }

        Text {
            id: label
            x: Theme.rem
            width: parent.width - 2 * Theme.rem
            anchors.verticalCenter: parent.verticalCenter
            text: item.profile.name
            elide: Text.ElideRight
            color: item.selected ? (item._hot ? Theme.primary70 : Theme.primary60) : Theme.neutral20
            font.family: Theme.font
            font.pixelSize: Theme.rem
            font.weight: item.selected ? 620 : Font.Normal
        }

        TapHandler {
            onTapped: (eventPoint, button) => {
                // shift clicks take priority over control clicks
                if (Native.shift)
                    item.shiftClicked();
                else if (Native.ctrl)
                    item.ctrlClicked();
                else
                    Router.navigate(`/profile/${item.gameId}/${item.profile.id}`);
            }
            onDoubleTapped: {
                if (!Native.ctrl && !Native.shift)
                    item.renaming = true;
            }
        }
        HoverHandler {
            cursorShape: Qt.PointingHandCursor
        }
    }

    // the buttons revealed on hover
    Row {
        id: options
        visible: !item.renaming
        anchors.right: parent.right
        anchors.rightMargin: 0.25 * Theme.rem
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        anchors.topMargin: 2
        anchors.bottomMargin: 2
        opacity: item._hot ? 1 : 0
        enabled: item._hot

        Behavior on opacity {
            NumberAnimation {
                duration: 100
            }
        }

        // fades the profile name out under the buttons
        Rectangle {
            width: Theme.rem
            height: parent.height
            gradient: Gradient {
                orientation: Gradient.Horizontal
                GradientStop {
                    position: 0
                    color: "transparent"
                }
                GradientStop {
                    position: 1
                    color: item.color
                }
            }
        }

        Rectangle {
            width: buttons.width
            height: parent.height
            color: item.color

            Row {
                id: buttons
                height: parent.height

                component OptionButton: Btn {
                    variant: "plain"
                    radius: 4
                    height: parent.height
                    width: Math.max(height, 1.25 * Theme.rem)
                    horizontalPadding: 0.1 * Theme.rem
                    verticalPadding: 0.2 * Theme.rem
                }

                AsyncButton {
                    height: parent.height
                    width: Math.max(height, 1.25 * Theme.rem)
                    variant: "plain"
                    btn.radius: 4
                    btn.horizontalPadding: 0.1 * Theme.rem
                    btn.verticalPadding: 0.2 * Theme.rem
                    btn.iconRotation: 90
                    iconName: item.profile.pinned ? "thumbTackSlash" : "thumbTack"
                    tooltip: item.profile.pinned ? I18n.t("profile.sidebar.unpin_profile_btn") : I18n.t("profile.sidebar.pin_profile_btn")
                    action: () => Api.overwriteProfileMetadata(item.profile.id, item._metadata({ pinned: !item.profile.pinned })).then(() => Globals.refetchProfiles(), e => Globals.refetchProfiles().then(() => {
                            throw e;
                        }))
                }

                OptionButton {
                    visible: Native.shift
                    iconName: "copy"
                    tooltip: I18n.t("profile.sidebar.duplicate_profile_btn")
                }
                OptionButton {
                    visible: Native.shift
                    iconName: "clipboard"
                    tooltip: I18n.t("profile.sidebar.copy_id_profile_btn")
                }
                OptionButton {
                    visible: Native.shift
                    iconName: "trashCan"
                    tooltip: I18n.t("global.phrases.delete")
                    onClicked: Api.deleteProfile(item.profile.id).then(() => Globals.refetchProfiles(), e => Globals.refetchProfiles().then(() => Ui.window.reportError(e)))
                }

                ContextMenu {
                    id: menu
                    objectName: "profileMenu"
                    height: parent.height
                    width: Math.max(height, 1.25 * Theme.rem)
                    trigger.variant: "plain"
                    trigger.radius: 4
                    trigger.iconName: "ellipsisVertical"
                    trigger.horizontalPadding: 0.1 * Theme.rem
                    trigger.verticalPadding: 0.2 * Theme.rem
                    trigger.tooltip: menu.opened ? "" : I18n.t("profile.sidebar.ellipsis_btn")
                    items: [
                        {
                            icon: "penToSquare",
                            label: I18n.t("profile.sidebar.rename_profile_btn"),
                            action: () => item.renaming = true
                        },
                        {
                            icon: "trashCan",
                            label: I18n.t("global.phrases.delete"),
                            action: () => deletePrompt.open()
                        },
                        {
                            icon: "copy",
                            label: I18n.t("profile.sidebar.duplicate_profile_btn"),
                            // TODO: implement duplicate profile
                            action: () => {}
                        },
                        {
                            icon: "share",
                            label: I18n.t("profile.sidebar.share_profile_btn"),
                            // TODO: implement share profile
                            action: () => {}
                        },
                        {
                            spacer: true
                        },
                        {
                            icon: "clipboard",
                            label: I18n.t("profile.sidebar.copy_id_profile_btn"),
                            // TODO: implement copy profile ID
                            action: () => {}
                        },
                        {
                            icon: "folderOpen",
                            iconOffset: 2,
                            label: I18n.t("profile.sidebar.open_folder_profile_btn"),
                            // TODO: implement open folder
                            action: () => {}
                        }
                    ]
                }
            }
        }
    }

    PromptDialog {
        id: deletePrompt
        title: I18n.t("global.phrases.confirm")
        question: I18n.t("profile.delete_msg", { profileName: item.profile.name })
        okType: "danger"
        okText: I18n.t("global.phrases.delete")
        onAccepted: {
            close();
            item._delete().catch(e => Ui.window.reportError(e));
        }
    }
}
