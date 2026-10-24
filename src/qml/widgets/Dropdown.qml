import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Effects
import Manderrow

// A button that toggles a floating panel, ported from TogglableDropdown.tsx, which used corvu's
// Popover.
//
// The panel's contents are a component, created each time it opens, and the panel takes their
// implicit size, or as much of it as the window has room for beside the trigger. Contents taller
// than that are given the room there is, so a list among them should fill the height left to it.
//
// While it is open, the panel holds the keyboard focus, as corvu's popovers did:
// - Opening it moves the focus into it. The contents can choose where, with a function
//   `initialFocusItem()` that returns the item to focus, such as a list's selected option.
//   Otherwise, opened from the keyboard, the focus moves to its first control, and opened with the
//   pointer, the panel itself holds the focus until a key moves it, as in a menu.
// - Tab and Shift+Tab move through its controls, and around from the last to the first. The arrow
//   keys move between the items of a list in it (a FocusGroup), and elsewhere in it, through its
//   controls as Tab does.
// - Escape closes it, as does pressing outside it. If the focus is still in it, it goes back to
//   the trigger.
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
    /// The panel's greatest height, which the room in the window can lessen.
    property real panelMaxHeight: Infinity
    property alias trigger: trigger
    readonly property alias opened: panel.opened
    /// The contents of the panel.
    default property Component content

    implicitWidth: trigger.implicitWidth
    implicitHeight: trigger.implicitHeight

    function open() {
        panel.active = true;
        const popup = panel.popup;
        // the popup takes the focus as it opens
        popup.open();
        const contents = (popup.contentItem as Loader).item;
        const initial = contents && typeof contents.initialFocusItem === "function" ? contents.initialFocusItem() : null;
        if (initial)
            initial.forceActiveFocus(Ui.focusVisible ? Qt.TabFocusReason : Qt.PopupFocusReason);
        else if (Ui.focusVisible)
            Ui.focusNavigator.moveTab(true);
    }
    function close() {
        const popup = panel.popup;
        if (popup && popup.activeFocus)
            trigger.forceActiveFocus(Ui.focusVisible ? Qt.TabFocusReason : Qt.PopupFocusReason);
        panel.close();
    }

    /// Handles a key that the panel's contents left unhandled.
    function _handleKey(event) {
        const navigator = Ui.focusNavigator;
        const plain = !(event.modifiers & (Qt.ShiftModifier | Qt.ControlModifier | Qt.AltModifier | Qt.MetaModifier));
        if (event.key === Qt.Key_Escape && plain) {
            close();
            event.accepted = true;
        } else if (!navigator.handleKey(event) && (event.key === Qt.Key_Up || event.key === Qt.Key_Down) && plain) {
            // outside a list, or past its ends
            event.accepted = navigator.moveTab(event.key === Qt.Key_Down);
        }
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
        Accessible.role: Accessible.ButtonMenu
        onClicked: root.opened ? root.close() : root.open()
    }

    PopupLoader {
        id: panel

        sourceComponent: Popup {
            id: popup

            property bool above: false
            /// The room for the panel below and above the trigger, in the window below the title
            /// bar, which the overlay of popups doesn't take presses on.
            property real roomBelow: Infinity
            property real roomAbove: Infinity

            function place() {
                const window = Ui.window;
                if (!window)
                    return;
                const top = trigger.mapToItem(null, 0, 0).y;
                roomBelow = window.height - margins - (top + trigger.height + root.offset);
                roomAbove = top - root.offset - Theme.titlebarHeight - margins;
                // flips above the trigger when there isn't room below it, and there's more above
                above = roomBelow < Math.min(implicitHeight, root.panelMaxHeight) && roomAbove > roomBelow;
            }

            parent: trigger
            focus: true
            padding: 0
            margins: 4
            topPadding: root.panelPadding
            bottomPadding: root.panelPadding
            // the panel's contents handle Escape themselves, to put the focus back on the trigger,
            // unless the popup holds the focus itself
            closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutsideParent
            width: Math.max(root.fillToTriggerWidth ? trigger.width : 0, implicitWidth)
            height: Math.max(0, Math.min(implicitHeight, root.panelMaxHeight, above ? roomAbove : roomBelow))
            x: root.placement === "bottom-start" ? 0 : (trigger.width - width) / 2
            y: above ? -height - root.offset : trigger.height + root.offset

            onAboutToShow: place()

            // the trigger moves as the window resizes, once the layout catches up
            Connections {
                target: Ui.window
                function onWidthChanged() {
                    Qt.callLater(popup.place);
                }
                function onHeightChanged() {
                    Qt.callLater(popup.place);
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
                    border.width: Ui.hairline
                    radius: 4

                    PixelSnap {}
                }
            }

            contentItem: Loader {
                id: panelContent
                clip: true
                // holds the focus within the panel while none of its contents do
                focus: true
                sourceComponent: root.content
                Keys.onPressed: event => root._handleKey(event)
            }
        }
    }
}
