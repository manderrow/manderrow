pragma Singleton
import QtQuick
import Manderrow

// Game connections and their logs, ported from api/console.ts.
//
// Each connection is a QtObject with `connId`, `profileId`, `status` ("connecting", "connected",
// or "disconnected"), `createdTime`, and `events`, a ListModel of displayable log lines.
QtObject {
    id: store

    property var connections: ({})
    /// Incremented whenever a connection is added, for bindings that list connections.
    property int connectionsUpdate: 0
    property var focusedConnection: null
    property var doctorReports: []

    readonly property Component _connectionComponent: Component {
        QtObject {
            property int connId
            property var profileId
            property string status: "connecting"
            property date createdTime: new Date()
            property ListModel events: ListModel {}
            /// The distinct type labels and scopes in `events`, which size the log's columns.
            property var typeLabels: []
            property var scopes: []

            function clear() {
                events.clear();
            }

            function handleEvent(event) {
                store._handleEvent(this, event);
            }
        }
    }

    function connectionIds() {
        connectionsUpdate; // track
        return Object.keys(connections).map(Number);
    }

    function _add(connId, profileId) {
        const conn = _connectionComponent.createObject(store, { connId, profileId });
        connections[connId] = conn;
        connectionsUpdate++;
        return conn;
    }

    function getOrInit(connId) {
        return connections[connId] || _add(connId, undefined);
    }

    /// Allocates a connection for a game about to be launched.
    function allocate(profileId) {
        return Api.allocateIpcConnection().then(connId => {
            if (connections[connId] !== undefined)
                throw new Error("Illegal state");
            return _add(connId, profileId);
        });
    }

    function dismissDoctorReport(index) {
        const reports = doctorReports.slice();
        reports.splice(index, 1);
        doctorReports = reports;
    }

    readonly property var _outputLabels: ({ Out: "STDOUT", Err: "STDERR" })

    function _quote(s) {
        return JSON.stringify(s);
    }

    function _safeOsString(s) {
        if ("Unicode" in s)
            return _quote(s.Unicode);
        if ("NonUnicodeBytes" in s)
            return JSON.stringify(s.NonUnicodeBytes);
        return JSON.stringify(s.NonUnicodeWide);
    }

    /// Converts an event into a row of the console. `filter` is the visibility toggle that
    /// controls the row, and `search` is the text matched by the log search, if searchable.
    function _row(event) {
        const row = { type: "", dataType: "", scope: "", message: "", filter: "", searchable: false };
        switch (event.type) {
        case "Output":
            row.type = row.filter = _outputLabels[event.channel];
            if ("Unicode" in event.line) {
                row.message = event.line.Unicode;
                row.searchable = true;
            } else {
                row.message = JSON.stringify(event.line.Bytes);
            }
            break;
        case "Log":
            row.type = row.dataType = row.filter = event.level;
            row.scope = `${event.scope}:`;
            row.message = event.message;
            row.searchable = true;
            break;
        case "Connect":
            row.type = row.dataType = "CONNECT";
            row.message = "Game connected to Manderrow";
            break;
        case "Start":
            {
                row.type = "START";
                let message = "";
                for (const key of Object.keys(event.env)) {
                    const value = event.env[key];
                    const shown = "Unicode" in value ? _quote(value.Unicode) : JSON.stringify(value);
                    message += `${key}=${shown} `;
                }
                message += _safeOsString(event.command) + " ";
                for (const arg of event.args)
                    message += " " + _safeOsString(arg);
                row.message = message;
            }
            break;
        case "Started":
            row.type = row.dataType = "STARTED";
            row.message = `Game process started with id ${event.pid}`;
            break;
        case "Exit":
            row.type = "EXIT";
            row.message = event.code ? String(event.code) : "Unknown exit code";
            break;
        case "Crash":
            row.type = row.dataType = "CRASH";
            row.message = event.error;
            break;
        case "Error":
            row.type = row.dataType = "ERROR";
            row.message = String(event.error);
            break;
        case "Disconnect":
            row.type = "DISCONNECT";
            row.message = "Game disconnected from Manderrow";
            break;
        default:
            return null;
        }
        return row;
    }

    function _handleEvent(conn, event) {
        if (event.type === "DoctorReport") {
            doctorReports = doctorReports.concat([event]);
            return;
        }
        if (event.type === "Connect")
            conn.status = "connected";
        else if (event.type === "Disconnect")
            conn.status = "disconnected";
        const row = _row(event);
        if (row === null)
            return;
        if (!conn.typeLabels.includes(row.type))
            conn.typeLabels = conn.typeLabels.concat([row.type]);
        if (row.scope !== "" && !conn.scopes.includes(row.scope))
            conn.scopes = conn.scopes.concat([row.scope]);
        conn.events.append(row);
    }

    function init() {
        Api.getIpcConnections().then(ids => {
            for (const id of ids)
                getOrInit(id);
        }).catch(e => console.error("Failed to load IPC connections", e));
    }

    readonly property var _subscriptions: [
        Api.listen("ipc_message", payload => {
            store.getOrInit(payload.connId).handleEvent(payload);
        }),
        Api.listen("ipc_closed", connId => {
            const conn = store.connections[connId];
            if (conn !== undefined)
                conn.status = "disconnected";
        })
    ]
}
