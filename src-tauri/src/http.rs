use std::io::{BufRead, Read};

use anyhow::Result;
use bytes::{Buf, Bytes};
use tokio::sync::mpsc::Receiver;

use crate::Reqwest;

pub struct StreamReadable {
    rx: Receiver<Result<Bytes, std::io::Error>>,
    bytes: Bytes,
}

impl StreamReadable {
    fn new(rx: Receiver<Result<Bytes, std::io::Error>>) -> Self {
        Self {
            rx,
            bytes: Bytes::new(),
        }
    }
}

impl Read for StreamReadable {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        if self.fill_buf()?.is_empty() {
            return Ok(0);
        }

        let bytes = &mut self.bytes;

        // copy_to_slice requires the bytes to have enough remaining bytes
        // to fill buf.
        let n = buf.len().min(bytes.remaining());

        // <Bytes as Buf>::copy_to_slice copies and consumes the bytes
        bytes.copy_to_slice(&mut buf[..n]);

        Ok(n)
    }
}

impl BufRead for StreamReadable {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        let bytes = &mut self.bytes;

        if !bytes.has_remaining() {
            if let Some(new_bytes) = self.rx.blocking_recv() {
                // new_bytes are guaranteed to be non-empty.
                *bytes = new_bytes?;
            }
        }

        Ok(&*bytes)
    }

    fn consume(&mut self, amt: usize) {
        self.bytes.advance(amt);
    }
}

pub async fn fetch_as_blocking(request: reqwest::RequestBuilder) -> Result<StreamReadable> {
    let mut resp = request.send().await?.error_for_status()?;
    let (tx, rx) = tokio::sync::mpsc::channel(1);
    tokio::task::spawn(async move {
        while let Some(r) = resp.chunk().await.transpose() {
            if tx
                .send(r.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)))
                .await
                .is_err()
            {
                break;
            }
        }
    });
    Ok(StreamReadable::new(rx))
}
