//! Where the books come from (RS16): the data repositories and their catalogs (`core/repositories.py`), the
//! sources of a repository and the user's own (`core/catalog.py`), reading a file or a URL (`data/fetch.py`), the
//! database written to (`data/db.py`: connect, cleanup), a ready database (`data/ready.py`) and back to zero
//! (`core/reset.py`). The schema, the import formats and the catalogs shipped with isnady are read from
//! `data/sources.json`, written from the Python isnady by `tools/gen_tables.py`.
//!
//! Importing a book (the importers, the chain reader, the narrator matcher) comes with RS17; until then `import`,
//! `update` and `languages add` say so.

pub mod catalog;
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

fn download(location: &str, timeout: Option<u64>) -> Result<Vec<u8>, ResourceError> {
    let secs = timeout.unwrap_or(TIMEOUT_SECONDS);
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(secs))
        .try_proxy_from_env(true)
        .build();
    let resp = agent
        .get(location)
        .set("User-Agent", &user_agent())
        .set("Accept", "application/json, */*")
        .call();
    match resp {
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

/// The raw bytes of a local file or a URL (`fetch.read_bytes`, without authentication).
pub fn read_bytes(location: &str, timeout: Option<u64>) -> Result<Vec<u8>, ResourceError> {
    if is_url(location) {
        return download(location, timeout);
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

/// Importing comes with the chain reader (RS17): said where an import is asked for.
pub const IMPORT_LATER: &str = "Importing books comes with the chain reader in isnady-rs; until then import them with \
     the Python isnady (iy catalog import, or File → Data Sources). Both read and write the same database.";
