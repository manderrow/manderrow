import { Channel, invoke } from "@tauri-apps/api/core";
import { C2SChannel } from "./components/global/Console";
import { Game, ModListing, ModMetadata, ModPackage, ModVersion } from "./types";
import { invokeWithListener, Listener, TaskEvent, Id as TaskId } from "./api/tasks";

/**
 * An error thrown from native code.
 */
export class NativeError extends Error {
  readonly messages: readonly string[];
  /**
   * A native stack trace. Inspecting this can help to determine where in
   * native code the error originated from.
   */
  readonly backtrace: string;

  constructor(messages: readonly string[], backtrace: string) {
    super(messages[0]);
    this.messages = messages;
    this.backtrace = backtrace;
  }

  get [Symbol.toStringTag]() {
    return `NativeError: ${this.messages.join("\n")}\nBacktrace:\n${this.backtrace}`;
  }
}

export class AbortedError extends Error {
  constructor() {
    super("Aborted by the user");
  }
}

export async function wrapInvoke<T>(f: () => Promise<T>): Promise<T> {
  try {
    return await f();
  } catch (e: any) {
    console.error("Error in invoke", e);
    if (e === "Aborted") {
      throw new AbortedError();
    } else if (e instanceof Object && "Error" in e) {
      throw new NativeError(e.Error.messages, e.Error.backtrace);
    } else {
      throw new Error(e.toString());
    }
  }
}

export async function getGames(): Promise<Game[]> {
  return await wrapInvoke(() => invoke("get_games", {}));
}

export enum GameSortColumn {
  Relevance = "Relevance",
  Name = "Name",
  Popularity = "Popularity",
  ModDownloads = "ModDownloads",
}

export async function searchGames(query: string, sort: readonly SortOption<GameSortColumn>[]): Promise<number[]> {
  return await wrapInvoke(() => invoke("search_games", { query, sort }));
}

export async function getGamesPopularity(): Promise<{ [key: string]: number }> {
  return JSON.parse(await wrapInvoke<string>(() => invoke("get_games_popularity", {})));
}

export async function getGameModDownloads(): Promise<{ [key: string]: number }> {
  return JSON.parse(await wrapInvoke<string>(() => invoke("get_game_mods_downloads", {})));
}

export async function fetchModIndex(game: string, options: { refresh: boolean }, listener: (event: TaskEvent) => void) {
  await invokeWithListener(listener, (taskId) => invoke("fetch_mod_index", { game, ...options, taskId }));
}

export enum ModSortColumn {
  Relevance = "Relevance",
  Downloads = "Downloads",
  Name = "Name",
  Owner = "Owner",
}

export interface SortOption<C> {
  column: C;
  descending: boolean;
}

export async function countModIndex(game: string, query: string): Promise<number> {
  return await wrapInvoke(() => invoke("count_mod_index", { game, query }));
}

export async function queryModIndex(
  game: string,
  query: string,
  sort: readonly SortOption<ModSortColumn>[],
  options: { skip?: number; limit?: Exclude<number, 0> },
): Promise<{
  mods: ModListing[];
  count: number;
}> {
  return await wrapInvoke(() => invoke("query_mod_index", { game, query, sort, ...options }));
}

// TODO: figure out how to define this for arbitrary lengths
export type GetFromModIndexResult<ModIds extends readonly ModId[]> = ModIds extends readonly [ModId]
  ? [ModListing]
  : ModListing[] & { length: ModIds["length"] };

export interface ModId {
  owner: string;
  name: string;
}

export async function getFromModIndex<const ModIds extends readonly ModId[]>(
  game: string,
  mod_ids: ModIds,
): Promise<GetFromModIndexResult<ModIds>> {
  return await wrapInvoke(() => invoke("get_from_mod_index", { game, mod_ids }));
}

export async function getPreferredLocales(): Promise<string[]> {
  return await wrapInvoke(() => invoke("get_preferred_locales"));
}

export interface Profile {
  name: string;
  game: string;
}

export interface ProfileWithId extends Profile {
  id: string;
}

export async function getProfiles(): Promise<ProfileWithId[]> {
  return await wrapInvoke(() => invoke("get_profiles", {}));
}

export async function createProfile(game: string, name: string): Promise<string> {
  return await wrapInvoke(() => invoke("create_profile", { game, name }));
}

export async function deleteProfile(id: string): Promise<void> {
  return await wrapInvoke(() => invoke("delete_profile", { id }));
}

export async function launchProfile(id: string, channel: C2SChannel, options: { modded: boolean }): Promise<void> {
  return await wrapInvoke(() => invoke("launch_profile", { id, channel, ...options }));
}

export async function getProfileMods(id: string): Promise<ModPackage[]> {
  return await wrapInvoke(() => invoke("get_profile_mods", { id }));
}

export async function installProfileMod(
  id: string,
  mod: ModMetadata,
  version: ModVersion,
  listener: Listener,
): Promise<void> {
  await invokeWithListener(listener, (taskId) => invoke("install_profile_mod", { id, mod, version, taskId }));
}

export async function uninstallProfileMod(id: string, owner: string, name: string): Promise<void> {
  return await wrapInvoke(() => invoke("uninstall_profile_mod", { id, owner, name }));
}

export interface ModSpec {
  type: "Online";
  url: string;
}

export interface PathDiff {
  path: string;
  diff: Diff;
}

export enum Diff {
  Created = "Created",
  Deleted = "Deleted",
  Modified = "Modified",
}

export interface Modpack {
  name: string;
  mods: ModSpec[];
  diff: PathDiff[];
}

export async function previewImportModpackFromThunderstoreCode(
  thunderstoreId: string,
  game: string,
  profileId: string | undefined,
  listener: Listener,
): Promise<Modpack> {
  return await invokeWithListener(listener, (taskId) =>
    invoke("preview_import_modpack_from_thunderstore_code", { thunderstoreId, game, profileId, taskId }),
  );
}

export interface ModProgressRegistration {
  url: string;
  task: TaskId;
}

export async function importModpackFromThunderstoreCode(
  thunderstoreId: string,
  game: string,
  profileId: string | undefined,
  modProgressChannel: Channel<ModProgressRegistration>,
  listener: Listener,
): Promise<string> {
  return await invokeWithListener(listener, (taskId) =>
    invoke("import_modpack_from_thunderstore_code", { thunderstoreId, game, profileId, modProgressChannel, taskId }),
  );
}
