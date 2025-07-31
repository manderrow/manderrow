import { createSignal, For, Match, Show, Switch } from "solid-js";

import { NativeError } from "../../api";
import { t } from "../../i18n/i18n";

import { ActionContext } from "./AsyncButton";
import { DefaultDialog } from "./Dialog";

import styles from "./ErrorDialog.module.css";
import Fa from "solid-fa";
import { faCircleExclamation } from "@fortawesome/free-solid-svg-icons";

export default function ErrorDialog(props: { err: Exclude<unknown, undefined | null>; reset: () => Promise<void> | void }) {
  return (
    <DefaultDialog class={styles.errorDialog}>
      <div class={styles.error}>
        <h2>{t("error.title")}</h2>
        <p>{t("error.deescalation_msg")}</p>

        <div class={styles.report}>
          <Switch fallback={<p>{(props.err as any).toString()}</p>}>
            <Match when={props.err instanceof NativeError}>
              <For each={(props.err as NativeError).messages}>{(msg) => <p>{msg}</p>}</For>
              <details class={styles.spoiler}>
                <summary>
                  <h3>{t("error.native_stack_trace")}:</h3>
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
                  <h3>{t("error.js_stack_trace")}:</h3>
                </summary>
                <div class={styles.pre}>
                  <pre>{stack()}</pre>
                </div>
              </details>
            )}
          </Show>
        </div>

        <p>{t("error.report_msg")}</p>
        <p>{t("error.ignore_msg")}</p>
      </div>
      <div class={styles.buttons}>
        <ActionContext>
          {(busy, wrapOnClick) => (
            <button
              class={styles.inlineButton}
              disabled={busy()}
              on:click={(e) => {
                e.stopPropagation();
                wrapOnClick(props.reset);
              }}
            >
              {t("error.ignore_btn")}
            </button>
          )}
        </ActionContext>

        {/* TODO: Add link to report button */}
        <button class={styles.inlineButton}>{t("error.report_btn")}</button>
      </div>
    </DefaultDialog>
  );
}

export function ErrorIndicator(props: {
  /// Whether to display an icon if an error occurs.
  icon: boolean;
  /// A short message to be displayed if an error occurs.
  message?: string;
  err: Exclude<unknown, undefined | null>;
  reset: () => Promise<void> | void;
}) {
  const [dialogOpen, setDialogOpen] = createSignal(false);

  return <ActionContext>
    {(busy, wrapOnClick) => (<>
      <button class={styles.errorIndicator} type="button" disabled={busy()} onClick={() => setDialogOpen(true)}>
        <Show when={props.icon}>
          <Fa icon={faCircleExclamation} />
        </Show>
        {props.message}
      </button>

      <Show when={dialogOpen()}>
        <ErrorDialog err={props.err} reset={() => {
          setDialogOpen(false);
          wrapOnClick(props.reset);
        }} />
      </Show>
    </>)}
  </ActionContext>;
}
