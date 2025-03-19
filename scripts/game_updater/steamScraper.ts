import * as cheerio from "cheerio";
import games from "../../src-tauri/src/games/games.json" with {type: "json"};

const steamSupportedGames = games.filter((gameInfo) => gameInfo.storePlatformMetadata[0].storeIdentifier != null);
const steamGameIds = steamSupportedGames.map((gameInfo) => gameInfo.storePlatformMetadata[0].storeIdentifier);
const thunderstoreGameUrls = steamSupportedGames.map((gameInfo) => gameInfo.thunderstoreUrl);

const gameIds: string[] = [];

const textEncoder = new TextEncoder();

for (let i = 0; i < thunderstoreGameUrls.length; i++) {
  const url = new URL(thunderstoreGameUrls[i]);
  const thunderstoreId = url.pathname.slice(1).split("/")[1];

  gameIds.push(thunderstoreId);
}

const adultAge = `birthtime=${Math.floor(new Date(2006, 1, 1).getTime() / 1000)}`;

async function scrape() {
  const gameReviews = await Promise.all(
    steamGameIds.map(async (id) => {
      try {
        const request = await fetch("https://store.steampowered.com/app/" + id, {
          headers: {
            Cookie: adultAge,
          },
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
    if (gameFinal[gameIds[i]] != null) continue;

    const reviews = gameReviews[i]?.reviewCount;
    gameFinal[gameIds[i]] = reviews != null ? parseInt(reviews) : null;
  }

  const gameJSON = JSON.stringify(gameFinal, null, 2);

  await Deno.writeFile("../../src-tauri/src/games/gameReviews.json", textEncoder.encode(gameJSON));
}

await scrape();
