import QtQuick
import Manderrow

// The mods available online, ported from OnlineModList in ModList.tsx.
Item {
    id: root

    property string game
    property Profile profile: null

    property bool loading: false
    /// Incremented each time the mod index finishes loading, which re-creates the fetcher.
    property int loadedGeneration: 0

    /// Loads the mod index. With `refresh`, it is downloaded again.
    function load(refresh: bool): var {
        loading = true;
        const fetch = ModIndex.fetch(game, refresh);
        _progressTask = Tasks.get(fetch.taskId);
        return Async.promise(fetch).then(() => {
            // the component may have been destroyed by the time this runs
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
    property Task _progressTask: null

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
        profile: root.profile
        multiselect: false
        isLoading: root.loading
        progressTask: root._progressTask
        refresh: () => root.load(true)
        fetcher: {
            if (root.loadedGeneration === 0)
                return null;
            const perPage = 50;
            // the first page comes with the count, which spares searching the mod index twice
            return (game, query, sort) => {
                const columns = sort.map(option => option.column);
                const descending = sort.map(option => option.descending);
                const first = ModIndex.query(game, query, columns, descending, 0, perPage);
                return Async.promise(first).then(mods => ({
                            count: first.value,
                            mods: page => page === 0 ? mods : Async.promise(ModIndex.query(game, query, columns, descending, page * perPage, perPage)),
                            get: id => Async.promise(ModIndex.get(game, id.owner, id.name))
                        }));
            };
        }
    }
}
