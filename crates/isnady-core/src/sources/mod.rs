//! Where the books come from (RS16): the data repositories and their catalogs (`core/repositories.py`), the
//! sources of a repository and the user's own (`core/catalog.py`), reading a file or a URL (`data/fetch.py`), the
//! database written to (`data/db.py`: connect, cleanup), a ready database (`data/ready.py`) and back to zero
//! (`core/reset.py`). The schema, the import formats and the catalogs shipped with isnady are read from
//! `data/sources.json`, written from the Python isnady by `tools/gen_tables.py`.
//!
//! Importing a book: `importers` (one per data format) and `imports` (the import and everything derived from it
//! brought up to date, as `core/imports.py`).

pub mod catalog;
pub mod importers;
pub mod imports;
pub mod repos;
pub mod store;

use serde::Deserialize;
use serde_json::Value;
use std::path::Path;
use std::sync::OnceLock;
use std::time::Duration;

use crate::paths;

#[derive(Deserialize)]
pub struct Tables {
    pub schema_version: i64,
    pub oldest_upgradable: i64,
    pub schema: String,
    pub cleanup: String,
    pub migrations: Vec<(i64, Vec<String>)>,
    /// (format id, title, description), as `list_importers()` lists them
    pub importers: Vec<(String, String, String)>,
    /// the catalogs shipped with isnady (isnady/data/repos/*.json)
    pub repositories: Vec<Value>,
    pub order: Vec<String>,
    pub default_id: String,
    pub languages: Vec<(String, String)>,
    pub default_languages: Vec<String>,
    pub two_letters: Vec<(String, String)>,
    /// language → book id → {"en": the English it was written for, "text"} (`titles.about`)
    pub about: std::collections::HashMap<String, std::collections::HashMap<String, Value>>,
}

static TABLES: OnceLock<Tables> = OnceLock::new();

pub fn tables() -> &'static Tables {
    TABLES.get_or_init(|| {
        serde_json::from_str(include_str!("../../data/sources.json"))
            .expect("data/sources.json is written by tools/gen_tables.py")
    })
}

/// A resource could not be read; the message is meant for the user (`fetch.ResourceError`).
#[derive(Debug, Clone)]
pub struct ResourceError(pub String);

impl std::fmt::Display for ResourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

pub const TIMEOUT_SECONDS: u64 = 60;

fn user_agent() -> String {
    format!(
        "isnady/{} (+https://github.com/bayramkotan/isnady)",
        env!("CARGO_PKG_VERSION")
    )
}

/// `fetch.is_url`.
pub fn is_url(location: &str) -> bool {
    let l = location.trim_start().to_ascii_lowercase();
    l.starts_with("http://") || l.starts_with("https://")
}

/// How a resource is read (`fetch.Auth`): none, basic (username + password), bearer (a token) or apikey (a key sent
/// in a header or a query parameter). Credentials are used for the request only; the database keeps the kind alone.
#[derive(Debug, Clone)]
pub struct Auth {
    pub kind: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub token: Option<String>,
    pub key_name: Option<String>,
    pub key_in: String,
}

impl Default for Auth {
    fn default() -> Self {
        Auth {
            kind: "none".into(),
            username: None,
            password: None,
            token: None,
            key_name: None,
            key_in: "header".into(),
        }
    }
}

impl Auth {
    /// What the database records (`Auth.describe`).
    pub fn describe(&self) -> String {
        if self.kind == "apikey" {
            format!(
                "apikey ({}: {})",
                self.key_in,
                self.key_name.as_deref().unwrap_or("None")
            )
        } else {
            self.kind.clone()
        }
    }
}

fn base64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for c in data.chunks(3) {
        let b = [c[0], *c.get(1).unwrap_or(&0), *c.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        for i in 0..4 {
            if i <= c.len() {
                out.push(T[((n >> (18 - 6 * i)) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// Base64 back to bytes (a data URL's body); None when it is not base64.
pub fn base64_decode(text: &str) -> Option<Vec<u8>> {
    let body = text.split_once("base64,").map(|x| x.1).unwrap_or(text);
    let mut out = Vec::with_capacity(body.len() * 3 / 4);
    let (mut acc, mut bits) = (0u32, 0u32);
    for c in body.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            b'=' | b'\n' | b'\r' | b' ' => continue,
            _ => return None,
        } as u32;
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    Some(out)
}

fn download(location: &str, auth: &Auth, timeout: Option<u64>) -> Result<Vec<u8>, ResourceError> {
    let secs = timeout.unwrap_or(TIMEOUT_SECONDS);
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(secs))
        .try_proxy_from_env(true)
        .build();
    let mut url = location.to_string();
    let mut headers: Vec<(String, String)> = vec![
        ("User-Agent".into(), user_agent()),
        ("Accept".into(), "application/json, */*".into()),
    ];
    match auth.kind.as_str() {
        "none" => {}
        "basic" => {
            let (Some(u), Some(p)) = (&auth.username, &auth.password) else {
                return Err(ResourceError(
                    "Basic authentication needs both a username and a password.".into(),
                ));
            };
            headers.push((
                "Authorization".into(),
                format!("Basic {}", base64(format!("{u}:{p}").as_bytes())),
            ));
        }
        "bearer" => {
            let Some(t) = auth.token.as_deref().filter(|t| !t.is_empty()) else {
                return Err(ResourceError("Bearer authentication needs a token.".into()));
            };
            headers.push(("Authorization".into(), format!("Bearer {t}")));
        }
        "apikey" => {
            let (Some(t), Some(name)) = (
                auth.token.as_deref().filter(|t| !t.is_empty()),
                auth.key_name.as_deref().filter(|t| !t.is_empty()),
            ) else {
                return Err(ResourceError(
                    "API key authentication needs a key and the header/parameter name to send it in.".into(),
                ));
            };
            if auth.key_in == "query" {
                if let Ok(mut u) = url::Url::parse(&url) {
                    u.query_pairs_mut().append_pair(name, t);
                    url = u.to_string();
                }
            } else {
                headers.push((name.to_string(), t.to_string()));
            }
        }
        other => {
            return Err(ResourceError(format!(
                "Unknown authentication type: {other}"
            )))
        }
    }
    let mut req = agent.get(&url);
    for (k, v) in &headers {
        req = req.set(k, v);
    }
    match req.call() {
        Ok(r) => {
            let mut out = vec![];
            std::io::Read::read_to_end(&mut r.into_reader(), &mut out).map_err(|e| {
                if e.kind() == std::io::ErrorKind::TimedOut {
                    ResourceError(format!("Timed out after {secs}s: {location}"))
                } else {
                    ResourceError(format!("Could not reach {location}: {e}"))
                }
            })?;
            Ok(out)
        }
        Err(ureq::Error::Status(code, _)) => Err(ResourceError(match code {
            401 | 403 => format!(
                "Access denied ({code}) for {location}. The resource may need authentication, or the credentials \
                 were rejected."
            ),
            404 => format!("Not found (404): {location}"),
            _ => format!("HTTP {code} while reading {location}"),
        })),
        Err(e) => {
            let text = e.to_string();
            if text.to_lowercase().contains("timed out") {
                Err(ResourceError(format!(
                    "Timed out after {secs}s: {location}"
                )))
            } else {
                Err(ResourceError(format!("Could not reach {location}: {text}")))
            }
        }
    }
}

// ------------------------------------------------------------------ downloads ahead of time (fetch.prefetch)
type Download = std::thread::JoinHandle<Result<Vec<u8>, ResourceError>>;

fn ahead() -> &'static std::sync::Mutex<std::collections::HashMap<String, Download>> {
    static AHEAD: OnceLock<std::sync::Mutex<std::collections::HashMap<String, Download>>> =
        OnceLock::new();
    AHEAD.get_or_init(Default::default)
}

/// What runs when a download ahead of time is in: it may start the files it names.
pub type Then = Box<dyn FnOnce(&[u8]) + Send>;

/// Start downloading a public URL now; `read_bytes_with` takes it when it is asked for (`fetch.prefetch`).
pub fn prefetch(url: &str, then: Option<Then>) {
    if !is_url(url) {
        return;
    }
    let Ok(mut a) = ahead().lock() else {
        return;
    };
    if a.contains_key(url) {
        return;
    }
    let u = url.to_string();
    let job = std::thread::spawn(move || {
        let data = download(&u, &Auth::default(), None)?;
        if let Some(f) = then {
            f(&data);
        }
        Ok(data)
    });
    a.insert(url.to_string(), job);
}

/// Drop the downloads nobody asked for (`fetch.forget_prefetched`); their threads end by themselves.
pub fn forget_prefetched() {
    if let Ok(mut a) = ahead().lock() {
        a.clear();
    }
}

/// The raw bytes of a local file or a URL, read with this authentication (`fetch.read_bytes`).
pub fn read_bytes_with(
    location: &str,
    auth: &Auth,
    timeout: Option<u64>,
) -> Result<Vec<u8>, ResourceError> {
    if is_url(location) && auth.kind == "none" {
        let job = ahead().lock().ok().and_then(|mut a| a.remove(location));
        if let Some(j) = job {
            if let Ok(Ok(data)) = j.join() {
                return Ok(data);
            }
        }
    }
    if is_url(location) {
        return download(location, auth, timeout);
    }
    read_bytes(location, timeout)
}

/// The raw bytes of a local file or a URL (`fetch.read_bytes`, without authentication).
pub fn read_bytes(location: &str, timeout: Option<u64>) -> Result<Vec<u8>, ResourceError> {
    if is_url(location) {
        return download(location, &Auth::default(), timeout);
    }
    let path = match location.strip_prefix("file://") {
        Some(rest) => std::path::PathBuf::from(rest),
        None => paths::expand_user(location),
    };
    if !path.exists() {
        return Err(ResourceError(format!("File not found: {}", path.display())));
    }
    if path.is_dir() {
        return Err(ResourceError(format!(
            "Expected a file but got a folder: {}",
            path.display()
        )));
    }
    std::fs::read(&path).map_err(|e| ResourceError(format!("{}: {e}", path.display())))
}

/// A large file (the ready database) written to disk as it comes, not held in memory: a URL or a local file
/// (`fetch.save_to`). `progress(done, total)` after every MiB (total 0 when the server does not say). The bytes written.
pub fn save_to(
    location: &str,
    target: &Path,
    progress: &mut dyn FnMut(u64, u64),
    timeout: Option<u64>,
) -> Result<u64, ResourceError> {
    use std::io::{Read as _, Write as _};
    if let Some(dir) = target.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let (mut source, total): (Box<dyn std::io::Read>, u64) = if is_url(location) {
        let secs = timeout.unwrap_or(TIMEOUT_SECONDS);
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(secs))
            .timeout_read(Duration::from_secs(secs))
            .try_proxy_from_env(true)
            .build();
        match agent
            .get(location)
            .set("User-Agent", &user_agent())
            .set("Accept", "application/json, */*")
            .call()
        {
            Ok(r) => {
                let total = r
                    .header("Content-Length")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0);
                (Box::new(r.into_reader()), total)
            }
            Err(ureq::Error::Status(404, _)) => {
                return Err(ResourceError(format!("Not found (404): {location}")))
            }
            Err(ureq::Error::Status(code, _)) => {
                return Err(ResourceError(format!(
                    "HTTP {code} while reading {location}"
                )))
            }
            Err(e) => {
                let text = e.to_string();
                return Err(ResourceError(
                    if text.to_lowercase().contains("timed out") {
                        format!("Timed out after {secs}s: {location}")
                    } else {
                        format!("Could not reach {location}: {text}")
                    },
                ));
            }
        }
    } else {
        let path = match location.strip_prefix("file://") {
            Some(rest) => std::path::PathBuf::from(rest),
            None => paths::expand_user(location),
        };
        if !path.is_file() {
            return Err(ResourceError(format!("File not found: {}", path.display())));
        }
        let total = path.metadata().map(|m| m.len()).unwrap_or(0);
        let f = std::fs::File::open(&path)
            .map_err(|e| ResourceError(format!("{}: {e}", path.display())))?;
        (Box::new(f), total)
    };
    let fail = |e: std::io::Error| {
        let _ = std::fs::remove_file(target);
        ResourceError(format!("Could not reach {location}: {e}"))
    };
    let mut out = std::fs::File::create(target)
        .map_err(|e| ResourceError(format!("{}: {e}", target.display())))?;
    let mut block = vec![0u8; 1 << 20];
    let mut done = 0u64;
    loop {
        // a whole MiB (or the end) before each progress call, as Python's read(1 MiB)
        let mut filled = 0;
        while filled < block.len() {
            let n = source.read(&mut block[filled..]).map_err(fail)?;
            if n == 0 {
                break;
            }
            filled += n;
        }
        if filled == 0 {
            break;
        }
        out.write_all(&block[..filled]).map_err(fail)?;
        done += filled as u64;
        progress(done, total);
    }
    Ok(done)
}

/// The licence's tier: A may be shared further, B with conditions, C (unknown) not (`db.tier_for_license`).
pub fn tier_for_license(license: Option<&str>) -> String {
    const A: [&str; 11] = [
        "unlicense",
        "cc0",
        "cc0-1.0",
        "public domain",
        "public-domain",
        "pd",
        "cc-by",
        "cc-by-4.0",
        "cc by",
        "cc by 4.0",
        "mit",
    ];
    const B: [&str; 10] = [
        "odbl",
        "odbl-1.0",
        "cc-by-sa",
        "cc-by-sa-4.0",
        "cc by-sa",
        "cc-by-nc",
        "cc-by-nc-4.0",
        "cc-by-nc-sa",
        "cc-by-nc-sa-4.0",
        "cc by-nc-sa",
    ];
    let Some(l) = license.filter(|l| !l.is_empty()) else {
        return "C".into();
    };
    let name = crate::py::trim(l).to_lowercase();
    if A.contains(&name.as_str()) {
        "A".into()
    } else if B.contains(&name.as_str()) || name.contains("-nc") || name.contains("-sa") {
        "B".into()
    } else {
        "C".into()
    }
}

/// The time now in UTC to the second, as `db.now_iso` writes it ("2026-11-09T14:00:00+00:00").
pub fn now_iso() -> String {
    chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S+00:00")
        .to_string()
}

/// A link found inside a resource, against the resource's own location (`fetch.resolve`).
pub fn resolve(base: &str, relative: &str) -> String {
    if is_url(relative) || Path::new(relative).is_absolute() {
        return relative.to_string();
    }
    if is_url(base) {
        return url::Url::parse(base)
            .and_then(|b| b.join(relative))
            .map(|u| u.to_string())
            .unwrap_or_else(|_| relative.to_string());
    }
    let b = paths::expand_user(base);
    b.parent()
        .unwrap_or_else(|| Path::new(""))
        .join(relative)
        .display()
        .to_string()
}

/// A location of a catalog made absolute against the catalog's own address (`repositories.absolute`).
pub fn absolute(base: &str, location: &str) -> String {
    if !base.is_empty() && !is_url(location) && !Path::new(location).is_absolute() {
        resolve(base, location)
    } else {
        location.to_string()
    }
}

/// The import formats: (id, title, description) (`iy formats`).
pub fn importers() -> &'static [(String, String, String)] {
    &tables().importers
}

pub fn has_importer(format_id: &str) -> bool {
    tables().importers.iter().any(|(id, _, _)| id == format_id)
}

/// `slugify` of the importers.
pub fn slugify(text: &str) -> String {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    let re = RE.get_or_init(|| regex::Regex::new("[^a-z0-9]+").unwrap());
    let slug = re
        .replace_all(&text.to_lowercase(), "-")
        .trim_matches('-')
        .to_string();
    let slug: String = slug.chars().take(80).collect();
    if slug.is_empty() {
        "source".to_string()
    } else {
        slug
    }
}

/// Write a JSON file through a temporary one, as the Python isnady does (`tmp.replace(path)`).
pub(crate) fn write_json(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path)
}

/// A format whose importer is not here yet: said where an import of it is asked for.
pub fn import_later(format_id: &str) -> String {
    format!(
        "Importing the format '{format_id}' comes to isnady-rs next; until then import it with the Python isnady \
         (iy catalog import, or File → Data Sources). Both read and write the same database."
    )
}
