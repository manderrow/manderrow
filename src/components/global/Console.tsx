import { Channel } from "@tauri-apps/api/core";
import { Accessor, For, Match, Setter, Switch, createEffect, createSignal, onCleanup, useContext } from "solid-js";

import { C2SMessage, DoctorReport, SafeOsString, sendS2CMessage } from "../../api/ipc";
import styles from "./Console.module.css";
import Dialog, { dialogStyles } from "./Dialog";
import { t } from "../../i18n/i18n";
import { ErrorContext } from "./ErrorBoundary";

const translateUnchecked = t as (key: string, args: Object | undefined) => string;

export type ConnectionStatus = "connecting" | "connected" | "disconnected";

export class ConsoleConnection {
  readonly channel: Channel<C2SMessage>;
  readonly status: Accessor<ConnectionStatus>;
  readonly setStatus: (value: ConnectionStatus) => void;
  // TODO: don't use a signal for these
  readonly events: Accessor<C2SMessage[]>;
  readonly setEvents: Setter<C2SMessage[]>;

  constructor() {
    this.channel = new Channel<C2SMessage>();
    const [status, setStatus] = createSignal<ConnectionStatus>("connecting");
    this.status = status;
    this.setStatus = setStatus;
    const [events, setEvents] = createSignal<C2SMessage[]>([]);
    this.events = events;
    this.setEvents = setEvents;
  }

  clear() {
    this.setEvents([]);
  }

  close() {
    this.channel.onmessage = () => {};
  }
}

export default function Console(props: { conn: ConsoleConnection | undefined }) {
  const [doctorReports, setDoctorReports] = createSignal<DoctorReport[]>([]);

  function handleLogEvent(event: C2SMessage) {
    if (props.conn !== undefined) {
      if ("Connect" in event) {
        props.conn.setStatus("connected");
      } else if ("Disconnect" in event) {
        props.conn.setStatus("disconnected");
      } else if ("DoctorReport" in event) {
        setDoctorReports((reports) => [...reports, event.DoctorReport]);
      } else {
        props.conn.setEvents((events) => [...events, event]);
      }
    }
  }

  createEffect<ConsoleConnection | undefined>((prevConn) => {
    if (prevConn !== props.conn) {
      if (prevConn !== undefined) {
        prevConn.close();
      }
      if (props.conn !== undefined) {
        props.conn.channel.onmessage = handleLogEvent;
      }
    }
    return props.conn;
  });

  onCleanup(() => props.conn?.close());

  return (
    <>
      <h2 class={styles.heading}>
        <span class={styles.statusIndicator} data-connected={props.conn?.status() === "connected"}></span>
        {props.conn?.status() === "connected" ? "Connected" : "Disconnected"}{" "}
      </h2>
      <div class={styles.console}>
        <For each={props.conn?.events()} fallback={<p>Game not running.</p>}>
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
                  <span class={styles.event__type}>[CONNECT]</span>
                  {" Wrapper connected to Manderrow"}
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
                  <span class={styles.event__type}>{Object.keys(event)[0]}</span>{" "}
                  <Switch fallback={JSON.stringify(Object.values(event)[0])}>
                    <Match when={event.Exit}>{event.Exit.code}</Match>
                  </Switch>
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

function DoctorDialog(props: { report: DoctorReport; onDismiss: () => void }) {
  const reportErr = useContext(ErrorContext)!;

  return (
    <Dialog>
      <div class={dialogStyles.dialog__container}>
        <h2 class={dialogStyles.dialog__title}>Uh oh!</h2>
        <p class={styles.dialog__message}>
          {translateUnchecked(
            props.report.message ?? `doctor.${props.report.translation_key}.message`,
            props.report.message_args,
          )}
        </p>

        <ul>
          <For each={props.report.fixes}>
            {(fix) => (
              <li>
                <div>
                  {translateUnchecked(`doctor.${props.report.translation_key}.fixes.${fix.id}.label`, fix.label)}
                </div>
                <div>
                  {translateUnchecked(
                    `doctor.${props.report.translation_key}.fixes.${fix.id}.description`,
                    fix.description,
                  )}
                </div>
                <button
                  on:click={async () => {
                    try {
                      await sendS2CMessage({
                        PatientResponse: {
                          id: props.report.id,
                          choice: fix.id,
                        },
                      });
                    } catch (e) {
                      reportErr(e);
                    } finally {
                      console.log("Dismissing");
                      props.onDismiss();
                    }
                  }}
                >
                  {translateUnchecked(
                    `doctor.${props.report.translation_key}.fixes.${fix.id}.confirm_label`,
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
