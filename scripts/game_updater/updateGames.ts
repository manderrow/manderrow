import { parseArgs } from "@std/cli";
import { ThunderstoreCommunityApiResponse } from "./types.d.ts";

enum UpdateMode {
  ALL = "all",
  ASSETS = "assets",
  GAMES = "games",
  DOWNLOADS = "downloads",
}

const THUNDERSTORE_COMMUNITIES_API =
  "https://thunderstore.io/api/cyberstorm/community/?ordering=-aggregated_fields__package_count";

const flags = parseArgs(Deno.args, {
  string: ["mode"],
  default: { mode: "all" },
});

const mode = flags.mode.toLowerCase() as UpdateMode;

const textEncoder = new TextEncoder();

const gamesRequests = await fetch(THUNDERSTORE_COMMUNITIES_API);
const { results: games } = (await gamesRequests.json()) as ThunderstoreCommunityApiResponse;

if (mode === UpdateMode.ALL || mode === UpdateMode.DOWNLOADS) {
  const sorted: Record<string, number> = {};
  for (const game of games) {
    sorted[game.identifier] = game.total_download_count;
  }
  Deno.writeFile("../../src-tauri/src/gameModDownloads.json", textEncoder.encode(JSON.stringify(sorted, null, 2)));
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
