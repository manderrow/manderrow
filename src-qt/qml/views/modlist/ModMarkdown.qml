import QtQuick
import Manderrow

// A mod's README or changelog, ported from ModMarkdown.tsx.
Item {
    id: root

    property var mod
    property var selectedVersion
    /// "readme" or "changelog"
    property string endpoint: "readme"

    property var result: undefined
    property var error: null
    property var _task: null
    property int _generation: 0

    readonly property string _version: mod ? (selectedVersion !== undefined ? selectedVersion : Util.displayVersion(mod).version_number) : ""
    readonly property bool _hasMarkdown: result !== undefined && !!result.markdown
    /// Whether it is being fetched, or the Markdown is still being created, so that it appears
    /// all at once.
    readonly property bool _loading: (result === undefined && error === null) || (_hasMarkdown && !markdown.ready)

    implicitHeight: _hasMarkdown && markdown.ready ? markdown.implicitHeight : status.implicitHeight

    function load() {
        const generation = ++_generation;
        result = undefined;
        error = null;
        _task = null;
        if (!mod)
            return;
        Api.fetchModMarkdown(mod.owner, mod.name, _version, endpoint, event => {
            // the component may have been destroyed by the time this runs
            if (root && event.event === "created")
                root._task = event.task;
        }).then(r => {
            if (root && generation === root._generation)
                root.result = r;
        }, e => {
            if (root && generation === root._generation)
                root.error = e;
        });
    }

    /// What is shown, which is loaded again whenever it changes. Different mods can have the same
    /// version number.
    readonly property string _key: mod ? [mod.owner, mod.name, _version, endpoint].join("/") : ""
    property bool _completed: false
    on_KeyChanged: if (_completed)
        load()
    Component.onCompleted: {
        _completed = true;
        load();
    }

    Item {
        id: status
        visible: !markdown.visible
        width: parent.width
        implicitHeight: root._loading ? bar.implicitHeight : message.implicitHeight

        ProgressBar {
            id: bar
            visible: root._loading
            width: Math.min(parent.width, 12 * Theme.rem)
            progress: root._task ? root._task.progress : undefined
        }

        Text {
            id: message
            visible: !bar.visible
            width: parent.width
            wrapMode: Text.Wrap
            text: root.error !== null ? String(root.error) : I18n.t(`profile.mod_markdown.no_${root.endpoint}_provided`)
            color: Theme.neutral05
            font.family: Theme.font
            font.pixelSize: Theme.rem
        }
    }

    Markdown {
        id: markdown
        visible: root._hasMarkdown && ready
        width: parent.width
        source: root._hasMarkdown ? root.result.markdown : ""
    }
}
