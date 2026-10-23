import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Effects
import Manderrow

// A button that toggles a floating panel, ported from TogglableDropdown.tsx.
//
// The panel's contents are a component, created each time it opens, and the panel takes their
// implicit size.
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
    readonly property alias opened: panel.opened
    /// The contents of the panel.
    default property Component content

    implicitWidth: trigger.implicitWidth
    implicitHeight: trigger.implicitHeight

    function open() {
        panel.active = true;
        // the panel takes the focus when opened from the keyboard, so the keyboard can reach its
        // contents, but leaves it where it is for the pointer
        panel.popup.focus = Ui.focusVisible;
        panel.popup.open();
    }
    function close() {
        panel.close();
    }

    Btn {
        id: trigger
        anchors.fill: parent
        text: root.label
        iconName: root.hideCaret ? "" : "chevron-down"
        iconRotation: root.opened ? 180 : 0
        horizontalPadding: 0.75 * fontSize
        verticalPadding: 0.25 * fontSize
        spacing: 0.35 * fontSize
        forceActive: root.opened
        onClicked: root.opened ? root.close() : root.open()
    }

    PopupLoader {
        id: panel

        sourceComponent: Popup {
            id: popup

            property bool above: false

            parent: trigger
            padding: 0
            margins: 4
            topPadding: root.panelPadding
            bottomPadding: root.panelPadding
            closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutsideParent
            width: Math.max(root.fillToTriggerWidth ? trigger.width : 0, implicitWidth)
            height: Math.min(implicitHeight, root.panelMaxHeight)
            x: root.placement === "bottom-start" ? 0 : (trigger.width - width) / 2
            y: above ? -height - root.offset : trigger.height + root.offset

            onAboutToShow: {
                // flip above the trigger when there isn't room below it
                const window = Ui.window;
                if (window) {
                    const top = trigger.mapToItem(null, 0, 0).y;
                    const below = window.height - (top + trigger.height + root.offset);
                    above = below < height && top - root.offset > below;
                }
            }

            enter: Transition {
                SmoothNumberAnimation {
                    property: "opacity"
                    from: 0
                    to: 1
                    duration: 100
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

            contentItem: Loader {
                id: panelContent
                clip: true
                focus: true
                sourceComponent: root.content
                // the arrow keys move through the panel's items, as in a menu
                Keys.onUpPressed: event => event.accepted = Ui.focusNavigator.moveFocusWithin(panelContent, false)
                Keys.onDownPressed: event => event.accepted = Ui.focusNavigator.moveFocusWithin(panelContent, true)
            }
        }
    }
}
