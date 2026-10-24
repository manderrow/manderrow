import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Manderrow

// Actions on the selected installed mods, ported from BulkActions.tsx.
Item {
    id: bulk

    // overflow: hidden
    clip: true

    /// `[{ owner, name, version }]`
    property var mods: []
    property var installContext

    signal removeRequested(var mod)
    signal cancelRequested

    readonly property int maxCards: 5
    readonly property color bulkBorder: Theme.neutral75
    property string search: ""

    function installedMod(id) {
        return installContext.getInstalled(id.owner, id.name);
    }

    readonly property real totalSize: mods.reduce((total, id) => {
        const mod = installedMod(id);
        return total + (mod && mod.version.file_size ? mod.version.file_size : 0);
    }, 0)

    // The table's rows are no narrower than their widest name, owner, version, and size, as in the
    // web UI's grid of them, whose columns are sized to what is in them. Its rows aren't all
    // created at once, so they're measured in the fonts of a row's parts.
    readonly property real rowPadding: 0.35 * Theme.rem
    readonly property real columnGap: Theme.rem
    readonly property real partGap: 0.25 * Theme.rem
    FontMetrics {
        id: partMetrics
        font.family: Theme.font
        font.pixelSize: Theme.rem
    }
    FontMetrics {
        id: lowPartMetrics
        font.family: Theme.font
        font.pixelSize: 0.85 * Theme.rem
    }
    FontMetrics {
        id: separatorMetrics
        font.pixelSize: 0.85 * Theme.rem
    }
    /// The selected mods matching the search, which the table shows.
    readonly property var shownMods: mods.filter(id => Util.concatenateMod(id.owner, id.name, id.version).toLowerCase().includes(search.toLowerCase()))
    readonly property real minimumRowWidth: {
        let parts = 0;
        let size = 0;
        for (const id of shownMods) {
            const mod = installedMod(id);
            if (!mod)
                continue;
            parts = Math.max(parts, Math.ceil(partMetrics.advanceWidth(mod.name)) + Math.ceil(lowPartMetrics.advanceWidth(mod.owner)) + Math.ceil(lowPartMetrics.advanceWidth(mod.version.version_number)) + 2 * Math.ceil(separatorMetrics.advanceWidth("•")) + 4 * partGap);
            size = Math.max(size, Math.ceil(partMetrics.advanceWidth(Util.humanizeFileSize(mod.version.file_size))) + 1);
        }
        // the remove buttons are as wide as two of the font's ems: an icon and half that each side
        return 2 * rowPadding + 2 * Theme.rem + 2 * columnGap + parts + size;
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // the selected mods fanned out like cards along a circle
        Item {
            id: cards
            Layout.fillWidth: true
            Layout.preferredHeight: 9 * Theme.rem
            clip: true

            // offset-path: circle(20% at center bottom)
            readonly property real pathRadius: 0.2 * Math.hypot(width, height) / Math.SQRT2
            readonly property int total: Math.min(bulk.mods.length, bulk.maxCards)
            readonly property real spacing: 36
            readonly property real cardsOffset: 180 / 2 - spacing / 2 * (total - 1)
            readonly property real cardWidth: Math.max(width * 0.1, 4 * Theme.rem)

            Repeater {
                model: bulk.mods.slice(0, bulk.maxCards)

                delegate: Rectangle {
                    id: card

                    required property var modelData
                    required property int index
                    readonly property real angle: cards.cardsOffset + cards.spacing * index
                    // the path starts at the circle's right and runs clockwise
                    readonly property real phi: (angle / 360 - 0.5) * 2 * Math.PI
                    readonly property bool overflow: index === bulk.maxCards - 1 && bulk.mods.length > bulk.maxCards

                    width: cards.cardWidth
                    height: width * 7 / 5
                    x: cards.width / 2 + cards.pathRadius * Math.cos(phi) - width / 2
                    y: cards.height + cards.pathRadius * Math.sin(phi) - height / 2
                    // offset-rotate: auto
                    rotation: phi * 180 / Math.PI + 90
                    radius: 8
                    color: Theme.neutral80
                    border.color: Theme.neutral70
                    border.width: Ui.hairline

                    RoundedImage {
                        id: art
                        anchors.fill: parent
                        anchors.margins: Ui.hairline
                        radius: card.radius - Ui.hairline
                        source: Util.getIconUrl(Util.getQualifiedModName(card.modelData.owner, card.modelData.name, card.modelData.version))
                        // replaced by the blurred copy, through which it would show otherwise
                        opacity: card.overflow ? 0 : 1
                        mipmap: card.overflow
                    }

                    // backdrop-filter: blur(6px)
                    ImageEffect {
                        visible: card.overflow && art.status === Image.Ready
                        anchors.fill: art
                        image: art.image
                        cornerRadius: art.radius
                        blurRadius: 6
                        blurFadesEdges: false
                    }

                    // the number of selected mods without a card
                    Rectangle {
                        visible: card.overflow
                        anchors.fill: parent
                        radius: card.radius
                        color: Qt.rgba(0, 0, 0, 0.5)

                        Text {
                            anchors.centerIn: parent
                            text: `+${bulk.mods.length - bulk.maxCards + 1}`
                            color: Theme.neutral05
                            font.family: Theme.font
                            font.pixelSize: Theme.rem
                            style: Text.Raised
                            styleColor: Qt.rgba(0, 0, 0, 0.7)
                        }
                    }
                }
            }
        }

        // the header
        Item {
            Layout.fillWidth: true
            Layout.rightMargin: Theme.rem
            implicitHeight: headerRow.implicitHeight + Theme.rem

            BoxBackground {
                topLeftRadius: 0.5 * Theme.rem
                topRightRadius: 0.5 * Theme.rem
                border.color: bulk.bulkBorder
            }

            RowLayout {
                id: headerRow
                anchors.fill: parent
                anchors.margins: 0.5 * Theme.rem

                // wraps rather than running under the search field when the header is narrow
                SText {
                    id: title
                    Layout.fillWidth: true
                    Layout.maximumWidth: Math.ceil(implicitWidth)
                    Layout.minimumWidth: Util.longestWordWidth(titleMetrics, text)
                    wrapMode: TextEdit.WordWrap
                    text: I18n.t("modlist.installed.multiselect_title")
                    color: Theme.neutral20
                    font.family: Theme.font
                    font.pixelSize: 1.5 * Theme.rem

                    FontMetrics {
                        id: titleMetrics
                        font: title.font
                    }
                }
                Item {
                    Layout.fillWidth: true
                }
                Field {
                    placeholderText: I18n.t("global.phrases.search")
                    onTextChanged: bulk.search = text
                }
            }
        }

        // the table of selected mods, which scrolls (overflow: auto), sideways too when its rows are
        // wider than it
        ScrollArea {
            Layout.fillWidth: true
            Layout.fillHeight: true
            flickable: table

            ListView {
                id: table

                // padding-right: 1em, before the scroll bar
                readonly property real tableWidth: width - Theme.rem
                readonly property real rowWidth: Math.max(tableWidth, bulk.minimumRowWidth)

                contentWidth: Math.max(width, rowWidth)
                clip: true
                boundsBehavior: Flickable.StopAtBounds
                // one stop in the Tab order, whose items the arrow keys move between
                FocusGroup {
                    view: table
                }
                model: bulk.shownMods

                // the table's border, without a bottom edge, which its rows reach past when they're
                // wider than it
                Rectangle {
                    parent: table
                    x: -table.contentX
                    width: table.tableWidth
                    height: table.height + Ui.hairline
                    color: "transparent"
                    border.color: bulk.bulkBorder
                    border.width: Ui.hairline
                    z: 1
                }

                delegate: Rectangle {
                    id: entry

                    required property var modelData
                    required property int index
                    readonly property var mod: bulk.installedMod(modelData)

                    width: table.rowWidth
                    implicitHeight: entryRow.implicitHeight + 2 * bulk.rowPadding + Ui.hairline
                    color: entryHover.hovered ? Theme.neutral75 : index % 2 === 1 ? Theme.neutral80 : "transparent"

                    HoverHandler {
                        id: entryHover
                    }
                    Rectangle {
                        anchors.bottom: parent.bottom
                        width: parent.width
                        height: Ui.hairline
                        color: bulk.bulkBorder
                    }

                    RowLayout {
                        id: entryRow
                        anchors.fill: parent
                        anchors.margins: bulk.rowPadding
                        anchors.bottomMargin: bulk.rowPadding + Ui.hairline
                        spacing: bulk.columnGap

                        Btn {
                            variant: "plain"
                            iconName: "x"
                            horizontalPadding: 0.5 * fontSize
                            verticalPadding: 0.25 * fontSize
                            activeBgColor: "transparent"
                            activeTextColor: Theme.danger5
                            tooltip: I18n.t("global.phrases.remove")
                            onClicked: bulk.removeRequested(entry.mod)
                        }

                        Row {
                            Layout.fillWidth: true
                            // the rows are as wide as the widest of these, but these are measured
                            // apart from them, so they're kept within the row anyway
                            Layout.minimumWidth: 0
                            Layout.preferredHeight: implicitHeight
                            clip: true
                            spacing: bulk.partGap

                            component Part: Text {
                                property bool low: false
                                color: low ? Theme.neutral35 : Theme.neutral05
                                font.family: Theme.font
                                font.pixelSize: low ? 0.85 * Theme.rem : Theme.rem
                                anchors.verticalCenter: parent.verticalCenter
                            }
                            component Separator: Text {
                                text: "•"
                                color: Theme.neutral65
                                font.pixelSize: 0.85 * Theme.rem
                                anchors.verticalCenter: parent.verticalCenter
                            }

                            Part {
                                text: entry.mod ? entry.mod.name : ""
                            }
                            Separator {}
                            Part {
                                low: true
                                text: entry.mod ? entry.mod.owner : ""
                            }
                            Separator {}
                            Part {
                                low: true
                                text: entry.mod ? entry.mod.version.version_number : ""
                            }
                        }

                        SText {
                            Layout.minimumWidth: Math.min(implicitWidth, 4 * Theme.rem)
                            text: entry.mod ? Util.humanizeFileSize(entry.mod.version.file_size) : ""
                            color: Theme.neutral05
                            font.family: Theme.font
                            font.pixelSize: Theme.rem
                        }
                    }
                }
            }
        }

        // the summary
        Item {
            Layout.fillWidth: true
            Layout.rightMargin: Theme.rem
            implicitHeight: summaryRow.implicitHeight + Theme.rem

            BoxBackground {
                bottomLeftRadius: 0.5 * Theme.rem
                bottomRightRadius: 0.5 * Theme.rem
                border.color: bulk.bulkBorder
            }

            RowLayout {
                id: summaryRow
                anchors.fill: parent
                anchors.margins: 0.5 * Theme.rem

                // wraps rather than running under the size when the row is narrow
                SText {
                    id: count
                    Layout.fillWidth: true
                    Layout.minimumWidth: Util.longestWordWidth(countMetrics, text)
                    wrapMode: TextEdit.WordWrap
                    text: I18n.t(bulk.mods.length > 1 ? "modlist.installed.selected_count_plural" : "modlist.installed.selected_count_single", {
                        count: bulk.mods.length
                    })
                    color: Theme.neutral05
                    font.family: Theme.font
                    font.pixelSize: Theme.rem

                    FontMetrics {
                        id: countMetrics
                        font: count.font
                    }
                }
                Row {
                    spacing: 0.3 * Theme.rem
                    Icon {
                        name: "hard-drive"
                        anchors.verticalCenter: parent.verticalCenter
                    }
                    SText {
                        text: Util.humanizeFileSize(bulk.totalSize)
                        color: Theme.neutral05
                        font.family: Theme.font
                        font.pixelSize: Theme.rem
                    }
                }
            }
        }

        // grid-template-columns: repeat(auto-fit, minmax(3rem, 6rem))
        Flow {
            id: actions
            Layout.fillWidth: true
            Layout.rightMargin: Theme.rem
            Layout.margins: 0.5 * Theme.rem
            Layout.bottomMargin: 0.25 * Theme.rem + 0.5 * Theme.rem
            spacing: 0.5 * Theme.rem

            readonly property int columns: Math.max(1, Math.floor((width + spacing) / (6 * Theme.rem + spacing)))
            readonly property real buttonWidth: Math.min(6 * Theme.rem, (width - (columns - 1) * spacing) / columns)
            leftPadding: (width - (Math.min(columns, 5) * buttonWidth + (Math.min(columns, 5) - 1) * spacing)) / 2

            component Action: Btn {
                width: actions.buttonWidth
                radius: 6
                horizontalPadding: 0.5 * Theme.rem
            }

            Action {
                text: I18n.t("modlist.installed.enable_all")
            }
            Action {
                text: I18n.t("modlist.installed.disable_all")
            }
            Action {
                text: I18n.t("modlist.installed.delete_all")
            }
            Action {
                text: I18n.t("modlist.installed.update_all")
            }
            Action {
                text: I18n.t("global.phrases.cancel")
                onClicked: bulk.cancelRequested()
            }
        }
    }
}
