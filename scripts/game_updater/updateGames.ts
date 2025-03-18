import { parseArgs } from "@std/cli";

import { ThunderstoreCommunityApiResponse, ThunderstoreCommunityGame } from "./types.d.ts";

import games from "../../src-tauri/src/games/games.json" with {type: "json"}

enum UpdateMode {
  ALL = "all",
  ASSETS = "assets",
  GAMES = "games",
  DOWNLOADS = "downloads",
}

const THUNDERSTORE_COMMUNITIES_API =
  "https://thunderstore.io/api/cyberstorm/community/";

const flags = parseArgs(Deno.args, {
  string: ["mode"],
  default: { mode: "all" },
});

const mode = flags.mode.toLowerCase() as UpdateMode;

const textEncoder = new TextEncoder();

if (mode === UpdateMode.ALL || mode === UpdateMode.DOWNLOADS) {
  const sorted: [string, number][] = [];
  const thunderstoreIds = new Set(games.map(game => game.thunderstoreId));
  const { results } = await (await fetch(THUNDERSTORE_COMMUNITIES_API)).json() as ThunderstoreCommunityApiResponse;
  for (const game of results) {
    sorted.push([game.identifier, game.total_download_count]);
    thunderstoreIds.delete(game.identifier);
  }
  await Promise.all(Array.from(thunderstoreIds).map(async thunderstoreId => {
    const resp = await fetch(THUNDERSTORE_COMMUNITIES_API + thunderstoreId);
    if (resp.status !== 200) {
      throw new Error(`${thunderstoreId}: ${resp.status} ${resp.statusText}`);
    }
    const { total_download_count } = (await resp.json()) as ThunderstoreCommunityGame;
    sorted.push([thunderstoreId, total_download_count]);
  }));
  sorted.sort(([a, _], [b, __]) => a.localeCompare(b));
  Deno.writeFile("../../src-tauri/src/games/gameModDownloads.json", textEncoder.encode(JSON.stringify(Object.fromEntries(sorted), null, 2)));
}

async function getImage(url: string) {
  return await (await fetch(url)).bytes();
}
if (mode === UpdateMode.ALL || mode === UpdateMode.ASSETS) {
  for (const game of games) {
    const imagesToInclude = [
      {
        folder: "game_covers",
        url: game.cover_image_url,
      },
    ];

    Promise.allSettled(
      imagesToInclude
        .filter(({ url }) => url != null)
        .map(({ folder, url }) =>
          getImage(url)
            .then((data) => Deno.writeFile(`../../public/img/${folder}/${game.identifier}.png`, data))
            .catch((error) => console.log(`Error occurred fetching ${game.name}'s background image\n\n${error}`)),
        ),
    );
  }
}
