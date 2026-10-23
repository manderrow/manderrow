// The shape of src/games/games.json, which src/games/mod.rs deserializes.

export interface Game {
  id: string;
  name: string;
  exeNames: string[];
  instanceType: "Game" | "Server";
  packageLoader: PackageLoader;
  storePlatformMetadata: StorePlatformMetadata[];
  thunderstoreId: string;
  thunderstoreUrl: string;
}

export type PackageLoader =
  | "BepInEx"
  | "MelonLoader"
  | "NorthStar"
  | "GodotML"
  | "AncientDungeonVR"
  | "ShimLoader"
  | "Lovely"
  | "ReturnOfModding"
  | "GDWeave";

export type StorePlatformMetadata =
  | ((
      | (({ storePlatform: "Steam" } | { storePlatform: "SteamDirect" }) & { storePageIdentifier: string })
      | { storePlatform: "Epic" }
      | { storePlatform: "Xbox" }
    ) & { storeIdentifier: string })
  | { storePlatform: "Oculus" }
  | { storePlatform: "Origin" }
  | { storePlatform: "Other" };
