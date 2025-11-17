import { invoke } from "@tauri-apps/api/core";
import { Listener, invokeWithListener } from "../tasks";

export type Endpoint = "readme" | "changelog";

export function fetchModMarkdown(
  owner: string,
  name: string,
  version: string,
  endpoint: Endpoint,
  listener: Listener,
): Promise<{ markdown: string | null }> {
  return invokeWithListener(listener, (taskId) => {
    return invoke("thunderstore_fetch_mod_markdown", { owner, name, version, endpoint, taskId });
  });
}
