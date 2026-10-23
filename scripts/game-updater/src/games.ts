import { projectRootDir } from "./paths.ts";

import type { Game } from "./types.d.ts";

import games from "../../../src/games/games.json" with { type: "json" };
export const gamesJsonPath = projectRootDir + "/src/games/games.json";

export default games as Game[];