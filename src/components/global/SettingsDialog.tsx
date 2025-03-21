import { createUniqueId } from "solid-js";
import { DefaultDialog, DismissCallback } from "./Dialog";

import styles from "./SettingsDialog.module.css";
import { settings, SettingsPatch, updateSettings } from "../../api/settings";
import Fa from "solid-fa";
import { faClockRotateLeft } from "@fortawesome/free-solid-svg-icons";

export default function SettingsDialog(props: { onDismiss: DismissCallback }) {
  const idPrefix = createUniqueId();

  function onChange(mutator: (e: HTMLInputElement) => SettingsPatch) {
    return ((e: InputEvent) => {
      updateSettings(mutator(e.target as HTMLInputElement));
    }) as (e: Event) => void;
  }

  function onReset(key: keyof SettingsPatch) {
    return ((_: MouseEvent) => {
      updateSettings({ [key]: "default" });
    }) as (e: Event) => void;
  }

  return (
    <>
      <DefaultDialog onDismiss={props.onDismiss}>
        <div class={styles.settings}>
          <div class={styles.header}>
            <h2>Settings</h2>
          </div>

          <div>
            <label for={`${idPrefix}_openConsoleOnLaunch`}>Open console on launch?</label>
            <input
              type="checkbox"
              id={`${idPrefix}_openConsoleOnLaunch`}
              checked={settings.loaded.openConsoleOnLaunch.value}
              on:change={onChange((e) => ({ openConsoleOnLaunch: { override: e.checked } }))}
            />
            <Show when={!settings.loaded.openConsoleOnLaunch.isDefault}>
              <button on:click={onReset("openConsoleOnLaunch")}>
                <Fa icon={faClockRotateLeft} />
              </button>
            </Show>
          </div>
        </div>
      </DefaultDialog>
    </>
  );
}
