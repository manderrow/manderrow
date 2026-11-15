import { projectRootDir } from "./paths.ts";

import type { Game } from "../../../src/types.d.ts";

import games from "../../../src-tauri/src/games/games.json" with { type: "json" };
export const gamesJsonPath = projectRootDir + "/src-tauri/src/games/games.json";

// games.json's text: JSON.stringify's layout, but with arrays of strings, like exeNames, on one line, as Prettier lays
// them out at a print width of 120
export function gamesJson(games: Game[]): string {
  return (
    JSON.stringify(games, null, 2).replace(
      /\[\n\s*("[^\n]*"(?:,\n\s*"[^\n]*")*)\n\s*\]/g,
      (_, items: string) => `[${items.replace(/,\n\s*/g, ", ")}]`,
    ) + "\n"
  );
}

export default games as Game[];