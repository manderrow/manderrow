import QtQuick
import Manderrow

// The inline editor for naming a profile, ported from SidebarProfileNameEditor.
Field {
    id: editor

    property string initialValue
    /// Called with the new name. Returns a promise.
    property var submit: function (name) {}

    signal cancelled

    property bool _busy: false
    property bool _done: false

    function _submit() {
        if (_done || _busy)
            return;
        if (text === "") {
            _done = true;
            cancelled();
            return;
        }
        _busy = true;
        Promise.resolve(submit(text)).then(() => {
            editor._done = true;
            editor.cancelled(); // close editor after submitting
        }, e => {
            editor._busy = false;
            editor._done = true;
            Ui.window.reportError(e);
            editor.cancelled();
        });
    }

    text: initialValue
    enabled: !_busy
    leftPadding: Theme.rem - 1
    rightPadding: Theme.rem - 1
    topPadding: 0.35 * Theme.rem - 1
    bottomPadding: 0.35 * Theme.rem - 1
    borderColor: Theme.primary80
    // the web UI's input had no type attribute, so the rounded text input style didn't apply
    radius: 0

    Component.onCompleted: {
        forceActiveFocus();
        selectAll();
    }
    onAccepted: _submit()
    onActiveFocusChanged: if (!activeFocus)
        _submit()
    Keys.onEscapePressed: {
        _done = true;
        cancelled();
    }
}
