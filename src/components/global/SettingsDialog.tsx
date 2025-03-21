import { createUniqueId, For, Match, Show, Switch } from "solid-js";
import { DefaultDialog, DismissCallback } from "./Dialog";

import styles from "./SettingsDialog.module.css";
import { SettingsPatch, updateSettings, settings, settingsUI } from "../../api/settings";
import Fa from "solid-fa";
import { faClockRotateLeft } from "@fortawesome/free-solid-svg-icons";
import { t } from "../../i18n/i18n";

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

          <For each={settingsUI().sections}>
            {(section) => (
              <div>
                <h3>{t(`settings.section.${section.id}`)}</h3>

                <For each={section.settings}>
                  {(setting) => (
                    <div>
                      <label for={`${idPrefix}_${setting.key}`}>{t(`settings.settings.${setting.key}`)}</label>
                      <Switch>
                        <Match when={setting.input.type === "Toggle"}>
                          <input
                            type="checkbox"
                            id={`${idPrefix}_${setting.key}`}
                            checked={settings()[setting.key].value as unknown as boolean}
                            on:change={onChange((e) => ({ [setting.key]: { override: e.checked } }))}
                          />
                        </Match>
                        <Match when={setting.input.type === "Text"}>
                          <input
                            type="text"
                            id={`${idPrefix}_${setting.key}`}
                            value={settings()[setting.key].value as unknown as string}
                            on:change={onChange((e) => ({ [setting.key]: { override: e.value } }))}
                          />
                        </Match>
                      </Switch>
                      <Show when={!settings().openConsoleOnLaunch.isDefault}>
                        <button on:click={onReset(setting.key)}>
                          <Fa icon={faClockRotateLeft} />
                        </button>
                      </Show>
                    </div>
                  )}
                </For>
              </div>
            )}
          </For>
        </div>
      </DefaultDialog>
    </>
  );
}
