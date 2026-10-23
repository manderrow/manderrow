//! The `web` image provider, which loads `image://web/<https URL>` with the backend's HTTP client
//! rather than Qt's. Qt's TLS comes from plugins that vary by platform and installation: on
//! Windows, it prefers whichever OpenSSL it finds on the PATH, then Schannel. The backend's client
//! is the one that already fetches the mod index, so images load wherever the index does. Images
//! are cached on disk by HTTP's rules, as the browser that the web UI ran in did.

#[cxx_qt::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qimage.h");
        type QImage = cxx_qt_lib::QImage;
        include!("cxx-qt-lib/qsize.h");
        type QSize = cxx_qt_lib::QSize;
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qqmlimageproviderbase.h");
        type QQmlImageProviderBase = cxx_qt_lib::QQmlImageProviderBase;

        include!(<QtQuick/QQuickImageProvider>);
        type QQuickTextureFactory;
        // a QObject without a meta-object of its own, so not one that cxx-qt can cast to
        type QQuickAsyncImageProvider;

        #[Self = "QQuickTextureFactory"]
        #[rust_name = "texture_factory_for_image"]
        fn textureFactoryForImage(image: &QImage) -> *mut QQuickTextureFactory;
    }

    unsafe extern "C++Qt" {
        #[qobject]
        type QQuickImageResponse;
    }

    #[auto_cxx_name]
    unsafe extern "RustQt" {
        #[qobject]
        #[base = QQuickAsyncImageProvider]
        type WebImageProvider = super::WebImageProviderRust;

        #[cxx_override]
        fn request_image_response(
            self: Pin<&mut WebImageProvider>,
            id: &QString,
            requested_size: &QSize,
        ) -> *mut QQuickImageResponse;
    }

    #[auto_cxx_name]
    unsafe extern "RustQt" {
        #[qobject]
        #[base = QQuickImageResponse]
        type WebImageResponse = super::WebImageResponseRust;

        #[cxx_override]
        fn texture_factory(self: &WebImageResponse) -> *mut QQuickTextureFactory;

        #[cxx_override]
        fn error_string(self: &WebImageResponse) -> QString;

        #[cxx_override]
        fn cancel(self: Pin<&mut WebImageResponse>);

        #[qsignal]
        #[inherit]
        fn finished(self: Pin<&mut WebImageResponse>);
    }

    impl cxx_qt::Constructor<()> for WebImageProvider {}
    impl cxx_qt::Constructor<()> for WebImageResponse {}
    impl cxx_qt::Threading for WebImageResponse {}

    #[namespace = "rust::cxxqt1"]
    unsafe extern "C++" {
        include!("cxx-qt/casting.h");

        #[rust_name = "upcast_web_image_provider"]
        unsafe fn upcastPtr(provider: *const WebImageProvider) -> *const QQmlImageProviderBase;
        #[rust_name = "downcast_web_image_provider"]
        unsafe fn downcastPtr(base: *const QQmlImageProviderBase) -> *const WebImageProvider;
    }

    #[namespace = "rust::cxxqtlib1"]
    unsafe extern "C++" {
        include!("cxx-qt-lib/common.h");

        #[rust_name = "web_image_provider_new"]
        fn make_unique() -> UniquePtr<WebImageProvider>;

        #[rust_name = "web_image_response_new"]
        fn make_unique() -> UniquePtr<WebImageResponse>;
    }
}

use core::pin::Pin;

use anyhow::ensure;
use bytes::Bytes;
use cxx_qt::casting::Upcast;
use cxx_qt::{CxxQtThread, CxxQtType, Threading};
use cxx_qt_lib::{
    AspectRatioMode, QImage, QQmlEngine, QQmlImageProviderBase, QSize, QString, TransformationMode,
};
use tokio::task::AbortHandle;

use crate::util::http_cache;
use crate::Reqwest;

use ffi::{QQuickImageResponse, QQuickTextureFactory, WebImageProvider, WebImageResponse};

// SAFETY: the casts are C++'s, between the provider and a class it derives from
unsafe impl Upcast<QQmlImageProviderBase> for WebImageProvider {
    unsafe fn upcast_ptr(this: *const Self) -> *const QQmlImageProviderBase {
        unsafe { ffi::upcast_web_image_provider(this) }
    }

    unsafe fn from_base_ptr(base: *const QQmlImageProviderBase) -> *const Self {
        unsafe { ffi::downcast_web_image_provider(base) }
    }
}

/// Larger images are not loaded.
const MAX_SIZE: usize = 32 << 20;

/// Adds the `web` image provider to an engine.
pub fn add_web_image_provider(engine: Pin<&mut QQmlEngine>) {
    let mut provider = ffi::web_image_provider_new();
    let base = Upcast::<QQmlImageProviderBase>::upcast_pin(provider.pin_mut());
    let base = std::ptr::from_ref::<QQmlImageProviderBase>(&base).cast_mut();
    // owned by the engine from here on
    _ = provider.into_raw();
    // SAFETY: the provider is a new image provider that nothing else owns
    unsafe { engine.add_image_provider(&QString::from("web"), base) };
}

#[derive(Default)]
pub struct WebImageProviderRust;

// The base classes take no parent, unlike the QObject that cxx-qt's default constructor assumes.
impl cxx_qt::Constructor<()> for WebImageProvider {
    type NewArguments = ();
    type BaseArguments = ();
    type InitializeArguments = ();

    fn route_arguments(_: ()) -> ((), (), ()) {
        ((), (), ())
    }

    fn new(_: ()) -> WebImageProviderRust {
        WebImageProviderRust
    }
}

impl cxx_qt::Constructor<()> for WebImageResponse {
    type NewArguments = ();
    type BaseArguments = ();
    type InitializeArguments = ();

    fn route_arguments(_: ()) -> ((), (), ()) {
        ((), (), ())
    }

    fn new(_: ()) -> WebImageResponseRust {
        WebImageResponseRust::default()
    }
}

impl ffi::WebImageProvider {
    fn request_image_response(
        self: Pin<&mut Self>,
        id: &QString,
        requested_size: &QSize,
    ) -> *mut QQuickImageResponse {
        let mut response = ffi::web_image_response_new();
        response.pin_mut().start(id.to_string(), requested_size);
        let base = Upcast::<QQuickImageResponse>::upcast_pin(response.pin_mut());
        let base = std::ptr::from_ref::<QQuickImageResponse>(&base).cast_mut();
        // owned by the engine from here on, which deletes it once it is finished
        _ = response.into_raw();
        base
    }
}

#[derive(Default)]
pub struct WebImageResponseRust {
    /// The size that the image is decoded down to, or zero in a dimension that isn't constrained.
    requested_width: i32,
    requested_height: i32,
    image: QImage,
    error: QString,
    task: Option<AbortHandle>,
}

impl Drop for WebImageResponseRust {
    fn drop(&mut self) {
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}

impl ffi::WebImageResponse {
    fn start(mut self: Pin<&mut Self>, url: String, requested_size: &QSize) {
        let mut rust = self.as_mut().rust_mut();
        rust.requested_width = requested_size.width().max(0);
        rust.requested_height = requested_size.height().max(0);
        let delivery = Delivery(Some(self.qt_thread()));
        let task = super::runtime().spawn(async move {
            let result = fetch(&url).await.map_err(|e| {
                slog_scope::debug!("Failed to load image {url:?}: {e:#}");
                format!("{e:#}")
            });
            delivery.finish(result);
        });
        self.rust_mut().task = Some(task.abort_handle());
    }

    /// Decodes the fetched image, or records why it couldn't be fetched, and finishes.
    fn finish(mut self: Pin<&mut Self>, result: Result<Bytes, String>) {
        let decoded = result.and_then(|data| {
            QImage::from_data(&data, None).ok_or_else(|| "The image couldn't be decoded".to_owned())
        });
        match decoded {
            Ok(image) => {
                let image = self.scale(image);
                self.as_mut().rust_mut().image = image;
            }
            Err(error) => self.as_mut().rust_mut().error = QString::from(&error),
        }
        self.finished();
    }

    /// Scales an image down to the requested size, as Image's sourceSize does: covering it, as
    /// the app's images are cropped to fill their area, and never enlarging it.
    fn scale(&self, image: QImage) -> QImage {
        let (width, height) = (image.width(), image.height());
        if width <= 0 || height <= 0 {
            return image;
        }
        let scale = f64::max(
            f64::from(self.requested_width) / f64::from(width),
            f64::from(self.requested_height) / f64::from(height),
        );
        if scale <= 0.0 || scale >= 1.0 {
            return image;
        }
        image.scaled(
            (f64::from(width) * scale).ceil() as i32,
            (f64::from(height) * scale).ceil() as i32,
            AspectRatioMode::IgnoreAspectRatio,
            TransformationMode::SmoothTransformation,
        )
    }

    fn texture_factory(&self) -> *mut QQuickTextureFactory {
        ffi::QQuickTextureFactory::texture_factory_for_image(&self.image)
    }

    fn error_string(&self) -> QString {
        self.error.clone()
    }

    fn cancel(self: Pin<&mut Self>) {
        // The delivery is dropped with the task, which still finishes the response, as the
        // engine waits for that to clean it up.
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}

/// Finishes the response once, on its thread: when it is told to, or when it is dropped, as an
/// aborted task is.
struct Delivery(Option<CxxQtThread<WebImageResponse>>);

impl Delivery {
    fn finish(mut self, result: Result<Bytes, String>) {
        if let Some(thread) = self.0.take() {
            // fails only once the response has been destroyed, when there is nothing to finish
            _ = thread.queue(move |response| response.finish(result));
        }
    }
}

impl Drop for Delivery {
    fn drop(&mut self) {
        if let Some(thread) = self.0.take() {
            _ = thread.queue(|response| response.finish(Err("Cancelled".to_owned())));
        }
    }
}

async fn fetch(url: &str) -> anyhow::Result<Bytes> {
    let url = reqwest::Url::parse(url)?;
    ensure!(url.scheme() == "https", "Only HTTPS images are loaded");
    http_cache::get(&super::app().state::<Reqwest>(), &url, MAX_SIZE).await
}
