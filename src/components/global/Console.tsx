import { For, Match, Show, Switch, createSignal, onCleanup, onMount } from "solid-js";

import Dialog from "./Dialog";
import styles from "./Console.module.css";
import { Channel } from "@tauri-apps/api/core";

type SafeOsString = { Unicode: string } | { NonUnicodeBytes: number[] } | { NonUnicodeWide: number[] } | { NonUnicodeOther: string };

type C2SMessage =
  | {
      Start: {
        command: SafeOsString;
        args: SafeOsString[];
        env: { [key: string]: SafeOsString };
      };
    }
  | {
      Log: {
        level: "Error" | "Warn" | "Info" | "Debug" | "Trace";
        message: string;
      };
    }
  | {
      Output: {
        channel: "Out" | "Err";
        line:
          | {
              Unicode: string;
            }
          | {
              Bytes: number[];
            };
      };
    }
  | {
      Exit: {
        code?: number;
      };
    }
  | {
      Crash: {
        error: string;
      };
    };

const [events, setEvents] = createSignal<C2SMessage[]>([]);

export type C2SChannel = Channel<C2SMessage>;

export function createC2SChannel() {
  return new Channel();
}

export function clearConsole() {
  setEvents([]);
}

export default function Console({ channel }: { channel: Channel<C2SMessage> }) {
  const [displaying, setDisplaying] = createSignal(true);

  onMount(() => {
    channel.onmessage = (event) => {
      setDisplaying(true);
      setEvents([...events(), event]);
    };
  });

  onCleanup(() => {
    // clear handler
    channel.onmessage = () => {};
  });

  return (
    <Show when={displaying()}>
      <Dialog>
        <Show
          when={events().length !== 0}
          fallback={
            <>
              <p>Waiting for game...</p>
              <progress />
            </>
          }
        >
          <div class={styles.console}>
            <For each={events()}>
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
                  return <div>
                    <span class={styles.event__type}>
                      <Switch>
                        <Match when={event.Output.channel === "Out"}>stdout</Match>
                        <Match when={event.Output.channel === "Err"}>stderr</Match>
                      </Switch>
                    </span>{" "}
                    {line}
                  </div>;
                } else if ("Log" in event) {
                  return <div>
                    <span class={styles.event__type}>{event.Log.level}</span> <pre>{event.Log.message}</pre>
                  </div>;
                } else if ("Start" in event) {
                  return <div>
                    <span class={styles.event__type}>Start</span> <DisplaySafeOsString string={event.Start.command} />
                        <For each={event.Start.args}>
                          {(arg) => (
                            <>
                              {" "}
                              <DisplaySafeOsString string={arg} />
                            </>
                          )}
                        </For>
                  </div>;
                } else if ("Exit" in event) {
                  return <div>
                    <span class={styles.event__type}>{Object.keys(event)[0]}</span>{" "}
                    <Switch fallback={JSON.stringify(Object.values(event)[0])}>
                      <Match when={event.Exit}>{event.Exit.code}</Match>
                    </Switch>
                  </div>;
                }
              }}
            </For>
          </div>
        </Show>
        <button on:click={() => setDisplaying(false)}>Dismiss</button>
      </Dialog>
    </Show>
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
