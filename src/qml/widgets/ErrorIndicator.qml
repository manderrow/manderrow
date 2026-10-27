pragma ComponentBehavior: Bound
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
    iconName: showIcon ? "circle-alert" : ""
    iconFilled: true
    spacing: 8
    onClicked: errorDialog.open()

    PopupLoader {
        id: errorDialog
        sourceComponent: ErrorDialog {
            err: indicator.err
            reset: () => indicator.reset()
        }
    }
}
