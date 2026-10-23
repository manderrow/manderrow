import QtQuick
import QtQuick.Controls.Basic

// Creates a popup when it opens, and destroys it once it has closed, so that popups which are
// seldom shown, such as dialogs and menus, take no memory or time to create until they are. Set
// `sourceComponent` to the popup.
Loader {
    id: loader

    /// The popup, while it exists.
    readonly property Popup popup: item as Popup
    readonly property bool opened: popup !== null && popup.opened

    function open() {
        active = true;
        popup.open();
    }
    function close() {
        if (popup)
            popup.close();
    }

    active: false

    Connections {
        target: loader.popup

        function onClosed() {
            // once the popup is done closing, unless it opened again
            Qt.callLater(() => {
                if (loader.popup && !loader.popup.visible)
                    loader.active = false;
            });
        }
    }
}
