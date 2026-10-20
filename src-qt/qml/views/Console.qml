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

    function isVisible(row) {
        if (row.filter !== "" && !visibleLevels[row.filter])
            return false;
        return !row.searchable || row.message.includes(searchInput);
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // the header
        Item {
            Layout.fillWidth: true
            Layout.rightMargin: 1.5 * Theme.rem
            implicitHeight: headerRow.implicitHeight + 1

            Rectangle {
                anchors.bottom: parent.bottom
                width: parent.width
                height: 1
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
                        implicitWidth: 1
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
                            Layout.maximumWidth: implicitWidth
                            spacing: 0.25 * Theme.rem

                            Repeater {
                                model: consoleView.visibleLevelsOptions

                                delegate: Rectangle {
                                    id: toggle

                                    required property string modelData
                                    readonly property bool on: !!consoleView.visibleLevels[modelData]

                                    implicitWidth: toggleText.implicitWidth + Theme.rem
                                    implicitHeight: toggleText.implicitHeight + 0.5 * Theme.rem
                                    radius: height / 2
                                    color: "transparent"
                                    border.color: on ? Theme.primary50 : Theme.neutral70

                                    Text {
                                        id: toggleText
                                        anchors.centerIn: parent
                                        text: toggle.modelData
                                        color: toggle.on ? Theme.neutral10 : Theme.neutral20
                                        font.family: Theme.font
                                        font.pixelSize: 0.8 * Theme.rem
                                    }
                                    HoverHandler {
                                        cursorShape: Qt.PointingHandCursor
                                    }
                                    TapHandler {
                                        onTapped: {
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

                // whether the log is live, or when it was created
                Column {
                    id: status
                    Layout.alignment: Qt.AlignTop
                    Layout.bottomMargin: 0.5 * Theme.rem
                    readonly property bool live: !consoleView.connection || consoleView.connection.status !== "disconnected"
                    readonly property bool connected: !!consoleView.connection && consoleView.connection.status === "connected"

                    SText {
                        anchors.horizontalCenter: parent.horizontalCenter
                        bottomPadding: 0.25 * Theme.rem
                        text: status.live ? I18n.t("console.live_log") : I18n.t("console.log_created_at")
                        color: Theme.neutral30
                        font.family: Theme.font
                        font.pixelSize: 0.75 * Theme.rem
                    }

                    Row {
                        visible: status.live
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
                            text: status.connected ? I18n.t("console.live_log_connected") : I18n.t("console.live_log_disconnected")
                            color: Theme.neutral05
                            font.family: Theme.font
                            font.pixelSize: Theme.rem
                        }
                    }

                    SText {
                        visible: !status.live
                        text: consoleView.connection ? Util.formatDateTime(consoleView.connection.createdTime) : ""
                        color: Theme.neutral05
                        font.family: Theme.font
                        font.pixelSize: Theme.rem
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
            ScrollBar.vertical: AppScrollBar {
                policy: ScrollBar.AlwaysOn
            }
            WheelScroll {}
            model: consoleView.connection ? consoleView.connection.events : null

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

            // stay scrolled to the bottom unless the user has scrolled up
            property bool userScrolling: false
            onMovementEnded: userScrolling = contentHeight - height > contentY + 4
            onContentYChanged: if (moving || dragging || flicking)
                userScrolling = contentHeight - height > contentY + 4
            onCountChanged: if (!userScrolling)
                Qt.callLater(positionViewAtEnd)

            SText {
                visible: !consoleView.connection || log.count === 0
                x: log.leftMargin
                text: consoleView.connection ? I18n.t("console.launching_log_placeholder") : I18n.t("console.game_not_running")
                color: Theme.neutral20
                font.family: Theme.font
                font.pixelSize: Theme.rem
            }

            delegate: Item {
                id: line

                required property string type
                required property string dataType
                required property string scope
                required property string message
                required property string filter
                required property bool searchable
                readonly property bool shown: consoleView.isVisible({ filter, searchable, message })

                width: log.width - log.leftMargin - log.rightMargin
                height: shown ? Math.max(typeCell.implicitHeight, messageCell.implicitHeight) + 2 : 0
                visible: shown

                // [TYPE], with the brackets at either end of the column
                Rectangle {
                    id: typeCell
                    width: log.typeWidth
                    implicitHeight: typeText.implicitHeight
                    color: line.dataType === "STARTED" || line.dataType === "CONNECT" ? "green" : line.dataType === "ERROR" || line.dataType === "CRASH" ? "red" : "transparent"

                    component Bracket: Text {
                        color: Theme.neutral20
                        font.family: Theme.fontMonospace
                        font.pixelSize: log.fontSize
                        font.bold: true
                    }
                    Bracket {
                        text: "["
                    }
                    Bracket {
                        id: typeText
                        x: (parent.width - width) / 2
                        text: line.type
                    }
                    Bracket {
                        anchors.right: parent.right
                        text: "]"
                    }
                }

                Text {
                    x: typeCell.width + 8
                    width: log.scopeWidth
                    horizontalAlignment: Text.AlignRight
                    text: line.scope
                    color: Theme.neutral20
                    font.family: Theme.fontMonospace
                    font.pixelSize: log.fontSize
                    font.italic: true
                }

                TextEdit {
                    id: messageCell
                    x: typeCell.width + 8 + log.scopeWidth + 8
                    width: parent.width - x
                    text: line.message
                    textFormat: TextEdit.PlainText
                    // white-space: pre-wrap with word-break: break-word. (The line wrap checkbox
                    // was never wired up in the web UI.)
                    wrapMode: TextEdit.WrapAtWordBoundaryOrAnywhere
                    readOnly: true
                    selectByMouse: true
                    selectionColor: Theme.primary50
                    color: Theme.neutral20
                    font.family: Theme.fontMonospace
                    font.pixelSize: log.fontSize
                }
            }
        }
    }
}
