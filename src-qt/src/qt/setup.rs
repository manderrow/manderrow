//! The configuration of the application, around the creation of the `QGuiApplication`.

#[cxx_qt::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!(<QtCore/QCoreApplication>);
        type QCoreApplication;
        include!(<QtGui/QGuiApplication>);
        type QGuiApplication;
        include!(<QtCore/QResource>);
        type QResource;
        include!(<QtGui/QFontDatabase>);
        type QFontDatabase;
        include!(<QtGui/QIcon>);
        type QIcon;
        include!(<QtQml/QQmlDebuggingEnabler>);
        type QQmlDebuggingEnabler;
        include!(<QtQuick/QQuickWindow>);
        type QQuickWindow;

        #[Self = "QResource"]
        #[rust_name = "register_resource"]
        unsafe fn registerResource(rcc_data: *const u8, map_root: &QString) -> bool;

        #[Self = "QCoreApplication"]
        #[rust_name = "set_application_name"]
        fn setApplicationName(name: &QString);

        #[Self = "QCoreApplication"]
        #[rust_name = "set_organization_name"]
        fn setOrganizationName(name: &QString);

        #[Self = "QGuiApplication"]
        #[rust_name = "set_window_icon"]
        fn setWindowIcon(icon: &QIcon);

        #[Self = "QFontDatabase"]
        #[rust_name = "add_application_font"]
        fn addApplicationFont(file_name: &QString) -> i32;

        #[Self = "QQmlDebuggingEnabler"]
        #[rust_name = "enable_debugging"]
        fn enableDebugging(print_warning: bool);

        #[Self = "QQuickWindow"]
        #[rust_name = "set_default_alpha_buffer"]
        fn setDefaultAlphaBuffer(use_alpha: bool);
    }

    #[namespace = "rust::cxxqtlib1"]
    unsafe extern "C++" {
        include!("cxx-qt-lib/common.h");

        #[rust_name = "qicon_new"]
        fn make_unique(file_name: &QString) -> UniquePtr<QIcon>;
    }

    unsafe extern "C++" {
        include!("cxx-qt-lib/qqmlapplicationengine.h");
        type QQmlApplicationEngine = cxx_qt_lib::QQmlApplicationEngine;
        include!("cxx-qt-lib/qurl.h");
        type QUrl = cxx_qt_lib::QUrl;
    }

    #[namespace = "manderrow"]
    unsafe extern "C++" {
        include!("manderrow/cpp/qt.h");

        #[rust_name = "load_root"]
        fn loadRoot(engine: Pin<&mut QQmlApplicationEngine>, url: &QUrl) -> QString;
    }
}

use core::pin::Pin;

use cxx_qt_lib::{QQmlApplicationEngine, QQuickStyle, QString, QUrl};

/// Registers a binary resource bundle produced by `rcc --binary`.
pub fn register_resources(data: &'static [u8]) -> bool {
    // SAFETY: the data lives for as long as the app, as Qt reads it in place
    unsafe { ffi::QResource::register_resource(data.as_ptr(), &QString::default()) }
}

/// Configuration that must happen before the `QGuiApplication` is created.
pub fn configure_before_app() {
    // Every control is styled by the app, so use the style that is designed to be customized.
    QQuickStyle::set_style(&QString::from("Basic"));
    // windows can be made translucent, which Qt only allows for windows created with an alpha
    // channel
    ffi::QQuickWindow::set_default_alpha_buffer(true);
    #[cfg(windows)]
    super::native_win32::configure_before_app();
    ffi::QCoreApplication::set_application_name(&QString::from("Manderrow"));
    ffi::QCoreApplication::set_organization_name(&QString::from("Manderrow"));
    cxx_qt_lib::QGuiApplication::set_desktop_file_name(&QString::from("io.github.manderrow"));
}

/// Lets QML tools, like qmlprofiler, attach. For development builds.
pub fn enable_qml_debugging() {
    ffi::QQmlDebuggingEnabler::enable_debugging(true);
}

/// Loads the root QML file, returning the errors that kept it from creating its window, if any.
pub fn load_root(engine: Pin<&mut QQmlApplicationEngine>, url: &QUrl) -> Result<(), QString> {
    let errors = ffi::load_root(engine, url);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Configuration that must happen after the `QGuiApplication` is created.
pub fn configure_app() {
    ffi::QGuiApplication::set_window_icon(&ffi::qicon_new(&QString::from(":/assets/icon.png")));
    for font in [
        "Inter-Variable.ttf",
        "Inter-Italic-Variable.ttf",
        // Static instances, as Qt's fontconfig backend only registers a variable font's default
        // instance, which for Source Code Pro is ExtraLight, under a family name of its own.
        "SourceCodePro-Regular.ttf",
        "SourceCodePro-Bold.ttf",
        "SourceCodePro-Italic.ttf",
        "SourceCodePro-BoldItalic.ttf",
        "ManderrowIcons.ttf",
    ] {
        ffi::QFontDatabase::add_application_font(&QString::from(&format!(":/assets/fonts/{font}")));
    }
}
