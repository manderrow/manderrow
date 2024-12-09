use rkyv_intern::Intern;
use uuid::Uuid;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModRef<'a> {
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
    pub versions: Vec<ModVersionRef<'a>>,
    pub uuid4: Uuid,
}

#[derive(
    Debug,
    Clone,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
    serde::Deserialize,
    serde::Serialize,
)]
#[rkyv(derive(Debug))]
pub struct Mod {
    #[rkyv(with = Intern)]
    pub name: String,
    pub full_name: String,
    #[rkyv(with = Intern)]
    pub owner: String,
    pub package_url: Option<String>,
    pub donation_link: Option<String>,
    #[rkyv(with = Intern)]
    pub date_created: String,
    #[rkyv(with = Intern)]
    pub date_updated: String,
    pub rating_score: u32,
    pub is_pinned: bool,
    pub is_deprecated: bool,
    pub has_nsfw_content: bool,
    pub categories: Vec<InternedString>,
    pub versions: Vec<ModVersion>,
    pub uuid4: Uuid,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModVersionRef<'a> {
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

#[derive(
    Debug,
    Clone,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
    serde::Deserialize,
    serde::Serialize,
)]
#[rkyv(derive(Debug))]
pub struct ModVersion {
    #[rkyv(with = Intern)]
    pub name: String,
    pub full_name: String,
    #[rkyv(with = Intern)]
    pub description: String,
    #[rkyv(with = Intern)]
    pub icon: String,
    #[rkyv(with = Intern)]
    pub version_number: String,
    pub dependencies: Vec<InternedString>,
    pub download_url: String,
    pub downloads: u64,
    pub date_created: String,
    pub website_url: Option<String>,
    pub is_active: bool,
    pub uuid4: Uuid,
    pub file_size: u64,
}

#[derive(
    Debug,
    Clone,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
    serde::Deserialize,
    serde::Serialize,
)]
#[rkyv(derive(Debug))]
#[serde(transparent)]
pub struct InternedString(#[rkyv(with = Intern)] pub String);
