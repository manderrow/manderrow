import QtQuick
import QtQuick.Layouts
import Manderrow

// A button in TitleBarActions: transparent with faded text, until it is hovered.
Btn {
    Layout.fillHeight: true
    variant: "plain"
    fontSize: 0.8 * Theme.rem
    radius: 4
    horizontalPadding: 0.5 * Theme.rem
    verticalPadding: 0
    textColor: Theme.neutral30
    activeTextColor: Theme.neutral05
    activeBgColor: Theme.neutral80
}
