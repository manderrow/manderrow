import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Effects
import QtQuick.Shapes
import Manderrow

// The game selection screen, ported from GameSelect.tsx. It covers the profile view and zooms
// away once a game is chosen.
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

    color: Theme.hsl(214, 28, 5)
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
        NumberAnimation {
            properties: "opacity,scale"
            duration: page.zoomDuration
            easing.type: Easing.InOutQuad
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

    // gradient blobs behind the page, from GameBlobs.module.css
    Item {
        id: blobs
        anchors.fill: parent
        readonly property real vmax: Math.max(width, height)
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

    Flickable {
        id: flick
        anchors.fill: parent
        contentWidth: width
        contentHeight: inner.height
        boundsBehavior: Flickable.StopAtBounds
        ScrollBar.vertical: AppScrollBar {
            policy: ScrollBar.AlwaysOn
        }

        Item {
            id: inner
            width: flick.width
            height: Math.max(flick.height, main.y + main.height)

            Row {
                id: language
                anchors.right: parent.right
                anchors.rightMargin: 0.5 * Theme.rem
                y: 0.5 * Theme.rem
                spacing: 0.25 * Theme.rem

                FaIcon {
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

                Item {
                    anchors.horizontalCenter: parent.horizontalCenter
                    width: title.implicitWidth
                    height: title.implicitHeight

                    Text {
                        id: title
                        text: I18n.t("game_select.title")
                        font.family: Theme.font
                        font.pixelSize: 4 * Theme.rem
                        font.weight: 650
                        visible: false
                        layer.enabled: true
                    }

                    // the title is filled with linear-gradient(180deg, primary-60 20%, primary-40)
                    Rectangle {
                        id: titleFill
                        anchors.fill: parent
                        visible: false
                        layer.enabled: true
                        gradient: Gradient {
                            GradientStop {
                                position: 0.2
                                color: Theme.primary60
                            }
                            GradientStop {
                                position: 1
                                color: Theme.primary40
                            }
                        }
                    }

                    MultiEffect {
                        anchors.fill: parent
                        source: titleFill
                        maskEnabled: true
                        maskSource: title
                    }

                    // the lines on either side of the title
                    Rectangle {
                        width: parent.width * 0.2
                        height: 2
                        anchors.verticalCenter: parent.verticalCenter
                        x: -width - 0.5 * 4 * Theme.rem
                        gradient: Gradient {
                            orientation: Gradient.Horizontal
                            GradientStop {
                                position: 0
                                color: "transparent"
                            }
                            GradientStop {
                                position: 1
                                color: Theme.neutral50
                            }
                        }
                    }
                    Rectangle {
                        width: parent.width * 0.2
                        height: 2
                        anchors.verticalCenter: parent.verticalCenter
                        x: parent.width + 0.5 * 4 * Theme.rem
                        gradient: Gradient {
                            orientation: Gradient.Horizontal
                            GradientStop {
                                position: 0
                                color: Theme.neutral50
                            }
                            GradientStop {
                                position: 1
                                color: "transparent"
                            }
                        }
                    }
                }

                Text {
                    anchors.horizontalCenter: parent.horizontalCenter
                    width: Math.min(implicitWidth, parent.width - 2 * Theme.rem)
                    horizontalAlignment: Text.AlignHCenter
                    wrapMode: Text.Wrap
                    text: I18n.t("game_select.subtitle")
                    color: Theme.neutral20
                    font.family: Theme.font
                    font.pixelSize: 1.25 * Theme.rem
                    font.weight: Font.Light
                }
            }

            Item {
                id: main
                y: header.y + header.height + 5 * Theme.rem
                width: parent.width
                height: searchBox.height + 0.5 * Theme.rem + gameList.height + 3 * Theme.rem

                GameList {
                    id: gameList
                    y: searchBox.height + 0.5 * Theme.rem + Theme.rem
                    x: Theme.rem
                    width: parent.width - 2 * Theme.rem
                    games: page.filteredGames
                    displayType: page.displayType
                    searching: page.search.length > 0
                    viewportTop: flick.contentY - (main.y + y)
                    viewportHeight: flick.height
                    onSelected: game => {
                        Router.navigate(`/profile/${game.id}/`, { replace: page.replace });
                        page.beginDismiss();
                    }
                }
            }
        }
    }

    }

    // the search bar sticks to the top of the page while scrolling
    Item {
        id: searchBox
        readonly property real naturalY: main.y + Theme.rem - flick.contentY
        y: Math.max(0, naturalY)
        x: Theme.rem
        width: parent.width - 2 * Theme.rem
        height: searchContent.height + 2 * Theme.rem
        z: 2

        Item {
            id: searchContent
            y: Theme.rem
            anchors.horizontalCenter: parent.horizontalCenter
            width: Math.min(parent.width, 75 * Ui.ch)
            height: searchField.implicitHeight

            // backdrop-filter: blur(8px)
            MultiEffect {
                anchors.fill: parent
                source: ShaderEffectSource {
                    sourceItem: backdrop
                    sourceRect: Qt.rect(searchBox.x + searchContent.x, searchBox.y + searchContent.y, searchContent.width, searchContent.height)
                }
                blurEnabled: true
                blur: 1
                blurMax: 16
                maskEnabled: true
                maskSource: searchShape
            }

            Rectangle {
                id: searchShape
                anchors.fill: parent
                radius: 0.5 * Theme.rem
                color: Theme.alpha(Theme.neutral80, 0.5)
                border.color: Theme.alpha(Theme.neutral60, 0.5)
                layer.enabled: true
            }

            Row {
                anchors.fill: parent

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
                    iconName: page.displayType === "card" ? "list" : "tableCellsLarge"
                    tooltip: I18n.t("game_select.search.display_type_btn", {
                        type: page.displayType === "card" ? I18n.t("game_select.search.card_display_type") : I18n.t("game_select.search.list_display_type")
                    })
                    onClicked: page.displayType = page.displayType === "card" ? "list" : "card"
                }
            }
        }
    }
}
