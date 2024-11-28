export interface Game {
	id: string,
	name: string,
	thunderstore_id: string,
}

export interface Mod {
	name: string,
	full_name: string,
	owner: string,
	package_url?: string,
	donation_link?: string,
	date_created: string,
	date_updated: string,
	rating_score: number,
	is_pinned: boolean,
	is_deprecated: boolean,
	has_nsfw_content: boolean,
	categories: string[],
	versions: ModVersion[],
	uuid4: string,
}

export interface ModVersion {
	name: string,
	full_name: string,
	description: string,
	icon: string,
	version_number: string,
	dependencies: string[],
	download_url: string,
	downloads: number,
	date_updated: string,
	website_url?: string,
	is_active: boolean,
	uuid4: string,
	file_size: number,
}