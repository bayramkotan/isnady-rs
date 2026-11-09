//! Data repositories: where isnady finds its sources (`core/repositories.py`). isnady's own repository
//! (bayramkotan/isnady-data) is the default; the datasets others publish ship as optional repositories with their
//! catalogs; the user can add any repository that has a catalog. A catalog read from the network is kept in
//! data_dir()/repos/ for a day.

use serde_json::{Map, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime};

use super::catalog::{entry_from_catalog, Entry};
use super::{is_url, read_bytes, tables, write_json, ResourceError};
use crate::stats::pyjson;
use crate::{config, paths};

pub const REFRESH_SECONDS: u64 = 24 * 3600;
pub const CATALOG_TIMEOUT: u64 = 10;

#[derive(Debug, Clone, PartialEq)]
pub struct Repository {
    pub id: String,
    pub name: String,
    pub url: Option<String>,
    pub homepage: Option<String>,
    pub description: String,
    pub builtin: bool,
    pub default: bool,
}

fn s(v: &Value, k: &str) -> Option<String> {
    v.get(k)
        .and_then(Value::as_str)
        .filter(|x| !x.is_empty())
        .map(str::to_string)
}

/// The catalogs shipped with isnady, by id.
pub fn bundled() -> &'static HashMap<String, Value> {
    static B: OnceLock<HashMap<String, Value>> = OnceLock::new();
    B.get_or_init(|| {
        tables()
            .repositories
            .iter()
            .filter_map(|c| Some((s(c, "id")?, c.clone())))
            .collect()
    })
}

fn repo_of(cat: &Value, builtin: bool) -> Repository {
    let id = s(cat, "id").unwrap_or_default();
    Repository {
        name: s(cat, "name").unwrap_or_else(|| id.clone()),
        url: s(cat, "url"),
        homepage: s(cat, "homepage"),
        description: cat
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        builtin,
        default: id == tables().default_id,
        id,
    }
}

pub fn builtin_repositories() -> Vec<Repository> {
    let order = &tables().order;
    let mut ids: Vec<&String> = bundled().keys().collect();
    ids.sort_by(|a, b| {
        let ia = order.iter().position(|o| o == *a).unwrap_or(order.len());
        let ib = order.iter().position(|o| o == *b).unwrap_or(order.len());
        (ia, a).cmp(&(ib, b))
    });
    ids.into_iter()
        .map(|i| repo_of(&bundled()[i], true))
        .collect()
}

fn user_file() -> PathBuf {
    paths::data_dir().join("user_repositories.json")
}

pub fn user_repositories() -> Vec<Repository> {
    let Ok(raw) = std::fs::read_to_string(user_file()) else {
        return vec![];
    };
    let Ok(Value::Array(rows)) = serde_json::from_str::<Value>(&raw) else {
        return vec![];
    };
    rows.iter()
        .filter(|r| r.is_object() && s(r, "id").is_some())
        .map(|r| Repository {
            id: s(r, "id").unwrap_or_default(),
            name: r
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            url: r.get("url").and_then(Value::as_str).map(str::to_string),
            homepage: r
                .get("homepage")
                .and_then(Value::as_str)
                .map(str::to_string),
            description: r
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            builtin: false,
            default: false,
        })
        .collect()
}

fn opt(v: &Option<String>) -> Value {
    v.as_ref()
        .map(|x| Value::String(x.clone()))
        .unwrap_or(Value::Null)
}

fn save_user(repos: &[Repository]) -> Result<(), String> {
    let rows: Vec<Value> = repos
        .iter()
        .map(|r| {
            let mut m = Map::new();
            m.insert("id".into(), Value::String(r.id.clone()));
            m.insert("name".into(), Value::String(r.name.clone()));
            m.insert("url".into(), opt(&r.url));
            m.insert("homepage".into(), opt(&r.homepage));
            m.insert("description".into(), Value::String(r.description.clone()));
            Value::Object(m)
        })
        .collect();
    write_json(&user_file(), &pyjson::dumps_indent(&Value::Array(rows), 2))
        .map_err(|e| e.to_string())
}

pub fn all_repositories() -> Vec<Repository> {
    let mut out = builtin_repositories();
    out.extend(user_repositories());
    out
}

pub fn find(repo_id: &str) -> Option<Repository> {
    all_repositories().into_iter().find(|r| r.id == repo_id)
}

/// The repository chosen in Settings → Data sources (the default one unless the user chose another).
pub fn selected() -> Repository {
    let chosen = match config::effective("sources.repository") {
        Value::String(x) if !x.is_empty() => x,
        _ => tables().default_id.clone(),
    };
    find(&chosen)
        .or_else(|| find(&tables().default_id))
        .unwrap_or_else(|| builtin_repositories().remove(0))
}

pub fn select(repo_id: &str) -> Result<Repository, String> {
    let repo =
        find(repo_id).ok_or_else(|| format!("No repository '{repo_id}'. See: iy repo list"))?;
    let value = if repo.id == tables().default_id {
        String::new()
    } else {
        repo.id.clone()
    };
    config::set("sources.repository", Value::String(value))?;
    Ok(repo)
}

fn re(p: &str) -> regex::Regex {
    regex::Regex::new(p).unwrap()
}

/// Where a repository's catalog may be, from what the user typed (`repositories.candidates`).
pub fn candidates(text: &str) -> Vec<String> {
    let text = text.trim().trim_end_matches('/');
    let gh = re(r"^https?://github\.com/([^/]+)/([^/#?]+)(?:/(?:tree|blob)/([^/]+)(/.*)?)?$");
    if let Some(m) = gh.captures(text) {
        let repo = m[2].strip_suffix(".git").unwrap_or(&m[2]);
        let branch = m.get(3).map(|x| x.as_str()).unwrap_or("HEAD");
        let sub = m.get(4).map(|x| x.as_str()).unwrap_or("");
        let base = format!(
            "https://raw.githubusercontent.com/{}/{repo}/{branch}{sub}",
            &m[1]
        );
        if base.ends_with(".json") {
            return vec![base];
        }
        return vec![
            format!("{base}/catalog.json"),
            format!("{base}/isnady.json"),
        ];
    }
    let hf = re(
        r"^https?://huggingface\.co/datasets/([^/]+)/([^/#?]+)(?:/(?:tree|blob|resolve)/([^/]+)(/.*)?)?$",
    );
    if let Some(m) = hf.captures(text) {
        let branch = m.get(3).map(|x| x.as_str()).unwrap_or("main");
        let base = format!(
            "https://huggingface.co/datasets/{}/{}/resolve/{branch}{}",
            &m[1],
            &m[2],
            m.get(4).map(|x| x.as_str()).unwrap_or("")
        );
        if base.ends_with(".json") {
            return vec![base];
        }
        return vec![
            format!("{base}/catalog.json"),
            format!("{base}/isnady.json"),
        ];
    }
    if is_url(text) {
        if text.ends_with(".json") {
            return vec![text.to_string()];
        }
        return vec![format!("{text}/catalog.json"), text.to_string()];
    }
    let path = paths::expand_user(text);
    if path.is_dir() {
        return vec![
            path.join("catalog.json").display().to_string(),
            path.join("isnady.json").display().to_string(),
        ];
    }
    vec![path.display().to_string()]
}

fn read_catalog(location: &str) -> Result<Value, ResourceError> {
    let bytes = read_bytes(location, Some(CATALOG_TIMEOUT))?;
    let text = String::from_utf8_lossy(&bytes);
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    let cat: Value = serde_json::from_str(text)
        .map_err(|e| ResourceError(format!("{location} is not a JSON catalog ({e})")))?;
    if !cat.get("entries").is_some_and(Value::is_array) {
        return Err(ResourceError(format!(
            "{location} is not an isnady catalog (it needs \"entries\": [...]). See tools/build_data_repo.py in \
             isnady for the format."
        )));
    }
    Ok(cat)
}

fn known_external(text: &str) -> Option<&'static Value> {
    let norm = text.to_lowercase();
    let norm = norm.trim_end_matches('/');
    let norm = norm.strip_suffix(".git").unwrap_or(norm);
    let mut ids: Vec<&String> = bundled().keys().collect();
    ids.sort();
    ids.into_iter().map(|i| &bundled()[i]).find(|cat| {
        let home = cat
            .get("homepage")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_lowercase();
        let home = home.trim_end_matches('/');
        !home.is_empty() && norm.starts_with(home)
    })
}

/// Why adding a repository did not work.
pub enum AddError {
    Known(String),
    NotFound(String),
}

impl std::fmt::Display for AddError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AddError::Known(m) | AddError::NotFound(m) => f.write_str(m),
        }
    }
}

/// Add a repository by its address; its catalog is read now, to check it (`repositories.add_repository`).
pub fn add_repository(text: &str, name: Option<&str>) -> Result<Repository, AddError> {
    if let Some(known) = known_external(text) {
        let n = known.get("name").and_then(Value::as_str).unwrap_or("");
        let id = known.get("id").and_then(Value::as_str).unwrap_or("");
        return Err(AddError::Known(format!(
            "{n} is already in the list (repository '{id}'). Choose it: iy repo use {id}"
        )));
    }
    let mut errors = vec![];
    let name = name.filter(|n| !n.is_empty());
    for location in candidates(text) {
        let cat = match read_catalog(&location) {
            Ok(c) => c,
            Err(e) => {
                errors.push(e.0);
                continue;
            }
        };
        let label = name
            .map(str::to_string)
            .or_else(|| s(&cat, "name"))
            .or_else(|| s(&cat, "id"))
            .unwrap_or_else(|| "repository".into());
        let base = re("[^a-z0-9]+")
            .replace_all(&label.to_lowercase(), "-")
            .trim_matches('-')
            .to_string();
        let base = if base.is_empty() {
            "repository".to_string()
        } else {
            base
        };
        let (mut rid, mut n) = (base.clone(), 1);
        while find(&rid).is_some() {
            n += 1;
            rid = format!("{base}-{n}");
        }
        let repo = Repository {
            name: name
                .map(str::to_string)
                .or_else(|| s(&cat, "name"))
                .unwrap_or_else(|| rid.clone()),
            url: Some(location.clone()),
            homepage: s(&cat, "homepage").or_else(|| is_url(text).then(|| text.to_string())),
            description: cat
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            builtin: false,
            default: false,
            id: rid.clone(),
        };
        let mut repos = user_repositories();
        repos.push(repo.clone());
        save_user(&repos).map_err(AddError::NotFound)?;
        let _ = write_json(&cache_path(&rid), &pyjson::dumps(&cat));
        return Ok(repo);
    }
    Err(AddError::NotFound(format!(
        "No isnady catalog found there. A repository needs a catalog.json listing its sources (see \
         tools/build_data_repo.py). Tried:\n  {}",
        errors.join("\n  ")
    )))
}

pub fn remove_repository(repo_id: &str) -> bool {
    let repos = user_repositories();
    let kept: Vec<Repository> = repos.iter().filter(|r| r.id != repo_id).cloned().collect();
    if kept.len() == repos.len() {
        return false;
    }
    let _ = save_user(&kept);
    let _ = std::fs::remove_file(cache_path(repo_id));
    if config::effective("sources.repository").as_str() == Some(repo_id) {
        let _ = config::set("sources.repository", Value::String(String::new()));
    }
    true
}

fn cache_path(repo_id: &str) -> PathBuf {
    paths::data_dir()
        .join("repos")
        .join(format!("{repo_id}.json"))
}

/// Where a catalog came from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum From {
    Network,
    Saved,
    Shipped,
}

static LAST: Mutex<Option<HashMap<String, (Value, From)>>> = Mutex::new(None);

/// Forget the catalogs read in this run (after the repositories' folder is deleted).
pub fn forget() {
    if let Ok(mut g) = LAST.lock() {
        *g = None;
    }
}

fn layout(cat: &Value) -> &'static str {
    for e in cat
        .get("entries")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let loc = e.get("location").and_then(Value::as_str).unwrap_or("");
        if e.get("kind").and_then(Value::as_str) == Some("hadith") && !loc.contains("://") {
            return if loc.starts_with("hadith/") {
                "kind/road"
            } else {
                "road"
            };
        }
    }
    ""
}

/// (catalog, where it came from). Never fails for a built-in repository (`repositories.catalog`).
pub fn catalog(repo: &Repository, refresh: bool) -> Result<(Value, From), ResourceError> {
    if !refresh {
        if let Some(hit) = LAST
            .lock()
            .ok()
            .and_then(|g| g.as_ref().and_then(|m| m.get(&repo.id).cloned()))
        {
            return Ok(hit);
        }
    }
    let cache = cache_path(&repo.id);
    let age = std::fs::metadata(&cache)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| SystemTime::now().duration_since(t).ok());
    let mut fresh = age.is_some_and(|a| a < Duration::from_secs(REFRESH_SECONDS));
    if fresh {
        if let Some(ship) = bundled().get(&repo.id) {
            fresh = match std::fs::read_to_string(&cache)
                .ok()
                .and_then(|t| serde_json::from_str::<Value>(&t).ok())
            {
                Some(saved) => {
                    !["library", "quran"]
                        .iter()
                        .any(|k| ship.get(*k).is_some() && saved.get(*k).is_none())
                        && layout(ship) == layout(&saved)
                }
                None => false,
            };
        }
    }
    let mut result = None;
    if let Some(url) = &repo.url {
        if refresh || !fresh {
            if let Ok(cat) = read_catalog(url) {
                let _ = write_json(&cache, &pyjson::dumps(&cat));
                result = Some((cat, From::Network));
            }
        }
    }
    if result.is_none() && cache.exists() {
        result = std::fs::read_to_string(&cache)
            .ok()
            .and_then(|t| serde_json::from_str::<Value>(&t).ok())
            .map(|c| (c, From::Saved));
    }
    let result = match result {
        Some(r) => r,
        None => match bundled().get(&repo.id) {
            Some(c) => (c.clone(), From::Shipped),
            None => {
                return Err(ResourceError(format!(
                    "The catalog of {} cannot be read ({}).",
                    repo.name,
                    repo.url.clone().unwrap_or_else(|| "None".into())
                )))
            }
        },
    };
    if let Ok(mut g) = LAST.lock() {
        g.get_or_insert_with(HashMap::new)
            .insert(repo.id.clone(), result.clone());
    }
    Ok(result)
}

/// The repository's sources as catalog entries, locations made absolute (`repositories.entries`).
pub fn entries(repo: &Repository, refresh: bool) -> Result<Vec<Entry>, ResourceError> {
    let (cat, _) = catalog(repo, refresh)?;
    let base = repo.url.clone().unwrap_or_default();
    let mut out = vec![];
    for key in ["entries", "library"] {
        for raw in cat.get(key).and_then(Value::as_array).into_iter().flatten() {
            if let Some(e) = entry_from_catalog(raw, &repo.id, &base) {
                out.push(e);
            }
        }
    }
    Ok(out)
}
