import { Accessor, Setter, createSignal } from "solid-js";

import { C2SMessage, DoctorReport, allocateIpcConnection, getIpcConnections } from "./api/ipc";
import { listen } from "@tauri-apps/api/event";

export type ConnectionStatus = "connecting" | "connected" | "disconnected";

/**
 * Watch connectionsUpdate for reactivity
 */
export const connections = new Map<number, ConsoleConnection>();
export const [connectionsUpdate, setConnectionsUpdate] = createSignal(0);

function getOrInitConnection(connId: number): ConsoleConnection {
  let conn = connections.get(connId);
  if (conn === undefined) {
    conn = new ConsoleConnection(connId);
    connections.set(connId, conn);
    setConnectionsUpdate((connections) => connections + 1);
  }
  return conn;
}

(async () => {
  for (const conn of await getIpcConnections()) {
    getOrInitConnection(conn);
  }
})();

listen<IdentifiedC2SMessage>("ipc_message", (event) => {
  getOrInitConnection(event.payload.connId).handleEvent(event.payload);
});

listen<number>("ipc_closed", (event) => {
  let conn = connections.get(event.payload);
  if (conn !== undefined) {
    conn.setStatus("disconnected");
  }
});

export type Event = C2SMessage | FrontendEvent;

type FrontendEvent = { type: "Error"; error: unknown };

type IdentifiedC2SMessage = C2SMessage & { connId: number };
export type IdentifiedDoctorReport = DoctorReport & { connId: number };

export const [doctorReports, setDoctorReports] = createSignal<IdentifiedDoctorReport[]>([]);

export class ConsoleConnection {
  readonly id: number;
  readonly profileId?: string;
  readonly status: Accessor<ConnectionStatus>;
  readonly setStatus: (value: ConnectionStatus) => void;
  // TODO: don't use a signal for these
  readonly events: Accessor<Event[]>;
  readonly setEvents: Setter<Event[]>;
  readonly createdTime: Date;

  constructor(id: number, profileId?: string) {
    this.id = id;
    this.profileId = profileId;
    const [status, setStatus] = createSignal<ConnectionStatus>("connecting");
    this.status = status;
    this.setStatus = setStatus;
    const [events, setEvents] = createSignal<Event[]>([]);
    this.events = events;
    this.setEvents = setEvents;
    this.createdTime = new Date();
  }

  static async allocate(profileId?: string): Promise<ConsoleConnection> {
    const connId = await allocateIpcConnection();
    if (connections.has(connId)) throw new Error("Illegal state");
    const conn = new ConsoleConnection(connId, profileId);
    connections.set(connId, conn);
    setConnectionsUpdate((connections) => connections + 1);
    return conn;
  }

  clear() {
    this.setEvents([]);
  }

  handleEvent(event: IdentifiedC2SMessage | FrontendEvent) {
    if (event.type === "DoctorReport") {
      setDoctorReports((reports) => [...reports, event]);
      return;
    }

    if (event.type === "Connect") {
      this.setStatus("connected");
    } else if (event.type === "Disconnect") {
      this.setStatus("disconnected");
    }

    this.setEvents((events) => [...events, event]);
  }
}

export const [focusedConnection, setFocusedConnection] = createSignal<ConsoleConnection>();
