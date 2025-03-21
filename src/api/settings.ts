import { invoke } from "@tauri-apps/api/core";
import { wrapInvoke } from "../api";
import { listen } from "@tauri-apps/api/event";
import { createSignal } from "solid-js";

export interface Setting<T> {
  value: T;
  isDefault: boolean;
}

export interface Settings {
  openConsoleOnLaunch: Setting<boolean>;
}

export type Change<T> = { override: T } | "default";

export interface SettingsPatch {
  openConsoleOnLaunch?: Change<boolean>;
}

const [_settings, setSettings] = createSignal<Settings>();

export const settings = {
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
  get loaded() {
    const settings = _settings();
    if (settings === undefined) {
      throw new Error("Settings are not loaded");
    }
    return settings;
  },
  error: undefined,
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

export function updateSettings(patch: SettingsPatch): Promise<void> {
  return wrapInvoke(() => invoke("update_settings", { patch }));
}
