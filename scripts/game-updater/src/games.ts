import { projectRootDir } from "./paths.ts";

import { Game } from "../../../src/types.d.ts";

import games from "../../../src-tauri/src/games/games.json" with { type: "json" };
export const gamesJsonPath = projectRootDir + "/src-tauri/src/games/games.json";

export default games as Game[];