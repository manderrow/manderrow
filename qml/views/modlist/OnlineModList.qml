import QtQuick
import Manderrow

// The mods available online, ported from OnlineModList in ModList.tsx.
Item {
    id: root

    property string game
    property var installContext

    property bool loading: false
    property var progress: Tasks.initProgress()
    /// Incremented each time the mod index finishes loading, which re-creates the fetcher.
    property int loadedGeneration: 0

    /// Loads the mod index. With `refresh`, it is downloaded again.
    function load(refresh) {
        loading = true;
        progress = Tasks.initProgress();
        return Api.fetchModIndex(game, refresh, event => {
            // the component may have been destroyed by the time this runs
            if (root && event.event === "created")
                root._progressTask = event.task;
        }).then(() => {
            if (!root)
                return;
            root.loading = false;
            root.loadedGeneration++;
        }, e => {
            if (!root)
                return;
            root.loading = false;
            throw e;
        });
    }
    property var _progressTask: null

    property bool _completed: false
    Component.onCompleted: {
        _completed = true;
        load(false).catch(e => Ui.window.reportError(e));
    }
    onGameChanged: if (_completed)
        load(false).catch(e => Ui.window.reportError(e))

    ModListView {
        anchors.fill: parent
        game: root.game
        installContext: root.installContext
        multiselect: false
        isLoading: root.loading
        progress: root._progressTask ? root._progressTask.progress : root.progress
        refresh: () => root.load(true)
        fetcher: {
            if (root.loadedGeneration === 0)
                return null;
            const perPage = 50;
            // the first page comes with the count, which spares searching the mod index twice
            return (game, query, sort) => Api.queryModIndex(game, query, sort, 0, perPage).then(first => ({
                        count: first.count,
                        mods: page => page === 0 ? first.mods : Api.queryModIndex(game, query, sort, page * perPage, perPage).then(result => result.mods),
                        get: id => Api.getFromModIndex(game, [id]).then(mods => mods[0])
                    }));
        }
    }
}
