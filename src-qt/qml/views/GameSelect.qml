import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Shapes
import Manderrow

// The game selection screen, ported from GameSelect.tsx. It covers the profile view and zooms
// away once a game is chosen.
//
// The page is a ListView of rows of games, so rows beyond the viewport are created ahead of time
// across frames, and rows far away are released.
Rectangle {
    id: page
    objectName: "gameSelect"

    /// Whether choosing a game replaces the current history entry.
    property bool replace: true
    /// False once the page should animate away.
    property bool shouldShow: true

    signal beginDismiss
    signal finishDismiss

    readonly property int zoomDuration: 250

    // "card" or "list"
    property string displayType: "card"
    property string search: ""
    property string sort: Api.gameSortColumn.modDownloads
    property var filteredGames: Globals.initialSortedGames
    property int _searchGeneration: 0

    readonly property bool cards: displayType === "card"
    readonly property real maxListWidth: 5 * 14 * Theme.rem + 6 * Theme.rem
    // .main's padding, then the list's own padding in the card layout
    readonly property real listWidth: Math.min(width - 2 * Theme.rem, maxListWidth)
    readonly property real innerWidth: listWidth - (cards ? 2 * Theme.rem : 0)
    readonly property real innerX: (width - listWidth) / 2 + (cards ? Theme.rem : 0)
    readonly property real cardWidth: 14 * Theme.rem
    readonly property real cardHeight: cardWidth * 4 / 3
    readonly property real gap: Theme.rem
    readonly property int columns: Math.max(1, Math.floor((innerWidth + gap) / (cardWidth + gap)))
    readonly property real gameRowHeight: 2 * 0.75 * Theme.rem + Math.max(1.5 * Theme.rem, Theme.rem * 1.2 + 2 * 0.35 * Theme.rem)

    /// The games in rows of the current layout.
    readonly property var rows: {
        const games = filteredGames;
        if (!cards)
            return games.map(game => [game]);
        const out = [];
        for (let i = 0; i < games.length; i += columns)
            out.push(games.slice(i, i + columns));
        return out;
    }

    color: Theme.surface(Theme.hsl(214, 28, 5))
    clip: true

    function _refresh() {
        const generation = ++_searchGeneration;
        Api.searchGames(search, [
            { column: Api.gameSortColumn.relevance, descending: true },
            { column: sort, descending: sort !== Api.gameSortColumn.name }
        ]).then(games => {
            if (!page)
                return;
            // keep the existing delegates when the results are unchanged
            const current = page.filteredGames;
            const same = games.length === current.length && games.every((game, i) => game === current[i]);
            if (generation === page._searchGeneration && !same)
                page.filteredGames = games;
        }).catch(e => Ui.window.reportError(e));
    }

    onSearchChanged: _refresh()
    onSortChanged: _refresh()

    function select(game) {
        Router.navigate(`/profile/${game.id}/`, { replace: page.replace });
        page.beginDismiss();
    }

    function setDefaultAndSelect(game) {
        return Api.updateSettings({ defaultGame: { override: game.id } }).then(() => page.select(game));
    }

    // the zoom animation: in from 110% while fading in, and back out while fading out
    opacity: 0
    scale: 1.1
    states: State {
        name: "shown"
        when: page.shouldShow
        PropertyChanges {
            page.opacity: 1
            page.scale: 1
        }
    }
    transitions: Transition {
        SmoothNumberAnimation {
            properties: "opacity,scale"
            duration: page.zoomDuration
        }
    }
    // the page ignores input while it zooms away
    enabled: shouldShow

    onShouldShowChanged: if (!shouldShow)
        dismissTimer.start()
    Component.onCompleted: if (!shouldShow)
        finishDismiss()

    Timer {
        id: dismissTimer
        interval: page.zoomDuration
        onTriggered: page.finishDismiss()
    }

    // everything that scrolls under the search bar, which blurs it
    Item {
        id: backdrop
        anchors.fill: parent

        // gradient blobs behind the page, from GameBlobs.module.css. They fill the page's
        // content, so they scroll with it.
        Item {
            id: blobs
            y: list.originY - list.contentY
            width: page.width
            height: Math.max(page.height, list.contentHeight)
            clip: true
            readonly property real vmax: Math.max(page.width, page.height)
            readonly property real rem: Theme.rem

            component Blob: Shape {
                id: blob
                property color color
                property real stop: 0.5
                property bool elliptical: false

                ShapePath {
                    strokeWidth: -1
                    strokeColor: "transparent"
                    fillGradient: RadialGradient {
                        centerX: blob.width / 2
                        centerY: blob.height / 2
                        // "circle"/"ellipse" farthest-corner sizing
                        centerRadius: blob.elliptical ? blob.width / 2 * Math.SQRT2 : Math.hypot(blob.width, blob.height) / 2
                        focalX: centerX
                        focalY: centerY
                        GradientStop {
                            position: 0
                            color: blob.color
                        }
                        GradientStop {
                            position: blob.stop
                            color: "transparent"
                        }
                    }
                    PathRectangle {
                        width: blob.width
                        height: blob.height
                    }
                }
                transform: Scale {
                    yScale: blob.elliptical ? blob.height / blob.width : 1
                    origin.y: blob.height / 2
                }
            }

            Blob {
                width: 140 * blobs.vmax / 100
                height: 100 * blobs.rem
                x: blobs.width / 2 - width / 2
                y: -height / 2
                color: Theme.alpha(Theme.primary40, 0.7)
                stop: 0.5
                elliptical: true
            }
            Blob {
                width: 135 * blobs.rem
                height: width
                x: -0.3 * width
                y: -0.15 * height
                color: Theme.alpha(Theme.accent4, 0.3)
                stop: 0.6
            }
            Blob {
                width: 2 * blobs.vmax
                height: width
                x: 0
                y: -0.7 * blobs.vmax
                color: Theme.alpha(Theme.accent4, 0.45)
                stop: 0.6
            }
            Blob {
                width: 150 * blobs.rem
                height: width
                x: blobs.width * 0.1 - width / 2
                y: 0.1 * height
                color: Theme.alpha(Theme.ok4, 0.3)
                stop: 0.5
            }
        }

        ListView {
            id: list
            objectName: "gameList"
            anchors.fill: parent
            model: page.rows
            // rows this far beyond the viewport are created ahead of time, across frames
            cacheBuffer: Math.max(0, 2 * height)
            boundsBehavior: Flickable.StopAtBounds
            ScrollBar.vertical: AppScrollBar {}
            WheelScroll {}
            // one stop in the Tab order, whose items the arrow keys move between
            FocusGroup {
                view: list
            }

            property int hoveredRow: -1

            header: Item {
                width: list.width
                height: searchSlot.y + searchSlot.height + 0.5 * Theme.rem + Theme.rem

                Row {
                    id: language
                    anchors.right: parent.right
                    anchors.rightMargin: 0.5 * Theme.rem
                    y: 0.5 * Theme.rem
                    spacing: 0.25 * Theme.rem

                    Icon {
                        name: "globe"
                        anchors.verticalCenter: parent.verticalCenter
                    }

                    SelectDropdown {
                        objectName: "languageSelect"
                        options: I18n.rawLocales.map(locale => ({
                                    label: I18n.localeNames[locale],
                                    value: locale,
                                    selected: I18n.locale === locale
                                }))
                        onChanged: value => I18n.locale = value
                    }
                }

                Column {
                    id: header
                    y: language.y + language.height + 0.5 * Theme.rem + 3 * Theme.rem
                    width: parent.width
                    spacing: 0.75 * Theme.rem

                    GradientTitle {
                        anchors.horizontalCenter: parent.horizontalCenter
                        text: I18n.t("game_select.title")
                    }

                    SText {
                        anchors.horizontalCenter: parent.horizontalCenter
                        width: Math.min(implicitWidth, parent.width - 2 * Theme.rem)
                        horizontalAlignment: TextEdit.AlignHCenter
                        wrapMode: TextEdit.Wrap
                        text: I18n.t("game_select.subtitle")
                        color: Theme.neutral20
                        font.family: Theme.font
                        font.pixelSize: 1.25 * Theme.rem
                        font.weight: Font.Light
                    }
                }

                // where the search bar sits until it sticks to the top
                Item {
                    id: searchSlot
                    // .header's padding-bottom, then .main's and .gameSearch's padding-top
                    y: header.y + header.height + 5 * Theme.rem + Theme.rem + Theme.rem
                    width: parent.width
                    height: searchBox.contentHeight
                }

                Component.onCompleted: searchBox.slot = searchSlot
            }

            footer: Item {
                width: list.width
                // .main's padding-bottom
                height: (page.filteredGames.length === 0 ? emptyMessage.implicitHeight + Theme.rem : 0) + 2 * Theme.rem

                SText {
                    id: emptyMessage
                    visible: page.filteredGames.length === 0
                    width: parent.width
                    y: 0.5 * Theme.rem
                    horizontalAlignment: TextEdit.AlignHCenter
                    text: I18n.t("game_select.no_games_msg")
                    color: Theme.neutral05
                    font.family: Theme.font
                    font.pixelSize: Theme.rem
                    font.weight: Font.Medium
                }
            }

            delegate: Item {
                id: row

                required property var modelData
                required property int index
                readonly property real rowWidth: modelData.length * page.cardWidth + (modelData.length - 1) * page.gap

                width: list.width
                height: page.cards ? page.cardHeight + page.gap : page.gameRowHeight

                Repeater {
                    model: page.cards ? row.modelData : 0

                    delegate: GameCard {
                        required property var modelData
                        required property int index
                        // rows are centred, except while searching
                        x: page.innerX + (page.search.length > 0 ? 0 : (page.innerWidth - row.rowWidth) / 2) + index * (page.cardWidth + page.gap)
                        width: page.cardWidth
                        height: page.cardHeight
                        game: Globals.games[modelData]
                        setDefault: () => page.setDefaultAndSelect(game)
                        onSelected: page.select(game)
                    }
                }

                Loader {
                    active: !page.cards
                    x: page.innerX
                    width: page.innerWidth
                    height: page.gameRowHeight
                    sourceComponent: GameRow {
                        game: Globals.games[row.modelData[0]]
                        index: row.index
                        hoveredRow: list.hoveredRow
                        onHoveredChanged: {
                            if (hovered)
                                list.hoveredRow = row.index;
                            else if (list.hoveredRow === row.index)
                                list.hoveredRow = -1;
                        }
                        setDefault: () => page.setDefaultAndSelect(game)
                        onSelected: page.select(game)
                    }
                }
            }
        }
    }

    // the search bar sticks to the top of the page while scrolling
    Item {
        id: searchBox

        property Item slot: null
        // the border is outside the input
        readonly property real contentHeight: searchField.implicitHeight + 2
        // the slot scrolls with the list's header; reading contentY re-evaluates this as it does
        readonly property real naturalY: slot ? slot.mapToItem(page, 0, list.contentY * 0).y : 0

        y: Math.max(0, naturalY - Theme.rem)
        x: Theme.rem
        width: parent.width - 2 * Theme.rem
        height: searchContent.height + 2 * Theme.rem
        z: 2

        Item {
            id: searchContent
            y: Theme.rem
            anchors.horizontalCenter: parent.horizontalCenter
            width: Math.min(parent.width, 75 * Ui.ch)
            height: searchBox.contentHeight

            // backdrop-filter: blur(8px)
            BackdropBlur {
                anchors.fill: parent
                source: backdrop
                sourceRect: Qt.rect(searchBox.x + searchContent.x, searchBox.y + searchContent.y, searchContent.width, searchContent.height)
                radius: 8
                cornerRadius: searchShape.radius
                // opaque, since the content beneath it isn't hidden
                backgroundColor: Theme.alpha(page.color, 1)
            }

            Rectangle {
                id: searchShape
                anchors.fill: parent
                radius: 0.5 * Theme.rem
                color: Theme.alpha(Theme.neutral80, 0.5)
                border.color: Theme.alpha(Theme.neutral60, 0.5)
            }

            Row {
                anchors.fill: parent
                anchors.margins: 1

                Field {
                    id: searchField
                    variant: "bare"
                    width: parent.width - sortSelect.width - displayToggle.width
                    fontSize: 1.1 * Theme.rem
                    leftPadding: 0.75 * fontSize
                    rightPadding: 0.75 * fontSize
                    topPadding: 0.5 * fontSize
                    bottomPadding: 0.5 * fontSize
                    placeholderText: I18n.t("game_select.search.input_placeholder")
                    placeholderTextColor: Qt.rgba(196 / 255, 196 / 255, 196 / 255, 1)
                    maximumLength: 100
                    focus: true
                    Component.onCompleted: forceActiveFocus()
                    onTextChanged: page.search = text
                }

                SelectDropdown {
                    id: sortSelect
                    height: parent.height
                    preset: I18n.t("global.select_dropdown.sort_by")
                    trigger.bgColor: "transparent"
                    trigger.activeBgColor: Theme.alpha(Theme.neutral10, 0.1)
                    trigger.radius: 0.5 * Theme.rem
                    options: [
                        {
                            value: Api.gameSortColumn.modDownloads,
                            label: I18n.t("global.game_sort_column.mod_downloads"),
                            selected: page.sort === Api.gameSortColumn.modDownloads
                        },
                        {
                            value: Api.gameSortColumn.popularity,
                            label: I18n.t("global.game_sort_column.popularity"),
                            selected: page.sort === Api.gameSortColumn.popularity
                        },
                        {
                            value: Api.gameSortColumn.name,
                            label: I18n.t("global.game_sort_column.name"),
                            selected: page.sort === Api.gameSortColumn.name
                        }
                    ]
                    onChanged: value => page.sort = value
                }

                Btn {
                    id: displayToggle
                    height: parent.height
                    variant: "plain"
                    radius: 0.5 * Theme.rem
                    activeBgColor: Theme.alpha(Theme.neutral10, 0.1)
                    iconName: page.displayType === "card" ? "list" : "grid-2x2"
                    tooltip: I18n.t("game_select.search.display_type_btn", {
                        type: page.displayType === "card" ? I18n.t("game_select.search.card_display_type") : I18n.t("game_select.search.list_display_type")
                    })
                    onClicked: page.displayType = page.displayType === "card" ? "list" : "card"
                }
            }
        }
    }
}
