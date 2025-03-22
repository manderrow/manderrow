import { createUniqueId, For, Match, Show, Switch } from "solid-js";
import { DefaultDialog, DismissCallback } from "./Dialog";

import styles from "./SettingsDialog.module.css";
import { Settings, SettingsPatch, updateSettings, settings, settingsUI } from "../../api/settings";
import Fa from "solid-fa";
import { faClockRotateLeft } from "@fortawesome/free-solid-svg-icons";
import { t } from "../../i18n/i18n";
import { Setting, TextSetting, ToggleSetting } from "../../api/settings/ui";

export default function SettingsDialog(props: { onDismiss: DismissCallback }) {
  const idPrefix = createUniqueId();

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
                          <ToggleInput idPrefix={idPrefix} setting={setting as ToggleSetting} />
                        </Match>
                        <Match when={setting.input.type === "Text"}>
                          <TextInput idPrefix={idPrefix} setting={setting as TextSetting} />
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

function onChange<S extends Setting>(setting: S, mutator: (e: HTMLInputElement) => Settings[S["key"]]["value"]) {
  return ((e: InputEvent) => {
    updateSettings({ [setting.key]: { override: mutator(e.target as HTMLInputElement) } });
  }) as (e: Event) => void;
}

function onReset(key: keyof SettingsPatch) {
  return ((_: MouseEvent) => {
    updateSettings({ [key]: "default" });
  }) as (e: Event) => void;
}

type SettingType<T extends Setting> = T["input"]["type"] extends "Text"
  ? string
  : T["input"]["type"] extends "Toggle"
  ? boolean
  : unknown;

function get<T extends Setting>(setting: T): SettingType<T> {
  return settings()[setting.key].value as SettingType<T>;
}

function ToggleInput(props: { idPrefix: string; setting: ToggleSetting }) {
  return (
    <input
      type="checkbox"
      id={`${props.idPrefix}_${props.setting.key}`}
      checked={get(props.setting)}
      on:change={onChange(props.setting, (e) => e.checked)}
    />
  );
}

function TextInput(props: { idPrefix: string; setting: TextSetting }) {
  return (
    <input
      type="text"
      id={`${props.idPrefix}_${props.setting.key}`}
      value={get(props.setting) as unknown as string}
      on:change={onChange(props.setting, (e) => e.value)}
    />
  );
}
