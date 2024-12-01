import { createResource } from "solid-js";
import { getGames } from "./api";

export const [games] = createResource(async () => {
  const games = await getGames();
  games.sort((a, b) => a.name.localeCompare(b.name));
  return games;
});
export const [gamesById] = createResource(games, (games) => new Map(games.map((game) => [game.id, game])));
