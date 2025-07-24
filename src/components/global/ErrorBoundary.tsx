import { catchError, createContext, createSignal, JSX, Show } from "solid-js";

import { AbortedError } from "../../api";

import ErrorDialog from "./ErrorDialog";

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
      <Show when={error()} fallback={catchError(() => props.children, onError)}>
        {(err) => <ErrorDialog err={err()} reset={() => setError(undefined)} />}
      </Show>
    </ErrorContext.Provider>
  );
}
