export const THUNDERSTORE_COMMUNITIES_API = "https://thunderstore.io/api/cyberstorm/community/";

export interface ThunderstoreCommunityApiResponse {
  count: number;
  next?: number; // Not sure if next or previous is a number as always null so far
  previous?: number;
  results: ThunderstoreCommunityGame[];
}

export interface ThunderstoreCommunityGame {
  name: string;
  identifier: string;
  short_description?: string;
  description: string;
  discord_url: string;
  wiki_url: string;
  /**
   * ISO timestamp
   */
  datetime_created: string;
  background_image_url: string;
  hero_image_url: string;
  cover_image_url: string;
  icon_url?: string;
  total_download_count: number;
  total_package_count: number;
}
