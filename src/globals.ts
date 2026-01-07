import { createResource, createSignal } from "solid-js";

import { GameSortColumn, getGameModDownloads, getGames, getGamesPopularity, getProfiles, searchGames } from "./api/api";
import { Game } from "./types";
import { settingsResource, settingsUIResource } from "./api/settings";
import { createSignalResource } from "./utils/utils";

export const [gamesResource] = createResource<[readonly Game[], Map<string, Game>], never>(async () => {
  const games = Object.freeze(await getGames());
  const byId = new Map(games.map((game) => [game.id, game]));
  return [games, byId];
});
export const games = () => gamesResource.latest![0];
export const gamesById = () => gamesResource.latest![1];

export const [initialSortedGamesResource] = createResource<readonly number[], never>(async () => {
  return Object.freeze(
    await searchGames("", [
      {
        column: GameSortColumn.ModDownloads,
        descending: true,
      },
    ]),
  );
});
export const initialSortedGames = () => initialSortedGamesResource.latest!;

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

export const [profiles, { refetch: refetchProfiles }] = createResource(async () => {
  const profiles = await getProfiles();
  profiles.sort((a, b) => a.name.localeCompare(b.name));
  return profiles;
});

export const initialGame = createSignalResource(async () => (await settingsResource.loaded).defaultGame.value);

const [_shifting, setShifting] = createSignal(false);
const [_ctrling, setCtrling] = createSignal(false);

export const shifting = _shifting;
export const ctrling = _ctrling;

function onShiftDown(e: KeyboardEvent) {
  if (e.key === "Shift") setShifting(true);
  if (e.key === "Control") setCtrling(true);
}
function onShiftUp(e: KeyboardEvent) {
  if (e.key === "Shift") setShifting(false);
  if (e.key === "Control") setCtrling(false);
}

document.addEventListener("keydown", onShiftDown);
document.addEventListener("keyup", onShiftUp);

// You can use this for testing splashscreen errors. Add it to coreResources.
// const [dummyResource] = createResource(() => {
//   return new Promise((_, reject) => setTimeout(() => reject("this is a made up error"), 2000));
// })

/**
 * The splashscreen will wait for these resources to load for a better user experience.
 */
export const coreResources = Object.freeze([
  settingsResource,
  settingsUIResource,
  gamesResource,
  initialSortedGamesResource,
  gamesPopularityResource,
  gamesModDownloadsResource,
  profiles,
  initialGame,
]);
