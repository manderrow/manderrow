//! The Qt Quick frontend host.

mod bridge;

use std::sync::OnceLock;

use anyhow::{bail, Context};
use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QUrl};

use crate::app::AppHandle;
use crate::single_instance::InstanceLock;

static APP: OnceLock<AppHandle> = OnceLock::new();
static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

/// Compiled by the build script from the `assets` directory.
static ASSETS: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets.rcc"));

const MAIN_QML: &str = "qrc:/qt/qml/Manderrow/qml/Main.qml";

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

    if !bridge::manderrow_register_resources(ASSETS) {
        bail!("Failed to register the asset bundle");
    }

    bridge::manderrow_configure_before_app();
    let mut qapp = QGuiApplication::new();
    bridge::manderrow_configure_app();

    // Another launch of the app asks this instance to come to the front.
    instance.listen(move || {
        _ = app.emit("single_instance", ());
    });

    let mut engine = QQmlApplicationEngine::new();
    engine
        .as_mut()
        .context("Failed to create the QML engine")?
        .load(&QUrl::from(MAIN_QML));

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
