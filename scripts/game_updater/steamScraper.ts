import * as cheerio from "cheerio";
import games from "../../src-tauri/src/games/games.json" with {type: "json"};

const steamStoreIdentifiers = games.map(
  (gameInfo) =>
    gameInfo.storePlatformMetadata.find(
      (meta) => meta.storePlatform === "Steam" || meta.storePlatform === "SteamDirect",
    )?.storeIdentifier,
);

const textEncoder = new TextEncoder();

const adultAge = `birthtime=${Math.floor(new Date(2006, 1, 1).getTime() / 1000)}`;

async function scrape() {
  const gameReviews = await Promise.all(
    steamStoreIdentifiers.map(async (id) => {
      if (id === undefined) return { reviewCount: null };
      try {
        const request = await fetch("https://store.steampowered.com/app/" + id, {
          headers: {
            Cookie: adultAge,
          },
          redirect: "error",
        });

        const $ = cheerio.load(await request.text());

        const reviewCount = $("meta[itemprop=reviewCount]").attr("content");

        if (reviewCount == null) {
          console.error(`https://store.steampowered.com/app/${id} loaded but had no review count!`);
        }

        return { reviewCount };
      } catch (err) {
        console.error(`https://store.steampowered.com/app/${id} failed to resolve: `, err);

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

  await Deno.writeFile("../../src-tauri/src/games/gameReviews.json", textEncoder.encode(gameJSON));
}

await scrape();
