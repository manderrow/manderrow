import QtQuick
import QtQuick.Layouts
import Manderrow

// A dropdown in TitleBarActions, whose trigger is styled like a TitleBarButton. Its panel's
// contents are its children, as for a Dropdown.
Dropdown {
    Layout.fillHeight: true
    placement: "bottom-start"
    offset: 4
    trigger.variant: "plain"
    trigger.fontSize: 0.8 * Theme.rem
    trigger.iconSize: 0.8 * Theme.rem
    trigger.iconTrailing: true
    trigger.radius: 4
    trigger.horizontalPadding: 0.5 * Theme.rem
    trigger.verticalPadding: 0
    trigger.textColor: Theme.neutral30
    trigger.activeTextColor: Theme.neutral05
    trigger.activeBgColor: Theme.neutral80
}
