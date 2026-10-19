import QtQuick
import Manderrow

// The select and set-default buttons of a game on the game selection screen.
Grid {
    id: actions

    property bool horizontal: false
    /// Sets the game as the default and selects it. Returns a promise.
    property var setDefault: function () {}

    signal selected

    columns: horizontal ? 2 : 1
    spacing: 0.5 * Theme.rem
    horizontalItemAlignment: Grid.AlignHCenter
    verticalItemAlignment: Grid.AlignVCenter

    Btn {
        text: I18n.t("game_select.select_btn")
        fontWeight: Font.DemiBold
        bgColor: Qt.rgba(245 / 255, 245 / 255, 245 / 255, 1)
        activeBgColor: Qt.rgba(215 / 255, 215 / 255, 215 / 255, 1)
        textColor: "black"
        onClicked: actions.selected()
    }

    AsyncButton {
        text: I18n.t("game_select.set_default_btn")
        btn.bgColor: Qt.rgba(225 / 255, 225 / 255, 225 / 255, 0.2)
        btn.activeBgColor: Qt.rgba(225 / 255, 225 / 255, 225 / 255, 0.3)
        btn.borderColor: Qt.rgba(225 / 255, 225 / 255, 225 / 255, 0.3)
        btn.borderWidth: 2
        action: () => actions.setDefault()
    }
}
