import { createResource } from "solid-js";

import { getGames, getGamesPopularity, getProfiles } from "./api";
import { Game } from "./types";

export const [gamesResource] = createResource<[Game[], Map<string, Game>], unknown>(async () => {
  const games = await getGames();
  games.sort((a, b) => a.name.localeCompare(b.name));
  const byId = new Map(games.map((game) => [game.id, game]));
  return [games, byId];
});
export const games = () => gamesResource.latest![0];
export const gamesById = () => gamesResource.latest![1];

export const [gamesPopularityResource] = createResource<{ [key: string]: number }>(async () => {
  const gamesPopularity = await getGamesPopularity();

  return Object.freeze(gamesPopularity);
});
export const gamesPopularity = () => gamesPopularityResource.latest!;

export const [profiles, { refetch: refetchProfiles }] = createResource(
  async () => {
    const profiles = await getProfiles();
    profiles.sort((a, b) => a.name.localeCompare(b.name));
    return profiles;
  },
  { initialValue: [] }
);
