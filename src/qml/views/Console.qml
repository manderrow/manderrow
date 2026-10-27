import QtQml.Models
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import QtQuick.Shapes
import Manderrow

// The game log viewer, ported from Console.tsx.
Item {
    id: consoleView

    readonly property var connection: ConsoleStore.focusedConnection
    readonly property var visibleLevelsOptions: Api.logLevels.concat(["STDOUT", "STDERR"])
    property var visibleLevels: ({
            CRITICAL: true,
            ERROR: true,
            WARN: true,
            INFO: true,
            DEBUG: false,
            TRACE: false,
            STDERR: false,
            STDOUT: false
        })
    property string searchInput: ""

    /// Whether the levels and the search show a line of the log.
    function shows(line) {
        if (line.filter !== "" && !visibleLevels[line.filter])
            return false;
        return !line.searchable || line.message.includes(searchInput);
    }

    /// The text of the shown lines from one end of the log's selection to the other, a line of
    /// the log to a line of text, with its type, scope, and message separated by spaces.
    function textBetween(start, end) {
        const events = connection.events;
        const lines = [];
        const last = Math.min(Math.floor(end.place / 3), log.count - 1);
        for (let row = Math.floor(start.place / 3); row <= last; row++) {
            const line = events.get(shownLines.mapToSource(shownLines.index(row, 0)).row);
            const cells = [];
            [line.type, line.scope, line.message].forEach((text, column) => {
                const place = row * 3 + column;
                if (place < start.place || place > end.place)
                    return;
                const cell = text.slice(place === start.place ? start.position : 0, place === end.place ? end.position : text.length);
                if (cell !== "")
                    cells.push(cell);
            });
            // as when the selection starts at the end of a line
            if (cells.length > 0)
                lines.push(cells.join(" "));
        }
        return lines.join("\n");
    }

    /// The lengths of the lines of the shown rows' messages, for estimating their heights, which
    /// are found as they are needed.
    property var _lineLengths: []
    /// The events whose rows `_lineLengths` are of, and how many of them have been looked at.
    property var _lineLengthsOf: null
    property int _lineLengthsScanned: 0

    function lineLengths(row) {
        const events = connection ? connection.events : null;
        if (_lineLengthsOf !== events || (events && events.count < _lineLengthsScanned))
            _forgetLineLengths();
        _lineLengthsOf = events;
        while (events && _lineLengths.length <= row && _lineLengthsScanned < events.count) {
            const line = events.get(_lineLengthsScanned++);
            if (shows(line))
                _lineLengths.push(line.message.split("\n").map(text => text.length));
        }
        return _lineLengths[row] || [0];
    }

    function _forgetLineLengths() {
        _lineLengths = [];
        _lineLengthsOf = null;
        _lineLengthsScanned = 0;
    }

    /// The lines of the connection's log that the levels and the search show.
    SortFilterProxyModel {
        id: shownLines
        // the proxy warns about a null model
        model: consoleView.connection ? consoleView.connection.events : noLines
        filters: [
            FunctionFilter {
                component Line: QtObject {
                    property string filter
                    property bool searchable
                    property string message
                }
                function filter(line: Line): bool {
                    return consoleView.shows(line);
                }
            }
        ]
    }

    ListModel {
        id: noLines
    }

    // the filter reads these, which the model doesn't watch
    onVisibleLevelsChanged: refilter()
    onSearchInputChanged: refilter()
    onConnectionChanged: logSelection.clear()

    function refilter() {
        logSelection.clear();
        _forgetLineLengths();
        shownLines.invalidate();
        Qt.callLater(logBar.reestimate);
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // the header
        Item {
            Layout.fillWidth: true
            Layout.rightMargin: 1.5 * Theme.rem
            implicitHeight: headerRow.implicitHeight + Ui.hairline

            Rectangle {
                anchors.bottom: parent.bottom
                width: parent.width
                height: Ui.hairline
                color: Theme.neutral70
            }

            RowLayout {
                id: headerRow
                width: parent.width
                spacing: 0.5 * Theme.rem

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 0

                    component Subgroup: RowLayout {
                        property bool separated: true
                        spacing: 0.25 * Theme.rem
                    }
                    component Separator: Rectangle {
                        Layout.fillHeight: true
                        Layout.leftMargin: 0.25 * Theme.rem
                        Layout.rightMargin: 0.5 * Theme.rem
                        implicitWidth: Ui.hairline
                        color: Theme.neutral80
                    }
                    component Label: Text {
                        color: Theme.neutral05
                        font.family: Theme.font
                        font.pixelSize: Theme.rem
                    }

                    RowLayout {
                        Layout.bottomMargin: 0.5 * Theme.rem
                        spacing: 0

                        Subgroup {
                            Label {
                                text: "View log:"
                            }
                            SelectDropdown {
                                options: ConsoleStore.connectionIds().map(id => ({
                                            label: String(id),
                                            value: id,
                                            selected: !!consoleView.connection && consoleView.connection.connId === id
                                        }))
                                onChanged: (id, selected) => {
                                    if (selected)
                                        ConsoleStore.focusedConnection = ConsoleStore.connections[id];
                                }
                            }
                        }
                        Separator {}
                        Subgroup {
                            Label {
                                text: "Line wrap"
                            }
                            Checkbox {
                                id: lineWrap
                                checked: true
                                onToggled: checked => lineWrap.checked = checked
                            }
                        }
                        Separator {}
                        Subgroup {
                            Field {
                                variant: "pill"
                                fontSize: 0.9 * Theme.rem
                                leftPadding: 0.75 * fontSize + borderWidth
                                rightPadding: 0.75 * fontSize + borderWidth
                                topPadding: 0.25 * fontSize + borderWidth
                                bottomPadding: 0.25 * fontSize + borderWidth
                                placeholderText: "Search log..."
                                onTextChanged: consoleView.searchInput = text
                            }
                        }
                    }

                    RowLayout {
                        Layout.bottomMargin: 0.5 * Theme.rem
                        spacing: 0

                        Flow {
                            Layout.fillWidth: true
                            // the toggles' width in one row, as the Flow's implicit width is
                            // that of the rows it last wrapped them into, which would keep it narrow
                            Layout.maximumWidth: {
                                let width = -spacing;
                                for (const child of children) {
                                    if (child.implicitWidth > 0)
                                        width += child.implicitWidth + spacing;
                                }
                                return Math.max(width, 0);
                            }
                            spacing: 0.25 * Theme.rem

                            Repeater {
                                model: consoleView.visibleLevelsOptions

                                delegate: Item {
                                    id: toggle

                                    required property string modelData
                                    readonly property bool on: !!consoleView.visibleLevels[modelData]

                                    implicitWidth: toggleText.implicitWidth + Theme.rem
                                    implicitHeight: toggleText.implicitHeight + 0.5 * Theme.rem

                                    BoxBackground {
                                        radius: height / 2
                                        border.color: toggle.on ? Theme.primary50 : Theme.neutral70
                                    }

                                    Text {
                                        id: toggleText
                                        anchors.centerIn: parent
                                        text: toggle.modelData
                                        color: toggle.on ? Theme.neutral10 : Theme.neutral20
                                        font.family: Theme.font
                                        font.pixelSize: 0.8 * Theme.rem
                                    }
                                    ClickArea {
                                        radius: toggle.height / 2
                                        onActivated: {
                                            const levels = Object.assign({}, consoleView.visibleLevels);
                                            levels[toggle.modelData] = !toggle.on;
                                            consoleView.visibleLevels = levels;
                                        }
                                    }
                                }
                            }
                        }
                        Separator {}
                        Label {
                            text: "Scope toggles go here in future"
                        }
                    }
                }

                // whether the log is live, or when it was created, at the end of the header
                ColumnLayout {
                    id: status
                    Layout.alignment: Qt.AlignVCenter | Qt.AlignRight
                    Layout.bottomMargin: 0.5 * Theme.rem
                    spacing: 0
                    readonly property bool live: !consoleView.connection || consoleView.connection.status !== "disconnected"
                    readonly property bool connected: !!consoleView.connection && consoleView.connection.status === "connected"

                    SText {
                        Layout.alignment: Qt.AlignHCenter
                        bottomPadding: 0.25 * Theme.rem
                        text: status.live ? Strings.console_.live_log : Strings.console_.log_created_at
                        color: Theme.neutral30
                        font.pixelSize: 0.75 * Theme.rem
                    }

                    Row {
                        visible: status.live
                        Layout.alignment: Qt.AlignLeft
                        spacing: 0.25 * Theme.rem

                        // radial-gradient(45px 45px 45deg, circle cover, ...)
                        Shape {
                            width: 1.25 * Theme.rem
                            height: width
                            preferredRendererType: Shape.CurveRenderer
                            anchors.verticalCenter: parent.verticalCenter

                            ShapePath {
                                strokeWidth: -1
                                strokeColor: "transparent"
                                fillGradient: RadialGradient {
                                    centerX: 45
                                    centerY: 45
                                    centerRadius: Math.hypot(45, 45)
                                    focalX: 45
                                    focalY: 45
                                    GradientStop {
                                        position: 0
                                        color: status.connected ? "lightgreen" : "yellow"
                                    }
                                    GradientStop {
                                        position: status.connected ? 0.95 : 0.97
                                        color: status.connected ? "green" : "red"
                                    }
                                    GradientStop {
                                        position: 1
                                        color: status.connected ? "blue" : "orange"
                                    }
                                }
                                PathAngleArc {
                                    centerX: 0.625 * Theme.rem
                                    centerY: centerX
                                    radiusX: centerX
                                    radiusY: centerX
                                    sweepAngle: 360
                                }
                            }
                        }

                        Text {
                            anchors.verticalCenter: parent.verticalCenter
                            text: status.connected ? Strings.console_.live_log_connected : Strings.console_.live_log_disconnected
                            color: Theme.neutral05
                            font.family: Theme.font
                            font.pixelSize: Theme.rem
                        }
                    }

                    SText {
                        visible: !status.live
                        Layout.alignment: Qt.AlignLeft
                        text: consoleView.connection ? Util.formatDateTime(consoleView.connection.createdTime) : ""
                    }
                }
            }
        }

        // the log
        ListView {
            id: log
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            leftMargin: 0.5 * Theme.rem
            rightMargin: 1.5 * Theme.rem
            topMargin: 0.5 * Theme.rem
            bottomMargin: Theme.rem
            boundsBehavior: Flickable.StopAtBounds
            WheelScroll {
                id: logScroll
            }
            model: shownLines

            readonly property real fontSize: 0.9 * Theme.rem
            readonly property real typeWidth: {
                if (!consoleView.connection)
                    return 0;
                let widest = 0;
                for (const label of consoleView.connection.typeLabels)
                    widest = Math.max(widest, boldMetrics.advanceWidth(`[${label}]`));
                return Math.ceil(widest);
            }
            readonly property real scopeWidth: {
                if (!consoleView.connection)
                    return 0;
                let widest = 0;
                for (const scope of consoleView.connection.scopes)
                    widest = Math.max(widest, italicMetrics.advanceWidth(scope));
                return Math.ceil(widest);
            }

            /// How many characters fit on a line of a message, and how tall the line is, which
            /// estimate the heights of the lines that haven't been laid out.
            readonly property int charsPerLine: Math.max(1, Math.floor((width - leftMargin - rightMargin - typeWidth - scopeWidth - 16) / regularMetrics.advanceWidth("0")))
            readonly property real lineHeight: regularMetrics.height
            onCharsPerLineChanged: Qt.callLater(logBar.reestimate)
            onLineHeightChanged: Qt.callLater(logBar.reestimate)

            FontMetrics {
                id: regularMetrics
                font.family: Theme.fontMonospace
                font.pixelSize: log.fontSize
            }
            FontMetrics {
                id: boldMetrics
                font.family: Theme.fontMonospace
                font.pixelSize: log.fontSize
                font.bold: true
            }
            FontMetrics {
                id: italicMetrics
                font.family: Theme.fontMonospace
                font.pixelSize: log.fontSize
                font.italic: true
            }

            // stay scrolled to the bottom unless the user has scrolled up. Only the user's scrolling
            // decides it, not the view's own changes to contentY, as when it opens at the top.
            // WheelScroll does all of their scrolling but the scroll bar's, and it isn't the view's
            // own movement, which `moving` is of.
            property bool userScrolling: false
            /// Where the content is scrolled to the bottom. It starts at originY, not 0: ListView
            /// moves it as it estimates the size of the lines it hasn't laid out from those it has.
            readonly property real bottomY: Math.max(originY - topMargin, originY + contentHeight + bottomMargin - height)
            function scrollToBottom() {
                // lays out the last lines, which the bottom's position depends on, though it leaves
                // out the bottom margin
                positionViewAtEnd();
                contentY = bottomY;
            }
            onContentYChanged: if (logScroll.scrolling || logBar.pressed)
                userScrolling = contentY < bottomY - 4
            onCountChanged: if (!userScrolling)
                Qt.callLater(scrollToBottom)

            SText {
                visible: !consoleView.connection || consoleView.connection.events.count === 0
                x: log.leftMargin
                text: consoleView.connection ? Strings.console_.launching_log_placeholder : Strings.console_.game_not_running
                color: Theme.neutral20
                font.family: Theme.fontMonospace
                font.pixelSize: log.fontSize
            }

            // over the lines, selecting across them as on the web
            SelectionArea {
                id: logSelection
                parent: log
                content: log.contentItem
                placeOf: edit => edit.place
                textBetween: (start, end) => consoleView.textBetween(start, end)
                lastPlace: log.count * 3 - 1
            }

            ListScrollBar {
                id: logBar
                parent: log
                z: 1
                view: log
                estimate: row => {
                    let lines = 0;
                    for (const length of consoleView.lineLengths(row))
                        lines += Math.max(1, Math.ceil(length / log.charsPerLine));
                    return lines * log.lineHeight + 2;
                }
            }

            delegate: Item {
                id: line

                required property int index
                required property string type
                required property string dataType
                required property string scope
                required property string message

                width: log.width - log.leftMargin - log.rightMargin
                height: Math.max(typeCell.implicitHeight, messageCell.implicitHeight) + 2

                // [TYPE], with the brackets at either end of the column
                Rectangle {
                    id: typeCell
                    width: log.typeWidth
                    implicitHeight: typeText.implicitHeight
                    color: line.dataType === "STARTED" || line.dataType === "CONNECT" ? "green" : line.dataType === "ERROR" || line.dataType === "CRASH" ? "red" : "transparent"

                    // the brackets are ::before and ::after on the web, which can't be selected
                    component Bracket: Text {
                        color: Theme.neutral20
                        font.family: Theme.fontMonospace
                        font.pixelSize: log.fontSize
                        font.bold: true
                    }
                    Bracket {
                        text: "["
                    }
                    Cell {
                        id: typeText
                        place: line.index * 3
                        x: (parent.width - width) / 2
                        text: line.type
                        font.bold: true
                    }
                    Bracket {
                        anchors.right: parent.right
                        text: "]"
                    }
                }

                // selected by the log's SelectionArea, by their place in it
                component Cell: TextEdit {
                    required property int place
                    textFormat: TextEdit.PlainText
                    readOnly: true
                    selectByMouse: false
                    selectionColor: Theme.primary50
                    color: Theme.neutral20
                    font.family: Theme.fontMonospace
                    font.pixelSize: log.fontSize
                }

                Cell {
                    place: line.index * 3 + 1
                    x: typeCell.width + 8
                    width: log.scopeWidth
                    horizontalAlignment: Text.AlignRight
                    text: line.scope
                    font.italic: true
                }

                Cell {
                    id: messageCell
                    place: line.index * 3 + 2
                    x: typeCell.width + 8 + log.scopeWidth + 8
                    width: parent.width - x
                    text: line.message
                    // white-space: pre-wrap with word-break: break-word. (The line wrap checkbox
                    // was never wired up in the web UI.)
                    wrapMode: TextEdit.WrapAtWordBoundaryOrAnywhere
                }
            }
        }
    }
}
