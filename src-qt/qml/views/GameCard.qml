import QtQuick
import QtQuick.Effects
import Manderrow

// A game in the card layout of the game selection screen. The cover glows through a blurred,
// brightened copy of itself, and hovering reveals the game's name and actions.
Item {
    id: card

    required property var game
    property var setDefault: function () {}

    signal selected

    readonly property real inset: 5
    readonly property bool hot: hover.hovered || actions.activeFocus

    // overflow: hidden with border-radius: 4px
    layer.enabled: true
    layer.effect: MultiEffect {
        maskEnabled: true
        maskSource: roundedMask
    }

    Rectangle {
        id: roundedMask
        width: card.width
        height: card.height
        radius: 4
        visible: false
        layer.enabled: true
    }

    HoverHandler {
        id: hover
    }

    Image {
        id: cover
        anchors.fill: parent
        source: Util.gameCoverUrl(card.game.thunderstoreId)

        sourceSize: Qt.size(card.width, card.height)
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
        visible: false
        layer.enabled: true
    }

    // ::before: filter: blur(4px) brightness(1.9); opacity: 0.75
    MultiEffect {
        anchors.fill: parent
        source: cover
        blurEnabled: true
        blur: 1
        blurMax: 8
        brightness: card.hot ? 0.45 : 0.4
        opacity: card.hot ? 0.9 : 0.75

        Behavior on opacity {
            NumberAnimation {
                duration: card.hot ? 200 : 500
            }
        }
    }

    Rectangle {
        id: innerMask
        anchors.fill: inner
        radius: 4
        visible: false
        layer.enabled: true
    }

    Item {
        id: inner
        anchors.fill: parent
        anchors.margins: card.inset

        MultiEffect {
            anchors.fill: parent
            source: ShaderEffectSource {
                sourceItem: cover
                sourceRect: Qt.rect(card.inset, card.inset, inner.width, inner.height)
            }
            maskEnabled: true
            maskSource: innerMask
            // the content's backdrop-filter: blur(4px) while hovered
            blurEnabled: overlay.opacity > 0
            blur: overlay.opacity
            blurMax: 8
        }
    }

    // the name and actions, revealed on hover
    Item {
        id: overlay
        anchors.fill: parent
        opacity: card.hot ? 1 : 0

        Behavior on opacity {
            NumberAnimation {
                duration: 300
            }
        }

        Rectangle {
            anchors.fill: parent
            radius: 4
            color: Qt.rgba(0, 0, 0, 0.25)
        }

        Item {
            id: content
            anchors.fill: parent
            anchors.margins: 0.75 * Theme.rem
            layer.enabled: card.hot
            layer.effect: MultiEffect {
                shadowEnabled: true
                shadowColor: Qt.rgba(0, 0, 0, 0.4)
                shadowBlur: 0.5
                shadowHorizontalOffset: 0
                shadowVerticalOffset: 0
            }

            Btn {
                id: favorite
                anchors.right: parent.right
                variant: "plain"
                padAll: 0
                fontSize: 1.5 * Theme.rem
                iconName: "regular:star"
                textColor: Theme.neutral20
                activeBgColor: "transparent"
                tooltip: I18n.t("game_select.fav_btn")
            }

            Text {
                id: title
                anchors.left: parent.left
                anchors.right: favorite.left
                anchors.rightMargin: 0.25 * Theme.rem
                text: card.game.name
                wrapMode: Text.Wrap
                lineHeight: 1.1
                color: Theme.neutral05
                font.family: Theme.font
                font.pixelSize: 1.15 * Theme.rem
                font.weight: 850
            }

            GameActions {
                id: actions
                anchors.horizontalCenter: parent.horizontalCenter
                // margin-block: auto below the title
                y: Math.max(title.height + 1.15 * Theme.rem, (parent.height - height) / 2)
                setDefault: card.setDefault
                onSelected: card.selected()
            }
        }
    }
}
