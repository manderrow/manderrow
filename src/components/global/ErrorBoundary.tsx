import { For, JSX, Match, Show, Switch, catchError, createContext, createSignal } from "solid-js";

import { DefaultDialog } from "./Dialog";
import { AbortedError, NativeError } from "../../api";
import styles from "./ErrorBoundary.module.css";
import { AsyncButton } from "./AsyncButton";

export type ReportErrFn = (err: unknown) => void;

export const ErrorContext = createContext<ReportErrFn>(
  (e) => {
    // rethrow
    throw e;
  },
  { name: "Error" },
);

export default function ErrorBoundary(props: { children: JSX.Element }) {
  const [error, setError] = createSignal<unknown>();
  function onError(e: unknown) {
    if (e instanceof AbortedError) return;
    console.error(e);
    setError(e);
  }
  return (
    <ErrorContext.Provider value={onError}>
      <Show
        when={error()}
        fallback={catchError(() => props.children, onError)}
      >
        {(err) => <Error err={err()} reset={() => setError(undefined)} />}
      </Show>
    </ErrorContext.Provider>
  );
}

export function Error(props: { err: unknown; reset: () => Promise<void> | void }) {
  return (
    <DefaultDialog>
      <div class={styles.error}>
        <h2>Oops!</h2>
        <p>An error occurred, but don't worry, we caught it for you.</p>

        <div class={styles.report}>
          <Switch fallback={<p>{(props.err as any).toString()}</p>}>
            <Match when={props.err instanceof NativeError}>
              <For each={(props.err as NativeError).messages}>{(msg) => <p>{msg}</p>}</For>
              <details class={styles.spoiler}>
                <summary>
                  <h3>Native Stack Trace:</h3>
                </summary>
                <div class={styles.pre}>
                  <pre>{(props.err as NativeError).backtrace}</pre>
                </div>
              </details>
            </Match>
          </Switch>
          <Show when={(props.err as any).stack}>
            {(stack) => (
              <details class={styles.spoiler}>
                <summary>
                  <h3>JavaScript Stack Trace:</h3>
                </summary>
                <div class={styles.pre}>
                  <pre>{stack()}</pre>
                </div>
              </details>
            )}
          </Show>
        </div>

        <p>
          We're not perfect. That's why we invite you to <button class={styles.inlineButton}>report</button> this error
          to us if you think we could do better.
        </p>

        <p>
          Otherwise, feel free to{" "}
          <AsyncButton>
            {(busy, wrapOnClick) => (
              <button
                class={styles.inlineButton}
                disabled={busy()}
                on:click={(e) => {
                  e.stopPropagation();
                  wrapOnClick(props.reset);
                }}
              >
                ignore
              </button>
            )}
          </AsyncButton>{" "}
          this error and carry on modding.
        </p>
      </div>
    </DefaultDialog>
  );
}
