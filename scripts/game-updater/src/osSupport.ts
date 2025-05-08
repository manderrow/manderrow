import * as cheerio from "cheerio";

import games from "./games.ts";

const steamStoreIdentifiers = games.map((gameInfo) => {
  const metadata = gameInfo.storePlatformMetadata.find(
    (meta) => meta.storePlatform === "Steam" || meta.storePlatform === "SteamDirect",
  );
  if (metadata == null) return null;
  return metadata.storePageIdentifier ?? metadata.storeIdentifier;
});

const adultAge = `birthtime=${Math.floor(new Date(2006, 1, 1).getTime() / 1000)}`;

const gamePlatforms = await Promise.all(
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

      const platformImgs = $(".platform_img");

      const data = { win: false, mac: false, linux: false };
      for (const platformImg of platformImgs.toArray().map(e => $(e))) {
        if (platformImg.hasClass("win")) {
          data.win = true;
        } else if (platformImg.hasClass("mac")) {
          data.mac = true;
        } else if (platformImg.hasClass("linux")) {
          data.linux = true;
        }
      }

      return data;
    } catch (err) {
      if (
        err instanceof TypeError &&
        err.message === "Fetch failed: Encountered redirect while redirect mode is set to 'error'"
      ) {
        console.error(`${url} redirected`);
      } else {
        console.error(`${url} failed to resolve: `, err);
      }

      return {};
    }
  }),
);

for (let i = 0; i < gamePlatforms.length; i++) {
  const platforms = gamePlatforms[i];
  console.log(games[i].thunderstoreId);
  if (platforms.win) {
    console.log("  - Windows");
  }
  if (platforms.mac) {
    console.log("  - macOS");
  }
  if (platforms.linux) {
    console.log("  - Linux");
  }
}
