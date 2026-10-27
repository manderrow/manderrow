pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import Manderrow

// The search bar of a mod list, ported from ModSearch.tsx.
Item {
    id: search

    property string query: ""
    /// `[{ column, descending }]`, in priority order.
    property var sort: []
    property bool profileSortOrder: false

    signal queryEdited(string query)
    signal sortEdited(var sort)
    signal profileSortOrderEdited(bool order)

    implicitHeight: row.implicitHeight + 2 * Ui.hairline

    BoxBackground {
        radius: height / 2
        color: Theme.neutral80
        border.color: field.activeFocus ? Theme.primary50 : Theme.neutral75
    }

    RowLayout {
        id: row
        anchors.fill: parent
        anchors.margins: Ui.hairline
        anchors.rightMargin: Theme.rem + Ui.hairline
        spacing: 0.5 * Theme.rem

        Field {
            id: field
            Layout.fillWidth: true
            variant: "bare"
            leftPadding: Theme.rem
            rightPadding: 0.5 * Theme.rem
            topPadding: 0.5 * Theme.rem
            bottomPadding: 0.5 * Theme.rem
            placeholderText: "Search for mods"
            text: search.query
            onTextEdited: search.queryEdited(text)
        }

        Row {
            Layout.fillHeight: true
            // its implicit height is its own height, which its children take, so it would size
            // the bar after itself. The layouts' pixel snapping grows a bar sized like that
            // without end at some pixel ratios.
            Layout.preferredHeight: 0

            SelectDropdown {
                height: parent.height
                preset: "Sort By"
                offset: 4
                trigger.bgColor: "transparent"
                trigger.textColor: Theme.neutral20
                trigger.radius: 0
                // the original never wired these up: relevance always shows as selected
                options: [
                    {
                        value: ModIndex.Relevance,
                        label: Strings.global.mod_sort_column.relevance,
                        selected: true
                    },
                    {
                        value: ModIndex.Downloads,
                        label: Strings.global.mod_sort_column.downloads,
                        selected: false
                    },
                    {
                        value: ModIndex.Name,
                        label: Strings.global.mod_sort_column.name,
                        selected: false
                    },
                    {
                        value: ModIndex.Owner,
                        label: Strings.global.mod_sort_column.owner,
                        selected: false
                    },
                    {
                        value: ModIndex.Size,
                        label: Strings.global.mod_sort_column.size,
                        selected: false
                    }
                ]
            }

            Btn {
                height: parent.height
                horizontalPadding: 0.5 * fontSize
                radius: 0
                bgColor: "transparent"
                textColor: Theme.neutral20
                iconName: search.profileSortOrder ? "arrow-up-wide-narrow" : "arrow-down-narrow-wide"
                onClicked: search.profileSortOrderEdited(!search.profileSortOrder)
            }
        }

        Dropdown {
            id: advanced
            Layout.fillHeight: true
            label: "Advanced"
            trigger.bgColor: "transparent"
            trigger.textColor: Theme.neutral20

            // the sort columns, reordered by dragging
            Item {
                id: sortOptions

                readonly property real rowHeight: 2 * 8 + Theme.rem * 1.25
                property int dragFrom: -1
                property int dragTo: -1

                // the widest row, with its padding and the panel's
                implicitWidth: Math.ceil(Math.max(0, ...search.sort.map(option => metrics.advanceWidth(columnLabel(option)) + 8 + metrics.advanceWidth(orderLabel(option))))) + 2 * 8 + 2 * 8
                implicitHeight: search.sort.length * rowHeight + 2 * 8

                FontMetrics {
                    id: metrics
                    font.family: Theme.font
                    font.pixelSize: Theme.rem
                }

                function columnLabel(option) {
                    switch (option.column) {
                    case ModIndex.Relevance:
                        return Strings.global.mod_sort_column.relevance;
                    case ModIndex.Downloads:
                        return Strings.global.mod_sort_column.downloads;
                    case ModIndex.Name:
                        return Strings.global.mod_sort_column.name;
                    case ModIndex.Owner:
                        return Strings.global.mod_sort_column.owner;
                    default:
                        return Strings.global.mod_sort_column.size;
                    }
                }
                function orderLabel(option) {
                    return option.descending ? Strings.global.sort_order.descending : Strings.global.sort_order.ascending;
                }

                function displayIndex(index) {
                    if (dragFrom < 0 || index === dragFrom)
                        return index;
                    if (dragFrom < dragTo && index > dragFrom && index <= dragTo)
                        return index - 1;
                    if (dragFrom > dragTo && index < dragFrom && index >= dragTo)
                        return index + 1;
                    return index;
                }

                Repeater {
                    id: optionRepeater
                    model: search.sort

                    delegate: Item {
                        id: option

                        required property var modelData
                        required property int index
                        readonly property bool dragging: drag.active

                        x: 8
                        width: sortOptions.width - 16
                        height: sortOptions.rowHeight
                        // the DragHandler takes over y while dragging; the model is replaced on drop
                        y: 8 + sortOptions.displayIndex(index) * sortOptions.rowHeight
                        z: dragging ? 1 : 0
                        opacity: dragging ? 0.25 : 1

                        Behavior on y {
                            enabled: !option.dragging
                            SmoothNumberAnimation {
                                duration: 150
                                curve: [0.4, 0, 0.2, 1, 1, 1]
                            }
                        }

                        HoverHandler {
                            cursorShape: option.dragging ? Qt.ClosedHandCursor : Qt.OpenHandCursor
                        }
                        DragHandler {
                            id: drag
                            xAxis.enabled: false
                            onActiveChanged: {
                                if (active) {
                                    sortOptions.dragFrom = option.index;
                                    sortOptions.dragTo = option.index;
                                } else {
                                    const from = sortOptions.dragFrom;
                                    const to = sortOptions.dragTo;
                                    sortOptions.dragFrom = -1;
                                    sortOptions.dragTo = -1;
                                    if (from !== to) {
                                        const updated = search.sort.slice();
                                        updated.splice(to, 0, updated.splice(from, 1)[0]);
                                        search.sortEdited(updated);
                                    } else {
                                        option.y = 8 + option.index * sortOptions.rowHeight;
                                    }
                                }
                            }
                            onTranslationChanged: {
                                const target = Math.round((option.y - 8) / sortOptions.rowHeight);
                                sortOptions.dragTo = Math.max(0, Math.min(search.sort.length - 1, target));
                            }
                        }

                        Row {
                            x: 8
                            anchors.verticalCenter: parent.verticalCenter
                            spacing: 8

                            Text {
                                text: sortOptions.columnLabel(option.modelData)
                                color: Theme.neutral05
                                font.family: Theme.font
                                font.pixelSize: Theme.rem
                            }

                            Text {
                                text: sortOptions.orderLabel(option.modelData)
                                color: Theme.neutral05
                                font.family: Theme.font
                                font.pixelSize: Theme.rem

                                ClickArea {
                                    onActivated: {
                                        const updated = search.sort.slice();
                                        updated[option.index] = {
                                            column: option.modelData.column,
                                            descending: !option.modelData.descending
                                        };
                                        search.sortEdited(updated);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
