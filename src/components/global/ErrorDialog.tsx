import { For, Match, Show, Switch } from "solid-js";

import { NativeError } from "../../api";
import { t } from "../../i18n/i18n";

import { ActionContext } from "./AsyncButton";
import { DefaultDialog } from "./Dialog";

import styles from "./ErrorDialog.module.css";

export default function ErrorDialog(props: { err: unknown; reset: () => Promise<void> | void }) {
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
