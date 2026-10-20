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
}
