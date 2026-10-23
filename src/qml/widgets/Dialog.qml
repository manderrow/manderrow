import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Effects
import Manderrow

// A modal dialog, ported from Dialog.tsx (corvu dialogs). The backdrop dims and blurs the app.
Popup {
    id: dialog

    property bool hideCloseBtn: false
    /// The default card style: a padded, rounded panel with a shadow.
    property bool defaultStyle: true
    property real maxWidth: 75 * Ui.ch
    property color backgroundColor: Theme.neutral80
    property real cornerRadius: 0.5 * Theme.rem
    /// The dialog's contents.
    default property alias body: bodyItem.data
    readonly property alias bodyItem: bodyItem

    signal dismissed

    parent: Overlay.overlay
    // centered in the window below the title bar, which dialogs leave uncovered
    readonly property real _areaHeight: parent ? parent.height - Theme.titlebarHeight : implicitHeight
    x: parent ? (parent.width - width) / 2 : 0
    y: Theme.titlebarHeight + (_areaHeight - height) / 2
    width: Math.min(parent ? parent.width : maxWidth, maxWidth)
    height: Math.min(implicitHeight, _areaHeight)
    modal: true
    focus: true
    padding: defaultStyle ? Theme.rem : 0
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
    onClosed: dismissed()

    // the backdrop: rgb(0 0 0 / 0.25) with blur(2px), below the title bar like the web UI's
    Overlay.modal: Item {
        // it doesn't take the title bar's input, which isn't covered
        containmentMask: Ui.belowTitleBar

        BackdropBlur {
            y: Theme.titlebarHeight
            width: parent.width
            height: parent.height - y
            radius: 2
            quality: BackdropBlur.Gaussian
            // it covers all of the app, so the app needn't be drawn sharp beneath it
            hidesSource: true
            tint: Qt.rgba(0, 0, 0, 0.25)

            // the app behind isn't hovered or scrolled; pressing here closes the dialog
            PopupInputBlocker {
                blockPresses: false
            }
        }
    }

    background: Item {
        PopupInputBlocker {}

        RectangularShadow {
            visible: dialog.defaultStyle
            anchors.fill: bg
            radius: bg.radius
            blur: 1.25 * Theme.rem
            spread: -0.25 * Theme.rem
            color: Qt.rgba(0, 0, 0, 0.75)
        }

        Rectangle {
            id: bg
            anchors.fill: parent
            color: dialog.backgroundColor
            radius: dialog.cornerRadius
        }
    }

    contentItem: Item {
        implicitWidth: scroll.implicitWidth
        implicitHeight: scroll.implicitHeight
        // takes the dialog's focus, so that the keys its body leaves unhandled can scroll it
        focus: true
        Keys.onPressed: event => {
            if (Ui.focusNavigator)
                Ui.focusNavigator.handleKey(event);
        }

        // scrolls when the dialog is taller than the window
        Flickable {
            id: scroll
            anchors.fill: parent
            implicitWidth: bodyItem.implicitWidth
            implicitHeight: bodyItem.implicitHeight
            contentWidth: width
            contentHeight: bodyItem.implicitHeight
            interactive: contentHeight > height
            boundsBehavior: Flickable.StopAtBounds
            clip: contentHeight > height
            ScrollBar.vertical: AppScrollBar {}
            WheelScroll {}

            Item {
                id: bodyItem
                // the Flickable's own size, as the parent is its content item, which is sized to
                // the body
                width: scroll.width
                height: Math.max(implicitHeight, scroll.height)
                implicitWidth: childrenRect.width
                implicitHeight: childrenRect.height
            }
        }

        // in the dialog's corner, outside the padding, and over the body. It is in the popup's own
        // item, which the padding is part of, as Qt can skip delivering the pointer to items
        // outside their parent.
        Btn {
            parent: dialog.contentItem.parent
            z: 1
            visible: !dialog.hideCloseBtn
            anchors.top: parent ? parent.top : undefined
            anchors.right: parent ? parent.right : undefined
            variant: "plain"
            iconName: "x"
            fontSize: 1.5 * Theme.rem
            padAll: 0.25 * fontSize
            width: height
            onClicked: dialog.close()
        }
    }
}
