import { invoke } from "@tauri-apps/api/core";
import { wrapInvoke } from "../api";

export async function clearCache() {
  return await wrapInvoke(() => invoke<void>("clear_cache"))
}