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
        MultiEffect {
            y: Theme.titlebarHeight
            width: parent.width
            height: parent.height - y
            source: Ui.appContent
            blurEnabled: true
            blur: 1
            blurMax: 4
        }
        Rectangle {
            y: Theme.titlebarHeight
            width: parent.width
            height: parent.height - y
            color: Qt.rgba(0, 0, 0, 0.25)
        }
    }

    background: Item {
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

    // scrolls when the dialog is taller than the window
    contentItem: Flickable {
        implicitWidth: bodyItem.implicitWidth
        implicitHeight: bodyItem.implicitHeight
        contentWidth: width
        contentHeight: bodyItem.implicitHeight
        interactive: contentHeight > height
        acceptedButtons: Qt.NoButton
        boundsBehavior: Flickable.StopAtBounds
        clip: interactive
        ScrollBar.vertical: AppScrollBar {}

        Item {
            id: bodyItem
            width: parent.width
            height: Math.max(implicitHeight, parent.height)
            implicitWidth: childrenRect.width
            implicitHeight: childrenRect.height
        }
    }

    Btn {
        visible: !dialog.hideCloseBtn
        parent: dialog.background
        anchors.top: parent.top
        anchors.right: parent.right
        variant: "plain"
        iconName: "xmark"
        fontSize: 1.5 * Theme.rem
        padAll: 0.25 * fontSize
        width: height
        onClicked: dialog.close()
    }
}
