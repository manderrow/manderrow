import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { createResource } from "solid-js";

import { wrapInvoke } from "./api.ts";
import * as ui from "./settings/ui.ts";
import { createSignalResource } from "../utils/utils.ts";

export interface Setting<T> {
  value: T;
  isDefault: boolean;
}

export interface Settings {
  defaultGame: Setting<string | null>;
  openConsoleOnLaunch: Setting<boolean>;
}

export type SettingsT<T> = keyof {
  [key in keyof Settings as Settings[key]["value"] extends T ? key : never]: never;
};

export type Change<T> = { override: T } | "default";

export type SettingsPatch = {
  [setting in keyof Settings]?: Change<Settings[setting]["value"]>;
};

export const settingsResource = createSignalResource<Settings>(() => wrapInvoke(() => invoke("get_settings")));

export const settings = () => settingsResource.latestOrThrow;

listen<Settings>("settings", (event) => {
  settingsResource.value = event.payload;
});

export const [settingsUIResource] = createResource<ui.Settings>(async () => {
  return Object.freeze(await wrapInvoke(() => invoke<ui.Settings>("get_settings_ui")));
});
export const settingsUI = () => settingsUIResource.latest!;

export function updateSettings(patch: SettingsPatch): Promise<void> {
  return wrapInvoke(() => invoke("update_settings", { patch }));
}
