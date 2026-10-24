import QtQuick
import QtQuick.Layouts
import Manderrow

// A button that opens a menu of actions, ported from ContextMenu.tsx.
//
// Each item is `{ label, icon, action }` or `{ spacer: true }`.
Dropdown {
    id: menu

    property var items: []

    hideCaret: true
    placement: "bottom-start"

    ColumnLayout {
        spacing: 0

        Repeater {
            model: menu.items

            delegate: Item {
                id: entry

                required property var modelData

                Layout.fillWidth: true
                implicitWidth: entry.modelData.spacer ? 0 : itemBtn.implicitWidth + 2 * 0.35 * Theme.rem
                implicitHeight: entry.modelData.spacer ? 1 + 2 * 0.35 * Theme.rem : itemBtn.implicitHeight

                Rectangle {
                    visible: !!entry.modelData.spacer
                    anchors.verticalCenter: parent.verticalCenter
                    width: parent.width
                    height: 1
                    color: Theme.neutral70
                }

                Btn {
                    id: itemBtn
                    visible: !entry.modelData.spacer
                    x: 0.35 * Theme.rem
                    width: parent.width - 2 * x
                    variant: "plain"
                    radius: 4
                    fontSize: 0.9 * Theme.rem
                    horizontalPadding: 0.5 * fontSize
                    verticalPadding: 0.35 * fontSize
                    horizontalAlignment: Qt.AlignLeft
                    textColor: Theme.neutral20
                    activeTextColor: Theme.neutral10
                    text: entry.modelData.label || ""
                    Accessible.role: Accessible.MenuItem
                    onClicked: {
                        menu.close();
                        if (entry.modelData.action)
                            entry.modelData.action();
                    }

                    contentItem: Row {
                        spacing: 0.5 * Theme.rem

                        Item {
                            width: 17
                            height: label.height

                            Icon {
                                name: entry.modelData.icon || ""
                                size: itemBtn.fontSize
                                color: itemBtn.currentTextColor
                                anchors.centerIn: parent
                            }
                        }

                        Text {
                            id: label
                            text: itemBtn.text
                            font: itemBtn.font
                            color: itemBtn.currentTextColor
                        }
                    }
                }
            }
        }
    }
}
