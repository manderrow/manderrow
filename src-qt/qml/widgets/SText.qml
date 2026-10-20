import QtQuick
import Manderrow

// Text that can be selected and copied, as text on the web can. Use Text for labels of controls,
// which aren't selectable on the web, and for text that needs eliding.
TextEdit {
    readOnly: true
    selectByMouse: true
    textFormat: TextEdit.PlainText
    selectionColor: Theme.primary50
    selectedTextColor: "white"
    color: Theme.neutral05
    font.family: Theme.font
    font.pixelSize: Theme.rem

    HoverHandler {
        cursorShape: Qt.IBeamCursor
    }
}
