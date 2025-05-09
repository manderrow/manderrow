import { RawDictionary } from "../../i18n/i18n.ts";
import { SettingsT } from "../settings.ts";

export interface Settings {
  sections: Section[];
}

export interface Section {
  id: keyof RawDictionary["settings"]["section"];
  settings: Setting[];
}

export interface ToggleSetting {
  key: SettingsT<boolean>;
  input: { type: "toggle" };
}

export interface TextSetting {
  key: SettingsT<string>;
  input: { type: "text" };
}

export interface GameSelectSetting {
  key: SettingsT<string>;
  input: { type: "game_select" };
}

export type Setting = ToggleSetting | TextSetting | GameSelectSetting;
