import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Effects
import Manderrow

// A button that toggles a floating panel, ported from TogglableDropdown.tsx.
Item {
    id: root

    property string label
    property bool hideCaret: false
    /// Where the panel is placed relative to the trigger: "bottom" (centered) or "bottom-start".
    property string placement: "bottom"
    property real offset: 6
    property bool fillToTriggerWidth: false
    /// Vertical padding inside the panel.
    property real panelPadding: 0.5 * Theme.rem
    property real panelMaxHeight: Ui.window ? Ui.window.height * 0.5 : 400
    property alias trigger: trigger
    property alias panel: popup
    readonly property alias opened: popup.opened
    /// The contents of the panel.
    default property alias content: panelContent.data
    /// The panel content's implicit size, which the panel is sized to.
    property real contentWidth: panelContent.childrenRect.width
    property real contentHeight: panelContent.childrenRect.height

    signal aboutToShow

    implicitWidth: trigger.implicitWidth
    implicitHeight: trigger.implicitHeight

    function open() {
        popup.open();
    }
    function close() {
        popup.close();
    }

    Btn {
        id: trigger
        anchors.fill: parent
        text: root.label
        iconName: root.hideCaret ? "" : "caretDown"
        iconRotation: popup.opened ? 180 : 0
        horizontalPadding: 0.75 * fontSize
        verticalPadding: 0.25 * fontSize
        spacing: 0.35 * fontSize
        forceActive: popup.opened
        onClicked: popup.opened ? popup.close() : popup.open()
    }

    Popup {
        id: popup

        property bool above: false

        parent: trigger
        padding: 0
        margins: 4
        topPadding: root.panelPadding
        bottomPadding: root.panelPadding
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutsideParent
        width: Math.max(root.fillToTriggerWidth ? trigger.width : 0, root.contentWidth)
        height: Math.min(root.contentHeight + topPadding + bottomPadding, root.panelMaxHeight)
        x: root.placement === "bottom-start" ? 0 : (trigger.width - width) / 2
        y: above ? -height - root.offset : trigger.height + root.offset

        onAboutToShow: {
            root.aboutToShow();
            // flip above the trigger when there isn't room below it
            const window = Ui.window;
            if (window) {
                const top = trigger.mapToItem(null, 0, 0).y;
                const below = window.height - (top + trigger.height + root.offset);
                above = below < height && top - root.offset > below;
            }
        }

        enter: Transition {
            NumberAnimation {
                property: "opacity"
                from: 0
                to: 1
                duration: 100
                easing.type: Easing.Bezier
                easing.bezierCurve: Theme.ease
            }
        }
        exit: null

        background: Item {
            PopupInputBlocker {}

            RectangularShadow {
                anchors.fill: panelBg
                radius: panelBg.radius
                blur: 1.5 * Theme.rem
                color: Qt.rgba(0, 0, 0, 0.3)
            }

            Rectangle {
                id: panelBg
                anchors.fill: parent
                color: Theme.neutral80
                border.color: Theme.neutral70
                radius: 4
            }
        }

        contentItem: Item {
            id: panelContent
            clip: true
        }
    }
}
