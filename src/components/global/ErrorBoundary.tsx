import {
  JSX,
  Match,
  Show,
  ErrorBoundary as SolidErrorBoundary,
  Switch,
  createContext,
  createSignal,
} from "solid-js";

import Dialog from "./Dialog";
import { NativeError } from "../../api";
import styles from "./ErrorBoundary.module.css";

export const ErrorContext = createContext<(err: any) => void>(
  (e) => {
    // rethrow
    throw e;
  },
  { name: "Error" }
);

export default function ErrorBoundary(props: { children: JSX.Element }) {
  const [error, setError] = createSignal<Error>();
  return (
    <SolidErrorBoundary
      fallback={(err, reset) => <Error err={err} reset={reset} />}
    >
      <ErrorContext.Provider value={setError}>
        <Show when={error()} fallback={props.children}>
          {(err) => <Error err={err()} reset={() => setError(undefined)} />}
        </Show>
      </ErrorContext.Provider>
    </SolidErrorBoundary>
  );
}

function Error(props: { err: any; reset: () => void }) {
  return (
    <Dialog>
      <div class={styles.error}>
        <h2>Oops!</h2>
        <p>An error occurred, but don't worry, we caught it for you.</p>

        <div class={styles.report}>
          <Switch fallback={<p>{props.err}</p>}>
            <Match when={props.err instanceof NativeError}>
              <p>{props.err.message}</p>
              <details class={styles.spoiler}>
                <summary>
                  <h3>Native Stack Trace:</h3>
                </summary>
                <div class={styles.pre}>
                  <pre>{props.err.backtrace}</pre>
                </div>
              </details>
            </Match>
          </Switch>
          <Show when={props.err.stack}>
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
          We're not perfect. That's why we invite you to{" "}
          <button class={styles.inlineButton}>report</button> this error to us
          if you think we could do better.
        </p>

        <p>
          Otherwise, feel free to{" "}
          <button class={styles.inlineButton} on:click={props.reset}>
            ignore
          </button>{" "}
          this error and carry on modding.
        </p>
      </div>
    </Dialog>
  );
}
