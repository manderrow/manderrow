//! The `Profiles` QML singleton, which keeps the profiles, and the `Profile`s it gives QML.

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
        include!("manderrow/src/qt/call.cxxqt.h");
        type Call = crate::qt::call::Call;
        include!("manderrow/src/qt/mods.cxxqt.h");
        type Mod = crate::qt::mods::Mod;
    }

    #[auto_cxx_name]
    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_uncreatable]
        #[qproperty(QString, id, READ, CONSTANT)]
        #[qproperty(QString, name, READ, NOTIFY)]
        /// The id of the game it's for.
        #[qproperty(QString, game, READ, CONSTANT)]
        /// Whether it's listed before the others.
        #[qproperty(bool, pinned, READ, NOTIFY)]
        /// The directory of its files.
        #[qproperty(QString, path, READ, CONSTANT)]
        /// The `Mod`s installed in it, once `refreshMods` has read them.
        #[qproperty(QObjectList, mods, READ, NOTIFY)]
        /// Whether its mods have been read.
        #[qproperty(bool, mods_loaded, READ, NOTIFY = mods_changed)]
        type Profile = super::ProfileRust;

        /// Reads the mods installed in the profile again.
        #[qinvokable]
        fn refresh_mods(self: &Profile) -> *mut Call;

        /// The mod installed in the profile with an owner and name, or null.
        #[qinvokable]
        fn installed_mod(self: &Profile, owner: &QString, name: &QString) -> *mut Mod;

        /// Installs a version of a mod in the profile, with its dependencies. The call's task is
        /// the installation, whose `download` is the mod's.
        #[qinvokable]
        fn install_mod(
            self: &Profile,
            owner: &QString,
            name: &QString,
            version: &QString,
        ) -> *mut Call;

        #[qinvokable]
        fn uninstall_mod(self: &Profile, owner: &QString, name: &QString) -> *mut Call;

        /// Looks the installed mods up in the game's fetched mod index. The call returns
        /// summaries of those whose latest version isn't the one installed.
        #[qinvokable]
        fn find_updates(self: &Profile) -> *mut Call;

        #[qinvokable]
        fn rename(self: &Profile, name: &QString) -> *mut Call;

        #[qinvokable]
        fn set_pinned(self: &Profile, pinned: bool) -> *mut Call;

        /// Deletes the profile and its files.
        #[qinvokable]
        fn remove(self: &Profile) -> *mut Call;

        #[qobject]
        #[qml_element]
        #[qml_singleton]
        /// Every profile, by name.
        #[qproperty(QObjectList, all, READ, NOTIFY)]
        /// Whether the profiles have been read, or failed to be.
        #[qproperty(bool, loaded, READ, NOTIFY)]
        /// The messages of the error that kept the profiles from being read, if one did.
        #[qproperty(QStringList, error_messages, READ, NOTIFY = loaded_changed)]
        #[qproperty(QString, backtrace, READ, NOTIFY = loaded_changed)]
        type Profiles = super::ProfilesRust;

        /// The profile with an id, or null.
        #[qinvokable]
        fn by_id(self: &Profiles, id: &QString) -> *mut Profile;

        /// Creates a profile. The call returns its id.
        #[qinvokable]
        fn create(self: &Profiles, game: &QString, name: &QString) -> *mut Call;

        /// Reads the profiles again, for changes made to them other than through these.
        #[qinvokable]
        fn refresh(self: &Profiles) -> *mut Call;

        /// Looks up a modpack that's been shared as a Thunderstore profile code, to import into
        /// a game's profiles. The call returns the `Modpack`, and its task is the lookup.
        #[qinvokable]
        fn preview_modpack(self: &Profiles, code: &QString, game: &QString) -> *mut Call;
    }

    impl cxx_qt::Threading for Profiles {}
    impl cxx_qt::Initialize for Profiles {}

    #[namespace = "rust::cxxqtlib1"]
    unsafe extern "C++" {
        include!("cxx-qt-lib/common.h");
        #[rust_name = "new_profile"]
        fn make_unique() -> UniquePtr<Profile>;
    }
}

use core::pin::Pin;
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

use anyhow::Context as _;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QString, QStringList};
use indexmap::IndexMap;
use smol_str::SmolStr;
use uuid::Uuid;

pub use qobject::Profiles;

use super::call::{self, Call};
use super::child::Child;
use super::error::QmlError;
use super::modpacks::ModpackPreview;
use super::mods::{self, Mod, ModInfo, ModKey};
use super::outbox::Outbox;
use super::QObjectList;
use crate::profiles::{self, ModManifest, ProfileWithId};
use crate::{importing, mod_index};
use crate::{CommandError, Reqwest};

#[derive(Default)]
pub struct ProfileRust {
    id: QString,
    name: QString,
    game: QString,
    pinned: bool,
    path: QString,
    mods: QObjectList,
    mods_loaded: bool,
    uuid: Uuid,
    metadata: Option<profiles::Profile>,
    profiles: Option<Arc<Outbox<Profiles>>>,
    /// The installed mods, in the order they were read.
    installed: IndexMap<ModKey, Child<Mod>>,
    /// The mods that have been uninstalled, which are kept, as QML may still be showing them, and
    /// reused if they're installed again.
    uninstalled: HashMap<ModKey, Child<Mod>>,
}

#[derive(Default)]
pub struct ProfilesRust {
    all: QObjectList,
    loaded: bool,
    error_messages: QStringList,
    backtrace: QString,
    profiles: HashMap<Uuid, Child<qobject::Profile>>,
    /// The profiles that have been deleted, which are kept, as QML may still be showing them.
    /// QML lists them in arrays, which can't tell it they're gone before they are. There are few.
    removed: Vec<Child<qobject::Profile>>,
    outbox: Option<Arc<Outbox<Profiles>>>,
}

impl cxx_qt::Initialize for Profiles {
    fn initialize(mut self: Pin<&mut Self>) {
        let outbox = Outbox::new(self.qt_thread());
        self.as_mut().rust_mut().outbox = Some(outbox.clone());
        super::runtime().spawn(async move {
            let result = profiles::get_profiles().await;
            outbox.send(move |mut this| {
                match result {
                    Ok(list) => this.as_mut().apply(list),
                    Err(e) => {
                        if let Some(QmlError {
                            messages,
                            backtrace,
                        }) = QmlError::of(&CommandError::from(e))
                        {
                            let mut rust = this.as_mut().rust_mut();
                            rust.error_messages = messages;
                            rust.backtrace = backtrace;
                        }
                    }
                }
                this.as_mut().rust_mut().loaded = true;
                this.loaded_changed();
            });
        });
    }
}

impl Profiles {
    fn by_id(&self, id: &QString) -> *mut qobject::Profile {
        Uuid::try_parse(&id.to_string())
            .ok()
            .and_then(|id| self.profiles.get(&id))
            .map_or(std::ptr::null_mut(), Child::as_ptr)
    }

    fn create(&self, game: &QString, name: &QString) -> *mut Call {
        let game = SmolStr::from(game.to_string());
        let name = SmolStr::from(name.to_string());
        call::spawn(refreshing(
            self.outbox(),
            profiles::create_profile(game, name),
        ))
    }

    fn refresh(&self) -> *mut Call {
        call::spawn(refreshing(self.outbox(), async { anyhow::Ok(()) }))
    }

    fn preview_modpack(&self, code: &QString, game: &QString) -> *mut Call {
        let (code, game) = (code.to_string(), game.to_string());
        let profiles = self.outbox();
        call::spawn_with_task(move |task| async move {
            let thunderstore_id = Uuid::try_parse(code.trim())
                .with_context(|| format!("Invalid profile code {code:?}"))?;
            let app = super::app();
            let modpack = importing::preview_thunderstore_modpack(
                app,
                &app.state::<Reqwest>(),
                thunderstore_id,
                task,
            )
            .await?;
            Ok::<_, CommandError>(ModpackPreview {
                modpack,
                thunderstore_id,
                game,
                profiles,
            })
        })
    }

    fn outbox(&self) -> Arc<Outbox<Self>> {
        self.outbox.clone().expect("the singleton is initialized")
    }

    fn profile(self: Pin<&mut Self>, id: Uuid) -> Option<Pin<&mut qobject::Profile>> {
        self.rust_mut()
            .get_mut()
            .profiles
            .get_mut(&id)
            .map(Child::get_mut)
    }

    /// Updates the profiles to those read, keeping the objects of those that are still there.
    fn apply(mut self: Pin<&mut Self>, list: Vec<ProfileWithId>) {
        let outbox = self.outbox();
        let mut removed = std::mem::take(&mut self.as_mut().rust_mut().profiles);
        let mut profiles = HashMap::with_capacity(list.len());
        for ProfileWithId { id, metadata } in list {
            let mut profile = match removed.remove(&id) {
                Some(profile) => profile,
                None => {
                    let mut profile = qobject::new_profile();
                    {
                        let mut rust = profile.pin_mut().rust_mut();
                        rust.id = QString::from(&id.to_string());
                        rust.uuid = id;
                        rust.game = QString::from(metadata.game.as_str());
                        rust.path = QString::from(&*profiles::profile_path(id).to_string_lossy());
                        rust.profiles = Some(outbox.clone());
                    }
                    Child::adopt(self.as_mut(), profile)
                }
            };
            profile.get_mut().update(metadata);
            profiles.insert(id, profile);
        }
        self.as_mut()
            .rust_mut()
            .removed
            .extend(removed.into_values());

        let mut sorted = profiles.values().collect::<Vec<_>>();
        sorted.sort_by_cached_key(|profile| {
            let name = profile.name.to_string();
            (name.to_lowercase(), name)
        });
        let all = sorted.into_iter().map(Child::as_qobject).collect();
        let mut rust = self.as_mut().rust_mut();
        rust.all = all;
        rust.profiles = profiles;
        self.all_changed();
    }
}

impl qobject::Profile {
    fn update(mut self: Pin<&mut Self>, metadata: profiles::Profile) {
        let name = QString::from(metadata.name.as_str());
        let pinned = metadata.pinned;
        self.as_mut().rust_mut().metadata = Some(metadata);
        if self.name != name {
            self.as_mut().rust_mut().name = name;
            self.as_mut().name_changed();
        }
        if self.pinned != pinned {
            self.as_mut().rust_mut().pinned = pinned;
            self.pinned_changed();
        }
    }

    fn rename(&self, name: &QString) -> *mut Call {
        let name = SmolStr::from(name.to_string());
        self.write(|metadata| metadata.name = name)
    }

    fn set_pinned(&self, pinned: bool) -> *mut Call {
        self.write(|metadata| metadata.pinned = pinned)
    }

    /// Changes the profile's metadata.
    fn write(&self, change: impl FnOnce(&mut profiles::Profile)) -> *mut Call {
        let id = self.uuid;
        let mut metadata = self.metadata.clone().expect("a profile has its metadata");
        change(&mut metadata);
        call::spawn(refreshing(self.outbox(), async move {
            profiles::write_profile(id, &metadata)
                .await
                .context("Unable to write the profile")
        }))
    }

    fn remove(&self) -> *mut Call {
        call::spawn(refreshing(
            self.outbox(),
            profiles::delete_profile(self.uuid),
        ))
    }

    fn outbox(&self) -> Arc<Outbox<Profiles>> {
        self.profiles.clone().expect("a profile has its singleton")
    }

    fn refresh_mods(&self) -> *mut Call {
        call::spawn(refreshing_mods(self.outbox(), self.uuid, async {
            anyhow::Ok(())
        }))
    }

    fn installed_mod(&self, owner: &QString, name: &QString) -> *mut Mod {
        self.installed
            .get(&ModKey::of(owner, name))
            .map_or(std::ptr::null_mut(), Child::as_ptr)
    }

    fn install_mod(&self, owner: &QString, name: &QString, version: &QString) -> *mut Call {
        let (id, key, version) = (
            self.uuid,
            ModKey::of(owner, name),
            mods::parse_version(version),
        );
        let profiles = self.outbox();
        call::spawn_with_task(move |task| {
            refreshing_mods(profiles, id, async move {
                let app = super::app();
                profiles::install_profile_mod(
                    app,
                    &app.state::<Reqwest>(),
                    id,
                    &key.owner,
                    &key.name,
                    version?,
                    task,
                )
                .await
            })
        })
    }

    fn uninstall_mod(&self, owner: &QString, name: &QString) -> *mut Call {
        let (id, key) = (self.uuid, ModKey::of(owner, name));
        call::spawn(refreshing_mods(self.outbox(), id, async move {
            profiles::uninstall_profile_mod(id, &key.owner, &key.name).await
        }))
    }

    fn find_updates(&self) -> *mut Call {
        let game = self.game.to_string();
        let installed = self
            .installed
            .iter()
            .filter_map(|(key, m)| Some((key.clone(), m.installed_version()?)))
            .collect::<Vec<_>>();
        call::spawn(async move {
            let index = mod_index::read_mod_index(&game).await?;
            let ids = installed
                .iter()
                .map(|(key, _)| key.id())
                .collect::<Vec<_>>();
            let found = mod_index::get_from_mod_index(&index, &ids).await?;
            let updates = found
                .into_iter()
                .zip(&installed)
                .filter_map(|(m, (_, installed))| {
                    let m = m?;
                    let latest = m.versions.first()?;
                    (latest.version_number.get() != *installed).then(|| ModInfo::summary(m))
                })
                .collect::<Vec<_>>();
            anyhow::Ok(updates)
        })
    }

    /// Updates the installed mods to those read, keeping the objects of those that are still
    /// installed, in the order they were read.
    fn apply_mods(mut self: Pin<&mut Self>, manifests: Vec<ModManifest>) {
        let mut previous = std::mem::take(&mut self.as_mut().rust_mut().installed);
        let mut installed = IndexMap::with_capacity(manifests.len());
        for manifest in manifests {
            let key = ModKey {
                owner: manifest.owner.clone(),
                name: manifest.name.clone(),
            };
            let info = ModInfo::installed(manifest);
            let existing = previous
                .swap_remove(&key)
                .or_else(|| self.as_mut().rust_mut().uninstalled.remove(&key));
            let child = match existing {
                Some(mut child) => {
                    child.get_mut().update(info);
                    child
                }
                None => Child::adopt(self.as_mut(), mods::new_mod(info)),
            };
            installed.insert(key, child);
        }
        let mut rust = self.as_mut().rust_mut();
        rust.uninstalled.extend(previous);
        rust.mods = installed.values().map(Child::as_qobject).collect();
        rust.installed = installed;
        rust.mods_loaded = true;
        self.mods_changed();
    }
}

/// Runs a change to a profile's mods, then reads them again, which updates the profile before the
/// change's call returns.
async fn refreshing_mods<T, E>(
    profiles: Arc<Outbox<Profiles>>,
    id: Uuid,
    change: impl Future<Output = Result<T, E>>,
) -> Result<T, CommandError>
where
    E: Into<CommandError>,
{
    let result = change.await;
    match profiles::get_profile_mods(id).await {
        Ok(manifests) => profiles.send(move |this| {
            if let Some(profile) = this.profile(id) {
                profile.apply_mods(manifests);
            }
        }),
        Err(e) if result.is_ok() => return Err(e.into()),
        Err(e) => slog_scope::error!("Unable to read the mods of profile {id}: {e:#}"),
    }
    result.map_err(Into::into)
}

/// Runs a change to the profiles, then reads them again, which updates the singleton before the
/// change's call returns. They're read again even if the change failed, since it may have made
/// some of it.
pub(super) async fn refreshing<T, E>(
    profiles: Arc<Outbox<Profiles>>,
    change: impl Future<Output = Result<T, E>>,
) -> Result<T, CommandError>
where
    E: Into<CommandError>,
{
    let result = change.await;
    match profiles::get_profiles().await {
        Ok(list) => profiles.send(move |this| this.apply(list)),
        Err(e) => slog_scope::error!("Unable to read the profiles: {e:#}"),
    }
    result.map_err(Into::into)
}
