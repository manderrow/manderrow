pragma Singleton
import QtQuick

// References to window-level items, set by Main.qml.
QtObject {
    /// The main window.
    property var window: null
    /// The app content behind dialogs, which their overlay blurs.
    property Item appContent: null
    /// The width of the "0" glyph at the root font size, for widths specified in `ch`.
    property real ch: 8.8
    /// The device pixel ratio of the window's screen.
    property real dpr: 1
    /// The name of the profile being viewed, shown in the title bar.
    property string currentProfileName: ""
    /// A containmentMask for full-window items that leaves out the title bar.
    readonly property QtObject belowTitleBar: QtObject {
        function contains(point: point): bool {
            return point.y >= Theme.titlebarHeight;
        }
    }
    /// How opaque the window's background is, from the appearance settings: 1 unless the window
    /// is translucent. Theme.surface applies it.
    property real surfaceOpacity: 1
    readonly property bool translucent: surfaceOpacity < 1
    /// Whether keyboard focus should be shown, as by CSS's `:focus-visible`: the last input was
    /// from the keyboard rather than a pointer. Set by FocusNavigator.
    property bool focusVisible: false
    /// The window's FocusNavigator.
    property var focusNavigator: null
    /// The WheelScrolls of every scrolling view, which FocusNavigator can scroll with the
    /// keyboard. WheelScrolls add and remove themselves.
    readonly property var scrollers: []
    /// Every FocusGroup, which add and remove themselves.
    readonly property var focusGroups: []

    /// Whether `item` is `ancestor` or inside it. With the window's `activeFocusItem`, this is
    /// CSS's `:focus-within`.
    function isWithin(item, ancestor) {
        for (let i = item; i; i = i.parent) {
            if (i === ancestor)
                return true;
        }
        return false;
    }
}
