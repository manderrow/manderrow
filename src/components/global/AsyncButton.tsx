import { Accessor, ComponentProps, createSignal, JSX, Match, Show, splitProps, Switch } from "solid-js";

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
  props: Omit<ComponentProps<"button">, "onClick"> & {
    class?: string;
    busyClass?: string;
    btnStyle?: string;
    /// Optional external busy value.
    busy?: boolean;
    whenBusy?: (progress: Progress) => JSX.Element;

    // Defaults to in-place if unset
    progressStyle?: ProgressStyle;
  } & ProgressProps<P>,
) {
  type ProgressSignal = P extends true ? ReturnType<typeof createProgressProxyStore> : [undefined, undefined];
  const [progress, setProgress]: ProgressSignal = (
    props.progress === true ? createProgressProxyStore() : [props.progress, undefined]
  ) as ProgressSignal;
  const progressPercent = () =>
    progress == null
      ? null
      : progress.total == null
      ? 0
      : (progress.completed / (progress.total == 0 ? 1 : progress.total)) * 100;

  const [local, rest] = splitProps(props, [
    "class",
    "busyClass",
    "btnStyle",
    "busy",
    "onClick",
    "progress",
    "progressStyle",
    "whenBusy",
  ]);

  return (
    <ActionContext>
      {(busy, wrapOnClick) => (
        <button
          class={`${styles.buttonBase} ${local.class || ""} ${
            (local.progressStyle == null || local.progressStyle === "in-place") &&
            local.progress &&
            (local.busy || busy())
              ? styles.inPlaceBtn
              : ""
          }`}
          classList={
            local.busyClass
              ? {
                  [local.busyClass]: local.busy || busy(),
                }
              : {}
          }
          style={{
            "--percentage":
              local.progressStyle == null || local.progressStyle === "in-place"
                ? `${progressPercent() || 0}%`
                : undefined,
          }}
          disabled={local.busy || busy()}
          data-btn={local.btnStyle}
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
          {...rest}
        >
          <Show when={local.progress && (local.busy || busy())} fallback={props.children}>
            <Show
              when={local.whenBusy}
              fallback={
                <Switch>
                  <Match when={local.progressStyle === "circular"}>
                    <CircularProgressIndicator progress={progress!} />
                  </Match>
                  <Match when={local.progressStyle === "simple"}>
                    <SimpleProgressIndicator progress={progress!} />
                  </Match>
                  <Match when={local.progressStyle === "in-place" || local.progressStyle == null}>
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
