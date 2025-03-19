import { Accessor, createEffect, createSignal, JSX, Show } from "solid-js";
import { Error } from "./ErrorBoundary";
import { createProgressProxyStore, Listener, Progress } from "../../api/tasks";
import { Store } from "solid-js/store";
import { SimpleProgressIndicator } from "./Progress";

export function AsyncButton(props: {
  children: (busy: Accessor<boolean>, wrapOnClick: (f: () => Promise<void> | void) => Promise<void>) => JSX.Element;
}) {
  const [err, setErr] = createSignal<unknown>();
  const [busy, setBusy] = createSignal(false);
  return (
    <Show
      when={err()}
      fallback={props.children(busy, async (f) => {
        setBusy(true);
        try {
          await f();
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
  class?: string;
  onClick: (listener: Listener) => Promise<void> | void;
  type?: "submit" | "reset" | "button";
  ref?: (element: HTMLButtonElement) => void;
  whenBusy?: (progress: Progress) => JSX.Element;
  children: JSX.Element;
}) {
  const [progress, setProgress] = createProgressProxyStore();
  let ref!: HTMLButtonElement;
  createEffect(() => {
    if (props.ref) props.ref(ref);
  });
  return (
    <AsyncButton>
      {(busy, wrapOnClick) => (
        <button
          class={props.class}
          disabled={busy()}
          type={props.type}
          on:click={async (e) => {
            e.stopPropagation();
            await wrapOnClick(() => {
              return props.onClick((event) => {
                if (event.event === "created") {
                  setProgress(event.progress);
                }
              });
            });
          }}
          ref={ref}
        >
          <Show when={busy()} fallback={props.children}>
            <Show when={props.whenBusy} fallback={<SimpleProgressIndicator progress={progress} />}>
              {(whenBusy) => whenBusy()(progress)}
            </Show>
          </Show>
        </button>
      )}
    </AsyncButton>
  );
}
