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
    anchors.centerIn: parent
    width: Math.min(parent ? parent.width : maxWidth, maxWidth)
    height: Math.min(implicitHeight, parent ? parent.height : implicitHeight)
    modal: true
    focus: true
    padding: defaultStyle ? Theme.rem : 0
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
    onClosed: dismissed()

    // the backdrop: rgb(0 0 0 / 0.25) with blur(2px), below the title bar like the web UI's
    Overlay.modal: Item {
        BackdropBlur {
            y: Theme.titlebarHeight
            width: parent.width
            height: parent.height - y
            radius: 2
            quality: BackdropBlur.Gaussian
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
                width: parent.width
                height: Math.max(implicitHeight, parent.height)
                implicitWidth: childrenRect.width
                implicitHeight: childrenRect.height
            }
        }

        // in the dialog's corner, outside the padding, and over the body
        Btn {
            visible: !dialog.hideCloseBtn
            anchors.top: parent.top
            anchors.right: parent.right
            anchors.topMargin: -dialog.topPadding
            anchors.rightMargin: -dialog.rightPadding
            variant: "plain"
            iconName: "xmark"
            fontSize: 1.5 * Theme.rem
            padAll: 0.25 * fontSize
            width: height
            onClicked: dialog.close()
        }
    }
}
