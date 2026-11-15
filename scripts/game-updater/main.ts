import { exit } from "node:process";
import { parseArgs } from "node:util";

import addGames from "./src/addGames.ts";
import scrapeModDownloads from "./src/modDownloads.ts";
import { scrapeSteam as scrapeReviewCounts } from "./src/reviewCounts.ts";

enum UpdateMode {
  ALL = "all",
  REVIEW_COUNTS = "review-counts",
  MOD_DOWNLOADS = "mod-downloads",
}

const {
  values: { mode },
  positionals: [command],
} = parseArgs({
  args: Bun.argv.slice(2),
  options: { mode: { type: "string", default: UpdateMode.ALL } },
  allowPositionals: true,
});

switch (command) {
  case undefined:
    throw new Error("Usage: game-updater [OPTIONS] COMMAND [OPTIONS]");
  case "scrape":
    if (mode !== UpdateMode.ALL && mode !== UpdateMode.MOD_DOWNLOADS && mode !== UpdateMode.REVIEW_COUNTS) {
      throw new Error(`Invalid scraping mode: ${mode}`);
    }

    await Promise.allSettled([
      mode === UpdateMode.ALL || mode === UpdateMode.MOD_DOWNLOADS ? scrapeModDownloads() : undefined,
      mode === UpdateMode.ALL || mode === UpdateMode.REVIEW_COUNTS ? scrapeReviewCounts() : undefined,
    ]);
    break;
  case "add-games":
    await addGames();
    // Not sure why, but the script hangs here without this call
    exit(0);
    break;
  default:
    throw new Error(`Unrecognized command: ${command}`);
}
