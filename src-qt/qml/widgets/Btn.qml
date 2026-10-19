import QtQuick
import QtQuick.Controls.Basic
import Manderrow

// A button styled like the web UI's buttons (App.css). By default it is a pill with a neutral
// background; `variant` selects the other common styles.
AbstractButton {
    id: control

    /// "default", "primary", "ghost" (outlined), or "plain" (no background until hovered).
    property string variant: "default"
    property real fontSize: Theme.rem
    property int fontWeight: Font.Normal
    property string iconName
    property real iconSize: fontSize
    property real iconRotation: 0
    /// Places the icon after the text instead of before it.
    property bool iconTrailing: false
    property string tooltip
    property bool tooltipHideArrow: false
    /// Shows the hover style regardless of the pointer, e.g. while a menu it opened is open.
    property bool forceActive: false

    property color bgColor: variant === "primary" ? Theme.primary40 : variant === "default" ? Theme.neutral80 : "transparent"
    property color activeBgColor: variant === "primary" ? Theme.primary50 : Theme.neutral70
    property color textColor: Theme.neutral05
    property color activeTextColor: textColor
    property color borderColor: variant === "ghost" ? Theme.neutral70 : "transparent"
    property color activeBorderColor: borderColor
    property real borderWidth: variant === "ghost" ? 1 : 0
    /// The corner radius. Defaults to a pill shape.
    property real radius: height / 2
    property real leftRadius: radius
    property real rightRadius: radius
    property int horizontalAlignment: Qt.AlignHCenter

    /// Whether the hover/focus style applies.
    readonly property bool active: enabled && (hovered || visualFocus || forceActive)
    readonly property color currentTextColor: active ? activeTextColor : textColor

    /// Overrides both the horizontal and the vertical padding. (Setting `padding` has no effect,
    /// since the horizontal and vertical padding take precedence over it.)
    property var padAll

    hoverEnabled: true
    focusPolicy: Qt.TabFocus
    horizontalPadding: padAll !== undefined ? padAll : 1.5 * fontSize
    verticalPadding: padAll !== undefined ? padAll : 0.35 * fontSize
    spacing: 0.3 * fontSize
    opacity: enabled ? 1 : 0.5
    font.family: Theme.font
    font.pixelSize: fontSize
    font.weight: fontWeight

    HoverHandler {
        cursorShape: control.enabled ? Qt.PointingHandCursor : Qt.ForbiddenCursor
    }

    background: Rectangle {
        implicitWidth: 0
        implicitHeight: 0
        topLeftRadius: control.leftRadius
        bottomLeftRadius: control.leftRadius
        topRightRadius: control.rightRadius
        bottomRightRadius: control.rightRadius
        color: control.active ? control.activeBgColor : control.bgColor
        border.color: control.active ? control.activeBorderColor : control.borderColor
        border.width: control.borderWidth

        Behavior on color {
            ColorAnimation {
                duration: 100
            }
        }

        // focus-visible outline
        Rectangle {
            anchors.fill: parent
            topLeftRadius: control.leftRadius
            bottomLeftRadius: control.leftRadius
            topRightRadius: control.rightRadius
            bottomRightRadius: control.rightRadius
            color: "transparent"
            border.color: Theme.warn5
            border.width: 2
            visible: control.visualFocus
        }
    }

    contentItem: BtnContent {
        btn: control
    }

    Tip {
        text: control.tooltip
        hideArrow: control.tooltipHideArrow
        visible: control.tooltip !== "" && control.hovered
    }
}
