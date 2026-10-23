//! A disk cache for the web resources that the UI loads, such as mod icons, which follows HTTP's
//! caching rules as the web UI's browser did. Responses are reused while they are fresh, then
//! revalidated with the server, which can confirm them without sending them again. When the
//! server can't be reached, stale responses are used rather than nothing. The least recently used
//! are evicted to keep the cache within a budget.
//!
//! Each response is a file in the cache directory, named by its URL's hash, holding its status,
//! headers, and the time it was received, which are what the rules are applied to, then its body.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Once, Weak};
use std::time::{Duration, SystemTime};

use anyhow::{bail, ensure, Context, Result};
use bytes::Bytes;
use http_cache_semantics::{AfterResponse, BeforeRequest, CacheOptions, CachePolicy, ResponseLike};
use reqwest::header::{self, HeaderMap, HeaderName, HeaderValue};
use reqwest::{Client, Method, Request, StatusCode, Url};
use tokio::sync::Semaphore;

use super::IoErrorKindExt;

/// Fetches at a time, so that scrolling through a long list of mods doesn't open a connection for
/// every icon it passes.
static PERMITS: Semaphore = Semaphore::const_new(8);

/// The size that the cache is kept within.
const BUDGET: u64 = 256 << 20;

/// How often an entry's use is recorded, as its modification time, by which entries are evicted.
const TOUCH_INTERVAL: Duration = Duration::from_secs(60 * 60);

/// Identifies the format of an entry, which is changed whenever the format is.
const MAGIC: &[u8; 8] = b"mdrhttp\x01";

/// The prefix of entries that are still being written.
const TEMP_PREFIX: &str = ".tmp";

fn options() -> CacheOptions {
    CacheOptions {
        // a single user's cache, as a browser's is
        shared: false,
        ..Default::default()
    }
}

fn dir() -> PathBuf {
    manderrow_paths::cache_dir().join("http")
}

/// Fetches a resource, from the cache when HTTP's rules allow it, and caches it when they allow
/// that. Bodies larger than `max_size` are not loaded.
pub async fn get(client: &Client, url: &Url, max_size: usize) -> Result<Bytes> {
    static TRIM_ON_START: Once = Once::new();
    TRIM_ON_START.call_once(start_trim);

    let key = blake3::hash(url.as_str().as_bytes());
    let path = dir().join(key.to_hex().as_str());

    // Requests for the same resource wait for each other, so that only the first fetches it.
    let lock = lock_for(key);
    let _guard = lock.lock().await;

    let entry = {
        let (path, url) = (path.clone(), url.clone());
        tokio::task::spawn_blocking(move || load(&path, &url)).await?
    };
    let entry = match entry {
        Ok(entry) => entry,
        Err(e) => {
            slog_scope::debug!("Discarding the cached response for {url}: {e:#}");
            remove(path.clone()).await;
            None
        }
    };

    let request = request(url);
    let mut stale = None;
    let mut headers = HeaderMap::new();
    if let Some(entry) = entry {
        let policy = entry.policy(url);
        match policy.before_request(&request, SystemTime::now()) {
            BeforeRequest::Fresh(_) => return Ok(entry.body),
            // with the headers that ask the server to confirm it
            BeforeRequest::Stale {
                request,
                matches: true,
            } => {
                headers = request.headers;
                stale = Some((entry, policy));
            }
            BeforeRequest::Stale { matches: false, .. } => remove(path.clone()).await,
        }
    }

    let mut response = match fetch(client, url, headers, max_size).await {
        Ok(response) if !response.status.is_server_error() => response,
        result => {
            if let Some((entry, _)) = stale {
                let error = match result {
                    Ok(response) => format!("HTTP status {}", response.status),
                    Err(e) => format!("{e:#}"),
                };
                slog_scope::debug!("Using the stale cached response for {url}: {error}");
                return Ok(entry.body);
            }
            result?
        }
    };

    if response.status == StatusCode::NOT_MODIFIED {
        if let Some((entry, policy)) = stale.take() {
            match policy.after_response(&request, &response, response.time) {
                AfterResponse::NotModified(_, mut parts) => {
                    // added for responses that a cache returns, but this one isn't returned
                    parts.headers.remove(header::WARNING);
                    let entry = Entry {
                        status: parts.status,
                        headers: parts.headers,
                        time: response.time,
                        body: entry.body,
                    };
                    save(path, url, &entry).await;
                    return Ok(entry.body);
                }
                AfterResponse::Modified(..) => {}
            }
        }
        // confirming a response other than the one cached, which leaves the request to be made
        // again without asking for that
        remove(path.clone()).await;
        response = fetch(client, url, HeaderMap::new(), max_size).await?;
    }

    if !response.status.is_success() {
        bail!("HTTP status {} for {url}", response.status);
    }

    let policy = CachePolicy::new_options(&request, &response, response.time, options());
    // Responses that are never fresh are worth keeping only when they can be revalidated.
    let reusable = policy.time_to_live(response.time) > Duration::ZERO
        || response.headers.contains_key(header::ETAG)
        || response.headers.contains_key(header::LAST_MODIFIED);
    let entry = Entry {
        status: response.status,
        headers: response.headers,
        time: response.time,
        body: response.body,
    };
    if entry.status == StatusCode::OK && policy.is_storable() && reusable {
        save(path, url, &entry).await;
    } else if stale.is_some() {
        remove(path).await;
    }
    Ok(entry.body)
}

/// The request that is made for a resource, before the cache adds to it.
fn request(url: &Url) -> Request {
    Request::new(Method::GET, url.clone())
}

/// The lock that requests for the resource with this key hold.
fn lock_for(key: blake3::Hash) -> Arc<tokio::sync::Mutex<()>> {
    static LOCKS: LazyLock<
        parking_lot::Mutex<HashMap<blake3::Hash, Weak<tokio::sync::Mutex<()>>>>,
    > = LazyLock::new(Default::default);
    let mut locks = LOCKS.lock();
    if let Some(lock) = locks.get(&key).and_then(Weak::upgrade) {
        return lock;
    }
    // those of finished requests, of which there are only as many as there are requests at once
    locks.retain(|_, lock| lock.strong_count() > 0);
    let lock = Arc::default();
    locks.insert(key, Arc::downgrade(&lock));
    lock
}

struct Fetched {
    status: StatusCode,
    headers: HeaderMap,
    /// When the response was received.
    time: SystemTime,
    /// The body of a successful response.
    body: Bytes,
}

impl ResponseLike for Fetched {
    fn status(&self) -> StatusCode {
        self.status
    }

    fn headers(&self) -> &HeaderMap {
        &self.headers
    }
}

async fn fetch(client: &Client, url: &Url, headers: HeaderMap, max_size: usize) -> Result<Fetched> {
    let _permit = PERMITS.acquire().await?;
    let mut response = client.get(url.clone()).headers(headers).send().await?;
    let time = SystemTime::now();
    let mut body = Vec::new();
    if response.status().is_success() {
        let too_large = || format!("The resource at {url} is too large");
        if let Some(len) = response.content_length() {
            ensure!(len <= max_size as u64, too_large());
            body.reserve_exact(len as usize);
        }
        while let Some(chunk) = response.chunk().await? {
            ensure!(body.len() + chunk.len() <= max_size, too_large());
            body.extend_from_slice(&chunk);
        }
    }
    Ok(Fetched {
        status: response.status(),
        headers: response.headers().clone(),
        time,
        body: body.into(),
    })
}

/// A cached response.
struct Entry {
    status: StatusCode,
    headers: HeaderMap,
    /// When the response was received.
    time: SystemTime,
    body: Bytes,
}

impl ResponseLike for Entry {
    fn status(&self) -> StatusCode {
        self.status
    }

    fn headers(&self) -> &HeaderMap {
        &self.headers
    }
}

impl Entry {
    fn policy(&self, url: &Url) -> CachePolicy {
        CachePolicy::new_options(&request(url), self, self.time, options())
    }

    fn encode_head(&self, url: &Url) -> Vec<u8> {
        fn bytes(out: &mut Vec<u8>, bytes: &[u8]) {
            out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
            out.extend_from_slice(bytes);
        }

        let since_epoch = self
            .time
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default();
        let mut out = Vec::with_capacity(1024);
        out.extend_from_slice(MAGIC);
        bytes(&mut out, url.as_str().as_bytes());
        out.extend_from_slice(&since_epoch.as_secs().to_le_bytes());
        out.extend_from_slice(&since_epoch.subsec_nanos().to_le_bytes());
        out.extend_from_slice(&self.status.as_u16().to_le_bytes());
        out.extend_from_slice(&(self.headers.len() as u32).to_le_bytes());
        for (name, value) in &self.headers {
            bytes(&mut out, name.as_str().as_bytes());
            bytes(&mut out, value.as_bytes());
        }
        out.extend_from_slice(&(self.body.len() as u64).to_le_bytes());
        out
    }

    /// Decodes an entry, which is `None` if it is for another URL than the one expected.
    fn decode(data: Bytes, url: &Url) -> Result<Option<Self>> {
        let mut input = Input(&data);
        ensure!(input.take(MAGIC.len())? == MAGIC, "Unknown format");
        if input.bytes()? != url.as_str().as_bytes() {
            return Ok(None);
        }
        let time = SystemTime::UNIX_EPOCH
            + Duration::new(
                u64::from_le_bytes(input.array()?),
                u32::from_le_bytes(input.array()?),
            );
        let status = StatusCode::from_u16(u16::from_le_bytes(input.array()?))?;
        let count = u32::from_le_bytes(input.array()?);
        let mut headers = HeaderMap::new();
        for _ in 0..count {
            let name = HeaderName::from_bytes(input.bytes()?)?;
            let value = HeaderValue::from_bytes(input.bytes()?)?;
            headers.append(name, value);
        }
        let len = u64::from_le_bytes(input.array()?);
        ensure!(input.0.len() as u64 == len, "The body is truncated");
        let body = data.slice(data.len() - input.0.len()..);
        Ok(Some(Self {
            status,
            headers,
            time,
            body,
        }))
    }
}

struct Input<'a>(&'a [u8]);

impl<'a> Input<'a> {
    fn take(&mut self, len: usize) -> Result<&'a [u8]> {
        let (taken, rest) = self
            .0
            .split_at_checked(len)
            .context("The entry is truncated")?;
        self.0 = rest;
        Ok(taken)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N]> {
        Ok(self.take(N)?.try_into().unwrap())
    }

    fn bytes(&mut self) -> Result<&'a [u8]> {
        let len = u32::from_le_bytes(self.array()?);
        self.take(len as usize)
    }
}

/// Loads the entry at `path`, recording that it was used.
fn load(path: &Path, url: &Url) -> Result<Option<Entry>> {
    // writable, as setting the modification time needs that on Windows
    let mut file = match File::options().read(true).write(true).open(path) {
        Ok(file) => file,
        Err(e) if e.is_not_found() => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;
    let now = SystemTime::now();
    let modified = file.metadata()?.modified()?;
    if now
        .duration_since(modified)
        .is_ok_and(|since| since >= TOUCH_INTERVAL)
    {
        _ = file.set_modified(now);
    }
    Entry::decode(data.into(), url)
}

/// Saves an entry at `path`, replacing whichever is there. Failing to is only logged, as the
/// response it caches can still be used.
async fn save(path: PathBuf, url: &Url, entry: &Entry) {
    let head = entry.encode_head(url);
    let body = entry.body.clone();
    let result = tokio::task::spawn_blocking(move || {
        let dir = path.parent().context("The entry has no directory")?;
        fs::create_dir_all(dir)?;
        let mut file = tempfile::Builder::new()
            .prefix(TEMP_PREFIX)
            .tempfile_in(dir)?;
        file.write_all(&head)?;
        file.write_all(&body)?;
        file.persist(&path)?;
        Ok::<_, anyhow::Error>((head.len() + body.len()) as u64)
    })
    .await
    .map_err(anyhow::Error::from)
    .and_then(|result| result);
    match result {
        Ok(size) => {
            static WRITTEN: AtomicU64 = AtomicU64::new(0);
            if WRITTEN.fetch_add(size, Ordering::Relaxed) + size >= BUDGET / 8 {
                WRITTEN.store(0, Ordering::Relaxed);
                start_trim();
            }
        }
        Err(e) => slog_scope::warn!("Failed to cache the response for {url}: {e:#}"),
    }
}

async fn remove(path: PathBuf) {
    match tokio::fs::remove_file(&path).await {
        Ok(()) => {}
        Err(e) if e.is_not_found() => {}
        Err(e) => slog_scope::warn!("Failed to remove the cached response at {path:?}: {e}"),
    }
}

/// Trims the cache to its budget in the background, unless that is already happening.
fn start_trim() {
    static TRIMMING: AtomicBool = AtomicBool::new(false);
    if TRIMMING.swap(true, Ordering::Acquire) {
        return;
    }
    tokio::task::spawn_blocking(|| {
        if let Err(e) = trim(&dir(), BUDGET) {
            slog_scope::warn!("Failed to trim the HTTP cache: {e}");
        }
        TRIMMING.store(false, Ordering::Release);
    });
}

/// Evicts the least recently used entries once the cache exceeds its budget, until it is well
/// within it, so that this isn't needed again for a while.
fn trim(dir: &Path, budget: u64) -> io::Result<()> {
    let read_dir = match fs::read_dir(dir) {
        Ok(read_dir) => read_dir,
        Err(e) if e.is_not_found() => return Ok(()),
        Err(e) => return Err(e),
    };
    let now = SystemTime::now();
    let mut entries = Vec::new();
    let mut total = 0;
    for dir_entry in read_dir {
        let dir_entry = dir_entry?;
        let metadata = dir_entry.metadata()?;
        if !metadata.is_file() {
            continue;
        }
        let modified = metadata.modified()?;
        if dir_entry
            .file_name()
            .as_encoded_bytes()
            .starts_with(TEMP_PREFIX.as_bytes())
        {
            // left behind by a write that was interrupted, unless it is still being written
            if now
                .duration_since(modified)
                .is_ok_and(|since| since >= TOUCH_INTERVAL)
            {
                _ = fs::remove_file(dir_entry.path());
            }
            continue;
        }
        total += metadata.len();
        entries.push((modified, metadata.len(), dir_entry.path()));
    }
    if total <= budget {
        return Ok(());
    }
    entries.sort_unstable_by_key(|&(modified, ..)| modified);
    let mut evicted = 0;
    for (_, len, path) in entries {
        if total <= budget / 4 * 3 {
            break;
        }
        if fs::remove_file(&path).is_ok() {
            total -= len;
            evicted += 1;
        }
    }
    slog_scope::debug!("Evicted {evicted} responses from the HTTP cache");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(body: &'static [u8]) -> Entry {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("max-age=60"),
        );
        headers.insert(header::ETAG, HeaderValue::from_static("\"abc\""));
        Entry {
            status: StatusCode::OK,
            headers,
            time: SystemTime::UNIX_EPOCH + Duration::new(1_700_000_000, 123),
            body: Bytes::from_static(body),
        }
    }

    fn encode(entry: &Entry, url: &Url) -> Bytes {
        let mut data = entry.encode_head(url);
        data.extend_from_slice(&entry.body);
        data.into()
    }

    #[test]
    fn round_trip() {
        let url = Url::parse("https://example.com/icon.png").unwrap();
        let original = entry(b"body");
        let decoded = Entry::decode(encode(&original, &url), &url)
            .unwrap()
            .unwrap();
        assert_eq!(decoded.status, original.status);
        assert_eq!(decoded.headers, original.headers);
        assert_eq!(decoded.time, original.time);
        assert_eq!(decoded.body, original.body);
    }

    #[test]
    fn other_url() {
        let url = Url::parse("https://example.com/icon.png").unwrap();
        let other = Url::parse("https://example.com/other.png").unwrap();
        assert!(Entry::decode(encode(&entry(b"body"), &url), &other)
            .unwrap()
            .is_none());
    }

    #[test]
    fn truncated() {
        let url = Url::parse("https://example.com/icon.png").unwrap();
        let data = encode(&entry(b"body"), &url);
        for len in 0..data.len() {
            assert!(Entry::decode(data.slice(..len), &url).is_err());
        }
    }

    #[test]
    fn freshness() {
        let url = Url::parse("https://example.com/icon.png").unwrap();
        let mut entry = entry(b"body");
        entry.time = SystemTime::now();
        let policy = entry.policy(&url);
        assert!(matches!(
            policy.before_request(&request(&url), entry.time + Duration::from_secs(30)),
            BeforeRequest::Fresh(_)
        ));
        let BeforeRequest::Stale { request, matches } =
            policy.before_request(&request(&url), entry.time + Duration::from_secs(90))
        else {
            panic!("the response should be stale");
        };
        assert!(matches);
        assert_eq!(request.headers[header::IF_NONE_MATCH], "\"abc\"");
    }

    #[test]
    fn trims_least_recently_used() {
        let dir = tempfile::tempdir().unwrap();
        let now = SystemTime::now();
        for i in 0..8u64 {
            let file = File::create(dir.path().join(i.to_string())).unwrap();
            file.set_len(100).unwrap();
            file.set_modified(now - Duration::from_secs(1000 - i))
                .unwrap();
        }
        trim(dir.path(), 500).unwrap();
        let mut left = fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect::<Vec<_>>();
        left.sort();
        assert_eq!(left, ["5", "6", "7"]);
    }
}
