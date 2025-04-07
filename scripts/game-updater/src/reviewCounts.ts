import * as cheerio from "cheerio";

import games from "./games.ts";
import { projectRootDir } from "./paths.ts";

const steamStoreIdentifiers = games.map(
  (gameInfo) =>{
    const metadata = gameInfo.storePlatformMetadata.find(
      (meta) => meta.storePlatform === "Steam" || meta.storePlatform === "SteamDirect",
    );
    if (metadata == null) return null;
    return metadata.storePageIdentifier ?? metadata.storeIndentifier;
  },
);

const textEncoder = new TextEncoder();

const adultAge = `birthtime=${Math.floor(new Date(2006, 1, 1).getTime() / 1000)}`;

export async function scrapeSteam() {
  const gameReviews = await Promise.all(
    steamStoreIdentifiers.map(async (id) => {
      if (id === undefined) return { reviewCount: null };
      const url = `https://store.steampowered.com/app/${id}`;
      try {
        const request = await fetch(url, {
          headers: {
            Cookie: adultAge,
          },
          redirect: "error",
        });

        const $ = cheerio.load(await request.text());

        const reviewCount = $("meta[itemprop=reviewCount]").attr("content");

        if (reviewCount == null) {
          console.error(`${url} loaded but had no review count!`);
        }

        return { reviewCount };
      } catch (err) {
        if (
          err instanceof TypeError &&
          err.message === "Fetch failed: Encountered redirect while redirect mode is set to 'error'"
        ) {
          console.error(`${url} redirected`);
        } else {
          console.error(`${url} failed to resolve: `, err);
        }

        return { reviewCount: null };
      }
    }),
  );

  const gameFinal: Record<string, number | null> = {};

  for (let i = 0; i < gameReviews.length; i++) {
    if (gameFinal[games[i].thunderstoreId] != null) continue;

    const reviews = gameReviews[i]?.reviewCount;
    gameFinal[games[i].thunderstoreId] = reviews != null ? parseInt(reviews) : null;
  }

  const gameJSON = JSON.stringify(gameFinal, null, 2);

  await Deno.writeFile(projectRootDir + "/src-tauri/src/games/gameReviews.json", textEncoder.encode(gameJSON));
}
