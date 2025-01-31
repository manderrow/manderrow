export type Refetcher<T, R = unknown> = (info?: R) => T | Promise<T> | undefined | null;

export interface Game {
  id: string;
  name: string;
  data_folder_name: string;
  display_mode: number;
  exclusions_url: string;
  exe_names: string[];
  game_image: string;
  instance_type: "Game" | "Server";
  package_loader: PackageLoader;
  settings_identifier: string;
  steam_folder_name: string;
  store_platform_metadata: StorePlatformMetadata[];
  thunderstore_url: string;
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
  | (({ _storePlatform: "Steam" } | { _storePlatform: "SteamDirect" } | { _storePlatform: "Epic" } | { _storePlatform: "Xbox" }) & { store_identifier: string })
  | { _storePlatform: "Oculus" }
  | { _storePlatform: "Origin" }
  | { _storePlatform: "Other" };

export interface Mod {
  name: string;
  full_name: string;
  owner: string;
  package_url?: string;
  donation_link?: string;
  date_created: string;
  date_updated: string;
  rating_score: number;
  is_pinned: boolean;
  is_deprecated: boolean;
  has_nsfw_content: boolean;
  categories: string[];
  versions: ModVersion[];
  uuid4: string;
}

export interface ModVersion {
  name: string;
  full_name: string;
  description: string;
  icon: string;
  version_number: string;
  dependencies: string[];
  download_url: string;
  downloads: number;
  date_created: string;
  website_url?: string;
  is_active: boolean;
  uuid4: string;
  file_size: number;
}

export interface ModAndVersion {
  // This interface does not extend Mod because it would need to copy all the keys from mod into itself.
  // Instead, mod will be stored as a reference here.
  mod: Mod;
  version?: string;
}
