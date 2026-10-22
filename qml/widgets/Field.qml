import QtQuick
import QtQuick.Controls.Basic
import Manderrow

// A text input styled like the web UI's inputs.
TextField {
    id: field

    /// "default" (rounded rectangle), "pill", or "bare" (no background or border).
    property string variant: "default"
    property real fontSize: Theme.rem
    property color bgColor: Theme.neutral80
    property color borderColor: Theme.neutral70
    property real radius: variant === "pill" ? height / 2 : 4
    readonly property real borderWidth: variant === "bare" ? 0 : 1

    font.family: Theme.font
    font.pixelSize: fontSize
    color: Theme.neutral05
    // WebKit's default ::placeholder color, darkgray
    placeholderTextColor: "#a9a9a9"
    selectionColor: Theme.primary50
    selectedTextColor: "white"
    // the border adds to the size, as it does on the web
    leftPadding: 0.5 * fontSize + borderWidth
    rightPadding: 0.5 * fontSize + borderWidth
    topPadding: 0.35 * fontSize + borderWidth
    bottomPadding: 0.35 * fontSize + borderWidth
    selectByMouse: true

    background: Rectangle {
        visible: field.variant !== "bare"
        color: field.bgColor
        border.color: field.borderColor
        border.width: field.borderWidth
        radius: field.radius
    }
}
