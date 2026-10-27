//! The `Mod`s and `ModVersion`s that QML is given: listings and summaries from the mod index, and
//! the mods installed in profiles.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!(<QtCore/QObject>);
        include!("cxx-qt-lib/qlist.h");
        type QObjectList = crate::qt::QObjectList;
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;
        include!("cxx-qt-lib/qdatetime.h");
        type QDateTime = cxx_qt_lib::QDateTime;
        include!("manderrow/src/qt/call.cxxqt.h");
        type Call = crate::qt::call::Call;
    }

    #[auto_cxx_name]
    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_uncreatable]
        #[qproperty(QString, version_number, READ, CONSTANT)]
        #[qproperty(QString, description, READ, CONSTANT)]
        /// The versions it depends on, as `Owner-Name-1.2.3`.
        #[qproperty(QStringList, dependencies, READ, CONSTANT)]
        #[qproperty(f64, downloads, READ, CONSTANT)]
        #[qproperty(QDateTime, date_created, READ, CONSTANT)]
        #[qproperty(QString, website_url, READ, CONSTANT)]
        #[qproperty(bool, is_active, READ, CONSTANT)]
        /// The size of its package, in bytes.
        #[qproperty(f64, file_size, READ, CONSTANT)]
        type ModVersion = super::ModVersionRust;

        #[qobject]
        #[qml_element]
        #[qml_uncreatable]
        #[qproperty(QString, owner, READ, CONSTANT)]
        #[qproperty(QString, name, READ, CONSTANT)]
        #[qproperty(QString, donation_link, READ, NOTIFY = changed)]
        #[qproperty(QDateTime, date_created, READ, NOTIFY = changed)]
        #[qproperty(bool, is_deprecated, READ, NOTIFY = changed)]
        #[qproperty(bool, has_nsfw_content, READ, NOTIFY = changed)]
        #[qproperty(QStringList, categories, READ, NOTIFY = changed)]
        /// Its `ModVersion`s, from the latest: all of them for a listing, or the latest for a
        /// summary, as a list of mods shows.
        #[qproperty(QObjectList, versions, READ, NOTIFY = changed)]
        /// The downloads of all of its versions, for a summary.
        #[qproperty(f64, total_downloads, READ, NOTIFY = changed)]
        /// The version installed, for a mod installed in a profile, or null.
        #[qproperty(*mut ModVersion, version, READ, NOTIFY = changed)]
        /// The version it's shown with: the version installed, or the latest.
        #[qproperty(*mut ModVersion, display_version, READ, NOTIFY = changed)]
        type Mod = super::ModRust;

        #[qsignal]
        fn changed(self: Pin<&mut Mod>);

        /// Fetches the README of one of its versions. The call returns its Markdown, which is
        /// empty if it has none.
        #[qinvokable]
        fn readme(self: &Mod, version: &QString) -> *mut Call;

        /// Fetches the changelog of one of its versions, as `readme` does its README.
        #[qinvokable]
        fn changelog(self: &Mod, version: &QString) -> *mut Call;
    }

    #[namespace = "rust::cxxqtlib1"]
    unsafe extern "C++" {
        include!("cxx-qt-lib/common.h");
        #[rust_name = "new_mod_object"]
        fn make_unique() -> UniquePtr<Mod>;
        #[rust_name = "new_version_object"]
        fn make_unique() -> UniquePtr<ModVersion>;
    }
}

use core::pin::Pin;

use cxx::UniquePtr;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QDateTime, QString, QStringList, QTimeZone, QVariant};
use manderrow_types::mods::{ArchivedModRef, ArchivedModVersionRef, Timestamp};
use packed_semver::Version;

use super::call::{self, Call};
use super::child::Child;
use super::QObjectList;
use crate::mod_index::thunderstore::{self, ModMarkdown};
use crate::profiles::{ManifestVersion, ModManifest};
use crate::{tasks, Reqwest};

pub use qobject::{Mod, ModVersion};

/// What a `Mod` shows, which the backend sends the Qt thread to make one of.
pub struct ModInfo {
    pub owner: String,
    pub name: String,
    pub donation_link: Option<String>,
    pub date_created: Timestamp,
    pub is_deprecated: bool,
    pub has_nsfw_content: bool,
    pub categories: Vec<String>,
    pub versions: Vec<VersionInfo>,
    pub total_downloads: u64,
    pub version: Option<VersionInfo>,
}

/// What a `ModVersion` shows.
pub struct VersionInfo {
    pub version_number: Version,
    pub description: String,
    pub dependencies: Vec<String>,
    pub downloads: u64,
    pub date_created: Timestamp,
    pub website_url: Option<String>,
    pub is_active: bool,
    pub file_size: u64,
}

impl ModInfo {
    /// A mod in the mod index, with all of its versions.
    pub fn listing(m: &ArchivedModRef) -> Self {
        Self::from_index(m, m.versions.iter().map(VersionInfo::from_index).collect())
    }

    /// A mod in the mod index as a list shows it, with only its latest version, and the
    /// downloads of all of them. Most of a mod's size is in its older versions.
    pub fn summary(m: &ArchivedModRef) -> Self {
        Self::from_index(
            m,
            m.versions
                .first()
                .map(VersionInfo::from_index)
                .into_iter()
                .collect(),
        )
    }

    fn from_index(m: &ArchivedModRef, versions: Vec<VersionInfo>) -> Self {
        Self {
            owner: m.owner.to_string(),
            name: m.name.to_string(),
            donation_link: m.donation_link.as_ref().map(|link| link.to_string()),
            date_created: m.date_created.into(),
            is_deprecated: m.is_deprecated,
            has_nsfw_content: m.has_nsfw_content,
            categories: m
                .categories
                .iter()
                .map(|category| category.to_string())
                .collect(),
            versions,
            total_downloads: m.versions.iter().map(|v| v.downloads.to_native()).sum(),
            version: None,
        }
    }

    /// A mod installed in a profile.
    pub fn installed(manifest: ModManifest) -> Self {
        let version = VersionInfo::from_manifest(manifest.version);
        Self {
            owner: manifest.owner,
            name: manifest.name,
            donation_link: manifest.donation_link,
            date_created: manifest.date_created,
            is_deprecated: manifest.is_deprecated,
            has_nsfw_content: manifest.has_nsfw_content,
            categories: manifest.categories,
            versions: Vec::new(),
            total_downloads: version.downloads,
            version: Some(version),
        }
    }
}

impl VersionInfo {
    fn from_index(v: &ArchivedModVersionRef) -> Self {
        Self {
            version_number: v.version_number.get(),
            description: v.description.to_string(),
            dependencies: v.dependencies.iter().map(|d| d.to_string()).collect(),
            downloads: v.downloads.to_native(),
            date_created: v.date_created.into(),
            website_url: v.website_url.as_ref().map(|url| url.to_string()),
            is_active: v.is_active,
            file_size: v.file_size.to_native(),
        }
    }

    fn from_manifest(v: ManifestVersion) -> Self {
        Self {
            version_number: v.version_number,
            description: v.description,
            dependencies: v.dependencies,
            downloads: v.downloads,
            date_created: v.date_created,
            website_url: v.website_url,
            is_active: v.is_active,
            file_size: v.file_size,
        }
    }
}

pub struct ModVersionRust {
    version_number: QString,
    description: QString,
    dependencies: QStringList,
    downloads: f64,
    date_created: QDateTime,
    website_url: QString,
    is_active: bool,
    file_size: f64,
}

impl Default for ModVersionRust {
    fn default() -> Self {
        Self {
            version_number: QString::default(),
            description: QString::default(),
            dependencies: QStringList::default(),
            downloads: 0.0,
            date_created: QDateTime::default(),
            website_url: QString::default(),
            is_active: false,
            file_size: 0.0,
        }
    }
}

pub struct ModRust {
    owner: QString,
    name: QString,
    donation_link: QString,
    date_created: QDateTime,
    is_deprecated: bool,
    has_nsfw_content: bool,
    categories: QStringList,
    versions: QObjectList,
    total_downloads: f64,
    version: *mut ModVersion,
    display_version: *mut ModVersion,
    installed_version: Option<Version>,
}

impl Default for ModRust {
    fn default() -> Self {
        Self {
            owner: QString::default(),
            name: QString::default(),
            donation_link: QString::default(),
            date_created: QDateTime::default(),
            is_deprecated: false,
            has_nsfw_content: false,
            categories: QStringList::default(),
            versions: QObjectList::default(),
            total_downloads: 0.0,
            version: std::ptr::null_mut(),
            display_version: std::ptr::null_mut(),
            installed_version: None,
        }
    }
}

/// Makes a `Mod` to give QML.
pub fn new_mod(info: ModInfo) -> UniquePtr<Mod> {
    let mut object = qobject::new_mod_object();
    {
        let mut rust = object.pin_mut().rust_mut();
        rust.owner = QString::from(&info.owner);
        rust.name = QString::from(&info.name);
    }
    object.pin_mut().update(info);
    object
}

impl Mod {
    /// Updates the mod to what it is now, as an installed mod is when it's updated.
    pub fn update(mut self: Pin<&mut Self>, info: ModInfo) {
        let versions = info
            .versions
            .into_iter()
            .map(|version| new_version(self.as_mut(), version))
            .collect::<Vec<_>>();
        let installed_version = info.version.as_ref().map(|version| version.version_number);
        // an installed version that's unchanged keeps its object. An installed mod's past versions
        // stay its children, as QML may still be showing them.
        let version = match (info.version, self.installed_version) {
            (Some(_), Some(current)) if Some(current) == installed_version => self.version,
            (Some(version), _) => new_version(self.as_mut(), version).as_ptr(),
            (None, _) => std::ptr::null_mut(),
        };

        let mut rust = self.as_mut().rust_mut();
        rust.donation_link = info
            .donation_link
            .as_deref()
            .map(QString::from)
            .unwrap_or_default();
        rust.date_created = to_date_time(info.date_created);
        rust.is_deprecated = info.is_deprecated;
        rust.has_nsfw_content = info.has_nsfw_content;
        rust.categories = info.categories.iter().map(QString::from).collect();
        rust.total_downloads = info.total_downloads as f64;
        rust.versions = versions.iter().map(Child::as_qobject).collect();
        rust.version = version;
        rust.installed_version = installed_version;
        rust.display_version = if version.is_null() {
            versions.first().map_or(std::ptr::null_mut(), Child::as_ptr)
        } else {
            version
        };
        self.changed();
    }

    /// The mod's owner and name, for the backend.
    pub fn id(&self) -> (String, String) {
        (self.owner.to_string(), self.name.to_string())
    }

    pub fn installed_version(&self) -> Option<Version> {
        self.installed_version
    }

    fn readme(&self, version: &QString) -> *mut Call {
        self.markdown(version, ModMarkdown::Readme)
    }

    fn changelog(&self, version: &QString) -> *mut Call {
        self.markdown(version, ModMarkdown::Changelog)
    }

    fn markdown(&self, version: &QString, endpoint: ModMarkdown) -> *mut Call {
        let (owner, name) = self.id();
        let version = version.to_string();
        let task = tasks::allocate_task();
        call::start(
            Some(task),
            async move {
                let version = Version::from_str(&version)
                    .map_err(|e| anyhow::anyhow!("Invalid version {version:?}: {e}"))?;
                let app = super::app();
                let json = thunderstore::fetch_mod_markdown(
                    Some(app),
                    &slog_scope::logger(),
                    &app.state::<Reqwest>(),
                    &owner,
                    &name,
                    version,
                    endpoint,
                    Some(task),
                )
                .await?;
                #[derive(serde::Deserialize)]
                struct Markdown {
                    markdown: Option<String>,
                }
                let markdown =
                    serde_json::from_str::<Markdown>(&json).map_err(anyhow::Error::from)?;
                Ok(markdown.markdown.unwrap_or_default())
            },
            |call, markdown| call.return_value(QVariant::from(&QString::from(&markdown))),
        )
    }
}

/// Makes a version of a mod as its child.
fn new_version(parent: Pin<&mut Mod>, info: VersionInfo) -> Child<ModVersion> {
    let mut object = qobject::new_version_object();
    {
        let mut rust = object.pin_mut().rust_mut();
        rust.version_number = QString::from(&info.version_number.to_string());
        rust.description = QString::from(&info.description);
        rust.dependencies = info.dependencies.iter().map(QString::from).collect();
        rust.downloads = info.downloads as f64;
        rust.date_created = to_date_time(info.date_created);
        rust.website_url = info
            .website_url
            .as_deref()
            .map(QString::from)
            .unwrap_or_default();
        rust.is_active = info.is_active;
        rust.file_size = info.file_size as f64;
    }
    Child::adopt(parent, object)
}

fn to_date_time(timestamp: Timestamp) -> QDateTime {
    QDateTime::from_msecs_since_epoch(timestamp.get().timestamp_millis(), &QTimeZone::utc())
}
