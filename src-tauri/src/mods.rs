use uuid::Uuid;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Mod<'a> {
    pub name: &'a str,
    pub full_name: &'a str,
    pub owner: &'a str,
    #[serde(default)]
    pub package_url: Option<&'a str>,
    pub donation_link: Option<&'a str>,
    pub date_created: &'a str,
    pub date_updated: &'a str,
    pub rating_score: u32,
    pub is_pinned: bool,
    pub is_deprecated: bool,
    pub has_nsfw_content: bool,
    pub categories: Vec<&'a str>,
    pub versions: Vec<ModVersion<'a>>,
    pub uuid4: Uuid,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModVersion<'a> {
    pub name: &'a str,
    pub full_name: &'a str,
    pub description: &'a str,
    pub icon: &'a str,
    pub version_number: &'a str,
    pub dependencies: Vec<&'a str>,
    pub download_url: &'a str,
    pub downloads: u64,
    pub date_created: &'a str,
    pub website_url: Option<&'a str>,
    pub is_active: bool,
    pub uuid4: Uuid,
    pub file_size: u64,
}