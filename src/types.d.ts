declare module "solid-js" {
  namespace JSX {
    interface Directives {
      autofocus: true;
      bindValue: Signal<string>;
    }
  }
}

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

export enum PackageLoader {
  BepInEx = "BepInEx",
  MelonLoader = "MelonLoader",
  NorthStar = "NorthStar",
  GodotML = "GodotML",
  AncientDungeonVR = "AncientDungeonVR",
  ShimLoader = "ShimLoader",
  Lovely = "Lovely",
  ReturnOfModding = "ReturnOfModding",
  GDWeave = "GDWeave",
}

export type StorePlatformMetadata =
  | ((
      | (({ storePlatform: "Steam" } | { storePlatform: "SteamDirect" }) & { storePageIdentifier: string })
      | { storePlatform: "Epic" }
      | { storePlatform: "Xbox" }
    ) & { storeIdentifier: string })
  | { storePlatform: "Oculus" }
  | { storePlatform: "Origin" }
  | { storePlatform: "Other" };

export type Mod = ModListing | ModPackage;

export interface ModMetadata {
  name: string;
  owner: string;
  donation_link?: string;
  date_created: string;
  date_updated: string;
  rating_score: number;
  is_pinned: boolean;
  is_deprecated: boolean;
  has_nsfw_content: boolean;
  categories: string[];
}

/**
 * A mod listing with all available versions.
 */
export interface ModListing extends ModMetadata {
  versions: ModVersion[];
}

/**
 * A versioned mod package.
 */
export interface ModPackage extends ModMetadata {
  version: ModVersion;
}

export interface ModVersion {
  description: string;
  version_number: string;
  dependencies: string[];
  downloads: number;
  date_created: string;
  website_url?: string;
  is_active: boolean;
  file_size: number;
}
