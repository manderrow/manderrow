import { Channel, invoke } from "@tauri-apps/api/core";
import { Game, Mod } from "./types";
import { C2SChannel } from "./components/global/Console";

/**
 * An error thrown from native code.
 */
export class NativeError extends Error {
  /**
   * A native stack trace. Inspecting this can help to determine where in
   * native code the error originated from.
   */
  readonly backtrace: string;

  constructor(message: string, backtrace: string) {
    super(message);
    this.backtrace = backtrace;
  }

  get [Symbol.toStringTag]() {
    return `NativeError: ${this.message}\nBacktrace:\n${this.backtrace}`;
  }
}

async function wrapInvoke<T>(f: () => Promise<T>): Promise<T> {
  try {
    return await f();
  } catch (e: any) {
    if (e.message !== undefined) {
      throw new NativeError(e.message, e.backtrace);
    }
    throw new Error(e.toString());
  }
}

export async function getGames(): Promise<Game[]> {
  return await wrapInvoke(async () => await invoke("get_games", {}));
}

export async function getGamesPopularity(): Promise<{ [key: string]: number }> {
  return await wrapInvoke(async () => await invoke("get_games_popularity", {}));
}

export type FetchEvent = { type: "Progress"; completed: number; total: number };

export async function fetchModIndex(game: string, options: { refresh: boolean }, onEvent: (event: FetchEvent) => void) {
  const channel = new Channel<FetchEvent>();
  channel.onmessage = onEvent;
  await wrapInvoke(async () => await invoke("fetch_mod_index", { game, ...options, onEvent: channel }));
}

export enum SortColumn {
  Relevance = "Relevance",
  Downloads = "Downloads",
  Name = "Name",
  Owner = "Owner",
}

export interface SortOption {
  column: SortColumn;
  descending: boolean;
}

export async function queryModIndex(
  game: string,
  query: string,
  sort: SortOption[],
  options: { skip?: number; limit?: number }
): Promise<{
  mods: Mod[];
  count: number;
}> {
  return await wrapInvoke(async () => await invoke("query_mod_index", { game, query, sort, ...options }));
}

export async function getPreferredLocales(): Promise<string[]> {
  return await wrapInvoke(async () => await invoke("get_preferred_locales"));
}

export interface Profile {
  name: string;
  game: string;
}

export interface ProfileWithId extends Profile {
  id: string;
}

export async function getProfiles(): Promise<ProfileWithId[]> {
  return await wrapInvoke(async () => await invoke("get_profiles", {}));
}

export async function createProfile(game: string, name: string): Promise<string> {
  return await wrapInvoke(async () => await invoke("create_profile", { game, name }));
}

export async function deleteProfile(id: string): Promise<void> {
  return await wrapInvoke(async () => await invoke("delete_profile", { id }));
}

export async function launchProfile(id: string, channel: C2SChannel, options: { modded: boolean }): Promise<void> {
  return await wrapInvoke(async () => await invoke("launch_profile", { id, channel, ...options }));
}
