import { invoke } from "@tauri-apps/api/core";
import { Game, Mod } from "./types";

async function wrapInvoke<T>(f: () => Promise<T>): Promise<T> {
	try {
		return await f();
	} catch (e: any) {
		throw new Error(`${e.message}\nBacktrace:\n${e.backtrace}`);
	}
}

export async function getGames(): Promise<Game[]> {
	return await wrapInvoke(async () => await invoke('get_games', {}));
}

export async function fetchModIndex(game: string, options: { refresh: boolean }) {
	await wrapInvoke(async () => await invoke('fetch_mod_index', { game, ...options }));
}

export enum SortColumn {
	Relevance = "Relevance",
	Downloads = "Downloads",
	Name = "Name",
	Owner = "Owner",
}

export interface SortOption {
	column: SortColumn,
	descending: boolean,
}

export async function queryModIndex(game: string, query: string, sort: SortOption[]): Promise<{
	mods: Mod[],
	count: number,
}> {
	return await wrapInvoke(async () => await invoke('query_mod_index', { game, query, sort }));
}