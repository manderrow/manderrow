import { useNavigate } from "@solidjs/router";
import styles from "./Settings.module.css";
import TabRenderer, { Tab, TabContent } from "../../components/global/TabRenderer";
import { createUniqueId, For, Match, Switch, useContext, createSignal } from "solid-js";

import { Settings, SettingsPatch, updateSettings, settings, settingsUI } from "../../api/settings";
import { Fa } from "solid-fa";
import { faChevronLeft, faClockRotateLeft } from "@fortawesome/free-solid-svg-icons";
import { t } from "../../i18n/i18n";
import { GameSelectSetting, Setting, TextSetting, ToggleSetting } from "../../api/settings/ui";
import SelectDropdown from "../../components/global/SelectDropdown";
import { games } from "../../globals";
import { ErrorContext, ReportErrFn } from "../../components/global/ErrorBoundary.tsx";

export default function SettingsPage() {
  const idPrefix = createUniqueId();
  const navigate = useNavigate();

  const tabs: Tab<string>[] = settingsUI().sections.map((section) => ({
    id: section.id,
    name: t(`settings.section.${section.id}`),
    component: (
      <For each={section.settings}>
        {(setting) => (
          <div class={styles.option}>
            <label for={`${idPrefix}_${setting.key}`} id={`${idPrefix}-label_${setting.key}`}>
              {t(`settings.settings.${setting.key}`)}
            </label>
            <div class={styles.option__input}>
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
                disabled={settings()[setting.key].isDefault}
              >
                <Fa icon={faClockRotateLeft} />
              </button>
            </div>
          </div>
        )}
      </For>
    ),
  }));

  const [currentTab, setCurrentTab] = createSignal(tabs[0]);

  return (
    <main class={styles.settings}>
      <aside class={styles.settings__sidebar}>
        <div class={styles.settings__navbar}>
          <div class={styles.navbar__header}>
            <div class={styles.navbar__title}>
              <h1>Settings</h1>
              <button on:click={() => navigate(-1)} data-back>
                <Fa icon={faChevronLeft} />
              </button>
            </div>

            <input type="text" placeholder="Search for settings..." />
          </div>
          <TabRenderer
            id="settings"
            tabs={tabs}
            setter={setCurrentTab}
            styles={{
              tabs: {
                list: styles.settings__tabs,
                list__item: styles.settings__tab,
                list__itemActive: styles.settings__tabActive,
              },
            }}
          />
        </div>
      </aside>
      <div class={styles.options}>
        <TabContent currentTab={currentTab} tabs={tabs} />
      </div>
    </main>
  );
}

export function SettingCategory(props: { id: string }) {
  return <div>{props.id}</div>;
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
