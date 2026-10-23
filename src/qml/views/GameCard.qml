import QtQuick
import QtQuick.Effects
import Manderrow

// A game in the card layout of the game selection screen, from GameList.module.css. The cover
// glows around its edge through a blurred, brightened copy of itself, and hovering reveals the
// game's name and actions over a blurred cover.
//
// Many cards are created while scrolling, so a card is kept cheap: the cover is decoded once and
// drawn three ways by ImageEffects, and the hover overlay is only created while it shows.
FocusScope {
    id: card

    required property var game
    property var setDefault: function () {}

    signal selected

    readonly property real inset: 5
    readonly property real radius: 4
    // the card is a focus scope, so this includes focus on the overlay's buttons
    readonly property bool hot: hover.hovered || activeFocus

    activeFocusOnTab: true
    // Enter selects the game, as its button does
    Keys.onReturnPressed: card.selected()
    Keys.onEnterPressed: card.selected()

    HoverHandler {
        id: hover
    }

    // ::before: the cover with filter: blur(4px) brightness(1.9), clipped to the card's corners
    ImageEffect {
        id: glow
        anchors.fill: parent
        image: cover.image
        cornerRadius: card.radius
        blurRadius: 4
        brightness: 1.9
        opacity: 0.75

        states: State {
            name: "hot"
            when: card.hot
            PropertyChanges {
                glow.brightness: 2
                glow.opacity: 0.9
            }
        }
        transitions: [
            // the hover rule only transitions filter, so opacity changes at once on hover
            Transition {
                to: "hot"
                SmoothNumberAnimation {
                    property: "brightness"
                    duration: 200
                }
            },
            Transition {
                from: "hot"
                SmoothNumberAnimation {
                    property: "brightness"
                    duration: 700
                }
                SmoothNumberAnimation {
                    property: "opacity"
                    duration: 500
                }
            }
        ]
    }

    // the cover, inside the card's padding
    RoundedImage {
        id: cover
        anchors.fill: parent
        anchors.margins: card.inset
        radius: card.radius
        source: Util.gameCoverUrl(card.game.thunderstoreId)
        mipmap: true
    }

    // the name and actions, revealed on hover, within the card's padding like the web UI's content
    property real _overlayOpacity: hot ? 1 : 0
    Behavior on _overlayOpacity {
        SmoothNumberAnimation {
            duration: 300
        }
    }

    Loader {
        anchors.fill: parent
        anchors.margins: card.inset
        active: card.hot || card._overlayOpacity > 0
        opacity: card._overlayOpacity
        sourceComponent: overlay
    }

    Component {
        id: overlay

        Item {
            // backdrop-filter: blur(4px) over the cover
            ImageEffect {
                anchors.fill: parent
                image: cover.image
                cornerRadius: card.radius
                blurRadius: 4
                blurFadesEdges: false
            }

            Rectangle {
                anchors.fill: parent
                radius: card.radius
                color: Qt.rgba(0, 0, 0, 0.25)
            }

            Item {
                anchors.fill: parent
                anchors.margins: 0.75 * Theme.rem
                // text-shadow: 0 0 8px rgb(0 0 0 / 0.4)
                layer.enabled: true
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
                    iconName: "star"
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
                    lineHeight: 1.1 * 1.15 * Theme.rem
                    lineHeightMode: Text.FixedHeight
                    color: Theme.neutral05
                    font.family: Theme.font
                    font.pixelSize: 1.15 * Theme.rem
                    font.weight: 850
                    font.variableAxes: ({ "wght": 850 })
                }

                GameActions {
                    objectName: "gameActions"
                    anchors.horizontalCenter: parent.horizontalCenter
                    // margin-block: auto below the title's 1em margin
                    readonly property real titleBottom: title.height + 1.15 * Theme.rem
                    y: titleBottom + Math.max(0, (parent.height - titleBottom - height) / 2)
                    setDefault: card.setDefault
                    onSelected: card.selected()
                }
            }
        }
    }
}
