import { Accessor, For, Match, Setter, Show, Switch, createSignal, useContext } from "solid-js";

import { C2SMessage, DoctorReport, SafeOsString, allocateIpcConnection, sendS2CMessage } from "../../api/ipc";
import styles from "./Console.module.css";
import Dialog, { dialogStyles } from "./Dialog";
import { t } from "../../i18n/i18n";
import { ErrorContext } from "./ErrorBoundary";
import { listen } from "@tauri-apps/api/event";
import SelectDropdown from "./SelectDropdown";

const translateUnchecked = t as (key: string, args: Object | undefined) => string;

export type ConnectionStatus = "connecting" | "connected" | "disconnected";

const connections = new Map<number, ConsoleConnection>();
const [connectionsUpdate, setConnectionsUpdate] = createSignal(0);

listen<IdentifiedC2SMessage>("ipc_message", (event) => {
  console.log("ipc_message", event.payload);
  let conn = connections.get(event.payload.connId);
  if (conn === undefined) {
    conn = new ConsoleConnection(event.payload.connId);
    connections.set(event.payload.connId, conn);
    setConnectionsUpdate(connectionsUpdate() + 1);
  }
  conn.handleEvent(event.payload);
});

listen<number>("ipc_closed", (event) => {
  console.log("ipc_closed", event.payload);
  let conn = connections.get(event.payload);
  if (conn !== undefined) {
    conn.setStatus("disconnected");
  }
});

type IdentifiedC2SMessage = C2SMessage & { connId: number };
type IdentifiedDoctorReport = { connId: number; DoctorReport: DoctorReport };

const [doctorReports, setDoctorReports] = createSignal<IdentifiedDoctorReport[]>([]);

export class ConsoleConnection {
  readonly id: number;
  readonly status: Accessor<ConnectionStatus>;
  readonly setStatus: (value: ConnectionStatus) => void;
  // TODO: don't use a signal for these
  readonly events: Accessor<C2SMessage[]>;
  readonly setEvents: Setter<C2SMessage[]>;

  constructor(id: number) {
    this.id = id;
    const [status, setStatus] = createSignal<ConnectionStatus>("connecting");
    this.status = status;
    this.setStatus = setStatus;
    const [events, setEvents] = createSignal<C2SMessage[]>([]);
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

  handleEvent(event: IdentifiedC2SMessage) {
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

export function DoctorReports() {
  return (
    <For each={doctorReports()}>
      {(report, i) => (
        <DoctorDialog
          report={report}
          onDismiss={() => {
            setDoctorReports((reports) => {
              return [...reports.slice(0, i()), ...reports.slice(i() + 1)];
            });
          }}
        />
      )}
    </For>
  );
}

export const [focusedConnection, setFocusedConnection] = createSignal<ConsoleConnection>();

export default function Console() {
  function getSelectConnectionOptions() {
    // track updates
    connectionsUpdate();
    return Object.fromEntries(
      Array.from(connections.keys()).map((id) => [
        id.toString(),
        { value: id, selected: id === focusedConnection()?.id },
      ]),
    );
  }

  return (
    <>
      <h2 class={styles.heading}>
        <span class={styles.statusIndicator} data-connected={focusedConnection()?.status() === "connected"}></span>
        {focusedConnection()?.status() === "connected" ? "Connected" : "Disconnected"}{" "}
        <SelectDropdown
          label={{ labelText: "value" }}
          options={getSelectConnectionOptions()}
          onChanged={(id, selected) => {
            if (selected) {
              setFocusedConnection(connections.get(id));
            }
          }}
        />
      </h2>
      <div class={styles.console}>
        <For each={focusedConnection()?.events()} fallback={<p>Game not running.</p>}>
          {(event) => {
            if ("Output" in event) {
              let line: string;
              if ("Unicode" in event.Output.line) {
                line = event.Output.line.Unicode;
              } else if ("Bytes" in event.Output.line) {
                line = JSON.stringify(event.Output.line.Bytes);
              } else {
                throw Error();
              }
              return (
                <p>
                  <span class={styles.event__type}>
                    <Switch>
                      <Match when={event.Output.channel === "Out"}>[OUT]</Match>
                      <Match when={event.Output.channel === "Err"}>[ERR]</Match>
                    </Switch>
                  </span>{" "}
                  {line}
                </p>
              );
            } else if ("Log" in event) {
              return (
                <p>
                  <span class={styles.event__type}>[{event.Log.level}]</span> <span>{event.Log.scope}</span>:{" "}
                  <span>{event.Log.message}</span>
                </p>
              );
            } else if ("Connect" in event) {
              return (
                <p>
                  <span class={styles.event__type}>[CONNECT]</span> Agent connected to Manderrow from process{" "}
                  {event.Connect.pid}
                </p>
              );
            } else if ("Start" in event) {
              return (
                <p>
                  <span class={styles.event__type}>[START]</span>{" "}
                  <For each={Object.entries(event.Start.env)}>
                    {([k, v]) => {
                      if ("Unicode" in v) {
                        return (
                          <>
                            {k}={JSON.stringify(v.Unicode)}{" "}
                          </>
                        );
                      } else {
                        return (
                          <>
                            {k}={JSON.stringify(v)}{" "}
                          </>
                        );
                      }
                    }}
                  </For>
                  <DisplaySafeOsString string={event.Start.command} />{" "}
                  <For each={event.Start.args}>
                    {(arg) => (
                      <>
                        {" "}
                        <DisplaySafeOsString string={arg} />
                      </>
                    )}
                  </For>
                </p>
              );
            } else if ("Exit" in event) {
              return (
                <p>
                  <span class={styles.event__type}>[EXIT]</span>{" "}
                  <Show when={event.Exit.code} fallback="Unknown exit code">
                    <span>{event.Exit.code}</span>
                  </Show>
                </p>
              );
            } else if ("Crash" in event) {
              return (
                <p>
                  <span class={styles.event__type}>[CRASH]</span> <span>{event.Crash.error}</span>
                </p>
              );
            }
          }}
        </For>
        <div class={styles.scrollAnchor} />
      </div>
    </>
  );
}

function DisplaySafeOsString(props: { string: SafeOsString }) {
  const s = props.string;
  return (
    <Switch>
      <Match when={"Unicode" in s ? s.Unicode : null}>{(s) => JSON.stringify(s())}</Match>
      <Match when={"NonUnicodeBytes" in s ? s.NonUnicodeBytes : null}>{(b) => JSON.stringify(b())}</Match>
      <Match when={"NonUnicodeWide" in s ? s.NonUnicodeWide : null}>{(b) => JSON.stringify(b())}</Match>
    </Switch>
  );
}

function DoctorDialog(props: { report: IdentifiedDoctorReport; onDismiss: () => void }) {
  const reportErr = useContext(ErrorContext)!;

  const report = () => props.report.DoctorReport;

  return (
    <Dialog>
      <div class={dialogStyles.dialog__container}>
        <h2 class={dialogStyles.dialog__title}>Uh oh!</h2>
        <p class={styles.dialog__message}>
          {translateUnchecked(report().message ?? `doctor.${report().translation_key}.message`, report().message_args)}
        </p>

        <ul>
          <For each={report().fixes}>
            {(fix) => (
              <li>
                <div>{translateUnchecked(`doctor.${report().translation_key}.fixes.${fix.id}.label`, fix.label)}</div>
                <div>
                  {translateUnchecked(
                    `doctor.${report().translation_key}.fixes.${fix.id}.description`,
                    fix.description,
                  )}
                </div>
                <button
                  on:click={async () => {
                    try {
                      await sendS2CMessage(props.report.connId, {
                        PatientResponse: {
                          id: report().id,
                          choice: fix.id,
                        },
                      });
                    } catch (e) {
                      reportErr(e);
                    } finally {
                      props.onDismiss();
                    }
                  }}
                >
                  {translateUnchecked(
                    `doctor.${report().translation_key}.fixes.${fix.id}.confirm_label`,
                    fix.confirm_label,
                  )}
                </button>
              </li>
            )}
          </For>
        </ul>
      </div>
    </Dialog>
  );
}
