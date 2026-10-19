import QtQuick
import Manderrow

// A game in the list layout of the game selection screen.
Rectangle {
    id: row

    required property var game
    required property int index
    /// The index of the hovered row, which hides this row's top border when it's the one above.
    property int hoveredRow: -1
    property var setDefault: function () {}

    readonly property bool hovered: hover.hovered

    signal selected

    radius: hovered ? 5 : 0
    color: hovered ? Qt.rgba(225 / 255, 225 / 255, 225 / 255, 0.1) : "transparent"
    border.width: hovered ? 1 : 0
    border.color: Qt.rgba(215 / 255, 215 / 255, 215 / 255, 0.2)

    Behavior on color {
        ColorAnimation {
            duration: 200
            easing.type: Easing.OutQuad
        }
    }

    HoverHandler {
        id: hover
    }

    // the top border separating rows
    Rectangle {
        visible: !row.hovered && row.hoveredRow !== row.index - 1
        width: parent.width
        height: 1
        color: Qt.rgba(215 / 255, 215 / 255, 215 / 255, 0.2)
    }

    Row {
        anchors.left: parent.left
        anchors.leftMargin: 0.5 * Theme.rem
        anchors.verticalCenter: parent.verticalCenter
        spacing: 0.5 * Theme.rem

        Btn {
            anchors.verticalCenter: parent.verticalCenter
            variant: "plain"
            padAll: 0
            fontSize: 1.5 * Theme.rem
            iconName: "regular:star"
            textColor: Theme.neutral20
            activeBgColor: "transparent"
            tooltip: I18n.t("game_select.fav_btn")
        }

        Text {
            anchors.verticalCenter: parent.verticalCenter
            text: row.game.name
            color: Theme.neutral05
            font.family: Theme.font
            font.pixelSize: 1.15 * Theme.rem
            font.weight: 650
        }
    }

    GameActions {
        anchors.right: parent.right
        anchors.rightMargin: 0.5 * Theme.rem
        anchors.verticalCenter: parent.verticalCenter
        horizontal: true
        opacity: row.hovered ? 1 : 0
        setDefault: row.setDefault
        onSelected: row.selected()

        Behavior on opacity {
            NumberAnimation {
                duration: 200
            }
        }
    }
}
