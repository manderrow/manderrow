import { invoke } from "@tauri-apps/api/core";
import { wrapInvoke } from "../api";

export function closeSplashscreen() {
  return wrapInvoke(() => invoke("close_splashscreen"));
}

export function relaunch() {
  return wrapInvoke(() => invoke("relaunch"));
}