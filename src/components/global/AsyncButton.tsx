import { Accessor, createEffect, createSignal, JSX, Show } from "solid-js";
import { Error } from "./ErrorBoundary";
import { createProgressProxyStore, Listener, Progress } from "../../api/tasks";
import { Store } from "solid-js/store";
import { SimpleProgressIndicator } from "./Progress";

export default function AsyncButton(props: {
  children: (
    busy: Accessor<boolean>,
    progress: Store<Progress>,
    wrapOnClick: (f: (listener: Listener) => Promise<void> | void) => Promise<void>,
  ) => JSX.Element;
}) {
  const [err, setErr] = createSignal<unknown>();
  const [busy, setBusy] = createSignal(false);
  const [progress, setProgress] = createProgressProxyStore();
  return (
    <Show
      when={err()}
      fallback={props.children(busy, progress, async (f) => {
        setBusy(true);
        try {
          await f((event) => {
            if (event.event === "created") {
              setProgress(event.progress);
            }
          });
        } catch (e) {
          console.error(e);
          setErr(e);
        } finally {
          setBusy(false);
        }
      })}
    >
      {(err) => <Error err={err()} reset={() => setErr(undefined)} />}
    </Show>
  );
}

export function SimpleAsyncButton(props: {
  onClick: (listener: Listener) => Promise<void> | void;
  type?: "submit" | "reset" | "button";
  ref?: (element: HTMLButtonElement) => void;
  children: JSX.Element;
}) {
  let ref!: HTMLButtonElement;
  createEffect(() => {
    if (props.ref) props.ref(ref);
  });
  return (
    <AsyncButton>
      {(busy, progress, wrapOnClick) => (
        <button
          disabled={busy()}
          type={props.type}
          on:click={async (e) => {
            e.stopPropagation();
            await wrapOnClick(props.onClick);
          }}
          ref={ref}
        >
          <Show when={busy()} fallback={props.children}>
            <SimpleProgressIndicator progress={progress} />
          </Show>
        </button>
      )}
    </AsyncButton>
  );
}
