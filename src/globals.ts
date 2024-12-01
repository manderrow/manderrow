import { createResource } from "solid-js";
import { getGames } from "./api";

export const [games] = createResource(getGames);
export const [gamesById] = createResource(games, (games) => new Map(games.map((game) => [game.id, game])));
