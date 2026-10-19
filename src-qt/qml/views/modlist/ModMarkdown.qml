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

    implicitHeight: result !== undefined && result.markdown ? markdown.implicitHeight : status.implicitHeight

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

    on_VersionChanged: load()
    onEndpointChanged: load()
    Component.onCompleted: load()

    Item {
        id: status
        visible: root.result === undefined || !root.result.markdown
        width: parent.width
        implicitHeight: root.result === undefined && root.error === null ? bar.implicitHeight : message.implicitHeight

        ProgressBar {
            id: bar
            visible: root.result === undefined && root.error === null
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
        visible: root.result !== undefined && !!root.result.markdown
        width: parent.width
        source: visible ? root.result.markdown : ""
    }
}
