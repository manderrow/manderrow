pragma Singleton
import QtQuick

// In-app navigation, replacing @solidjs/router. Routes:
//   /profile/:gameId?/:profileId?
//   /settings
//   anything else is a 404
QtObject {
    id: router

    property string path: "/profile"
    /// Search parameters of the current route, such as the selected tab of each tab list.
    property var search: ({})

    property var _history: []

    readonly property var _segments: path.split("/").filter(s => s !== "")

    readonly property string route: {
        if (_segments.length === 0)
            return "notFound";
        if (_segments[0] === "profile" && _segments.length <= 3)
            return "profile";
        if (_segments[0] === "settings" && _segments.length === 1)
            return "settings";
        return "notFound";
    }

    readonly property var gameId: route === "profile" && _segments.length > 1 ? _segments[1] : undefined
    readonly property var profileId: route === "profile" && _segments.length > 2 ? _segments[2] : undefined

    /// Navigates to `to`, which is a path, or a negative number to go back in history.
    function navigate(to, options) {
        if (typeof to === "number") {
            for (let i = 0; i < -to && _history.length > 0; i++) {
                const entry = _history.pop();
                path = entry.path;
                search = entry.search;
            }
            return;
        }
        if (!(options && options.replace))
            _history.push({ path, search });
        search = {};
        path = to;
    }

    /// Updates search parameters in place, without adding a history entry.
    function setSearch(params) {
        search = Object.assign({}, search, params);
    }
}
