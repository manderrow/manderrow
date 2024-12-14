import {
  For,
  Match,
  Show,
  Switch,
  createSignal,
  onCleanup,
  onMount,
} from "solid-js";

import Dialog from "./Dialog";
import styles from "./Console.module.css";
import { Channel } from "@tauri-apps/api/core";

type SafeOsString =
  | { Unicode: string }
  | { NonUnicodeBytes: number[] }
  | { NonUnicodeWide: number[] }
  | { NonUnicodeOther: string };

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
              {(event) => (
                <div>
                  <Switch>
                    <Match when={event.Output}>
                      <span class={styles.event__type}>
                        <Switch>
                          <Match when={event.Output.channel === "Out"}>
                            stdout
                          </Match>
                          <Match when={event.Output.channel === "Err"}>
                            stderr
                          </Match>
                        </Switch>
                      </span>{" "}
                      <Switch
                        fallback={JSON.stringify(event.Output.line.Bytes)}
                      >
                        <Match when={event.Output.line.Unicode}>
                          {event.Output.line.Unicode}
                        </Match>
                      </Switch>
                    </Match>
                    <Match when={event.Log}>
                      <span class={styles.event__type}>{event.Log.level}</span>{" "}
                      <pre>{event.Log.message}</pre>
                    </Match>
                    <Match when={event.Start}>
                      <span class={styles.event__type}>Start</span>{" "}
                      <DisplaySafeOsString string={event.Start.command} />
                      <For each={event.Start.args}>
                        {arg => <>
                          {" "}
                          <DisplaySafeOsString string={arg} />
                        </>}
                      </For>
                    </Match>
                    <Match when={true}>
                      <span class={styles.event__type}>
                        {Object.keys(event)[0]}
                      </span>{" "}
                      <Switch
                        fallback={JSON.stringify(Object.values(event)[0])}
                      >
                        <Match when={event.Exit}>{event.Exit.code}</Match>
                      </Switch>
                    </Match>
                  </Switch>
                </div>
              )}
            </For>
          </div>
        </Show>
        <button on:click={() => setDisplaying(false)}>Dismiss</button>
      </Dialog>
    </Show>
  );
}

function DisplaySafeOsString(props: { string: SafeOsString }) {
  return (
    <Switch>
      <Match when={props.string.Unicode}>{(s) => JSON.stringify(s())}</Match>
      <Match when={props.string.NonUnicodeBytes}>
        {(b) => JSON.stringify(b())}
      </Match>
      <Match when={props.string.NonUnicodeWide}>
        {(b) => JSON.stringify(b())}
      </Match>
      <Match when={props.string.NonUnicodeOther}>
        {(b) => JSON.stringify(b())}
      </Match>
    </Switch>
  );
}
