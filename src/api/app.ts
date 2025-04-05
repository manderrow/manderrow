import { invoke } from "@tauri-apps/api/core";
import { wrapInvoke } from "../api";

export function relaunch(): Promise<never> {
  return wrapInvoke(() => invoke("relaunch"));
}
