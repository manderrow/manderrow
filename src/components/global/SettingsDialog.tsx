import { createUniqueId, For, Match, Switch, useContext } from "solid-js";
import { DefaultDialog, DismissCallback } from "./Dialog";

import styles from "./SettingsDialog.module.css";
import { Settings, SettingsPatch, updateSettings, settings, settingsUI } from "../../api/settings";
import Fa from "solid-fa";
import { faClockRotateLeft } from "@fortawesome/free-solid-svg-icons";
import { t } from "../../i18n/i18n";
import { GameSelectSetting, Setting, TextSetting, ToggleSetting } from "../../api/settings/ui";
import SelectDropdown from "./SelectDropdown";
import { games } from "../../globals";
import ErrorBoundary, { ErrorContext, ReportErrFn } from "./ErrorBoundary";

export default function SettingsDialog(props: { onDismiss: DismissCallback }) {
  const idPrefix = createUniqueId();

  return (
    <ErrorBoundary>
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
                    <div class={styles.option}>
                      <label for={`${idPrefix}_${setting.key}`} id={`${idPrefix}-label_${setting.key}`}>
                        {t(`settings.settings.${setting.key}`)}
                      </label>
                      <div class={styles.right}>
                        <Switch>
                          <Match when={setting.input.type === "toggle"}>
                            <ToggleInput idPrefix={idPrefix} setting={setting as ToggleSetting} />
                          </Match>
                          <Match when={setting.input.type === "text"}>
                            <TextInput idPrefix={idPrefix} setting={setting as TextSetting} />
                          </Match>
                          <Match when={setting.input.type === "game_select"}>
                            <GameSelectInput idPrefix={idPrefix} setting={setting as GameSelectSetting} />
                          </Match>
                        </Switch>
                        <button
                          class={styles.resetButton}
                          on:click={onReset(setting.key)}
                          data-disabled={settings()[setting.key].isDefault}
                        >
                          <Fa icon={faClockRotateLeft} />
                        </button>
                      </div>
                    </div>
                  )}
                </For>
              </div>
            )}
          </For>
        </div>
      </DefaultDialog>
    </ErrorBoundary>
  );
}

function overrideSetting<S extends Setting>(setting: S, override: SettingType<S>) {
  return updateSettings({ [setting.key]: { override } });
}

function onChange<S extends Setting>(
  reportErr: ReportErrFn,
  setting: S,
  mutator: (e: HTMLInputElement) => Settings[S["key"]]["value"],
) {
  return ((e: InputEvent) => {
    try {
      overrideSetting(setting, mutator(e.target as HTMLInputElement));
    } catch (e) {
      reportErr(e);
    }
  }) as (e: Event) => void;
}

function onReset(key: keyof SettingsPatch) {
  return ((_: MouseEvent) => {
    updateSettings({ [key]: "default" });
  }) as (e: Event) => void;
}

type SettingType<S extends Setting> = Settings[S["key"]]["value"];

function get<S extends Setting>(setting: S): SettingType<S> {
  return settings()[setting.key].value;
}

function ToggleInput(props: { idPrefix: string; setting: ToggleSetting }) {
  const reportErr = useContext(ErrorContext);
  return (
    <input
      type="checkbox"
      id={`${props.idPrefix}_${props.setting.key}`}
      checked={get(props.setting)}
      on:change={onChange(reportErr, props.setting, (e) => e.checked)}
    />
  );
}

function TextInput(props: { idPrefix: string; setting: TextSetting }) {
  const reportErr = useContext(ErrorContext);
  return (
    <input
      type="text"
      id={`${props.idPrefix}_${props.setting.key}`}
      value={get(props.setting)}
      on:change={onChange(reportErr, props.setting, (e) => e.value)}
    />
  );
}

function GameSelectInput(props: { idPrefix: string; setting: GameSelectSetting }) {
  const reportErr = useContext(ErrorContext);
  function onChanged(value: string, selected: boolean) {
    if (selected) {
      try {
        overrideSetting(props.setting, value);
      } catch (e) {
        reportErr(e);
      }
    }
  }
  return (
    <SelectDropdown
      label={{ labelText: "value" }}
      buttonId={`${props.idPrefix}_${props.setting.key}`}
      options={Object.fromEntries(
        games()
          .map<[string, { value: string; selected: boolean }]>((game) => [
            game.name,
            { value: game.id, selected: get(props.setting) === game.id },
          ])
          .sort((a, b) => a[0].localeCompare(b[0])),
      )}
      onChanged={onChanged}
    />
  );
}
