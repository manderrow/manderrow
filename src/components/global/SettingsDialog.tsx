import { createUniqueId } from "solid-js";
import { DefaultDialog, DismissCallback } from "./Dialog";

import styles from "./SettingsDialog.module.css";
import { Settings, settings, updateSettings } from "../../api/settings";

export default function SettingsDialog(props: { onDismiss: DismissCallback }) {
  const idPrefix = createUniqueId();

  function onChange(mutator: (e: HTMLInputElement) => Settings) {
    return ((e: InputEvent) => {
      updateSettings(mutator(e.target as HTMLInputElement));
    }) as (e: Event) => void;
  }

  return (
    <>
      <DefaultDialog onDismiss={props.onDismiss}>
        <div class={styles.settings}>
          <div class={styles.header}>
            <h2>Settings</h2>
          </div>

          <label for={`${idPrefix}_openConsoleOnLaunch`}>Open console on launch?</label>
          <input
            type="checkbox"
            id={`${idPrefix}_openConsoleOnLaunch`}
            checked={settings.loaded.openConsoleOnLaunch}
            on:change={onChange((e) => ({ ...settings.loaded, openConsoleOnLaunch: e.checked }))}
          />
        </div>
      </DefaultDialog>
    </>
  );
}
