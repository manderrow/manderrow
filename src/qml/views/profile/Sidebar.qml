import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Effects
import QtQuick.Layouts
import Manderrow

// The profile view's sidebar, ported from Profile.tsx and its styles. Narrow windows get a mini
// sidebar of icons that expands over the content.
Rectangle {
    id: sidebar
    objectName: "sidebar"

    function openImport() {
        importDialog.open();
    }

    required property string gameId
    property var profileId
    /// The profiles of this game, keyed by id.
    property var profiles: ({})
    /// Whether the window is narrow, so that the sidebar is a mini one.
    readonly property bool narrow: Theme.windowWidth <= 1200
    /// Whether the mini sidebar is expanded, which narrow windows remember while they are wide.
    property bool open: false
    readonly property bool expanded: narrow && open
    /// Whether the sidebar is collapsed to icons, which happens on narrow windows.
    readonly property bool mini: narrow && !open
    property bool creatingProfile: false
    /// The editor for naming a new profile, while it shows.
    property ProfileNameEditor _profileCreator: null
    property bool _reopenProfileCreator: false

    /// Shows the editor for naming a new profile. If it is already showing, its name is submitted
    /// first, then it shows again for another, as creating files does in VS Code.
    function createProfile() {
        if (!creatingProfile) {
            creatingProfile = true;
        } else if (_profileCreator) {
            _reopenProfileCreator = true;
            _profileCreator.commit();
        }
    }

    function _profileCreatorClosed() {
        creatingProfile = false;
        if (_reopenProfileCreator) {
            _reopenProfileCreator = false;
            // once the closed editor is gone
            Qt.callLater(() => sidebar.creatingProfile = true);
        }
    }

    property var focusedConnection: null

    signal backClicked
    signal launchClicked
    signal killClicked
    signal cancelClicked

    readonly property var game: Globals.gamesById[gameId]
    readonly property real fullWidth: 22 * Theme.rem
    readonly property real miniWidth: 3.3 * Theme.rem
    readonly property real pad: Theme.rem

    // profile searching and sorting
    property string profileQuery: ""
    /// true for ascending, false for descending
    property bool profileSortOrder: false
    property string profileSortType: "creation_date"

    readonly property var queriedProfiles: {
        const query = profileQuery.toLowerCase();
        return Object.keys(profiles).map(id => profiles[id]).filter(profile => profile.name.toLowerCase().includes(query)).sort((a, b) => {
            // TODO: sort by creation date
            return profileSortOrder ? a.name.localeCompare(b.name) : b.name.localeCompare(a.name);
        });
    }
    /// Pinned profiles are listed first.
    readonly property var listedProfiles: queriedProfiles.filter(p => p.pinned).concat(queriedProfiles.filter(p => !p.pinned))

    // multi-selection of profiles, ported from createMultiselectableList
    property var selectedProfiles: ({})
    property int pivotIndex: -1
    readonly property var effectivePivot: pivotIndex >= 0 ? queriedProfiles[pivotIndex] : profiles[profileId]

    function ctrlClickProfile(profile) {
        const selected = Object.assign({}, selectedProfiles);
        if (selected[profile.id])
            delete selected[profile.id];
        else
            selected[profile.id] = true;
        selectedProfiles = selected;
        pivotIndex = queriedProfiles.indexOf(profile);
    }
    function shiftClickProfile(profile) {
        const pivot = effectivePivot;
        if (pivot == null || profile === pivot)
            return;
        const pivotAt = queriedProfiles.findIndex(p => p.id === pivot.id);
        const index = queriedProfiles.indexOf(profile);
        const selected = {};
        for (const p of queriedProfiles.slice(Math.min(pivotAt, index), Math.max(pivotAt, index) + 1))
            selected[p.id] = true;
        selectedProfiles = selected;
    }
    function clearSelection() {
        selectedProfiles = {};
        pivotIndex = -1;
    }

    readonly property bool hasLiveConnection: focusedConnection !== null && focusedConnection.status !== "disconnected"

    width: mini ? miniWidth : fullWidth
    // expanded over the content on narrow windows, it hides the content beneath
    color: expanded ? Theme.neutral90 : Theme.surface(Theme.neutral90)

    // Rectangles let the pointer through, which the expanded mini sidebar must not do to the
    // content beneath it. Declared first, this is below the sidebar's own controls.
    PopupInputBlocker {}

    // border-right
    Rectangle {
        anchors.right: parent.right
        width: Ui.hairline
        height: parent.height
        color: Theme.neutral80
        z: 3
    }

    // the expanded mini sidebar floats over the content with a shadow
    RectangularShadow {
        visible: sidebar.expanded
        anchors.fill: parent
        blur: 1.5 * Theme.rem
        color: Qt.rgba(0, 0, 0, 0.5)
        z: -2
    }

    // the toggle for the mini sidebar, a tab on the sidebar's edge while it's expanded
    Btn {
        id: expandBtn
        visible: sidebar.narrow
        z: sidebar.expanded ? -1 : 1
        variant: "plain"
        // a pill, or a tab rounded away from the sidebar while it's expanded
        leftRadius: sidebar.expanded ? 0 : radius
        rightRadius: sidebar.expanded ? 0.75 * fontSize : radius
        fontSize: 1.5 * Theme.rem
        iconName: "chevrons-right"
        iconRotation: sidebar.expanded ? 180 : 0
        horizontalPadding: 0.35 * fontSize
        verticalPadding: (sidebar.expanded ? 1 : 0.75) * fontSize
        textColor: Theme.neutral20
        activeTextColor: Theme.primary55
        activeBgColor: sidebar.expanded ? Theme.neutral90 : Theme.neutral80
        bgColor: sidebar.expanded ? Theme.neutral90 : "transparent"
        x: sidebar.expanded ? sidebar.width - Ui.hairline : (sidebar.width - width) / 2
        y: (sidebar.height - height) / 2
        onClicked: sidebar.open = !sidebar.open

        // the tab's border, which reaches under the sidebar so that it is open on that side
        Rectangle {
            visible: sidebar.expanded
            anchors.fill: parent
            anchors.leftMargin: -radius
            color: "transparent"
            radius: expandBtn.rightRadius
            border.color: Theme.neutral70
            border.width: Ui.hairline

            PixelSnap {}
        }
    }

    // The sidebar's padding is left by each part that has any, so that the parts that reach its
    // edges stay within the column: Qt can skip delivering the pointer to items outside their
    // parent.
    ColumnLayout {
        anchors.fill: parent
        anchors.bottomMargin: sidebar.mini ? 0 : sidebar.pad
        spacing: 0

        // the game banner and launch buttons
        Item {
            id: mainActions
            Layout.fillWidth: true
            Layout.preferredHeight: sidebar.mini ? actionBtns.implicitHeight + 0.5 * Theme.rem : Math.max(10 * Theme.rem, nav.implicitHeight + actionBtns.implicitHeight + 2 * 0.75 * Theme.rem + 0.35 * Theme.rem)
            clip: true

            SharpImage {
                visible: !sidebar.mini
                anchors.fill: parent
                source: Util.gameHeroUrl(sidebar.gameId)
            }

            // darkens the top and bottom of the banner
            Item {
                visible: !sidebar.mini
                anchors.fill: parent

                Rectangle {
                    anchors.fill: parent
                    color: Qt.rgba(0, 0, 0, 0.1)
                }
                Rectangle {
                    width: parent.width
                    height: parent.height * 0.3
                    anchors.bottom: parent.bottom
                    gradient: Gradient {
                        GradientStop {
                            position: 0
                            color: "transparent"
                        }
                        GradientStop {
                            position: 1
                            color: Qt.rgba(0, 0, 0, 0.5)
                        }
                    }
                }
                Rectangle {
                    width: parent.width
                    height: parent.height * 0.5
                    gradient: Gradient {
                        GradientStop {
                            position: 0
                            color: Qt.rgba(0, 0, 0, 0.7)
                        }
                        GradientStop {
                            position: 1
                            color: "transparent"
                        }
                    }
                }
            }

            Rectangle {
                visible: !sidebar.mini
                anchors.bottom: parent.bottom
                width: parent.width
                height: Ui.hairline
                color: Theme.neutral80
            }

            RowLayout {
                id: nav
                visible: !sidebar.mini
                x: 0.75 * Theme.rem
                y: 0.75 * Theme.rem
                width: parent.width - 2 * x
                spacing: 0.7 * 1.35 * Theme.rem

                Btn {
                    variant: "plain"
                    iconName: "chevron-down"
                    fontSize: 1.35 * Theme.rem
                    horizontalPadding: 0.6 * fontSize
                    verticalPadding: 0.4 * fontSize
                    onClicked: sidebar.backClicked()
                }

                Text {
                    Layout.fillWidth: true
                    text: sidebar.game ? sidebar.game.name : ""
                    wrapMode: Text.Wrap
                    color: Qt.rgba(240 / 255, 240 / 255, 240 / 255, 1)
                    font.family: Theme.font
                    font.pixelSize: 1.75 * Theme.rem
                    font.weight: Font.Light
                    font.variableAxes: ({ "wght": Font.Light })
                    style: Text.Raised
                    styleColor: Qt.rgba(0, 0, 0, 0.25)
                }
            }

            GridLayout {
                id: actionBtns
                flow: sidebar.mini ? GridLayout.TopToBottom : GridLayout.LeftToRight
                x: sidebar.mini ? (parent.width - width) / 2 : 0.75 * Theme.rem
                y: sidebar.mini ? 0.5 * Theme.rem : parent.height - height - 0.75 * Theme.rem
                width: sidebar.mini ? implicitWidth : parent.width - 2 * 0.75 * Theme.rem
                columnSpacing: 0.5 * Theme.rem
                rowSpacing: 0.5 * Theme.rem

                component SessionButton: Btn {
                    id: session

                    property string label
                    property color tone1
                    property color tone2
                    property color tone3
                    property color edge

                    Layout.alignment: Qt.AlignHCenter
                    fontSize: sidebar.mini ? 1.2 * Theme.rem : Theme.rem
                    padAll: (sidebar.mini ? 0.3 : 0.5) * fontSize
                    Layout.preferredWidth: sidebar.mini ? implicitHeight : implicitWidth
                    text: sidebar.mini ? "" : label
                    borderColor: edge
                    activeBorderColor: edge
                    borderWidth: 2
                    // linear-gradient(20deg, tone-3 10%, tone-4 80%, tone-5 100%)
                    background: Rectangle {
                        radius: height / 2
                        border.color: session.edge
                        border.width: Ui.snapBorder(2)
                        gradient: Gradient {
                            orientation: Gradient.Horizontal
                            GradientStop {
                                position: 0.1
                                color: session.tone1
                            }
                            GradientStop {
                                position: 0.8
                                color: session.tone2
                            }
                            GradientStop {
                                position: 1
                                color: session.tone3
                            }
                        }

                        PixelSnap {}
                    }
                }

                SessionButton {
                    visible: !!sidebar.focusedConnection && sidebar.focusedConnection.status === "connected"
                    iconName: "skull"
                    label: I18n.t("profile.sidebar.kill_game_btn")
                    tone1: Theme.danger3
                    tone2: Theme.danger4
                    tone3: Theme.danger5
                    edge: Theme.danger7
                    onClicked: sidebar.killClicked()
                }

                SessionButton {
                    visible: sidebar.hasLiveConnection && sidebar.focusedConnection.status !== "connected"
                    iconName: "x"
                    label: I18n.t("profile.sidebar.cancel_launch_btn")
                    tone1: Theme.warn3
                    tone2: Theme.warn4
                    tone3: Theme.warn5
                    edge: Theme.warn7
                    onClicked: sidebar.cancelClicked()
                }

                LaunchButton {
                    objectName: "launchButton"
                    Layout.fillWidth: !sidebar.mini
                    Layout.alignment: Qt.AlignHCenter
                    mini: sidebar.mini
                    enabled: sidebar.profileId !== undefined
                    padding: (sidebar.mini ? 0.3 : 0.5) * font.pixelSize
                    onClicked: sidebar.launchClicked()
                }

                Btn {
                    visible: !sidebar.mini
                    Layout.alignment: Qt.AlignBottom
                    variant: "plain"
                    iconName: "settings"
                    iconFilled: true
                    horizontalPadding: 0.25 * fontSize
                    verticalPadding: 0.1 * fontSize
                    tooltip: I18n.t("profile.sidebar.game_settings_btn")
                }
            }
        }

        // the profiles of this game
        ColumnLayout {
            id: profilesGroup
            visible: !sidebar.mini
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.topMargin: 0.75 * Theme.rem
            spacing: 0

            RowLayout {
                Layout.fillWidth: true
                Layout.leftMargin: 0.5 * Theme.rem
                Layout.rightMargin: 0.5 * Theme.rem
                Layout.bottomMargin: 0.5 * Theme.rem

                Text {
                    Layout.fillWidth: true
                    text: I18n.t("profile.sidebar.profiles_title")
                    color: Theme.neutral20
                    font.family: Theme.font
                    font.pixelSize: 1.17 * Theme.rem
                    font.weight: 350
                    font.variableAxes: ({ "wght": 350 })
                }

                component ProfilesActionButton: Btn {
                    variant: "plain"
                    fontSize: 1.25 * Theme.rem
                    horizontalPadding: 0.25 * fontSize
                    verticalPadding: 0
                    activeBgColor: "transparent"
                    activeTextColor: Theme.primary50
                }

                ProfilesActionButton {
                    iconName: "plus"
                    tooltip: I18n.t("profile.sidebar.create_profile_tooltip")
                    onClicked: sidebar.createProfile()
                }
                ProfilesActionButton {
                    iconName: "file-input"
                    tooltip: I18n.t("profile.sidebar.import_profile_tooltip")
                    onClicked: importDialog.open()
                }
            }

            Item {
                id: searchForm
                Layout.fillWidth: true
                Layout.leftMargin: 0.5 * Theme.rem
                Layout.rightMargin: 0.5 * Theme.rem
                implicitHeight: searchRow.implicitHeight + 2 * Ui.hairline

                BoxBackground {
                    radius: 4
                    color: Theme.neutral80
                    border.color: Theme.neutral70
                }

                RowLayout {
                    id: searchRow
                    anchors.fill: parent
                    anchors.margins: Ui.hairline
                    spacing: 0

                    Field {
                        Layout.fillWidth: true
                        variant: "bare"
                        leftPadding: 0.4 * Theme.rem
                        rightPadding: 0.4 * Theme.rem
                        topPadding: 0.2 * Theme.rem
                        bottomPadding: 0.2 * Theme.rem
                        placeholderText: I18n.t("global.phrases.search")
                        maximumLength: 100
                        onTextChanged: sidebar.profileQuery = text
                    }

                    SelectDropdown {
                        Layout.fillHeight: true
                        preset: I18n.t("global.sort_title")
                        options: [
                            {
                                value: "alphabetical",
                                label: "A-Z",
                                selected: sidebar.profileSortType === "alphabetical"
                            },
                            {
                                value: "creation_date",
                                label: I18n.t("global.sort_type.creation_date"),
                                selected: sidebar.profileSortType === "creation_date"
                            }
                        ]
                        onChanged: value => sidebar.profileSortType = value
                    }

                    Btn {
                        Layout.fillHeight: true
                        horizontalPadding: 0.5 * Theme.rem
                        textColor: Theme.neutral20
                        iconName: sidebar.profileSortOrder ? "arrow-up-wide-narrow" : "arrow-down-narrow-wide"
                        onClicked: sidebar.profileSortOrder = !sidebar.profileSortOrder
                    }
                }
            }

            ListView {
                id: profileList
                objectName: "profileList"
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.topMargin: 0.5 * Theme.rem
                clip: true
                boundsBehavior: Flickable.StopAtBounds
                activeFocusOnTab: true
                ScrollBar.vertical: AppScrollBar {
                    autoHide: true
                }
                WheelScroll {}
                // one stop in the Tab order, whose items the arrow keys move between
                FocusGroup {
                    view: profileList
                }
                model: sidebar.listedProfiles

                // the selection clears once focus leaves the list
                property bool hadFocus: false
                onActiveFocusChanged: {
                    if (!activeFocus && hadFocus)
                        sidebar.clearSelection();
                    hadFocus = activeFocus;
                }

                header: Column {
                    width: profileList.width

                    Loader {
                        id: profileCreator
                        active: sidebar.creatingProfile
                        // a Loader keeps the size of the item it unloads, so the column skips it
                        visible: active
                        width: parent.width
                        onItemChanged: sidebar._profileCreator = item
                        sourceComponent: ProfileNameEditor {
                            objectName: "newProfileEditor"
                            initialValue: I18n.t("profile.default_profile_name")
                            submit: name => Api.createProfile(sidebar.gameId, name).then(() => Globals.refetchProfiles())
                            onClosed: sidebar._profileCreatorClosed()
                        }
                    }

                    Text {
                        visible: sidebar.queriedProfiles.length === 0 && !sidebar.creatingProfile
                        width: parent.width
                        horizontalAlignment: Text.AlignHCenter
                        topPadding: 0.35 * Theme.rem
                        bottomPadding: 0.35 * Theme.rem
                        text: I18n.t("profile.sidebar.no_profiles_search_msg")
                        color: Theme.neutral20
                        font.family: Theme.font
                        font.pixelSize: Theme.rem
                    }
                }

                delegate: SidebarProfileItem {
                    required property var modelData
                    width: profileList.width
                    profile: modelData
                    gameId: sidebar.gameId
                    selected: modelData.id === sidebar.profileId
                    highlighted: !!sidebar.selectedProfiles[modelData.id]
                    isPivot: sidebar.effectivePivot !== undefined && sidebar.effectivePivot !== null && sidebar.effectivePivot.id === modelData.id
                    listFocused: profileList.activeFocus
                    onCtrlClicked: {
                        profileList.forceActiveFocus();
                        sidebar.ctrlClickProfile(modelData);
                    }
                    onShiftClicked: {
                        profileList.forceActiveFocus();
                        sidebar.shiftClickProfile(modelData);
                    }
                }
            }
        }

        Item {
            visible: sidebar.mini
            Layout.fillHeight: true
        }

        // the settings link
        Item {
            Layout.fillWidth: true
            Layout.alignment: Qt.AlignBottom
            Layout.leftMargin: sidebar.mini ? 0 : sidebar.pad
            Layout.rightMargin: sidebar.mini ? 0 : sidebar.pad
            Layout.topMargin: sidebar.mini ? 0 : 0.75 * Theme.rem
            implicitHeight: settingsLink.implicitHeight + (sidebar.mini ? 0 : 0.75 * Theme.rem)

            Rectangle {
                visible: !sidebar.mini
                width: parent.width
                height: Ui.hairline
                color: Theme.neutral80
            }

            ClickArea {
                id: settingsHover
                anchors.fill: settingsLink
                activatesOnSpace: false
                onActivated: Router.navigate("/settings")
            }

            Row {
                id: settingsLink
                anchors.bottom: parent.bottom
                anchors.horizontalCenter: sidebar.mini ? parent.horizontalCenter : undefined
                spacing: 0.25 * 1.75 * Theme.rem
                topPadding: sidebar.mini ? 0.5 * 1.75 * Theme.rem : 0
                bottomPadding: sidebar.mini ? 0.5 * 1.75 * Theme.rem : 0

                Icon {
                    name: "settings"
                    filled: true
                    size: 1.75 * Theme.rem
                    color: settingsHover.hovered ? Theme.primary50 : Theme.neutral20
                    anchors.verticalCenter: parent.verticalCenter
                }
                Text {
                    visible: !sidebar.mini
                    anchors.verticalCenter: parent.verticalCenter
                    text: I18n.t("settings.link_title")
                    color: settingsHover.hovered ? Theme.primary50 : Theme.neutral30
                    font.family: Theme.font
                    font.pixelSize: 1.1 * Theme.rem
                }
            }
        }
    }

    PopupLoader {
        id: importDialog
        sourceComponent: ImportDialog {
            gameId: sidebar.gameId
            profileId: sidebar.profileId
        }
    }
}
