import { createResource } from "solid-js";

import { GameSortColumn, getGameModDownloads, getGames, getGamesPopularity, getProfiles, searchGames } from "./api";
import { Game } from "./types";

export const [gamesResource] = createResource<[readonly Game[], Map<string, Game>], never>(async () => {
  const games = Object.freeze(await getGames());
  const byId = new Map(games.map((game) => [game.id, game]));
  return [games, byId];
});
export const games = () => gamesResource.latest![0];
export const gamesById = () => gamesResource.latest![1];

export const [blankSearchGamesResource] = createResource<readonly number[], never>(async () => {
  return Object.freeze(await searchGames("", [{
    column: GameSortColumn.ModDownloads,
    descending: true,
  }]));
});
export const blankSearchGames = () => blankSearchGamesResource.latest!;

export const [gamesPopularityResource] = createResource<{ [key: string]: number }>(async () => {
  const gamesPopularity = await getGamesPopularity();

  return Object.freeze(gamesPopularity);
});
export const gamesPopularity = () => gamesPopularityResource.latest!;

export const [gamesModDownloadsResource] = createResource<{ [key: string]: number }>(async () => {
  const gameModsCount = await getGameModDownloads();

  return Object.freeze(gameModsCount);
});
export const gamesModDownloads = () => gamesModDownloadsResource.latest!;

export const [profiles, { refetch: refetchProfiles }] = createResource(
  async () => {
    const profiles = await getProfiles();
    profiles.sort((a, b) => a.name.localeCompare(b.name));
    return profiles;
  },
  { initialValue: [] },
);

// You can use this for testing splashscreen errors. Add it to coreResources.
// const [dummyResource] = createResource(() => {
//   return new Promise((_, reject) => setTimeout(() => reject("this is a made up error"), 2000));
// })

/**
 * The splashscreen will wait for these resources to load for a better user experience.
 */
export const coreResources = Object.freeze([gamesResource, blankSearchGamesResource, gamesPopularityResource, gamesModDownloadsResource, profiles]);
