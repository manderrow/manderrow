import QtQuick
import QtQuick.Layouts
import Manderrow

// The dependencies of a mod version, ported from ModViewDependencies in ModView.tsx.
Item {
    id: root

    property string gameId
    /// Dependency strings like "Owner-Name-1.2.3", or undefined while loading.
    property var dependencies

    implicitHeight: dependencies === undefined || dependencies.length === 0 ? message.implicitHeight : column.implicitHeight

    Text {
        id: message
        visible: root.dependencies === undefined || root.dependencies.length === 0
        width: parent.width
        wrapMode: Text.Wrap
        text: root.dependencies === undefined ? I18n.t("global.phrases.loading") : I18n.t("modlist.modview.no_dependencies_msg")
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

            delegate: Rectangle {
                id: dependency

                required property string modelData
                readonly property var parts: modelData.split("-")

                width: column.width
                height: row.implicitHeight + Theme.rem
                radius: 4
                color: hover.hovered ? Theme.neutral75 : Theme.alpha(Theme.neutral75, 0)
                border.color: Theme.neutral75

                Behavior on color {
                    SmoothColorAnimation {
                        duration: 100
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

                    Column {
                        Layout.fillWidth: true

                        Row {
                            spacing: 0.3 * Theme.rem

                            Text {
                                text: dependency.parts[1]
                                color: Theme.neutral15
                                font.family: Theme.font
                                font.pixelSize: Theme.rem
                            }
                            Icon {
                                name: "external-link"
                                size: 0.9 * Theme.rem
                                color: Theme.neutral30
                                anchors.verticalCenter: parent.verticalCenter
                            }
                        }
                        Text {
                            text: dependency.parts[0]
                            color: Theme.neutral30
                            font.family: Theme.font
                            font.pixelSize: 0.9 * Theme.rem
                        }
                    }

                    Text {
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
