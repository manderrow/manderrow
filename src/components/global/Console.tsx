import { Channel } from "@tauri-apps/api/core";
import { Accessor, For, Match, Switch, createEffect, createSignal, onCleanup, useContext } from "solid-js";

import { C2SMessage, DoctorReport, SafeOsString, sendS2CMessage } from "../../api/ipc";
import styles from "./Console.module.css";
import Dialog, { dialogStyles } from "./Dialog";
import { t } from "../../i18n/i18n";
import { ErrorContext } from "./ErrorBoundary";

const translateUnchecked = t as (key: string, args: Object | undefined) => string;

const [events, setEvents] = createSignal<C2SMessage[]>([]);

export type C2SChannel = Channel<C2SMessage>;

export function createC2SChannel() {
  return new Channel();
}

export function clearConsole() {
  setEvents([]);
}

export default function Console({ channel }: { channel: Accessor<Channel<C2SMessage> | undefined> }) {
  const [doctorReports, setDoctorReports] = createSignal<DoctorReport[]>([]);

  const [connected, setConnected] = createSignal(false);

  function handleLogEvent(event: C2SMessage) {
    if ("Connect" in event) {
      setConnected(true);
    } else if ("Disconnect" in event) {
      setConnected(false);
    } else if ("DoctorReport" in event) {
      console.log(event.DoctorReport);
      setDoctorReports((reports) => [...reports, event.DoctorReport]);
    } else {
      setEvents((events) => [...events, event]);
    }
  }

  function clearChannelHandler() {
    channel()!.onmessage = () => {};
  }

  createEffect(() => {
    if (channel() != null) {
      channel()!.onmessage = handleLogEvent;
    }
  });

  onCleanup(() => {
    // Clear handler
    if (channel() != null) clearChannelHandler();
  });

  return (
    <>
      <h2 class={styles.heading}>
        <span class={styles.statusIndicator} data-connected={connected()}></span>
        {connected() ? "Connected" : "Disconnected"}{" "}
      </h2>
      <div class={styles.console}>
        <For each={events()} fallback={<p>Game not running.</p>}>
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
                      <Match when={event.Output.channel === "Out"}>[LOG]</Match>
                      <Match when={event.Output.channel === "Err"}>[ERR]</Match>
                    </Switch>
                  </span>{" "}
                  {line}
                </p>
              );
            } else if ("Log" in event) {
              return (
                <p>
                  <span class={styles.event__type}>[{event.Log.level}]</span> <span>{event.Log.message}</span>
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
                  <span class={styles.event__type}>[START]</span> <DisplaySafeOsString string={event.Start.command} />
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
      <Match when={"NonUnicodeOther" in s ? s.NonUnicodeOther : null}>{(b) => JSON.stringify(b())}</Match>
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
