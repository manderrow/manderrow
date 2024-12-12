import { listen } from "@tauri-apps/api/event";
import {
  For,
  Match,
  Show,
  Switch,
  createSignal,
  onCleanup,
  onMount,
} from "solid-js";
import { createStore } from "solid-js/store";

import Dialog from "./Dialog";
import styles from "./Ipc.module.css";

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

export function clearConsole() {
  setEvents([]);
}

export default function Console() {
  const [displaying, setDisplaying] = createSignal(false);

  const state: { cleanedUp: boolean; unlistenFn?: () => void } = {
    cleanedUp: false,
  };

  onMount(async () => {
    state.unlistenFn = await listen<C2SMessage>(
      "ipc-message",
      ({ payload }) => {
        setDisplaying(true);
        setEvents([...events(), payload]);
      }
    );
    if (state.cleanedUp) {
      state.unlistenFn();
    }
  });

  onCleanup(() => {
    if (state.unlistenFn) {
      state.unlistenFn();
    }
  });

  return (
    <Show when={displaying()}>
      <Dialog>
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
                    <Switch fallback={JSON.stringify(event.Output.line.Bytes)}>
                      <Match when={event.Output.line.Unicode}>
                        {event.Output.line.Unicode}
                      </Match>
                    </Switch>
                  </Match>
                  <Match when={true}>
                    <span class={styles.event__type}>
                      {Object.keys(event)[0]}
                    </span>{" "}
                    <Switch fallback={JSON.stringify(Object.values(event)[0])}>
                      <Match when={event.Exit}>{event.Exit.code}</Match>
                    </Switch>
                  </Match>
                </Switch>
              </div>
            )}
          </For>
        </div>
        <button on:click={() => setDisplaying(false)}>Dismiss</button>
      </Dialog>
    </Show>
  );
}
