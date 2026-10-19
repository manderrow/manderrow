pragma Singleton
import QtQuick
import Manderrow

// Data that the app loads at startup, ported from globals.ts and api/settings.ts.
QtObject {
    id: globals

    property var games: []
    property var gamesById: ({})
    property var initialSortedGames: []
    property var gamesPopularity: ({})
    property var gamesModDownloads: ({})
    property var profiles: []
    property var settings: null
    property var settingsUI: null
    /// The default game from the settings when the app started, if any.
    property var initialGame: null

    /// Whether every core resource has loaded. The splashscreen is shown until then.
    property bool loaded: false
    /// The first error encountered while loading core resources.
    property var error: null

    function _sortProfiles(profiles) {
        profiles.sort((a, b) => a.name.localeCompare(b.name));
        return profiles;
    }

    function load() {
        const resources = [
            Api.getSettings().then(settings => {
                globals.settings = settings;
                globals.initialGame = settings.defaultGame.value;
            }),
            Api.getSettingsUI().then(ui => globals.settingsUI = ui),
            Api.getGames().then(games => {
                const byId = {};
                for (const game of games)
                    byId[game.id] = game;
                globals.gamesById = byId;
                globals.games = games;
            }),
            Api.searchGames("", [{ column: Api.gameSortColumn.modDownloads, descending: true }]).then(sorted => globals.initialSortedGames = sorted),
            Api.getGamesPopularity().then(popularity => globals.gamesPopularity = popularity),
            Api.getGameModDownloads().then(downloads => globals.gamesModDownloads = downloads),
            Api.getProfiles().then(profiles => globals.profiles = globals._sortProfiles(profiles))
        ];
        Promise.all(resources).then(() => globals.loaded = true, e => {
            if (globals.error === null)
                globals.error = e;
        });
    }

    function refetchProfiles() {
        return Api.getProfiles().then(profiles => {
            globals.profiles = globals._sortProfiles(profiles);
        });
    }

    readonly property var _subscriptions: [
        Api.listen("settings", settings => globals.settings = settings)
    ]
}
