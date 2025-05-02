import { Accessor, Setter, createSignal } from "solid-js";

import {
  C2SMessage,
  DoctorReport,
  allocateIpcConnection,
  getIpcConnections,
} from "./api/ipc";
import { listen } from "@tauri-apps/api/event";

export type ConnectionStatus = "connecting" | "connected" | "disconnected";

export const connections = new Map<number, ConsoleConnection>();
export const [connectionsUpdate, setConnectionsUpdate] = createSignal(0);

function getOrInitConnection(connId: number): ConsoleConnection {
  let conn = connections.get(connId);
  if (conn === undefined) {
    conn = new ConsoleConnection(connId);
    connections.set(connId, conn);
    setConnectionsUpdate(connectionsUpdate() + 1);
  }
  return conn;
}

(async () => {
  for (const conn of await getIpcConnections()) {
    getOrInitConnection(conn);
  }
})();

listen<IdentifiedC2SMessage>("ipc_message", (event) => {
  console.log("ipc_message", event.payload);
  getOrInitConnection(event.payload.connId).handleEvent(event.payload);
});

listen<number>("ipc_closed", (event) => {
  console.log("ipc_closed", event.payload);
  let conn = connections.get(event.payload);
  if (conn !== undefined) {
    conn.setStatus("disconnected");
  }
});

type Event = C2SMessage | FrontendEvent;

type FrontendEvent = { Error: { error: unknown } };

type IdentifiedC2SMessage = C2SMessage & { connId: number };
export type IdentifiedDoctorReport = { connId: number; DoctorReport: DoctorReport };

export const [doctorReports, setDoctorReports] = createSignal<IdentifiedDoctorReport[]>([]);

export class ConsoleConnection {
  readonly id: number;
  readonly status: Accessor<ConnectionStatus>;
  readonly setStatus: (value: ConnectionStatus) => void;
  // TODO: don't use a signal for these
  readonly events: Accessor<Event[]>;
  readonly setEvents: Setter<Event[]>;

  constructor(id: number) {
    this.id = id;
    const [status, setStatus] = createSignal<ConnectionStatus>("connecting");
    this.status = status;
    this.setStatus = setStatus;
    const [events, setEvents] = createSignal<Event[]>([]);
    this.events = events;
    this.setEvents = setEvents;
  }

  static async allocate(): Promise<ConsoleConnection> {
    const connId = await allocateIpcConnection();
    if (connections.has(connId)) throw new Error("Illegal state");
    const conn = new ConsoleConnection(connId);
    connections.set(connId, conn);
    setConnectionsUpdate(connectionsUpdate() + 1);
    console.log(connId, conn, connections);
    return conn;
  }

  clear() {
    this.setEvents([]);
  }

  handleEvent(event: IdentifiedC2SMessage | FrontendEvent) {
    if ("DoctorReport" in event) {
      setDoctorReports((reports) => [...reports, event]);
      return;
    }

    if ("Connect" in event) {
      this.setStatus("connected");
    } else if ("Disconnect" in event) {
      this.setStatus("disconnected");
    }

    this.setEvents((events) => [...events, event]);
  }
}

export const [focusedConnection, setFocusedConnection] = createSignal<ConsoleConnection>();
