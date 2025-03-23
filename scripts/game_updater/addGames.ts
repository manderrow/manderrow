import { ThunderstoreCommunityApiResponse, ThunderstoreCommunityGame } from "./types.d.ts";
import { Game, StorePlatformMetadata } from "../../src/types.d.ts";
import { TextLineStream } from "@std/streams";

const gamesJsonPath = "../../src-tauri/src/games/games.json";
import existingGames from "../../src-tauri/src/games/games.json" with {type: "json"}

const THUNDERSTORE_COMMUNITIES_API =
  "https://thunderstore.io/api/cyberstorm/community/";

const gamesRequests = await fetch(THUNDERSTORE_COMMUNITIES_API);
const { results: thunderstoreGames } = (await gamesRequests.json()) as ThunderstoreCommunityApiResponse;

const existingGamesMap = new Map(existingGames.map((game) => [game.id, game]));

const diff: ThunderstoreCommunityGame[] = [];

for (const game of thunderstoreGames) {
  if (!existingGamesMap.has(game.identifier)) diff.push(game);
}

const textEncoder = new TextEncoder();

const fields: (keyof Game)[] = [
  "packageLoader",
  "exeNames",
  "instanceType",
  "storePlatformMetadata",
];

const reader = Deno.stdin.readable.pipeThrough(new TextDecoderStream()).pipeThrough(new TextLineStream()).getReader();

async function getLine(): Promise<string> {
  return (await reader.read()).value!;
}

function shouldBreak(text: string) {
  return text.toLowerCase() === "done";
}

for (let i = 0; i < diff.length; ) {
  const game = diff[i];

  console.log(`${i + 1}/${diff.length}`);
  console.log(`Game: ${game.name} (${game.identifier})`);
  console.log(`Skip? (Input anything other than blank to skip)`);

  if ((await getLine()).trim().length !== 0) {
    console.log("Skipping...\n");
    i++;
    continue;
  };

  const gameData: Partial<Game> = {
    id: game.identifier,
    thunderstoreId: game.identifier,
    name: game.name,
    thunderstoreUrl: `https://thunderstore.io/c/${game.identifier}/api/v1/package-listing-index/`,
  };

  for (const field of fields) {
    switch (field) {
      case "storePlatformMetadata": {
        const storePlatformMetadata: StorePlatformMetadata[] = [];

        console.log("Enter store platform metadata (type done to finish)");
        while (true) {
          const metadata: Record<string, string> = {};

          console.log("Enter store platform: ");
          const platform = await getLine();
          metadata.storePlatform = platform as keyof StorePlatformMetadata;
          if (shouldBreak(platform)) break;

          console.log("Enter store identifier: ");
          const id = await getLine();
          if (id.trim() != null) metadata.storeIdentifier = id;
          if (shouldBreak(id)) break;

          storePlatformMetadata.push(metadata as StorePlatformMetadata);
        }
        gameData.storePlatformMetadata = storePlatformMetadata;

        break;
      }
      case "exeNames": {
        const exeNames: string[] = [];
        console.log(`Enter exe names (type done to finish): `);
        while (true) {
          console.log(`Enter an exe name: `);
          const text = await getLine();
          if (shouldBreak(text)) break;

          exeNames.push(text);
        }
        gameData.exeNames = exeNames;

        break;
      }
      default: {
        console.log(`Enter ${field}: `);
        const text = await getLine();
        // @ts-ignore Validation performed by conditions
        gameData[field] = text;
      }
    }
  }

  console.table(gameData);
  console.log("Does the above information look good? (Y/N)");

  if ((await getLine()).toLowerCase() === "y") {
    i++;

    existingGames.push(gameData as Game);

    Deno.writeFile(gamesJsonPath, textEncoder.encode(JSON.stringify(existingGames, null, 2)));
  } else {
    console.log("Restarting...");
  }

  console.log(`----`);
}
