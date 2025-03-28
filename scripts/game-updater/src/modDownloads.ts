import games from "./games.ts";
import { projectRootDir } from "./paths.ts";
import {
  THUNDERSTORE_COMMUNITIES_API,
  type ThunderstoreCommunityApiResponse,
  type ThunderstoreCommunityGame,
} from "./thunderstore.ts";

const textEncoder = new TextEncoder();

export default async function scrapeDownloads() {
  const sorted: [string, number][] = [];
  const thunderstoreIds = new Set(games.map((game) => game.thunderstoreId));
  const { results } = (await (await fetch(THUNDERSTORE_COMMUNITIES_API)).json()) as ThunderstoreCommunityApiResponse;
  for (const game of results) {
    if (!thunderstoreIds.delete(game.identifier)) {
      console.warn(
        `Thunderstore lists ${game.identifier} with ${game.total_download_count} mod downloads, but we don't have it`,
      );
      continue;
    }
    sorted.push([game.identifier, game.total_download_count]);
  }
  await Promise.all(
    Array.from(thunderstoreIds).map(async (thunderstoreId) => {
      const resp = await fetch(THUNDERSTORE_COMMUNITIES_API + thunderstoreId);
      if (resp.status !== 200) {
        throw new Error(`${thunderstoreId}: ${resp.status} ${resp.statusText}`);
      }
      const { total_download_count } = (await resp.json()) as ThunderstoreCommunityGame;
      sorted.push([thunderstoreId, total_download_count]);
    }),
  );
  sorted.sort(([a, _], [b, __]) => a.localeCompare(b));
  Deno.writeFile(
    projectRootDir + "/src-tauri/src/games/gameModDownloads.json",
    textEncoder.encode(JSON.stringify(Object.fromEntries(sorted), null, 2)),
  );
}
