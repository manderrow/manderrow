import QtQuick
import Manderrow

// A ColorAnimation timed like a CSS transition: `ease` by default, or another of Theme's timing
// functions.
ColorAnimation {
    /// The timing function, such as Theme.easeInOut.
    property var curve: Theme.ease

    easing.type: Easing.Bezier
    easing.bezierCurve: curve
}
