use bytes::Bytes;
use pin_project_lite::pin_project;
use reqwest::Response;
use tokio::io::{AsyncBufRead, AsyncRead};
use tokio_util::io::StreamReader;

pub type ResponseReader = StreamReader<ReqwestBytesStream, Bytes>;

mod private {
    use std::io::Result;

    use bytes::Bytes;
    use futures::{Stream, TryStreamExt};
    use reqwest::Response;

    pub type ReqwestBytesStream = impl Stream<Item = Result<Bytes>>;

    pub fn bytes_stream(response: Response) -> ReqwestBytesStream {
        response.bytes_stream().map_err(std::io::Error::other)
    }
}

pub use private::ReqwestBytesStream;

use crate::util::progress::Step;

use super::{Progress, UsizeExt};

pub trait ResponseExt {
    fn reader(self) -> ResponseReader;

    fn reader_with_progress(self, progress: &Progress) -> ProgressReader<'_, ResponseReader>;
}

impl ResponseExt for Response {
    fn reader(self) -> ResponseReader {
        ResponseReader::new(private::bytes_stream(self))
    }

    fn reader_with_progress(self, progress: &Progress) -> ProgressReader<'_, ResponseReader> {
        let expected = self.content_length();
        if expected.is_none() {
            slog_scope::warn!("No progress available for HTTP response, because no content length was provided by the server");
        }
        let mut step = progress.step();
        ProgressReader {
            reader: self.reader(),
            progress: expected.map(move |expected| {
                step.add(0, expected);
                step
            }),
        }
    }
}

pin_project! {
    pub struct ProgressReader<'a, R> {
        #[pin]
        reader: R,
        progress: Option<Step<'a>>,
    }
}

impl<'a, R: AsyncRead> AsyncRead for ProgressReader<'a, R> {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let this = self.project();
        let initial_filled = buf.filled().len();
        this.reader.poll_read(cx, buf).map_ok(|()| {
            if let Some(progress) = this.progress {
                progress.add(
                    (buf.filled().len() - initial_filled).as_u64(),
                    0,
                );
            }
            ()
        })
    }
}

impl<'a, R: AsyncBufRead> AsyncBufRead for ProgressReader<'a, R> {
    fn poll_fill_buf(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<&[u8]>> {
        let this = self.project();
        this.reader.poll_fill_buf(cx)
    }

    fn consume(self: std::pin::Pin<&mut Self>, amt: usize) {
        let this = self.project();
        this.reader.consume(amt);
        if let Some(progress) = this.progress {
            progress.add(
                amt.as_u64(),
                0,
            );
        }
    }
}
