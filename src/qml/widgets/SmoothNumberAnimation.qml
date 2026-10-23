import QtQuick
import Manderrow

// A NumberAnimation timed like a CSS transition: `ease` by default, or another of Theme's timing
// functions.
NumberAnimation {
    /// The timing function, such as Theme.easeInOut.
    property var curve: Theme.ease

    easing.type: Easing.Bezier
    easing.bezierCurve: curve
}
