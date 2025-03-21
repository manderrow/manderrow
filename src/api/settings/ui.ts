import { RawDictionary } from "../../i18n/i18n.ts";
import * as settings from "../settings.ts";

export interface Settings {
  sections: Section[];
}

export interface Section {
  id: keyof RawDictionary["settings"]["section"];
  settings: Setting[];
}

export interface Setting {
  key: keyof settings.Settings & keyof settings.SettingsPatch & keyof RawDictionary["settings"]["settings"];
  input: Input;
}

export type Input = { type: "Toggle" } | { type: "Text" };
