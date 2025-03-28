import { parseArgs } from "@std/cli";
import { exit } from "node:process";

import addGames from "./src/addGames.ts";
import scrapeModDownloads from "./src/modDownloads.ts";
import { scrapeSteam as scrapeReviewCounts } from "./src/reviewCounts.ts";

enum UpdateMode {
  ALL = "all",
  REVIEW_COUNTS = "review-counts",
  MOD_DOWNLOADS = "mod-downloads",
}

const args: {
  _: [string];
  mode: string;
} = parseArgs(Deno.args, {
  string: ["mode"],
  default: { mode: UpdateMode.ALL },
});

switch (args._[0]) {
  case null:
    throw new Error("Usage: game-updater [OPTIONS] COMMAND [OPTIONS]");
  case "scrape":
    if (
      args.mode !== UpdateMode.ALL &&
      args.mode !== UpdateMode.MOD_DOWNLOADS &&
      args.mode !== UpdateMode.REVIEW_COUNTS
    ) {
      throw new Error(`Invalid scraping mode: ${args.mode}`);
    }

    await Promise.allSettled([
      args.mode === UpdateMode.ALL || args.mode === UpdateMode.MOD_DOWNLOADS ? scrapeModDownloads() : undefined,
      args.mode === UpdateMode.ALL || args.mode === UpdateMode.REVIEW_COUNTS ? scrapeReviewCounts() : undefined,
    ]);
    break;
  case "add-games":
    await addGames();
    // Not sure why, but the script hangs here without this call
    exit(0);
    break;
  default:
    throw new Error(`Unrecognized command: ${args._[0]}`);
}
