import { invoke } from "@tauri-apps/api/core";

import { wrapInvoke } from "../api.ts";

export async function launchProfile(
  connId: number,
  target: { profile: string } | { vanilla: string },
  options: { modded: boolean },
): Promise<void> {
  return await wrapInvoke(() => invoke("launch_profile", { connId, target, ...options }));
}
