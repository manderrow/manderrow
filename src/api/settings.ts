import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { createResource, createSignal } from "solid-js";

import { wrapInvoke } from "../api.ts";
import * as ui from "./settings/ui.ts";

export interface Setting<T> {
  value: T;
  isDefault: boolean;
}

export interface Settings {
  openConsoleOnLaunch: Setting<boolean>;
}

export type SettingsT<T> = keyof {
  [key in keyof Settings as (Settings[key]["value"] extends T ? key : never)]: never;
};

export type Change<T> = { override: T } | "default";

export interface SettingsPatch {
  openConsoleOnLaunch?: Change<boolean>;
}

const [_settings, setSettings] = createSignal<Settings>();

export const settingsResource = {
  get state() {
    const settings = _settings();
    if (settings === undefined) {
      return "pending";
    } else {
      return "ready";
    }
  },
  get loading() {
    return _settings() === undefined;
  },
  get latest() {
    return _settings();
  },
  error: undefined,
};

export const settings = () => {
  const settings = _settings();
  if (settings === undefined) {
    throw new Error("Settings are not loaded");
  }
  return settings;
};

(async () => {
  let settings;
  try {
    settings = await wrapInvoke(() => invoke<Settings>("get_settings"));
  } catch (e) {
    console.error(e);
    settings = {} as unknown as Settings;
  }
  setSettings(settings);
})();

listen<Settings>("settings", (event) => {
  setSettings(event.payload);
});

export const [settingsUIResource] = createResource<ui.Settings>(async () => {
  return Object.freeze(await wrapInvoke(() => invoke<ui.Settings>("get_settings_ui")));
});
export const settingsUI = () => settingsUIResource.latest!;

export function updateSettings(patch: SettingsPatch): Promise<void> {
  return wrapInvoke(() => invoke("update_settings", { patch }));
}
