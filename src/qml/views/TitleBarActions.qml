import QtQuick
import QtQuick.Layouts
import Manderrow

// The title bar's buttons and dropdowns, which follow the app's name whatever the window's state.
// Its children are laid out in a row, in order, and stretched to its height: TitleBarButtons,
// TitleBarDropdowns, or any other item with `Layout.fillHeight`. TitleBar declares them.
Item {
    id: actions

    default property alias actions: row.data
    property real spacing: 0.25 * Theme.rem

    implicitWidth: row.implicitWidth
    // when the window is too narrow for every action
    clip: true

    RowLayout {
        id: row
        height: parent.height
        spacing: actions.spacing
    }
}
