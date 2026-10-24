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
    property bool iconFilled: false
    property real iconSize: fontSize
    property real iconRotation: 0
    /// Places the icon after the text instead of before it.
    property bool iconTrailing: false
    property string tooltip
    property bool tooltipHideArrow: false
    /// Shows the hover style regardless of the pointer, e.g. while a menu it opened is open.
    property bool forceActive: false
    /// Wraps the label between its words when the button is narrower than it, as web buttons do.
    property bool wrapText: false
    /// Its min-content width: the narrowest it can be without its content overflowing it, with its
    /// label wrapped if it wraps. Browsers keep buttons in flex and grid layouts this wide, so
    /// layouts that stand in for those should too.
    readonly property real minimumWidth: leftPadding + rightPadding + (!contentItem ? 0 : contentItem.minimumWidth !== undefined ? contentItem.minimumWidth : contentItem.implicitWidth)

    property color bgColor: variant === "primary" ? Theme.primary40 : variant === "default" ? Theme.neutral80 : "transparent"
    property color activeBgColor: variant === "primary" ? Theme.primary50 : Theme.neutral70
    property color textColor: Theme.neutral05
    property color activeTextColor: textColor
    property color borderColor: variant === "ghost" ? Theme.neutral70 : "transparent"
    property color activeBorderColor: borderColor
    property real borderWidth: variant === "ghost" ? 1 : 0
    /// The border's width, snapped to whole device pixels.
    readonly property real _borderWidth: Ui.snapBorder(borderWidth)
    /// The corner radius. Defaults to a pill shape.
    property real radius: height / 2
    property real leftRadius: radius
    property real rightRadius: radius
    property int horizontalAlignment: Qt.AlignHCenter

    /// Whether the hover/focus style applies.
    readonly property bool active: enabled && (hovered || visualFocus || forceActive)
    readonly property color currentTextColor: active ? activeTextColor : textColor

    /// Overrides both the horizontal and the vertical padding, inside the border. (Setting
    /// `padding` has no effect, since the horizontal and vertical padding take precedence over it.)
    property var padAll

    hoverEnabled: true
    focusPolicy: Qt.TabFocus
    // an icon's tooltip names it
    Accessible.name: text !== "" ? text : tooltip
    // Enter presses buttons on the web, as well as Space
    Keys.onReturnPressed: control.clicked()
    Keys.onEnterPressed: control.clicked()
    // the border adds to the size, as it does on the web
    horizontalPadding: (padAll !== undefined ? padAll : 1.5 * fontSize) + _borderWidth
    verticalPadding: (padAll !== undefined ? padAll : 0.35 * fontSize) + _borderWidth
    spacing: 0.3 * fontSize
    opacity: enabled ? 1 : 0.5
    font.family: Theme.font
    font.pixelSize: fontSize
    font.weight: fontWeight
    font.variableAxes: ({ "wght": fontWeight })

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
        color: control.active ? Theme.fadeFrom(control.activeBgColor, control.bgColor) : Theme.fadeFrom(control.bgColor, control.activeBgColor)
        border.color: control.active ? Theme.fadeFrom(control.activeBorderColor, control.borderColor) : Theme.fadeFrom(control.borderColor, control.activeBorderColor)
        border.width: control._borderWidth

        Behavior on color {
            SmoothColorAnimation {
                duration: 100
            }
        }

        PixelSnap {}

        FocusRing {
            target: control
            topLeftRadius: control.leftRadius
            bottomLeftRadius: control.leftRadius
            topRightRadius: control.rightRadius
            bottomRightRadius: control.rightRadius
        }
    }

    contentItem: BtnContent {
        btn: control
    }

    // shown while focused from the keyboard too, as corvu's tooltips were
    Tip {
        text: control.tooltip
        hideArrow: control.tooltipHideArrow
        shown: control.hovered || (control.activeFocus && Ui.focusVisible)
    }
}
