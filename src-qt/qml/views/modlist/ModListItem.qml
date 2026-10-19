import QtQuick
import QtQuick.Effects
import QtQuick.Layouts
import Manderrow

// A mod in a mod list, ported from ModListItem.tsx.
Item {
    id: item

    /// A mod listing (with `versions`) or an installed mod package (with `version`).
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
    readonly property bool _selectorShown: selectable && (forceSelectorVisibility || selected || selectorHover.hovered || hot)

    implicitHeight: row.implicitHeight + 2

    HoverHandler {
        id: hover
    }

    Rectangle {
        id: frame
        anchors.fill: parent
        radius: item.hot || item.focused ? 6 : 0
        color: item.hot ? (item.focused ? Theme.neutral70 : Theme.neutral80) : item.focused ? Theme.neutral80 : "transparent"
        border.width: 1
        border.color: tap.pressed ? Theme.neutral50 : item.focused ? Theme.primary60 : item.hot ? Theme.neutral70 : "transparent"

        Behavior on color {
            ColorAnimation {
                duration: 100
                easing.type: Easing.InOutQuad
            }
        }

        // the top border separating items
        Rectangle {
            visible: !item.hot && !item.focused && !item.previousHot
            width: parent.width
            height: 1
            color: Theme.neutral80
        }
    }

    TapHandler {
        id: tap
        onTapped: (eventPoint, button) => {
            if ((tap.point.modifiers & Qt.ShiftModifier) && item.selectable)
                item.shiftClicked();
            else
                item.focusToggled();
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

        // the bulk selection checkbox, which grows in on hover
        Item {
            id: selector
            visible: item.selectable
            Layout.fillHeight: true
            Layout.preferredWidth: item._selectorShown ? 18 + Theme.rem : 0.5 * Theme.rem

            Behavior on Layout.preferredWidth {
                NumberAnimation {
                    duration: 100
                    easing.type: Easing.InOutQuad
                }
            }

            HoverHandler {
                id: selectorHover
                cursorShape: Qt.PointingHandCursor
            }
            TapHandler {
                id: selectorTap
                onTapped: (eventPoint, button) => {
                    if (selectorTap.point.modifiers & Qt.ShiftModifier)
                        item.shiftClicked();
                    else
                        item.selectToggled();
                }
            }

            Rectangle {
                x: item._selectorShown ? 0.5 * Theme.rem : 0
                anchors.verticalCenter: parent.verticalCenter
                width: item._selectorShown ? 18 : 0
                height: item._selectorShown ? 18 : 0
                radius: 2
                color: "transparent"
                border.width: item._selectorShown ? 2 : 0
                border.color: item.selected || selectorHover.hovered ? Theme.neutral15 : Theme.neutral45

                Behavior on width {
                    NumberAnimation {
                        duration: 100
                    }
                }
                Behavior on height {
                    NumberAnimation {
                        duration: 100
                    }
                }

                FaIcon {
                    anchors.centerIn: parent
                    name: "check"
                    size: 12
                    color: Theme.neutral15
                    visible: item.selected && item._selectorShown
                }
            }
        }

        Item {
            Layout.fillWidth: true
            implicitHeight: content.implicitHeight + Theme.rem

            RowLayout {
                id: content
                x: item.selectable ? 0 : 0.5 * Theme.rem
                y: 0.5 * Theme.rem
                width: parent.width - x - 0.5 * Theme.rem
                spacing: 0.75 * Theme.rem

                // the icon, with rounded corners
                Item {
                    Layout.alignment: Qt.AlignTop
                    Layout.preferredWidth: 64
                    Layout.preferredHeight: 64

                    Image {
                        id: icon
                        anchors.fill: parent
                        source: Util.getIconUrl(Util.getQualifiedModName(item.mod.owner, item.mod.name, item.displayVersion.version_number))
                        sourceSize: Qt.size(64, 64)
                        asynchronous: true
                        visible: false
                        layer.enabled: true
                    }
                    Rectangle {
                        id: iconMask
                        anchors.fill: parent
                        radius: 6
                        visible: false
                        layer.enabled: true
                    }
                    MultiEffect {
                        anchors.fill: parent
                        source: icon
                        maskEnabled: true
                        maskSource: iconMask
                    }
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    Layout.alignment: Qt.AlignTop
                    spacing: 0.35 * Theme.rem

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 0.35 * Theme.rem

                        // takes its natural width, shrinking to 5ch only when the row is too narrow
                        Text {
                            Layout.minimumWidth: Math.min(implicitWidth, 5 * Ui.ch)
                            Layout.preferredWidth: implicitWidth
                            Layout.fillWidth: false
                            text: item.mod.name
                            elide: Text.ElideRight
                            color: Theme.neutral10
                            font.family: Theme.font
                            font.pixelSize: Theme.rem
                            font.weight: Font.Medium
                        }
                        Text {
                            text: "•"
                            color: Theme.neutral60
                            font.pixelSize: 0.75 * Theme.rem
                        }
                        Text {
                            text: item.mod.owner
                            color: Theme.neutral20
                            font.family: Theme.font
                            font.pixelSize: 0.9 * Theme.rem
                        }
                        Text {
                            visible: item.isPackage
                            text: "•"
                            color: Theme.neutral60
                            font.pixelSize: 0.75 * Theme.rem
                        }
                        Text {
                            visible: item.isPackage
                            text: item.isPackage ? item.mod.version.version_number : ""
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

                        FaIcon {
                            name: item.isPackage ? "regular:hardDrive" : "download"
                            size: 0.85 * Theme.rem
                            color: Theme.neutral30
                            anchors.verticalCenter: parent.verticalCenter
                        }
                        Text {
                            text: item.isPackage ? Util.humanizeFileSize(item.mod.version.file_size) : Util.formatRounded(item.mod.versions.reduce((total, v) => total + v.downloads, 0))
                            color: Theme.neutral30
                            font.family: Theme.font
                            font.pixelSize: 0.85 * Theme.rem
                        }
                    }

                    Text {
                        Layout.fillWidth: true
                        text: item.displayVersion.description
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
                    active: item.installContext !== null
                    sourceComponent: item.installed !== undefined ? uninstallButton : installButton
                }
            }
        }
    }

    Component {
        id: installButton
        InstallButton {
            mod: item.mod
            installContext: item.installContext
            fontSize: 1.5 * Theme.rem
            progressStyle: "circular"
            busyPlain: true
            // `visibility: hidden` in the web UI, which keeps the button's space
            opacity: item.hot || busy ? 1 : 0
            enabled: opacity > 0
            btn.width: btn.height
            btn.horizontalPadding: 0.65 * fontSize
            btn.verticalPadding: 0.25 * fontSize
            iconName: "downLong"
            tooltip: I18n.t("modlist.online.install_btn")
        }
    }

    Component {
        id: uninstallButton
        UninstallButton {
            mod: item.installed
            installContext: item.installContext
            fontSize: 1.5 * Theme.rem
            progressStyle: "circular"
            busyPlain: true
            // `visibility: hidden` in the web UI, which keeps the button's space
            opacity: item.hot || busy ? 1 : 0
            enabled: opacity > 0
            btn.width: btn.height
            btn.horizontalPadding: 0.65 * fontSize
            btn.verticalPadding: 0.25 * fontSize
            iconName: "trash"
            tooltip: I18n.t("modlist.installed.uninstall_btn")
            onUninstalled: item.uninstalled()
        }
    }
}
