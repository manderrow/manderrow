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

    font.family: Theme.font
    font.pixelSize: fontSize
    color: Theme.neutral05
    placeholderTextColor: Theme.alpha(Theme.neutral05, 0.5)
    selectionColor: Theme.primary50
    selectedTextColor: "white"
    leftPadding: 0.5 * fontSize
    rightPadding: 0.5 * fontSize
    topPadding: 0.35 * fontSize
    bottomPadding: 0.35 * fontSize
    selectByMouse: true

    background: Rectangle {
        visible: field.variant !== "bare"
        color: field.bgColor
        border.color: field.borderColor
        border.width: 1
        radius: field.radius
    }
}
