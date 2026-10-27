import QtQuick
import Manderrow

// The inline editor for naming a profile, ported from SidebarProfileNameEditor.
Field {
    id: editor

    property string initialValue
    /// Called with the new name. Returns a promise.
    /// Submits the name. It may return a call or a promise.
    property var submit: function (name) {}

    /// The editor is done, whether the name was submitted or not, and can be removed.
    signal closed

    property bool _busy: false
    property bool _done: false

    /// Submits the name, if it isn't empty, then closes the editor. Pressing enter or moving focus
    /// away does this.
    function commit() {
        if (_done || _busy)
            return;
        if (text === "") {
            cancel();
            return;
        }
        _busy = true;
        Async.resolve(submit(text)).then(() => {
            editor._done = true;
            editor.closed();
        }, e => {
            editor._busy = false;
            editor._done = true;
            Ui.window.reportError(e);
            editor.closed();
        });
    }

    /// Closes the editor without submitting the name. Pressing escape does this.
    function cancel() {
        if (_done)
            return;
        _done = true;
        closed();
    }

    text: initialValue
    enabled: !_busy
    // padding: calc(0.35em - 1px) calc(1em - 1px) inside the 1px border
    leftPadding: Theme.rem
    rightPadding: Theme.rem
    topPadding: 0.35 * Theme.rem
    bottomPadding: 0.35 * Theme.rem
    borderColor: Theme.primary80
    // the web UI's input had no type attribute, so the rounded text input style didn't apply
    radius: 0

    Component.onCompleted: {
        forceActiveFocus();
        selectAll();
    }
    onAccepted: commit()
    onActiveFocusChanged: if (!activeFocus)
        commit()
    Keys.onEscapePressed: cancel()
}
