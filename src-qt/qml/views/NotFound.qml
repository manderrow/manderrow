import QtQuick
import QtQuick.Layouts
import Manderrow

// The 404 page, ported from views/error/Error.tsx.
Item {
    Rectangle {
        anchors.fill: parent
        color: Theme.surface(Theme.neutral85)
    }

    ColumnLayout {
        anchors.horizontalCenter: parent.horizontalCenter
        y: Theme.rem
        spacing: 0

        SText {
            Layout.alignment: Qt.AlignHCenter
            text: "404"
            color: Theme.neutral20
            font.family: Theme.font
            font.pixelSize: 12 * Theme.rem
            font.weight: Font.ExtraLight
        }
        SText {
            Layout.alignment: Qt.AlignHCenter
            text: Router.path
            color: Theme.neutral30
            font.family: Theme.font
            font.pixelSize: Theme.rem
        }
        SText {
            Layout.alignment: Qt.AlignHCenter
            Layout.topMargin: 0.5 * Theme.rem
            Layout.bottomMargin: 0.5 * Theme.rem
            text: I18n.t("404_page.subtitle")
            color: Theme.neutral05
            font.family: Theme.font
            font.pixelSize: Theme.rem
        }
        Btn {
            Layout.alignment: Qt.AlignHCenter
            text: I18n.t("404_page.home_btn")
            onClicked: Router.navigate("/profile/")
        }
    }
}
