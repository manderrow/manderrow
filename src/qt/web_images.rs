//! The `web` image provider, which loads `image://web/<https URL>` with the backend's HTTP client
//! rather than Qt's. Qt's TLS comes from plugins that vary by platform and installation: on
//! Windows, it prefers whichever OpenSSL it finds on the PATH, then Schannel. The backend's client
//! is the one that already fetches the mod index, so images load wherever the index does. Images
//! are cached on disk by HTTP's rules, as the browser that the web UI ran in did.
//!
//! `image://web/<scale>x/<https URL>` draws a vector image at `scale` times its size, for a display
//! with that pixel ratio, while it is still sized as it would be without it.

#[cxx_qt::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qbytearray.h");
        type QByteArray = cxx_qt_lib::QByteArray;
        include!("cxx-qt-lib/qimage.h");
        type QImage = cxx_qt_lib::QImage;
        include!("cxx-qt-lib/qsize.h");
        type QSize = cxx_qt_lib::QSize;
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qqmlimageproviderbase.h");
        type QQmlImageProviderBase = cxx_qt_lib::QQmlImageProviderBase;

        include!(<QtQuick/QQuickImageProvider>);
        // a QObject without a meta-object of its own, so not one that cxx-qt can cast to
        type QQuickAsyncImageProvider;

        #[Self = "QQuickTextureFactory"]
        #[rust_name = "texture_factory_for_image"]
        fn textureFactoryForImage(image: &QImage) -> *mut QQuickTextureFactory;

        include!(<QtQuick/QQuickWindow>);
        type QQuickWindow;
        include!(<QtQuick/QSGTexture>);
        type QSGTexture;

        #[rust_name = "create_texture_from_image"]
        fn createTextureFromImage(self: &QQuickWindow, image: &QImage) -> *mut QSGTexture;

        include!(<QtCore/QBuffer>);
        type QBuffer;
        include!(<QtGui/QImageReader>);
        type QImageReader;

        #[rust_name = "set_data"]
        fn setData(self: Pin<&mut QBuffer>, data: &QByteArray);

        fn size(self: &QImageReader) -> QSize;
        fn format(self: &QImageReader) -> QByteArray;
        #[rust_name = "set_scaled_size"]
        fn setScaledSize(self: Pin<&mut QImageReader>, size: &QSize);
        fn read(self: Pin<&mut QImageReader>) -> QImage;
    }

    unsafe extern "C++Qt" {
        #[qobject]
        type QQuickImageResponse;
        #[qobject]
        type QQuickTextureFactory;
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

    #[auto_cxx_name]
    unsafe extern "RustQt" {
        /// A texture factory for an image that is shown at a size other than its own, as Image
        /// takes a texture factory's size for its implicit size, which is in logical pixels.
        #[qobject]
        #[base = QQuickTextureFactory]
        type SizedTextureFactory = super::SizedTextureFactoryRust;

        #[cxx_override]
        unsafe fn create_texture(
            self: &SizedTextureFactory,
            window: *mut QQuickWindow,
        ) -> *mut QSGTexture;

        #[cxx_override]
        fn texture_size(self: &SizedTextureFactory) -> QSize;

        #[cxx_override]
        fn texture_byte_count(self: &SizedTextureFactory) -> i32;

        #[cxx_override]
        fn image(self: &SizedTextureFactory) -> QImage;
    }

    impl cxx_qt::Constructor<()> for WebImageProvider {}
    impl cxx_qt::Constructor<()> for WebImageResponse {}
    impl cxx_qt::Constructor<()> for SizedTextureFactory {}
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

        #[rust_name = "sized_texture_factory_new"]
        fn make_unique() -> UniquePtr<SizedTextureFactory>;

        #[rust_name = "buffer_new"]
        fn make_unique() -> UniquePtr<QBuffer>;

        /// A reader of the image in a device, which must outlive it.
        #[rust_name = "image_reader_new"]
        unsafe fn make_unique(device: *mut QBuffer) -> UniquePtr<QImageReader>;
    }
}

use core::pin::Pin;

use anyhow::ensure;
use bytes::Bytes;
use cxx_qt::casting::Upcast;
use cxx_qt::{CxxQtThread, CxxQtType, Threading};
use cxx_qt_lib::{
    AspectRatioMode, QByteArray, QImage, QQmlEngine, QQmlImageProviderBase, QSize, QString,
    TransformationMode,
};
use tokio::task::AbortHandle;

use crate::util::http_cache;
use crate::Reqwest;

use ffi::{
    QQuickImageResponse, QQuickTextureFactory, QQuickWindow, QSGTexture, SizedTextureFactory,
    WebImageProvider, WebImageResponse,
};

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

impl cxx_qt::Constructor<()> for SizedTextureFactory {
    type NewArguments = ();
    type BaseArguments = ();
    type InitializeArguments = ();

    fn route_arguments(_: ()) -> ((), (), ()) {
        ((), (), ())
    }

    fn new(_: ()) -> SizedTextureFactoryRust {
        SizedTextureFactoryRust::default()
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
    /// How many times its size a vector image is drawn at.
    scale: f64,
    image: QImage,
    /// The size that the image is shown at, which a vector image's may be drawn at a multiple of.
    size: QSize,
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

impl WebImageResponseRust {
    /// Scales an image down to the requested size, as Image's sourceSize does: covering it, as
    /// the app's images are cropped to fill their area, and never enlarging it.
    fn scale_to_requested(&self, image: QImage) -> QImage {
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
}

impl ffi::WebImageResponse {
    fn start(mut self: Pin<&mut Self>, url: String, requested_size: &QSize) {
        let mut rust = self.as_mut().rust_mut();
        rust.requested_width = requested_size.width().max(0);
        rust.requested_height = requested_size.height().max(0);
        let (scale, url) = match url.split_once("x/") {
            Some((scale, url)) if !scale.starts_with("https:") => (
                scale.parse::<f64>().unwrap_or(1.0).clamp(1.0, 4.0),
                url.to_owned(),
            ),
            _ => (1.0, url),
        };
        rust.scale = scale;
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
        let scale = self.scale;
        let decoded = result.and_then(|data| {
            decode(&data, scale).ok_or_else(|| "The image couldn't be decoded".to_owned())
        });
        match decoded {
            Ok((image, size)) => {
                let mut rust = self.as_mut().rust_mut();
                if rust.requested_width > 0 || rust.requested_height > 0 {
                    let image = rust.scale_to_requested(image);
                    rust.size = image.size();
                    rust.image = image;
                } else {
                    rust.image = image;
                    rust.size = size;
                }
            }
            Err(error) => self.as_mut().rust_mut().error = QString::from(&error),
        }
        self.finished();
    }

    fn texture_factory(&self) -> *mut QQuickTextureFactory {
        if self.image.is_null() || self.size == self.image.size() {
            return ffi::QQuickTextureFactory::texture_factory_for_image(&self.image);
        }
        let mut factory = ffi::sized_texture_factory_new();
        let mut rust = factory.pin_mut().rust_mut();
        rust.image = self.image.clone();
        rust.size = self.size.clone();
        let base = Upcast::<QQuickTextureFactory>::upcast_pin(factory.pin_mut());
        let base = std::ptr::from_ref::<QQuickTextureFactory>(&base).cast_mut();
        // owned by the engine from here on
        _ = factory.into_raw();
        base
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

#[derive(Default)]
pub struct SizedTextureFactoryRust {
    image: QImage,
    /// The size that the image is shown at.
    size: QSize,
}

impl ffi::SizedTextureFactory {
    unsafe fn create_texture(&self, window: *mut QQuickWindow) -> *mut QSGTexture {
        // SAFETY: Qt passes the window that the texture is for
        match unsafe { window.as_ref() } {
            Some(window) => window.create_texture_from_image(&self.image),
            None => std::ptr::null_mut(),
        }
    }

    fn texture_size(&self) -> QSize {
        self.size.clone()
    }

    fn texture_byte_count(&self) -> i32 {
        let bytes = i64::from(self.image.depth())
            * i64::from(self.image.width())
            * i64::from(self.image.height())
            / 8;
        i32::try_from(bytes).unwrap_or(i32::MAX)
    }

    fn image(&self) -> QImage {
        self.image.clone()
    }
}

/// Decodes an image, and returns it with its size. A vector image, which has no pixels of its
/// own, is drawn at `scale` times its size, so that it is sharp on a display with that pixel
/// ratio, while its size is still the one before that.
fn decode(data: &[u8], scale: f64) -> Option<(QImage, QSize)> {
    let mut buffer = ffi::buffer_new();
    buffer.pin_mut().set_data(&QByteArray::from(data));
    // SAFETY: the buffer outlives the reader, which is dropped first. The reader opens it.
    let mut reader = unsafe { ffi::image_reader_new(buffer.as_mut_ptr()) };
    let size = reader.size();
    let vector = matches!(reader.format().as_slice(), b"svg" | b"svgz");
    if scale != 1.0 && vector && size.is_valid() {
        let scaled = |n: i32| (f64::from(n) * scale).round() as i32;
        reader
            .pin_mut()
            .set_scaled_size(&QSize::new(scaled(size.width()), scaled(size.height())));
    }
    let image = reader.pin_mut().read();
    drop(reader);
    if image.is_null() {
        return None;
    }
    let size = if size.is_valid() { size } else { image.size() };
    Some((image, size))
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
