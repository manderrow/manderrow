pragma Singleton
import QtQuick
import Manderrow

// The bridge to the Rust backend, ported from api/*.ts.
QtObject {
    id: api

    property var _pending: ({})
    property var _listeners: ({})
    property var _channels: ({})
    property int _nextChannelId: 0

    readonly property Connections _connections: Connections {
        target: Backend

        function onInvokeFinished(callId, ok, payload) {
            const pending = api._pending[callId];
            delete api._pending[callId];
            if (pending === undefined)
                return;
            const value = payload === "" ? undefined : JSON.parse(payload);
            if (ok)
                pending.resolve(value);
            else
                pending.reject(value);
        }

        function onBackendEvent(name, payload) {
            const value = JSON.parse(payload);
            if (name === "channel") {
                const channel = api._channels[value.id];
                if (channel !== undefined)
                    channel(value.payload);
                return;
            }
            const listeners = api._listeners[name];
            if (listeners === undefined)
                return;
            for (const listener of listeners.slice()) {
                try {
                    listener(value);
                } catch (e) {
                    console.error(`Error in ${name} listener:`, e, e.stack);
                }
            }
        }
    }

    /// Subscribes to a backend event. Returns a function that unsubscribes.
    function listen(name, listener) {
        const listeners = _listeners[name] || (_listeners[name] = []);
        listeners.push(listener);
        return () => {
            const i = listeners.indexOf(listener);
            if (i !== -1)
                listeners.splice(i, 1);
        };
    }

    /// Creates a channel that receives messages while a command runs. Returns its id.
    function createChannel(onMessage) {
        const id = _nextChannelId++;
        _channels[id] = onMessage;
        return id;
    }

    function closeChannel(id) {
        delete _channels[id];
    }

    function rawInvoke(command, args) {
        return new Promise((resolve, reject) => {
            const callId = Backend.invoke(command, JSON.stringify(args === undefined ? {} : args));
            api._pending[callId] = { resolve, reject };
        });
    }

    /// An error thrown from native code, with every message in its cause chain and a backtrace.
    function nativeError(messages, backtrace) {
        const e = new Error(messages[0]);
        e.isNative = true;
        e.messages = messages;
        e.backtrace = backtrace;
        e.toString = () => `NativeError: ${messages.join("\n")}`;
        return e;
    }

    function abortedError() {
        const e = new Error("Aborted by the user");
        e.isAborted = true;
        return e;
    }

    function _withCallerStack(e, stack) {
        if (stack !== undefined && e instanceof Error)
            e.stack = e.stack ? e.stack + "\n" + stack : stack;
        return e;
    }

    function invoke(command, args) {
        const stack = new Error().stack;
        return rawInvoke(command, args).catch(e => {
            console.error(`Error in invoke ${command}`, JSON.stringify(e));
            if (e === "Aborted")
                throw _withCallerStack(abortedError(), stack);
            if (e instanceof Object && "Error" in e)
                throw _withCallerStack(nativeError(e.Error.messages, e.Error.backtrace), stack);
            throw _withCallerStack(new Error(String(e)), stack);
        });
    }

    /// Invokes `f` with the id of a newly allocated task that has `listener` registered.
    function invokeWithListener(listener, f) {
        return Util.co(function* () {
            const taskId = yield invoke("allocate_task");
            Tasks.registerTaskListener(taskId, listener);
            return yield f(taskId);
        });
    }

    // app

    function benchExitSplash() {
        return invoke("bench_exit_splash");
    }
    function benchExitInteractive() {
        return invoke("bench_exit_interactive");
    }

    // games

    readonly property var gameSortColumn: ({
            relevance: "Relevance",
            name: "Name",
            popularity: "Popularity",
            modDownloads: "ModDownloads"
        })

    function getGames() {
        return invoke("get_games");
    }
    function searchGames(query, sort) {
        return invoke("search_games", { query, sort });
    }
    function getGamesPopularity() {
        return invoke("get_games_popularity");
    }
    function getGameModDownloads() {
        return invoke("get_game_mods_downloads");
    }

    // mod index

    readonly property var modSortColumn: ({
            relevance: "relevance",
            downloads: "downloads",
            name: "name",
            owner: "owner",
            size: "size"
        })

    function fetchModIndex(game, refresh, listener) {
        return invokeWithListener(listener, taskId => invoke("fetch_mod_index", { game, refresh, taskId }));
    }
    function countModIndex(game, query) {
        return invoke("count_mod_index", { game, query });
    }
    function queryModIndex(game, query, sort, skip, limit) {
        return invoke("query_mod_index", { game, query, sort, skip, limit });
    }
    function getFromModIndex(game, modIds) {
        return invoke("get_from_mod_index", { game, modIds });
    }
    function fetchModMarkdown(owner, name, version, endpoint, listener) {
        return invokeWithListener(listener, taskId => invoke("thunderstore_fetch_mod_markdown", { owner, name, version, endpoint, taskId }));
    }

    function getPreferredLocales() {
        return invoke("get_preferred_locales");
    }

    // profiles

    function getProfiles() {
        return invoke("get_profiles");
    }
    function createProfile(game, name) {
        return invoke("create_profile", { game, name });
    }
    function overwriteProfileMetadata(id, metadata) {
        return invoke("overwrite_profile_metadata", { id, metadata });
    }
    function deleteProfile(id) {
        return invoke("delete_profile", { id });
    }
    function getProfilePath(id) {
        return invoke("get_profile_path", { id });
    }
    function getProfileMods(id) {
        return invoke("get_profile_mods", { id });
    }
    function installProfileMod(id, mod, version, listener) {
        return invokeWithListener(listener, taskId => invoke("install_profile_mod", { id, mod, version, taskId }));
    }
    function uninstallProfileMod(id, owner, name) {
        return invoke("uninstall_profile_mod", { id, owner, name });
    }

    // importing

    function previewImportModpackFromThunderstoreCode(thunderstoreId, game, profileId, listener) {
        return invokeWithListener(listener, taskId => invoke("preview_import_modpack_from_thunderstore_code", { thunderstoreId, game, profileId, taskId }));
    }
    /// `onModProgress` receives `{ url, task }` for each mod as it starts installing.
    function importModpackFromThunderstoreCode(thunderstoreId, game, profileId, onModProgress, listener) {
        const modProgressChannel = createChannel(onModProgress);
        return invokeWithListener(listener, taskId => invoke("import_modpack_from_thunderstore_code", { thunderstoreId, game, profileId, modProgressChannel, taskId })).then(id => {
            closeChannel(modProgressChannel);
            return id;
        }, e => {
            closeChannel(modProgressChannel);
            throw e;
        });
    }

    // installing

    function clearCache() {
        return invoke("clear_cache");
    }

    // ipc

    readonly property var logLevels: ["CRITICAL", "ERROR", "WARN", "INFO", "DEBUG", "TRACE"]

    function allocateIpcConnection() {
        return invoke("allocate_ipc_connection");
    }
    function sendS2CMessage(connId, msg) {
        return invoke("send_s2c_message", { connId, msg });
    }
    function killIpcClient(connId) {
        return invoke("kill_ipc_client", { connId });
    }
    function getIpcConnections() {
        return invoke("get_ipc_connections");
    }

    // launching

    /// `target` is `{ profile: id }` or `{ vanilla: gameId }`.
    function launchProfile(connId, target, modded) {
        return invoke("launch_profile", { connId, target, modded });
    }

    // settings

    function getSettings() {
        return invoke("get_settings");
    }
    function getSettingsUI() {
        return invoke("get_settings_ui");
    }
    function updateSettings(patch) {
        return invoke("update_settings", { patch });
    }

    // tasks

    function cancelTask(id) {
        return invoke("cancel_task", { id });
    }
}
