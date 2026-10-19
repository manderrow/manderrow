import QtQuick
import Manderrow

// A button standing in for a failed action that opens the error's details, ported from
// ErrorIndicator in ErrorDialog.tsx.
Btn {
    id: indicator

    property var err
    /// Called when the user dismisses the error.
    property var reset: function () {}
    property bool showIcon: true
    property string message: "Error"

    text: message
    iconName: showIcon ? "circleExclamation" : ""
    spacing: 8
    onClicked: errorDialog.open()

    ErrorDialog {
        id: errorDialog
        err: indicator.err
        reset: () => indicator.reset()
    }
}
