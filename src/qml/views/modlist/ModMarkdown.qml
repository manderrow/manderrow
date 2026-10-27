import QtQuick
import Manderrow

// A mod's README or changelog, ported from ModMarkdown.tsx.
Item {
    id: root

    property Mod mod: null
    /// The version whose Markdown is shown, or empty for the mod's `displayVersion`.
    property string selectedVersion
    /// "readme" or "changelog"
    property string endpoint: "readme"
    /// Space on the Markdown's right, which the messages shown in its place don't leave.
    property real rightPadding: 0
    /// How far the Markdown reaches to the right, past this where some of it can't wrap to fit.
    readonly property real overflowWidth: markdown.visible ? markdown.overflowWidth : width

    /// The Markdown, once it's been fetched, which is empty if there's none.
    property var result: undefined
    property var error: null
    property Task _task: null
    property int _generation: 0

    readonly property string _version: mod ? (selectedVersion !== "" ? selectedVersion : mod.displayVersion.versionNumber) : ""
    readonly property bool _hasMarkdown: result !== undefined && result !== ""
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
        const fetch = endpoint === "changelog" ? mod.changelog(_version) : mod.readme(_version);
        _task = Tasks.get(fetch.taskId);
        Async.promise(fetch).then(r => {
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
            completed: root._task ? root._task.completed : 0
            total: root._task ? root._task.total : 0
        }

        Text {
            id: message
            visible: !bar.visible
            width: parent.width
            wrapMode: Text.Wrap
            text: root.error !== null ? String(root.error) : root.endpoint === "changelog" ? Strings.profile.mod_markdown.no_changelog_provided : Strings.profile.mod_markdown.no_readme_provided
            color: Theme.neutral05
            font.family: Theme.font
            font.pixelSize: Theme.rem
        }
    }

    Markdown {
        id: markdown
        visible: root._hasMarkdown && ready
        width: parent.width - root.rightPadding
        source: root._hasMarkdown ? root.result : ""
    }
}
