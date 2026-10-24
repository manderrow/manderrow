import QtQuick
import QtQuick.Controls.Basic
import Manderrow

// A profile in the sidebar, ported from SidebarProfileComponent.
Rectangle {
    id: profileItem

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
    // the options also show while the focus is within the item, as on the web
    readonly property bool _hot: hover.hovered || _menuOpen || Ui.isWithin(Window.activeFocusItem, profileItem)
    /// Whether a game launched from this profile is still connected or connecting.
    readonly property bool _running: {
        // read for the dependency on it, as the connections themselves notify nothing
        if (ConsoleStore.connectionsUpdate === 0)
            return false;
        const connections = ConsoleStore.connections;
        for (const id in connections) {
            const connection = connections[id];
            if (connection.profileId === profile.id && connection.status !== "disconnected")
                return true;
        }
        return false;
    }

    implicitHeight: renaming ? editorLoader.implicitHeight : name.implicitHeight
    // a link on the web
    activeFocusOnTab: !renaming
    Keys.onReturnPressed: profileItem._open()
    Keys.onEnterPressed: profileItem._open()

    function _open() {
        Router.navigate(`/profile/${gameId}/${profile.id}`);
    }
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
            profileItem._deleting = false;
            return Globals.refetchProfiles();
        };
        return Api.deleteProfile(profile.id).then(refetch, e => refetch().then(() => {
            throw e;
        }));
    }

    function _openFolder() {
        Api.getProfilePath(profile.id).then(path => {
            if (!Native.openFolder(path))
                throw new Error(`Failed to open folder: ${path}`);
        }).catch(e => Ui.window.reportError(e));
    }

    HoverHandler {
        id: hover
    }

    FocusRing {}

    // the pivot of a shift-click range selection
    Rectangle {
        anchors.fill: parent
        color: "transparent"
        border.color: Theme.primary45
        border.width: Ui.hairline
        visible: profileItem.isPivot && (profileItem.listFocused || profileItem._menuOpen)
        z: 2
    }

    Loader {
        id: editorLoader
        active: profileItem.renaming
        width: parent.width
        sourceComponent: ProfileNameEditor {
            initialValue: profileItem.profile.name
            submit: name => Api.overwriteProfileMetadata(profileItem.profile.id, profileItem._metadata({ name })).then(() => Globals.refetchProfiles(), e => Globals.refetchProfiles().then(() => {
                    throw e;
                }))
            onClosed: profileItem.renaming = false
        }
    }

    Item {
        id: name
        visible: !profileItem.renaming
        width: parent.width
        implicitHeight: label.implicitHeight + 2 * 0.35 * Theme.rem

        Icon {
            visible: profileItem._running
            name: "circle-play"
            filled: true
            size: 0.6 * Theme.rem
            color: Theme.primary60
            x: 0.2 * Theme.rem
            anchors.verticalCenter: parent.verticalCenter

            HoverHandler {
                id: playingHover
            }
            Tip {
                text: I18n.t("profile.sidebar.profile_running_icon")
                shown: playingHover.hovered
            }
        }

        Text {
            id: label
            x: Theme.rem
            width: parent.width - 2 * Theme.rem
            anchors.verticalCenter: parent.verticalCenter
            // HTML collapses whitespace
            text: profileItem.profile.name.replace(/\s+/g, " ").trim()
            elide: Text.ElideRight
            // .profileList__itemName sets its own color over the active item's primary color
            color: Theme.neutral20
            font.family: Theme.font
            font.pixelSize: Theme.rem
            font.weight: profileItem.selected ? 620 : Font.Normal
            font.variableAxes: ({ "wght": profileItem.selected ? 620 : 400 })
        }

        TapHandler {
            onTapped: (eventPoint, button) => {
                // shift clicks take priority over control clicks
                if (Native.shift)
                    profileItem.shiftClicked();
                else if (Native.ctrl)
                    profileItem.ctrlClicked();
                else
                    profileItem._open();
            }
            onDoubleTapped: {
                if (!Native.ctrl && !Native.shift)
                    profileItem.renaming = true;
            }
        }
        HoverHandler {
            cursorShape: Qt.PointingHandCursor
        }
    }

    // the buttons revealed on hover
    Row {
        id: options
        visible: !profileItem.renaming
        anchors.right: parent.right
        anchors.rightMargin: 0.25 * Theme.rem
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        anchors.topMargin: 2
        anchors.bottomMargin: 2
        opacity: profileItem._hot ? 1 : 0

        Behavior on opacity {
            SmoothNumberAnimation {
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
                    color: profileItem.color
                }
            }
        }

        Rectangle {
            width: buttons.width
            height: parent.height
            color: profileItem.color

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
                    iconName: profileItem.profile.pinned ? "pin-off" : "pin"
                    tooltip: profileItem.profile.pinned ? I18n.t("profile.sidebar.unpin_profile_btn") : I18n.t("profile.sidebar.pin_profile_btn")
                    action: () => Api.overwriteProfileMetadata(profileItem.profile.id, profileItem._metadata({ pinned: !profileItem.profile.pinned })).then(() => Globals.refetchProfiles(), e => Globals.refetchProfiles().then(() => {
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
                    onClicked: Native.copyText(profileItem.profile.id)
                }
                OptionButton {
                    visible: Native.shift
                    iconName: "trash-2"
                    tooltip: I18n.t("global.phrases.delete")
                    // leaves the profile first if it's being viewed, as deleting from the menu does
                    onClicked: profileItem._delete().catch(e => Ui.window.reportError(e))
                }

                ContextMenu {
                    id: menu
                    objectName: "profileMenu"
                    height: parent.height
                    width: Math.max(height, 1.25 * Theme.rem)
                    trigger.variant: "plain"
                    trigger.radius: 4
                    trigger.iconName: "ellipsis-vertical"
                    trigger.horizontalPadding: 0.1 * Theme.rem
                    trigger.verticalPadding: 0.2 * Theme.rem
                    trigger.tooltip: menu.opened ? "" : I18n.t("profile.sidebar.ellipsis_btn")
                    items: [
                        {
                            icon: "square-pen",
                            label: I18n.t("profile.sidebar.rename_profile_btn"),
                            action: () => profileItem.renaming = true
                        },
                        {
                            icon: "trash-2",
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
                            icon: "forward",
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
                            action: () => Native.copyText(profileItem.profile.id)
                        },
                        {
                            icon: "folder-open",
                            label: I18n.t("profile.sidebar.open_folder_profile_btn"),
                            action: () => profileItem._openFolder()
                        }
                    ]
                }
            }
        }
    }

    PopupLoader {
        id: deletePrompt
        sourceComponent: PromptDialog {
            title: I18n.t("global.phrases.confirm")
            question: I18n.t("profile.delete_msg", { profileName: profileItem.profile.name })
            okType: "danger"
            okText: I18n.t("global.phrases.delete")
            onAccepted: {
                close();
                profileItem._delete().catch(e => Ui.window.reportError(e));
            }
        }
    }
}
