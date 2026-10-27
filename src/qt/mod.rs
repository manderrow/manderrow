//! The Qt Quick frontend host.

mod app;
mod call;
mod child;
mod console;
mod error;
mod games;
mod markdown;
mod mod_index;
mod modpacks;
mod mods;
mod native;
#[cfg(target_vendor = "apple")]
mod native_macos;
#[cfg(all(unix, not(target_vendor = "apple")))]
mod native_unix;
#[cfg(windows)]
mod native_win32;
mod object_list;
mod outbox;
mod profiles;
mod rich_text;
mod settings;
mod setup;
mod tasks;
mod variant_map;
mod web_images;

use std::sync::OnceLock;
use std::time::Duration;

use anyhow::{bail, Context};
use cxx_qt::casting::Upcast;
use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QUrl};

use crate::app::AppHandle;
use crate::single_instance::InstanceLock;

pub use object_list::QObjectList;
pub use variant_map::QVariantMap;

static APP: OnceLock<AppHandle> = OnceLock::new();
static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

/// Compiled by the build script from the `assets` directory.
static ASSETS: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets.rcc"));

const MAIN_QML: &str = "qrc:/qt/qml/Manderrow/src/qml/Main.qml";

fn app() -> &'static AppHandle {
    APP.get().expect("app is not initialized")
}

pub fn runtime() -> &'static tokio::runtime::Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("manderrow-worker")
            .build()
            .expect("failed to build the async runtime")
    })
}

pub fn run(app: AppHandle, instance: InstanceLock) -> anyhow::Result<()> {
    if APP.set(app.clone()).is_err() {
        bail!("Qt frontend is already running");
    }

    if !setup::register_resources(ASSETS) {
        bail!("Failed to register the asset bundle");
    }

    setup::configure_before_app();
    // Development hook: MANDERROW_DEV_QML_DEBUG=1 lets qmlprofiler and other QML tools attach.
    if cfg!(debug_assertions) && std::env::var_os("MANDERROW_DEV_QML_DEBUG").is_some() {
        setup::enable_qml_debugging();
    }
    let mut qapp = QGuiApplication::new();
    setup::configure_app();

    // Another launch of the app asks this instance to come to the front.
    instance.listen(move || app.activation_events().emit(()));

    let mut engine = QQmlApplicationEngine::new();
    let mut engine_ref = engine.as_mut().context("Failed to create the QML engine")?;
    web_images::add_web_image_provider(engine_ref.as_mut().upcast_pin());
    // Without a window, the app would run on unseen, with nothing to quit it.
    if let Err(errors) = setup::load_root(engine_ref, &QUrl::from(MAIN_QML)) {
        bail!("Failed to load the user interface:\n{errors}");
    }

    // Qt frees memory all over, as views close and images leave its caches, so its heap is trimmed
    // every so often rather than after any one of those. A trim takes about a millisecond.
    if cfg!(all(target_os = "linux", target_env = "gnu")) {
        runtime().spawn(async {
            let mut interval = tokio::time::interval(Duration::from_secs(10));
            loop {
                interval.tick().await;
                crate::util::trim_native_heap();
            }
        });
    }

    let code = qapp
        .as_mut()
        .context("Failed to create the application")?
        .exec();

    // Destroy the UI before the backend state it refers to.
    drop(engine);
    drop(qapp);

    if code != 0 {
        bail!("Application exited with code {code}");
    }
    Ok(())
}
