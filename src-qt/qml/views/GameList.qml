import QtQuick
import QtQuick.Effects
import Manderrow

// The games on the game selection screen, ported from GameList.module.css and GameComponent in
// GameSelect.tsx. Cards wrap into rows that are centered, except while searching.
Item {
    id: list

    /// Indices into Globals.games, in display order.
    property var games: []
    /// "card" or "list"
    property string displayType: "card"
    property bool searching: false
    /// The visible region, relative to this item. Only games near it are instantiated.
    property real viewportTop: 0
    property real viewportHeight: 0

    signal selected(var game)

    readonly property bool cards: displayType === "card"
    readonly property real maxWidth: 5 * 14 * Theme.rem + 6 * Theme.rem
    readonly property real innerWidth: Math.min(width, maxWidth) - (cards ? 2 * Theme.rem : 0)
    readonly property real innerX: (width - Math.min(width, maxWidth)) / 2 + (cards ? Theme.rem : 0)

    readonly property real cardWidth: 14 * Theme.rem
    readonly property real cardHeight: cardWidth * 4 / 3
    readonly property real gap: Theme.rem
    readonly property int columns: Math.max(1, Math.floor((innerWidth + gap) / (cardWidth + gap)))
    readonly property real rowHeight: 2 * 0.75 * Theme.rem + Math.max(1.5 * Theme.rem, 1.15 * Theme.rem * 1.25, Theme.rem * 1.2 + 2 * 0.35 * Theme.rem)
    property int hoveredRow: -1

    implicitHeight: games.length === 0 ? emptyMessage.implicitHeight + Theme.rem : cards ? Math.ceil(games.length / columns) * (cardHeight + gap) - gap : games.length * rowHeight

    function cardPosition(index) {
        const row = Math.floor(index / columns);
        const col = index % columns;
        const inRow = Math.min(columns, games.length - row * columns);
        const rowWidth = inRow * cardWidth + (inRow - 1) * gap;
        const offset = searching ? 0 : (innerWidth - rowWidth) / 2;
        return Qt.point(innerX + offset + col * (cardWidth + gap), row * (cardHeight + gap));
    }

    function setDefaultAndSelect(game) {
        return Api.updateSettings({ defaultGame: { override: game.id } }).then(() => list.selected(game));
    }

    Text {
        id: emptyMessage
        visible: list.games.length === 0
        width: parent.width
        y: 0.5 * Theme.rem
        horizontalAlignment: Text.AlignHCenter
        text: I18n.t("game_select.no_games_msg")
        color: Theme.neutral05
        font.family: Theme.font
        font.pixelSize: Theme.rem
        font.weight: Font.Medium
    }

    Repeater {
        model: list.games

        delegate: Item {
            id: slot

            required property var modelData
            required property int index
            readonly property var game: Globals.games[modelData]
            readonly property point position: list.cards ? list.cardPosition(index) : Qt.point(list.innerX, index * list.rowHeight)

            x: position.x
            y: position.y
            width: list.cards ? list.cardWidth : list.innerWidth
            height: list.cards ? list.cardHeight : list.rowHeight

            Loader {
                anchors.fill: parent
                active: slot.y + slot.height > list.viewportTop - 200 && slot.y < list.viewportTop + list.viewportHeight + 200
                sourceComponent: list.cards ? cardComponent : rowComponent

                Component {
                    id: cardComponent
                    GameCard {
                        game: slot.game
                        onSelected: list.selected(slot.game)
                        setDefault: () => list.setDefaultAndSelect(slot.game)
                    }
                }

                Component {
                    id: rowComponent
                    GameRow {
                        game: slot.game
                        index: slot.index
                        hoveredRow: list.hoveredRow
                        onHoveredChanged: {
                            if (hovered)
                                list.hoveredRow = slot.index;
                            else if (list.hoveredRow === slot.index)
                                list.hoveredRow = -1;
                        }
                        onSelected: list.selected(slot.game)
                        setDefault: () => list.setDefaultAndSelect(slot.game)
                    }
                }
            }
        }
    }
}
