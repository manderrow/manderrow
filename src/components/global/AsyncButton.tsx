import { Accessor, createEffect, createSignal, JSX, Match, Show, Switch } from "solid-js";

import { createProgressProxyStore, Listener, Progress } from "../../api/tasks";

import { ErrorIndicator } from "./ErrorDialog";
import { CircularProgressIndicator, SimpleProgressIndicator } from "./Progress";

import styles from "./AsyncButton.module.css";
import { roundedNumberFormatter } from "../../utils";
import { t } from "../../i18n/i18n";

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
      {(err) => <ErrorIndicator icon={true} message="Error" err={err()} reset={() => setErr(undefined)} />}
    </Show>
  );
}

type ProgressProps<P extends true | Progress | undefined> = P extends true
  ? {
      progress: true;
      onClick: (listener: Listener) => Promise<void> | void;
    }
  : P extends Progress
  ? {
      progress: P;
      onClick: () => Promise<void> | void;
    }
  : {
      progress?: never;
      onClick: () => Promise<void> | void;
    };

export type ProgressStyle = "circular" | "simple" | "in-place";

export function SimpleAsyncButton<const P extends true | Progress | undefined>(
  props: {
    class?: string;
    type?: "submit" | "reset" | "button";
    style?: string;
    /// Optional external busy value.
    busy?: boolean;
    ref?: (element: HTMLButtonElement) => void;
    whenBusy?: (progress: Progress) => JSX.Element;
    children: JSX.Element;

    // Defaults to in-place if unset
    progressStyle?: ProgressStyle;
  } & ProgressProps<P>,
) {
  type ProgressSignal = P extends true ? ReturnType<typeof createProgressProxyStore> : [undefined, undefined];
  const [progress, setProgress]: ProgressSignal = (
    props.progress === true ? createProgressProxyStore() : [props.progress, undefined]
  ) as ProgressSignal;
  let ref!: HTMLButtonElement;
  createEffect(() => {
    if (props.ref) props.ref(ref);
  });

  const progressPercent = () =>
    progress == null
      ? null
      : progress.total == null
      ? 0
      : (progress.completed / (progress.total == 0 ? 1 : progress.total)) * 100;

  return (
    <ActionContext>
      {(busy, wrapOnClick) => (
        <button
          class={`${styles.buttonBase} ${props.class || ""} ${
            (props.progressStyle == null || props.progressStyle === "in-place") &&
            props.progress &&
            (props.busy || busy())
              ? styles.inPlaceBtn
              : ""
          }`}
          style={{
            "--percentage":
              props.progressStyle == null || props.progressStyle === "in-place"
                ? `${progressPercent() || 0}%`
                : undefined,
          }}
          disabled={props.busy || busy()}
          data-btn={props.style}
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
          <Show when={props.progress && (props.busy || busy())} fallback={props.children}>
            <Show
              when={props.whenBusy}
              fallback={
                <Switch>
                  <Match when={props.progressStyle === "circular"}>
                    <CircularProgressIndicator progress={progress!} />
                  </Match>
                  <Match when={props.progressStyle === "simple"}>
                    <SimpleProgressIndicator progress={progress!} />
                  </Match>
                  <Match when={props.progressStyle === "in-place" || props.progressStyle == null}>
                    <span class={styles.percentageText}>
                      {progressPercent() !== null
                        ? `${roundedNumberFormatter.format(progressPercent()!)}%`
                        : t("global.phrases.loading")}
                    </span>

                    <div class={styles.widthRetainer}>{props.children}</div>
                  </Match>
                </Switch>
              }
            >
              {(whenBusy) => whenBusy()(progress!)}
            </Show>
          </Show>
        </button>
      )}
    </ActionContext>
  );
}
