import { createResource, createSignal } from "solid-js";
import { getGames } from "./api";
import { Game } from "./types";
import { fetchDictionary, Locale } from "./i18n/i18n";
import * as i18n from "@solid-primitives/i18n";

export const [gamesResource] = createResource<[Game[], Map<string, Game>], unknown>(async () => {
  const games = await getGames();
  games.sort((a, b) => a.name.localeCompare(b.name));
  const byId = new Map(games.map((game) => [game.id, game]));
  return [games, byId];
});
export const games = () => gamesResource.latest![0];
export const gamesById = () => gamesResource.latest![1];

// export const [locale, setLocale] = createSignal<Locale>("en_ca");
// export const [dict] = createResource(locale, fetchDictionary);
