pragma ComponentBehavior: Bound
import QtQuick
import Manderrow

// A tooltip, shown while `shown`. Place it inside the item it describes. The tooltip's popup only
// exists while it shows.
Loader {
    id: tip

    property string text
    property bool hideArrow: false
    property bool shown: false

    active: shown && text !== ""

    sourceComponent: TipPopup {
        parent: tip.parent
        text: tip.text
        hideArrow: tip.hideArrow
        visible: true
    }
}
