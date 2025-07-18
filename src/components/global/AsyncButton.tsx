import { Accessor, createEffect, createSignal, JSX, Show } from "solid-js";
import { ErrorDialog } from "./ErrorBoundary";
import { createProgressProxyStore, Listener, Progress } from "../../api/tasks";
import { SimpleProgressIndicator } from "./Progress";

export function ActionContext(props: {
  children: (busy: Accessor<boolean>, wrapAction: (f: () => Promise<void> | void) => Promise<void>) => JSX.Element;
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
      {(err) => <ErrorDialog err={err()} reset={() => setErr(undefined)} />}
    </Show>
  );
}

type ProgressProps<Progress extends true | undefined> = Progress extends true
  ? {
      progress: true;
      onClick: (listener: Listener) => Promise<void> | void;
    }
  : {
      progress?: never;
      onClick: () => Promise<void> | void;
    };

export function SimpleAsyncButton<const P extends true | undefined>(
  props: {
    class?: string;
    type?: "submit" | "reset" | "button";
    ref?: (element: HTMLButtonElement) => void;
    whenBusy?: (progress: Progress) => JSX.Element;
    children: JSX.Element;
  } & ProgressProps<P>,
) {
  type ProgressSignal = P extends true ? ReturnType<typeof createProgressProxyStore> : [undefined, undefined];
  const [progress, setProgress]: ProgressSignal = (
    props.progress ? createProgressProxyStore() : [undefined, undefined]
  ) as ProgressSignal;
  let ref!: HTMLButtonElement;
  createEffect(() => {
    if (props.ref) props.ref(ref);
  });
  return (
    <ActionContext>
      {(busy, wrapOnClick) => (
        <button
          class={props.class}
          disabled={busy()}
          type={props.type}
          on:click={async (e) => {
            e.stopPropagation();
            await wrapOnClick(() => {
              if (props.progress) {
                return props.onClick((event) => {
                  if (event.event === "created") {
                    setProgress!(event.progress);
                  }
                });
              } else {
                return props.onClick();
              }
            });
          }}
          ref={ref}
        >
          <Show when={props.progress && busy()} fallback={props.children}>
            <Show when={props.whenBusy} fallback={<SimpleProgressIndicator progress={progress!} />}>
              {(whenBusy) => whenBusy()(progress!)}
            </Show>
          </Show>
        </button>
      )}
    </ActionContext>
  );
}
