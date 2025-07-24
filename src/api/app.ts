import { invoke } from "@tauri-apps/api/core";
import { wrapInvoke } from "../api";

export async function close(): Promise<never> {
  await wrapInvoke(() => invoke("close"));
  throw Error("this should be unreachable");
}

export function isMaximized(): Promise<boolean> {
  return wrapInvoke(() => invoke("is_maximized"));
}

export function minimize(): Promise<void> {
  return wrapInvoke(() => invoke("minimize"));
}

export function relaunch(): Promise<never> {
  return wrapInvoke(() => invoke("relaunch"));
}

export function setMaximized(desiredState?: boolean): Promise<never> {
  return wrapInvoke(() => invoke("set_maximized", { desiredState }));
}

export function startDragging(): Promise<void> {
  return wrapInvoke(() => invoke("start_dragging"));
}
