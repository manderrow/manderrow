import { invoke } from "@tauri-apps/api/core";

import { wrapInvoke } from "../api.ts";

export interface Patch {
  path: PathComponent[],
  change: Change,
}

/// If `number`, must be an integer.
export type PathComponent = string | number;

export type Value = null | boolean | number | string | Value[] | ValueRecord;
// this hack is ridiculous... thanks, TypeScript.
type ValueRecordT<T> = Record<string, T>;
interface ValueRecord extends ValueRecordT<Value> {}

export type Change = { "Set": Value } | { "Append": Value } | "Remove";

export interface Config {
  type: "Config";
  sections: Section[];
}

export interface Document {
  type: "Document";
  html: string;
  sections: DocumentSection[];
}

export type File = Config | Document;

export interface Section {
  path: PathComponent[];
  value: Value;
}

export interface DocumentSection {
  title: string;
  id: string;
  children: DocumentSection[];
}

export function scanModConfigs(profile: string): Promise<string[]> {
  return wrapInvoke(() => invoke("scan_mod_configs", { profile }));
}

export function readModConfig(profile: string, path: string): Promise<File> {
  return wrapInvoke(() => invoke("read_mod_config", { profile, path }));
}

export function updateModConfig(profile: string, path: string, patches: Patch[]): Promise<Config> {
  return wrapInvoke(() => invoke("update_mod_config", { profile, path, patches }));
}
