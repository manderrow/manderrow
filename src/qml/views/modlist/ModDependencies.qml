import QtQuick
import QtQuick.Layouts
import Manderrow

// The dependencies of a mod version, ported from ModViewDependencies in ModView.tsx.
Item {
    id: root

    property string gameId
    /// Dependency strings like "Owner-Name-1.2.3", or undefined while loading.
    property var dependencies
    /// How far the dependencies reach to the right: past its width when their names and versions
    /// don't fit beside each other, as each keeps the width of its longest word, as on the web.
    readonly property real overflowWidth: {
        let right = width;
        if (column.visible) {
            for (let i = 0; i < column.children.length; i++) {
                if (column.children[i].contentRight !== undefined)
                    right = Math.max(right, column.children[i].contentRight);
            }
        }
        return right;
    }

    implicitHeight: dependencies === undefined || dependencies.length === 0 ? message.implicitHeight : column.implicitHeight

    Text {
        id: message
        visible: root.dependencies === undefined || root.dependencies.length === 0
        width: parent.width
        wrapMode: Text.Wrap
        text: root.dependencies === undefined ? Strings.global.phrases.loading : Strings.modlist.modview.no_dependencies_msg
        color: Theme.neutral05
        font.family: Theme.font
        font.pixelSize: Theme.rem
    }

    Column {
        id: column
        visible: !message.visible
        width: parent.width - Theme.rem
        spacing: 0.5 * Theme.rem

        Repeater {
            model: root.dependencies || []

            delegate: Item {
                id: dependency

                required property string modelData
                readonly property var parts: modelData.split("-")
                /// Where its content ends, which is past its edge when the content doesn't fit.
                readonly property real contentRight: row.x + versionText.x + versionText.width

                width: column.width
                height: row.implicitHeight + Theme.rem

                BoxBackground {
                    radius: 4
                    color: hover.hovered ? Theme.neutral75 : Theme.alpha(Theme.neutral75, 0)
                    border.color: Theme.neutral75

                    Behavior on color {
                        SmoothColorAnimation {
                            duration: 100
                        }
                    }
                }

                HoverHandler {
                    id: hover
                    cursorShape: Qt.PointingHandCursor
                }
                TapHandler {
                    onTapped: Util.openUrl(Util.getModVersionUrl(root.gameId, dependency.parts[0], dependency.parts[1], dependency.parts[2]))
                }

                RowLayout {
                    id: row
                    anchors.fill: parent
                    anchors.margins: 0.5 * Theme.rem
                    spacing: 0.75 * Theme.rem

                    Item {
                        Layout.preferredWidth: 48
                        Layout.preferredHeight: 48

                        SharpImage {
                            anchors.fill: parent
                            source: Util.getIconUrl(dependency.modelData)
                        }
                    }

                    // no narrower than its longest word, past which the row overflows
                    Column {
                        Layout.fillWidth: true
                        // (rounded up, as layouts round widths to whole pixels)
                        Layout.minimumWidth: Math.ceil(Math.max(nameText.implicitWidth, externalIcon.width, ownerText.implicitWidth))
                        Layout.preferredWidth: Math.ceil(Math.max(nameText.implicitWidth + nameLine.spacing + externalIcon.width, ownerText.implicitWidth))

                        // the icon wraps below the name when they don't fit beside each other
                        Flow {
                            id: nameLine
                            width: parent.width
                            spacing: 0.3 * Theme.rem

                            Text {
                                id: nameText
                                text: dependency.parts[1]
                                color: Theme.neutral15
                                font.family: Theme.font
                                font.pixelSize: Theme.rem
                            }
                            // centered on the name's line
                            Item {
                                width: externalIcon.width
                                height: nameText.height

                                Icon {
                                    id: externalIcon
                                    name: "external-link"
                                    size: 0.9 * Theme.rem
                                    color: Theme.neutral30
                                    anchors.verticalCenter: parent.verticalCenter
                                }
                            }
                        }
                        Text {
                            id: ownerText
                            text: dependency.parts[0]
                            color: Theme.neutral30
                            font.family: Theme.font
                            font.pixelSize: 0.9 * Theme.rem
                        }
                    }

                    Text {
                        id: versionText
                        text: dependency.parts[2]
                        color: Theme.neutral20
                        font.family: Theme.font
                        font.pixelSize: Theme.rem
                    }
                }
            }
        }
    }
}
