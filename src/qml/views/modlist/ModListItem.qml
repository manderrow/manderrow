import QtQuick
import QtQuick.Layouts
import Manderrow

// A mod in a mod list, ported from ModListItem.tsx.
Item {
    id: modItem

    /// A mod summary from the mod index (with `versions` and `total_downloads`) or an installed mod
    /// package (with `version`).
    required property var mod
    property var installContext: null
    /// Whether this mod is shown in the mod view.
    property bool focused: false
    /// Whether this mod can be selected for bulk actions.
    property bool selectable: false
    property bool selected: false
    property bool forceSelectorVisibility: false
    /// Whether the item above this one is hovered, which hides this item's top border.
    property bool previousHot: false

    signal focusToggled
    signal selectToggled
    signal shiftClicked
    signal uninstalled

    readonly property bool isPackage: mod.version !== undefined
    readonly property var displayVersion: Util.displayVersion(mod)
    readonly property var installed: installContext ? installContext.installedOf(mod) : undefined
    readonly property bool hot: hover.hovered
    /// The buttons show while hovered or while the focus is within the item, as on the web.
    readonly property bool revealed: hot || Ui.isWithin(Window.activeFocusItem, modItem)
    /// Whether the checkbox shows: while the pointer is near the item's left edge, over the selector,
    /// rather than anywhere over the item, which only hints at it with a line along that edge.
    readonly property bool _selectorShown: selectable && (forceSelectorVisibility || selected || selectorHover.hovered)

    implicitHeight: row.implicitHeight + 2
    // a role="button" with tabIndex 0 on the web
    activeFocusOnTab: true
    Keys.onReturnPressed: modItem.focusToggled()
    Keys.onEnterPressed: modItem.focusToggled()
    Keys.onSpacePressed: modItem.focusToggled()

    HoverHandler {
        id: hover
    }

    Rectangle {
        id: frame
        anchors.fill: parent
        radius: modItem.hot || modItem.focused ? 6 : 0
        color: modItem.hot ? (modItem.focused ? Theme.neutral70 : Theme.neutral80) : modItem.focused ? Theme.neutral80 : Theme.alpha(Theme.neutral80, 0)
        border.width: 1
        border.color: tap.pressed ? Theme.neutral50 : modItem.focused ? Theme.primary60 : modItem.hot ? Theme.neutral70 : Theme.alpha(Theme.neutral70, 0)

        Behavior on color {
            SmoothColorAnimation {
                duration: 100
                curve: Theme.easeInOut
            }
        }
        Behavior on border.color {
            SmoothColorAnimation {
                duration: 100
                curve: Theme.easeInOut
            }
        }
        Behavior on radius {
            SmoothNumberAnimation {
                duration: 300
                curve: Theme.easeOut
            }
        }

        FocusRing {
            target: modItem
            radius: parent.radius
        }

        // the top border separating items
        Rectangle {
            visible: !modItem.hot && !modItem.focused && !modItem.previousHot
            width: parent.width
            height: 1
            color: Theme.neutral80
        }
    }

    TapHandler {
        id: tap
        onTapped: (eventPoint, button) => {
            if ((tap.point.modifiers & Qt.ShiftModifier) && modItem.selectable)
                modItem.shiftClicked();
            else
                modItem.focusToggled();
        }
    }
    HoverHandler {
        cursorShape: Qt.PointingHandCursor
    }

    RowLayout {
        id: row
        x: 1
        y: 1
        width: parent.width - 2
        spacing: 0

        // the bulk selection checkbox, which grows in when the pointer nears it
        Item {
            id: selector
            visible: modItem.selectable
            // above the content, which the region the pointer reveals it from overlaps
            z: 1
            Layout.fillHeight: true
            Layout.preferredWidth: modItem._selectorShown ? 18 + Theme.rem : 0.5 * Theme.rem

            Behavior on Layout.preferredWidth {
                SmoothNumberAnimation {
                    duration: 100
                    curve: Theme.easeInOut
                }
            }

            // the region that reveals and toggles the checkbox, which reaches 1em into the content
            Item {
                width: parent.width + Theme.rem
                height: parent.height

                HoverHandler {
                    id: selectorHover
                    cursorShape: Qt.PointingHandCursor
                }
                TapHandler {
                    id: selectorTap
                    // takes the press for itself, so that the item doesn't also open the mod
                    gesturePolicy: TapHandler.ReleaseWithinBounds
                    onTapped: (eventPoint, button) => {
                        if (selectorTap.point.modifiers & Qt.ShiftModifier)
                            modItem.shiftClicked();
                        else
                            modItem.selectToggled();
                    }
                }
            }

            // the checkbox, or the line hinting at it along the edge of the hovered item
            Rectangle {
                readonly property bool hint: !modItem._selectorShown && modItem.hot

                x: modItem._selectorShown ? 0.5 * Theme.rem : -1.5
                anchors.verticalCenter: parent.verticalCenter
                width: modItem._selectorShown ? 18 : hint ? 3 : 0
                height: modItem._selectorShown ? 18 : hint ? 30 : 0
                radius: 2
                color: "transparent"
                border.width: modItem._selectorShown ? 2 : hint ? 1.5 : 0
                border.color: modItem.selected || selectorHover.hovered || hint ? Theme.neutral15 : Theme.neutral45

                Behavior on x {
                    SmoothNumberAnimation {
                        duration: 100
                        curve: Theme.easeInOut
                    }
                }
                Behavior on border.width {
                    SmoothNumberAnimation {
                        duration: 100
                        curve: Theme.easeInOut
                    }
                }
                Behavior on width {
                    SmoothNumberAnimation {
                        duration: 100
                        curve: Theme.easeInOut
                    }
                }
                Behavior on height {
                    SmoothNumberAnimation {
                        duration: 100
                        curve: Theme.easeInOut
                    }
                }

                // the check is revealed like the Checkbox's, and hidden at once when unchecked. It
                // fills the box inside its border.
                CheckMark {
                    x: parent.border.width
                    y: parent.border.width
                    width: Math.max(0, parent.width - 2 * parent.border.width)
                    height: Math.max(0, parent.height - 2 * parent.border.width)
                    checked: modItem.selected && modItem._selectorShown
                    hideWhenUnchecked: true
                    fill: true
                    clip: false
                }
            }
        }

        // overflow: hidden
        Item {
            Layout.fillWidth: true
            implicitHeight: content.implicitHeight + Theme.rem
            clip: true

            RowLayout {
                id: content
                x: modItem.selectable ? 0 : 0.5 * Theme.rem
                y: 0.5 * Theme.rem
                width: parent.width - x - 0.5 * Theme.rem
                spacing: 0.75 * Theme.rem

                // the icon, with rounded corners
                RoundedImage {
                    Layout.alignment: Qt.AlignTop
                    Layout.preferredWidth: 64
                    Layout.preferredHeight: 64
                    radius: 6
                    source: Util.getIconUrl(Util.getQualifiedModName(modItem.mod.owner, modItem.mod.name, modItem.displayVersion.version_number))
                }

                // min-width: 0: the name and the rest of its line overflow it, toward the button,
                // rather than pushing the button out, once the name can't shrink any more
                ColumnLayout {
                    Layout.fillWidth: true
                    Layout.minimumWidth: 0
                    Layout.alignment: Qt.AlignTop
                    spacing: 0.35 * Theme.rem

                    RowLayout {
                        Layout.fillWidth: true
                        Layout.minimumWidth: 0
                        spacing: 0.35 * Theme.rem

                        // takes its natural width, shrinking to 5ch only when the row is too narrow
                        // (a layout's items that don't fill it are never narrower than preferred).
                        // Layouts round widths to whole pixels, so its width is rounded up, or it
                        // would be elided for want of a fraction of one.
                        Text {
                            Layout.fillWidth: true
                            Layout.minimumWidth: Math.min(Math.ceil(implicitWidth), 5 * Ui.ch)
                            Layout.maximumWidth: Math.ceil(implicitWidth)
                            text: modItem.mod.name
                            elide: Text.ElideRight
                            color: Theme.neutral10
                            font.family: Theme.font
                            font.pixelSize: Theme.rem
                            font.weight: Font.Medium
                            font.variableAxes: ({ "wght": Font.Medium })
                        }
                        Text {
                            text: "•"
                            color: Theme.neutral60
                            font.pixelSize: 0.75 * Theme.rem
                        }
                        Text {
                            text: modItem.mod.owner
                            color: Theme.neutral20
                            font.family: Theme.font
                            font.pixelSize: 0.9 * Theme.rem
                        }
                        Text {
                            visible: modItem.isPackage
                            text: "•"
                            color: Theme.neutral60
                            font.pixelSize: 0.75 * Theme.rem
                        }
                        Text {
                            visible: modItem.isPackage
                            text: modItem.isPackage ? modItem.mod.version.version_number : ""
                            color: Theme.neutral20
                            font.family: Theme.font
                            font.pixelSize: 0.9 * Theme.rem
                        }
                        Item {
                            Layout.fillWidth: true
                        }
                    }

                    Row {
                        spacing: 0.3 * Theme.rem

                        Icon {
                            name: modItem.isPackage ? "hard-drive" : "download"
                            size: 0.85 * Theme.rem
                            color: Theme.neutral30
                            anchors.verticalCenter: parent.verticalCenter
                        }
                        Text {
                            text: modItem.isPackage ? Util.humanizeFileSize(modItem.mod.version.file_size) : Util.formatRounded(modItem.mod.total_downloads)
                            color: Theme.neutral30
                            font.family: Theme.font
                            font.pixelSize: 0.85 * Theme.rem
                        }
                    }

                    Text {
                        Layout.fillWidth: true
                        text: modItem.displayVersion.description
                        elide: Text.ElideRight
                        maximumLineCount: 1
                        color: Theme.neutral30
                        font.family: Theme.font
                        font.pixelSize: 0.85 * Theme.rem
                    }
                }

                // install or uninstall, revealed on hover
                Loader {
                    Layout.alignment: Qt.AlignVCenter
                    active: modItem.installContext !== null
                    sourceComponent: modItem.installed !== undefined ? uninstallButton : installButton
                }
            }
        }
    }

    Component {
        id: installButton
        InstallButton {
            mod: modItem.mod
            installContext: modItem.installContext
            fontSize: 1.5 * Theme.rem
            progressStyle: "circular"
            busyPlain: true
            // `visibility: hidden` in the web UI, which keeps the button's space
            opacity: modItem.revealed || busy ? 1 : 0
            btn.width: btn.height
            btn.horizontalPadding: 0.65 * fontSize
            btn.verticalPadding: 0.25 * fontSize
            iconName: "move-down"
            tooltip: I18n.t("modlist.online.install_btn")
        }
    }

    Component {
        id: uninstallButton
        UninstallButton {
            mod: modItem.installed
            installContext: modItem.installContext
            fontSize: 1.5 * Theme.rem
            progressStyle: "circular"
            busyPlain: true
            // `visibility: hidden` in the web UI, which keeps the button's space
            opacity: modItem.revealed || busy ? 1 : 0
            btn.width: btn.height
            btn.horizontalPadding: 0.65 * fontSize
            btn.verticalPadding: 0.25 * fontSize
            iconName: "trash"
            tooltip: I18n.t("modlist.installed.uninstall_btn")
            onUninstalled: modItem.uninstalled()
        }
    }
}
