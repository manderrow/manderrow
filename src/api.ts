import { invoke } from "@tauri-apps/api/core";
import { Game, Mod } from "./types";

export async function getGames(): Promise<Game[]> {
	return await invoke('get_games', {});
}

export async function fetchModIndex(game: string) {
	await invoke('fetch_mod_index', { game });
}

export async function queryModIndex(game: string, query: string): Promise<{
	mods: Mod[],
	count: number,
}> {
	return await invoke('query_mod_index', { game, query });
}