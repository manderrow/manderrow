//! The `Modpack`s that QML imports as profiles, which `Profiles.previewModpack` looks up.

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
    }

    #[auto_cxx_name]
    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_uncreatable]
        #[qproperty(QString, owner, READ, CONSTANT)]
        #[qproperty(QString, name, READ, CONSTANT)]
        #[qproperty(QString, version, READ, CONSTANT)]
        /// The task installing it, once the modpack is being imported, or -1.
        #[qproperty(i32, task_id, READ, NOTIFY)]
        type ModpackMod = super::ModpackModRust;

        #[qobject]
        #[qml_element]
        #[qml_uncreatable]
        #[qproperty(QString, name, READ, CONSTANT)]
        /// Its `ModpackMod`s.
        #[qproperty(QObjectList, mods, READ, CONSTANT)]
        /// The paths of the files it adds to the profile, as its mods' configuration.
        #[qproperty(QStringList, files, READ, CONSTANT)]
        type Modpack = super::ModpackRust;

        /// Imports the modpack as a new profile, unless `profileId` is another. The call returns the
        /// profile's id. Each of its mods' `taskId` is set as it starts being installed.
        #[qinvokable]
        fn import_into(self: &Modpack, profile_id: &QString) -> *mut Call;
    }

    impl cxx_qt::Threading for Modpack {}

    #[namespace = "rust::cxxqtlib1"]
    unsafe extern "C++" {
        include!("cxx-qt-lib/common.h");
        #[rust_name = "new_modpack_object"]
        fn make_unique() -> UniquePtr<Modpack>;
        #[rust_name = "new_modpack_mod_object"]
        fn make_unique() -> UniquePtr<ModpackMod>;
    }
}

use core::pin::Pin;
use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Context as _;
use cxx::UniquePtr;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;
use uuid::Uuid;

use super::call::{self, Call};
use super::child::Child;
use super::mods::ModKey;
use super::outbox::Outbox;
use super::profiles::{refreshing, Profiles};
use super::QObjectList;
use crate::importing::{self, Modpack};
use crate::Reqwest;

pub struct ModpackModRust {
    owner: QString,
    name: QString,
    version: QString,
    task_id: i32,
}

impl Default for ModpackModRust {
    fn default() -> Self {
        Self {
            owner: QString::default(),
            name: QString::default(),
            version: QString::default(),
            task_id: -1,
        }
    }
}

#[derive(Default)]
pub struct ModpackRust {
    name: QString,
    mods: QObjectList,
    files: cxx_qt_lib::QStringList,
    thunderstore_id: Uuid,
    game: String,
    mod_objects: HashMap<ModKey, Child<qobject::ModpackMod>>,
    profiles: Option<Arc<Outbox<Profiles>>>,
}

/// A modpack shared as a Thunderstore profile code, looked up to import into a game's profiles,
/// which a call returns as a `Modpack`.
pub struct ModpackPreview {
    pub modpack: Modpack,
    pub thunderstore_id: Uuid,
    pub game: String,
    pub profiles: Arc<Outbox<Profiles>>,
}

impl call::Output for ModpackPreview {
    fn return_from(self, call: Pin<&mut Call>) {
        call.return_object(Some(new_modpack(self)));
    }
}

fn new_modpack(preview: ModpackPreview) -> UniquePtr<qobject::Modpack> {
    let ModpackPreview {
        modpack,
        thunderstore_id,
        game,
        profiles,
    } = preview;
    let mut object = qobject::new_modpack_object();
    let mut mods = QObjectList::default();
    let mut mod_objects = HashMap::with_capacity(modpack.mods.len());
    for m in modpack.mods {
        let mut mod_object = qobject::new_modpack_mod_object();
        {
            let mut rust = mod_object.pin_mut().rust_mut();
            rust.owner = QString::from(&m.owner);
            rust.name = QString::from(&m.name);
            rust.version = QString::from(&m.version);
        }
        let child = Child::adopt(object.pin_mut(), mod_object);
        mods.push(child.as_qobject());
        let key = ModKey {
            owner: m.owner,
            name: m.name,
        };
        mod_objects.insert(key, child);
    }
    let mut rust = object.pin_mut().rust_mut();
    rust.name = QString::from(&modpack.name);
    rust.mods = mods;
    rust.files = modpack.files.iter().map(QString::from).collect();
    rust.thunderstore_id = thunderstore_id;
    rust.game = game;
    rust.mod_objects = mod_objects;
    rust.profiles = Some(profiles);
    object
}

impl qobject::Modpack {
    fn import_into(&self, profile_id: &QString) -> *mut Call {
        let profile_id = match profile_id.to_string().as_str() {
            "" => Ok(None),
            id => Uuid::try_parse(id)
                .map(Some)
                .with_context(|| format!("Invalid profile id {id:?}")),
        };
        let (thunderstore_id, game) = (self.thunderstore_id, self.game.clone());
        let profiles = self.profiles.clone().expect("a modpack has the profiles");
        let thread = self.qt_thread();
        call::spawn_with_task(move |task| {
            refreshing(profiles, async move {
                let on_mod_task = move |owner: &str, name: &str, task| {
                    let key = ModKey {
                        owner: owner.to_owned(),
                        name: name.to_owned(),
                    };
                    // fails only if QML has dropped the modpack, when there's no one to tell
                    _ = thread.queue(move |modpack| modpack.set_task(&key, task));
                };
                let app = super::app();
                importing::import_thunderstore_modpack(
                    app,
                    &app.state::<Reqwest>(),
                    thunderstore_id,
                    &game,
                    profile_id?,
                    &on_mod_task,
                    task,
                )
                .await
            })
        })
    }

    fn set_task(self: Pin<&mut Self>, key: &ModKey, task: crate::tasks::Id) {
        if let Some(m) = self.rust_mut().get_mut().mod_objects.get_mut(key) {
            let mut m = m.get_mut();
            m.as_mut().rust_mut().task_id = task.to_qml();
            m.task_id_changed();
        }
    }
}
