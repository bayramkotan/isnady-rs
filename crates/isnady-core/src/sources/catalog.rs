//! The sources of a repository and the user's own (`core/catalog.py`): each with what of it is in the database now,
//! what the repository changed since it was imported (UPD5), removing what one imported, and the languages of the
//! books (LANG2).

use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Map, Value};
use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;

use super::repos::{self, Repository};
use super::{absolute, has_importer, slugify, store, tables, write_json, ResourceError};
use crate::stats::pyjson;
use crate::{paths, quran};

pub const LIBRARY_KINDS: [&str; 4] = ["quran", "sahifa", "sira", "tabaqat"];
pub const KIND_ORDER: [&str; 6] = ["quran", "hadith", "sahifa", "sira", "tabaqat", "rijal"];

#[derive(Debug, Clone, Default)]
pub struct Entry {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub format_id: String,
    pub location: String,
    pub description: String,
    pub license: Option<String>,
    pub source_name: String,
    pub source_key: String,
    pub book: Option<String>,
    pub languages: Vec<String>,
    pub builtin: bool,
    pub auth_kind: String,
    pub title_ar: String,
    pub tradition: String,
    pub repo: String,
    pub aliases: Vec<String>,
    pub hadith: Option<i64>,
    pub compiler: String,
    pub size: Option<i64>,
    pub translations: Option<i64>,
    pub sha256: Option<String>,
    pub has_fallback: bool,
    /// {"format", "location"}: the origin, when the repository's copy cannot be read
    pub fallback: Option<Value>,
    /// an API key's header or query parameter, and where it goes (header | query)
    pub key_name: Option<String>,
    pub key_in: String,
    /// a table's columns (format "table"), a book in several files, a work whose entries the lives quote
    pub mapping: Option<Value>,
    pub parts: Option<Value>,
    pub entries_work: Option<String>,
    pub origin_name: String,
    pub origin_url: String,
}

impl Entry {
    pub fn library(&self) -> bool {
        LIBRARY_KINDS.contains(&self.kind.as_str())
    }
}

fn st(v: &Value, k: &str) -> Option<String> {
    v.get(k).and_then(Value::as_str).map(str::to_string)
}

fn nonempty(v: &Value, k: &str) -> Option<String> {
    st(v, k).filter(|x| !x.is_empty())
}

fn strings(v: &Value, k: &str) -> Vec<String> {
    v.get(k)
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// A catalog entry of a repository, its location made absolute (`catalog.entry_from_catalog`); None for an entry
/// the Python isnady skips (no id, title, format or location).
pub fn entry_from_catalog(raw: &Value, repo_id: &str, base: &str) -> Option<Entry> {
    let id = st(raw, "id")?;
    let title = st(raw, "title")?;
    let format_id = st(raw, "format")?;
    let location = st(raw, "location")?;
    let kind = st(raw, "kind").unwrap_or_else(|| "hadith".into());
    let origin = raw.get("origin").cloned().unwrap_or(Value::Null);
    let fixed = match format_id.as_str() {
        "taqrib" => Some(("openiti-taqrib", "Ibn Hajar, Taqrib al-Tahdhib")),
        "najashi" => Some(("openiti-najashi", "al-Najashi, Rijal")),
        _ => None,
    };
    let mut source_key = nonempty(raw, "source_key");
    let mut source_name = nonempty(raw, "source_name");
    if let Some((k, n)) = fixed {
        if source_key.is_none() {
            source_key = Some(k.into());
            source_name = source_name.or_else(|| Some(n.into()));
        }
    }
    let library = LIBRARY_KINDS.contains(&kind.as_str());
    let source_name = source_name.unwrap_or_else(|| {
        if library {
            title.clone()
        } else {
            nonempty(&origin, "name").unwrap_or_else(|| title.clone())
        }
    });
    Some(Entry {
        location: absolute(base, &location),
        description: st(raw, "description").unwrap_or_default(),
        license: st(raw, "license"),
        source_key: source_key.unwrap_or_else(|| id.clone()),
        book: st(raw, "book"),
        languages: strings(raw, "languages"),
        builtin: true,
        auth_kind: "none".into(),
        title_ar: st(raw, "title_ar").unwrap_or_default(),
        tradition: st(raw, "tradition").unwrap_or_else(|| "sunni".into()),
        repo: repo_id.into(),
        aliases: strings(raw, "aliases"),
        hadith: raw.get("hadith").and_then(Value::as_i64),
        compiler: st(raw, "compiler").unwrap_or_default(),
        size: raw.get("size").and_then(Value::as_i64),
        translations: raw.get("translations").and_then(Value::as_i64),
        sha256: st(raw, "sha256"),
        has_fallback: raw.get("fallback").is_some_and(|f| !f.is_null()),
        fallback: raw.get("fallback").filter(|f| !f.is_null()).cloned(),
        key_name: None,
        key_in: "header".into(),
        mapping: raw.get("mapping").filter(|f| !f.is_null()).cloned(),
        parts: raw
            .get("parts")
            .and_then(Value::as_array)
            .filter(|a| !a.is_empty())
            .map(|a| {
                Value::Array(
                    a.iter()
                        .map(|p| match p {
                            Value::Object(o) => {
                                let mut o = o.clone();
                                let loc = o
                                    .get("location")
                                    .and_then(Value::as_str)
                                    .unwrap_or("")
                                    .to_string();
                                o.insert("location".into(), Value::String(absolute(base, &loc)));
                                Value::Object(o)
                            }
                            Value::String(l) => Value::String(absolute(base, l)),
                            other => other.clone(),
                        })
                        .collect(),
                )
            }),
        entries_work: st(raw, "entries"),
        origin_name: st(&origin, "name").unwrap_or_default(),
        origin_url: st(&origin, "url").unwrap_or_default(),
        source_name,
        id,
        title,
        kind,
        format_id,
    })
}

/// The sources of a repository: the one chosen in Settings → Data sources unless another is named.
pub fn builtin_entries(repo_id: Option<&str>) -> Result<Vec<Entry>, ResourceError> {
    let repo = match repo_id {
        Some(r) if !r.is_empty() => repos::find(r)
            .ok_or_else(|| ResourceError(format!("No repository '{r}'. See: iy repo list")))?,
        _ => repos::selected(),
    };
    repos::entries(&repo, false)
}

// ------------------------------------------------------------------ user entries
fn user_file() -> PathBuf {
    paths::data_dir().join("user_sources.json")
}

fn user_rows() -> Vec<Value> {
    std::fs::read_to_string(user_file())
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default()
        .into_iter()
        .filter(|r| r.is_object() && r.get("id").is_some_and(Value::is_string))
        .collect()
}

fn user_entry(r: &Value) -> Entry {
    Entry {
        id: st(r, "id").unwrap_or_default(),
        title: st(r, "title").unwrap_or_default(),
        kind: st(r, "kind").unwrap_or_else(|| "hadith".into()),
        format_id: st(r, "format_id").unwrap_or_default(),
        location: st(r, "location").unwrap_or_default(),
        description: st(r, "description").unwrap_or_default(),
        license: st(r, "license"),
        source_name: st(r, "source_name").unwrap_or_default(),
        source_key: st(r, "source_key").unwrap_or_default(),
        book: st(r, "book"),
        languages: strings(r, "languages"),
        builtin: false,
        auth_kind: st(r, "auth_kind").unwrap_or_else(|| "none".into()),
        title_ar: st(r, "title_ar").unwrap_or_default(),
        tradition: st(r, "tradition").unwrap_or_else(|| "sunni".into()),
        repo: st(r, "repo").unwrap_or_default(),
        aliases: strings(r, "aliases"),
        hadith: r.get("hadith").and_then(Value::as_i64),
        compiler: st(r, "compiler").unwrap_or_default(),
        size: r.get("size").and_then(Value::as_i64),
        translations: r.get("translations").and_then(Value::as_i64),
        sha256: st(r, "sha256"),
        has_fallback: r.get("fallback").is_some_and(|f| !f.is_null()),
        fallback: r.get("fallback").filter(|f| !f.is_null()).cloned(),
        key_name: st(r, "key_name"),
        key_in: st(r, "key_in").unwrap_or_else(|| "header".into()),
        mapping: r.get("mapping").filter(|f| !f.is_null()).cloned(),
        parts: r.get("parts").filter(|f| !f.is_null()).cloned(),
        entries_work: st(r, "entries_work"),
        origin_name: st(r, "origin_name").unwrap_or_default(),
        origin_url: st(r, "origin_url").unwrap_or_default(),
    }
}

pub fn user_entries() -> Vec<Entry> {
    user_rows().iter().map(user_entry).collect()
}

fn save_user(rows: &[Value]) -> Result<(), String> {
    write_json(
        &user_file(),
        &pyjson::dumps_indent(&Value::Array(rows.to_vec()), 2),
    )
    .map_err(|e| e.to_string())
}

fn short_id() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let digest = md5::compute(format!("{now}{}", std::process::id()));
    format!("{digest:x}")[..8].to_string()
}

/// A user source (User Resources); credentials are never stored, only the kind (`catalog.add_user_entry`).
#[allow(clippy::too_many_arguments)]
pub fn add_user_entry(
    title: &str,
    format_id: &str,
    location: &str,
    license: Option<&str>,
    auth_kind: &str,
    key_name: Option<&str>,
    key_in: &str,
    book: Option<&str>,
) -> Result<Entry, String> {
    if !has_importer(format_id) {
        let known: Vec<String> = tables().importers.iter().map(|i| i.0.clone()).collect();
        return Err(format!(
            "Unknown format '{format_id}'. Known formats: {}",
            known.join(", ")
        ));
    }
    if title.trim().is_empty() || location.trim().is_empty() {
        return Err("A user source needs a title and a file or URL.".into());
    }
    if !["none", "basic", "bearer", "apikey"].contains(&auth_kind) {
        return Err("Authentication must be none, basic, bearer or apikey.".into());
    }
    let title = title.trim();
    let row = json!({
        "id": format!("user-{}", short_id()), "title": title, "kind": "hadith", "format_id": format_id,
        "location": location.trim(), "description": "Added by you.", "license": license.filter(|l| !l.is_empty()),
        "source_name": title, "source_key": slugify(title), "book": book, "languages": [],
        "auth_kind": auth_kind, "key_name": key_name, "key_in": key_in, "title_ar": "", "tradition": "sunni",
        "repo": "", "mapping": null, "parts": null, "fallback": null, "origin_name": "", "origin_url": "",
        "aliases": [], "hadith": null, "compiler": "", "entries_work": null, "size": null, "translations": null,
        "sha256": null
    });
    let mut rows = user_rows();
    rows.push(row.clone());
    save_user(&rows)?;
    Ok(user_entry(&row))
}

pub fn delete_user_entry(entry_id: &str) -> bool {
    let rows = user_rows();
    let kept: Vec<Value> = rows
        .iter()
        .filter(|r| r.get("id").and_then(Value::as_str) != Some(entry_id))
        .cloned()
        .collect();
    let _ = save_user(&kept);
    kept.len() != rows.len()
}

fn names(e: &Entry, wanted: &str) -> bool {
    e.id == wanted
        || e.aliases.iter().any(|a| a == wanted)
        || e.book.as_deref().unwrap_or("") == wanted
}

/// A source by its id: in the chosen repository first, then in every other; "repo/id" names the repository.
pub fn find(entry_id: &str) -> Option<Entry> {
    let mut entry_id = entry_id.trim();
    if let Some(rest) = entry_id.strip_prefix("fawaz-") {
        entry_id = rest;
    }
    if let Some((rid, eid)) = entry_id.split_once('/') {
        repos::find(rid)?;
        return builtin_entries(Some(rid))
            .ok()?
            .into_iter()
            .find(|e| names(e, eid));
    }
    let chosen = repos::selected();
    let mut order: Vec<Repository> = vec![chosen.clone()];
    order.extend(
        repos::all_repositories()
            .into_iter()
            .filter(|r| r.id != chosen.id),
    );
    for repo in order {
        let Ok(list) = repos::entries(&repo, false) else {
            continue;
        };
        if let Some(found) = list.into_iter().find(|e| {
            names(e, entry_id)
                || (e.kind == "rijal"
                    && entry_id
                        == e.source_key
                            .strip_prefix("openiti-")
                            .unwrap_or(&e.source_key))
        }) {
            return Some(found);
        }
    }
    user_entries().into_iter().find(|e| e.id == entry_id)
}

// ------------------------------------------------------------------ status
#[derive(Debug, Clone, Default)]
pub struct Status {
    pub imported: bool,
    pub languages: Vec<String>,
    pub when: Option<String>,
    pub count: i64,
    pub tier: Option<String>,
}

/// What of this entry is in the database now (`catalog.status`).
pub fn status(conn: &Connection, entry: &Entry) -> rusqlite::Result<Status> {
    if entry.kind == "quran" {
        if !quran::available() {
            return Ok(Status::default());
        }
        let eds: Vec<String> = quran::editions()
            .into_iter()
            .filter(|e| e.key != quran::ARABIC)
            .map(|e| format!("{} — {}", e.language, e.author))
            .collect();
        return Ok(Status {
            imported: true,
            count: eds.len() as i64,
            languages: eds,
            ..Default::default()
        });
    }
    let src: Option<(i64, Option<String>, Option<String>)> = conn
        .query_row(
            "SELECT id, imported_at, tier FROM sources WHERE key = ?",
            [&entry.source_key],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    let Some((sid, when, tier)) = src else {
        return Ok(Status::default());
    };
    if ["sira", "tabaqat", "sahifa"].contains(&entry.kind.as_str()) {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM work_nodes n JOIN works w ON w.id = n.work_id WHERE w.source_id = ? \
             AND n.kind IN ('entry', 'paragraph', 'hadith') AND (n.label IS NULL OR n.parent_id NOT IN \
             (SELECT id FROM work_nodes WHERE label LIKE 'par:%'))",
            [sid],
            |r| r.get(0),
        )?;
        return Ok(Status {
            imported: count > 0,
            languages: vec![],
            when,
            count,
            tier,
        });
    }
    if let Some(book) = &entry.book {
        let mut stmt = conn.prepare(
            "SELECT DISTINCT e.language FROM editions e JOIN collections c ON c.id = e.collection_id
             WHERE e.source_id = ? AND c.key = ? ORDER BY e.language",
        )?;
        let langs: Vec<String> = stmt
            .query_map(params![sid, book], |r| r.get(0))?
            .collect::<Result<_, _>>()?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM hadiths h JOIN collections c ON c.id = h.collection_id WHERE c.key = ?",
            [book],
            |r| r.get(0),
        )?;
        return Ok(Status {
            imported: !langs.is_empty(),
            languages: langs,
            when,
            count,
            tier,
        });
    }
    let count: i64 = if entry.kind == "rijal" {
        conn.query_row(
            "SELECT COUNT(*) FROM persons WHERE source_id = ?",
            [sid],
            |r| r.get(0),
        )?
    } else {
        conn.query_row(
            "SELECT COALESCE(SUM(text_count), 0) FROM editions WHERE source_id = ?",
            [sid],
            |r| r.get(0),
        )?
    };
    Ok(Status {
        imported: true,
        languages: vec![],
        when,
        count,
        tier,
    })
}

// ------------------------------------------------------------------ languages
pub fn language_label(code: &str) -> String {
    tables()
        .languages
        .iter()
        .find(|(c, _)| c == code)
        .map(|(_, n)| n.clone())
        .unwrap_or_else(|| code.to_string())
}

/// A language's code from its name or a code of two or three letters (`catalog.code_for`).
pub fn code_for(name_or_code: &str) -> String {
    let v = name_or_code.trim().to_lowercase();
    if let Some((_, c)) = tables().two_letters.iter().find(|(k, _)| *k == v) {
        return c.clone();
    }
    tables()
        .languages
        .iter()
        .find(|(_, n)| n.to_lowercase() == v)
        .map(|(c, _)| c.clone())
        .unwrap_or(v)
}

/// A language as the list shows it: a code by its name, an edition's label as it is.
pub fn shown_language(l: &str) -> String {
    if l.chars().count() == 3 {
        language_label(l)
    } else {
        l.to_string()
    }
}

#[derive(Debug, Clone)]
pub struct LanguageRow {
    pub code: String,
    pub label: String,
    pub offered: usize,
    pub imported: usize,
}

/// Every language the repository's books offer, with in how many imported books it is imported (LANG2).
pub fn language_overview(conn: &Connection) -> Result<Vec<LanguageRow>, String> {
    let books: Vec<Entry> = builtin_entries(None)
        .map_err(|e| e.0)?
        .into_iter()
        .filter(|e| e.kind == "hadith" && e.book.is_some() && !e.languages.is_empty())
        .collect();
    let mut offered: HashMap<String, usize> = HashMap::new();
    for e in &books {
        for c in &e.languages {
            *offered.entry(c.clone()).or_default() += 1;
        }
    }
    let mut have: HashMap<String, usize> = HashMap::new();
    for e in &books {
        for label in status(conn, e).map_err(|e| e.to_string())?.languages {
            *have.entry(code_for(&label)).or_default() += 1;
        }
    }
    let known: Vec<String> = tables().languages.iter().map(|(c, _)| c.clone()).collect();
    let mut extra: Vec<String> = offered
        .keys()
        .filter(|c| !known.contains(c))
        .cloned()
        .collect();
    extra.sort();
    Ok(known
        .into_iter()
        .chain(extra)
        .filter(|c| offered.contains_key(c))
        .map(|c| LanguageRow {
            label: language_label(&c),
            offered: offered[&c],
            imported: have.get(&c).copied().unwrap_or(0),
            code: c,
        })
        .collect())
}

/// Take these languages out of every book of the repository; Arabic stays (`catalog.remove_languages`).
pub fn remove_languages(conn: &Connection, codes: &[String]) -> Result<usize, String> {
    let codes: Vec<&String> = codes.iter().filter(|c| *c != "ara").collect();
    if codes.is_empty() {
        return Ok(0);
    }
    let labels: Vec<String> = codes.iter().map(|c| language_label(c)).collect();
    let keys: Vec<String> = builtin_entries(None)
        .map_err(|e| e.0)?
        .into_iter()
        .filter(|e| e.kind == "hadith")
        .map(|e| e.source_key)
        .collect();
    if keys.is_empty() {
        return Ok(0);
    }
    let sql = format!(
        "DELETE FROM editions WHERE language IN ({}) AND source_id IN (SELECT id FROM sources WHERE key IN ({}))",
        vec!["?"; labels.len()].join(","),
        vec!["?"; keys.len()].join(",")
    );
    let all: Vec<&String> = labels.iter().chain(keys.iter()).collect();
    let removed = conn
        .execute(&sql, rusqlite::params_from_iter(all))
        .map_err(|e| e.to_string())?;
    store::cleanup(conn).map_err(|e| e.to_string())?;
    Ok(removed)
}

// ------------------------------------------------------------------ what changed in the repository (UPD5)
fn state_file() -> PathBuf {
    paths::data_dir().join("imported.json")
}

fn load_state() -> Map<String, Value> {
    std::fs::read_to_string(state_file())
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}

fn save_state(state: &Map<String, Value>) {
    let _ = write_json(
        &state_file(),
        &pyjson::dumps_indent(&Value::Object(state.clone()), 1),
    );
}

fn key_of(e: &Entry) -> String {
    format!("{}:{}", e.repo, e.id)
}

fn codes(s: &Status) -> Vec<String> {
    s.languages
        .iter()
        .map(|l| code_for(l))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn record(e: &Entry, s: &Status) -> Value {
    json!({"sha256": e.sha256, "languages": codes(s), "offered": e.languages})
}

/// Note these books as forgotten (`catalog.remember(..., forget=True)`).
pub fn forget(entries: &[Entry]) {
    let mut state = load_state();
    if let Some(books) = state
        .entry("books")
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
    {
        for e in entries {
            books.shift_remove(&key_of(e));
        }
    }
    save_state(&state);
}

#[derive(Debug, Clone)]
pub struct Update {
    pub entry: Entry,
    /// "new" | "changed" | "languages"
    pub why: String,
    pub languages: Vec<String>,
    pub new: Vec<String>,
}

/// What the repository has that is not here yet (`catalog.updates`). The first time, what is here is taken as up to
/// date.
pub fn updates(conn: &Connection, repo_id: Option<&str>) -> Result<Vec<Update>, String> {
    let entries: Vec<Entry> = builtin_entries(repo_id)
        .map_err(|e| e.0)?
        .into_iter()
        .filter(|e| e.kind != "quran")
        .collect();
    let mut state = load_state();
    let first = !state.contains_key("seen");
    let seen: BTreeSet<String> = state
        .get("seen")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let mut books = state
        .get("books")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut reads: BTreeSet<String> = BTreeSet::new();
    let mut rows = vec![];
    for e in entries.iter() {
        let s = status(conn, e).map_err(|x| x.to_string())?;
        if s.imported && e.kind == "hadith" {
            reads.extend(codes(&s));
        }
        rows.push((e, s));
    }
    let mut out = vec![];
    let mut changed = first;
    for (e, s) in rows {
        let key = key_of(e);
        if !s.imported {
            if !first && !seen.contains(&key) {
                out.push(Update {
                    entry: e.clone(),
                    why: "new".into(),
                    languages: vec![],
                    new: vec![],
                });
            }
            continue;
        }
        let rec = books.get(&key).cloned();
        let rec_sha = rec
            .as_ref()
            .and_then(|r| r.get("sha256"))
            .and_then(Value::as_str)
            .filter(|x| !x.is_empty())
            .map(str::to_string);
        if rec.is_none()
            || (e.sha256.as_deref().is_some_and(|x| !x.is_empty()) && rec_sha.is_none())
        {
            books.insert(key, record(e, &s));
            changed = true;
            continue;
        }
        if e.sha256.as_deref().is_some_and(|x| !x.is_empty()) && rec_sha != e.sha256 {
            out.push(Update {
                entry: e.clone(),
                why: "changed".into(),
                languages: codes(&s),
                new: vec![],
            });
            continue;
        }
        if e.kind == "hadith" && !e.languages.is_empty() {
            let have = codes(&s);
            let offered: Vec<String> = rec
                .as_ref()
                .and_then(|r| r.get("offered"))
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_else(|| e.languages.clone());
            let extra: Vec<String> = e
                .languages
                .iter()
                .filter(|c| reads.contains(*c) && !have.contains(c) && !offered.contains(c))
                .cloned()
                .collect();
            if !extra.is_empty() {
                let all: BTreeSet<String> = have.iter().chain(extra.iter()).cloned().collect();
                out.push(Update {
                    entry: e.clone(),
                    why: "languages".into(),
                    languages: all.into_iter().collect(),
                    new: extra,
                });
            }
        }
    }
    state.insert("books".into(), Value::Object(books));
    if first {
        let all: BTreeSet<String> = entries.iter().map(key_of).collect();
        state.insert("seen".into(), json!(all.into_iter().collect::<Vec<_>>()));
    }
    if changed {
        save_state(&state);
    }
    Ok(out)
}

// ------------------------------------------------------------------ removing
/// Remove what this entry imported; for one book of a shared source, that book only (`catalog.remove_entry`).
pub fn remove_entry(conn: &Connection, entry: &Entry) -> rusqlite::Result<bool> {
    if entry.builtin && entry.kind != "quran" {
        forget(std::slice::from_ref(entry));
    }
    if entry.kind == "quran" {
        let p = quran::path();
        if !p.exists() {
            return Ok(false);
        }
        let _ = std::fs::remove_file(p);
        return Ok(true);
    }
    let sid: Option<i64> = conn
        .query_row(
            "SELECT id FROM sources WHERE key = ?",
            [&entry.source_key],
            |r| r.get(0),
        )
        .optional()?;
    if ["sira", "tabaqat", "sahifa"].contains(&entry.kind.as_str()) {
        let Some(sid) = sid else {
            return Ok(false);
        };
        let tx = conn.unchecked_transaction()?;
        tx.execute("DELETE FROM works WHERE source_id = ?", [sid])?;
        tx.execute("DELETE FROM sources WHERE id = ?", [sid])?;
        tx.commit()?;
        return Ok(true);
    }
    let Some(sid) = sid else {
        return Ok(false);
    };
    if let Some(book) = &entry.book {
        let tx = conn.unchecked_transaction()?;
        let removed = tx.execute(
            "DELETE FROM editions WHERE source_id = ? AND collection_id IN (SELECT id FROM collections WHERE key = ?)",
            params![sid, book],
        )?;
        tx.execute(
            "DELETE FROM grades WHERE source_id = ? AND hadith_id IN (SELECT h.id FROM hadiths h \
             JOIN collections c ON c.id = h.collection_id WHERE c.key = ?)",
            params![sid, book],
        )?;
        tx.execute(
            "DELETE FROM isnads WHERE source_id = ? AND hadith_id IN (SELECT h.id FROM hadiths h \
             JOIN collections c ON c.id = h.collection_id WHERE c.key = ?)",
            params![sid, book],
        )?;
        let left: Option<i64> = tx
            .query_row("SELECT 1 FROM editions WHERE source_id = ?", [sid], |r| {
                r.get(0)
            })
            .optional()?;
        if left.is_none() {
            tx.execute("DELETE FROM sources WHERE id = ?", [sid])?;
        }
        tx.commit()?;
        store::cleanup(conn)?;
        return Ok(removed > 0);
    }
    store::remove_source(conn, &entry.source_key)
}

/// The order of the repository's list: each kind, each road under it (as isnady-data's folders).
pub fn road(e: &Entry) -> &'static str {
    if e.kind == "quran" {
        ""
    } else if e.tradition == "shia" {
        "shia"
    } else {
        "sunni"
    }
}

pub fn sort_for_list(entries: &mut [Entry]) {
    entries.sort_by_key(|e| {
        let k = KIND_ORDER.iter().position(|x| *x == e.kind).unwrap_or(9);
        let r = ["", "sunni", "shia"]
            .iter()
            .position(|x| *x == road(e))
            .unwrap_or(0);
        (k, r)
    });
}

// ------------------------------------------------------------------ importing (catalog.request_for, import_entry, …)
use super::importers::{ImportError, ImportReport};
use super::imports::{run_import, run_many, Failed, ImportRequest, Imported, Narrators};
use super::Auth;

pub const DEFAULT_LANGUAGES: [&str; 3] = ["ara", "tur", "eng"];

/// The import request of an entry (`catalog.request_for`).
pub fn request_for(
    entry: &Entry,
    languages: Option<&[String]>,
    auth: Option<&Auth>,
    use_fallback: bool,
) -> ImportRequest {
    let mut options = Map::new();
    let (mut format_id, mut location) = (entry.format_id.clone(), entry.location.clone());
    if use_fallback {
        if let Some(fb) = &entry.fallback {
            format_id = fb
                .get("format")
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or(format_id);
            location = fb
                .get("location")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
        }
    }
    let book = entry.book.clone().unwrap_or_else(|| entry.id.clone());
    if format_id == "openiti-work" {
        options.insert("book".into(), json!(book));
        options.insert("kind".into(), json!(entry.kind));
        options.insert("title".into(), json!(entry.title));
        options.insert("title_ar".into(), json!(entry.title_ar));
        options.insert("entries".into(), json!(entry.entries_work));
    } else if format_id == "table" {
        options.insert(
            "mapping".into(),
            entry.mapping.clone().unwrap_or_else(|| json!({})),
        );
        options.insert("book".into(), json!(book));
        options.insert("title".into(), json!(entry.title));
        options.insert("tradition".into(), json!(entry.tradition));
        options.insert("source_key".into(), json!(entry.source_key));
        if let (Some(parts), false) = (&entry.parts, use_fallback) {
            options.insert("parts".into(), parts.clone());
        }
    } else if entry.book.is_some() && entry.builtin && !entry.languages.is_empty() {
        let wanted: Vec<String> = match languages {
            Some(l) if !l.is_empty() => l.to_vec(),
            _ => DEFAULT_LANGUAGES.iter().map(|x| x.to_string()).collect(),
        };
        let mut chosen: Vec<String> = wanted
            .into_iter()
            .filter(|c| entry.languages.contains(c))
            .collect();
        if chosen.is_empty() {
            chosen = vec!["ara".into()];
        }
        options.insert("books".into(), json!([entry.book]));
        options.insert("languages".into(), json!(chosen));
    } else if entry.book.is_some() {
        options.insert("books".into(), json!([entry.book]));
        options.insert("languages".into(), json!(languages.unwrap_or(&[])));
    }
    ImportRequest {
        format_id,
        location,
        name: Some(entry.source_name.clone()),
        license: entry.license.clone(),
        tier: None,
        origin: if entry.builtin { "builtin" } else { "user" }.into(),
        auth: auth.cloned().unwrap_or_default(),
        options,
    }
}

/// The collection's tradition as the catalog gives it: Shia books apart from Sunni ones (`_set_tradition`).
fn set_tradition(conn: &Connection, entry: &Entry) -> rusqlite::Result<()> {
    if entry.kind == "hadith" && entry.builtin {
        if let Some(book) = &entry.book {
            conn.execute(
                "UPDATE collections SET tradition = ? WHERE key = ?",
                params![entry.tradition, book],
            )?;
        }
    }
    Ok(())
}

/// Note these books as imported now, and the repository's list as seen (`catalog.remember`).
pub fn remember(
    conn: &Connection,
    entries: &[Entry],
    seen: Option<&[Entry]>,
) -> Result<(), String> {
    let mut state = load_state();
    let mut books = state
        .get("books")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    for e in entries {
        if e.builtin && e.kind != "quran" {
            let s = status(conn, e).map_err(|x| x.to_string())?;
            books.insert(key_of(e), record(e, &s));
        }
    }
    state.insert("books".into(), Value::Object(books));
    if let Some(seen) = seen {
        let mut all: BTreeSet<String> = state
            .get("seen")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        all.extend(seen.iter().filter(|e| e.builtin).map(key_of));
        state.insert("seen".into(), json!(all.into_iter().collect::<Vec<_>>()));
    }
    save_state(&state);
    Ok(())
}

/// The same entry read from the repository's catalog again, when its location changed there (`_moved`).
fn moved(entry: &Entry) -> Option<Entry> {
    if !entry.builtin || entry.repo.is_empty() {
        return None;
    }
    let repo = repos::find(&entry.repo)?;
    repo.url.as_ref()?;
    let fresh = repos::entries(&repo, true)
        .ok()?
        .into_iter()
        .find(|e| e.id == entry.id)?;
    (fresh.location != entry.location).then_some(fresh)
}

fn lang() -> String {
    crate::i18n::ui_language()
}

/// Import one entry and bring everything derived up to date (`catalog.import_entry`). The Qur'an comes with its
/// importer, next.
pub fn import_entry(
    conn: &Connection,
    entry: &Entry,
    languages: Option<&[String]>,
    auth: Option<&Auth>,
    say: &dyn Fn(&str),
) -> Result<(ImportReport, Narrators), ImportError> {
    if entry.kind == "quran" {
        return Err(ImportError::Resource(super::import_later(&entry.format_id)));
    }
    let mut entry = entry.clone();
    let first = match run_import(conn, &request_for(&entry, languages, auth, false), say) {
        Err(ImportError::Resource(e)) => match moved(&entry) {
            Some(fresh) => {
                say(&crate::i18n::t(
                    &lang(),
                    "The repository has moved this book; reading it from its new place",
                    &[],
                ));
                entry = fresh;
                run_import(conn, &request_for(&entry, languages, auth, false), say)
            }
            None => Err(ImportError::Resource(e)),
        },
        other => other,
    };
    let result = match first {
        Err(ImportError::Resource(exc)) if entry.fallback.is_some() => {
            say(&crate::i18n::t(
                &lang(),
                "The repository's copy could not be read ({exc}); reading the original",
                &[("exc", exc)],
            ));
            run_import(conn, &request_for(&entry, languages, auth, true), say)?
        }
        other => other?,
    };
    set_tradition(conn, &entry)?;
    remember(conn, std::slice::from_ref(&entry), None).map_err(ImportError::Resource)?;
    Ok(result)
}

/// The languages chosen: one list for every book, or one per entry id (`import_all`'s `languages`).
pub enum Languages {
    Every(Option<Vec<String>>),
    Each(HashMap<String, Vec<String>>),
}

impl Languages {
    fn of(&self, id: &str) -> Option<Vec<String>> {
        match self {
            Languages::Every(l) => l.clone(),
            Languages::Each(m) => m.get(id).cloned(),
        }
    }
}

/// Every built-in source: the hadith collections first, then the rest (`catalog.import_all`).
pub fn import_all(
    conn: &Connection,
    languages: &Languages,
    say: &dyn Fn(&str),
) -> Result<(Imported, Failed, Narrators), String> {
    let everything = builtin_entries(None).map_err(|e| e.0)?;
    let mut entries: Vec<Entry> = everything
        .iter()
        .filter(|e| e.kind != "quran")
        .cloned()
        .collect();
    entries.sort_by_key(|e| {
        KIND_ORDER
            .iter()
            .position(|k| *k == e.kind)
            .unwrap_or(KIND_ORDER.len())
    });
    let requests: Vec<(String, ImportRequest)> = entries
        .iter()
        .map(|e| {
            (
                e.title.clone(),
                request_for(e, languages.of(&e.id).as_deref(), None, false),
            )
        })
        .collect();
    let (mut imported, mut failed, mut narrators) = run_many(conn, &requests, say)?;
    // a repository's own copy not reachable: the origin, once more
    let by_title: HashMap<&str, &Entry> = entries.iter().map(|e| (e.title.as_str(), e)).collect();
    let retry: Vec<(String, ImportRequest)> = failed
        .iter()
        .filter_map(|(label, _)| {
            let e = by_title.get(label.as_str())?;
            e.fallback.as_ref()?;
            Some((
                label.clone(),
                request_for(e, languages.of(&e.id).as_deref(), None, true),
            ))
        })
        .collect();
    if !retry.is_empty() {
        let (again, _still, n) = run_many(conn, &retry, say)?;
        narrators = n;
        let done: BTreeSet<String> = again.iter().map(|(l, _)| l.clone()).collect();
        imported.extend(again);
        failed.retain(|f| !done.contains(&f.0));
    }
    for e in &entries {
        set_tradition(conn, e).map_err(|x| x.to_string())?;
    }
    let done: BTreeSet<&str> = imported.iter().map(|(l, _)| l.as_str()).collect();
    let kept: Vec<Entry> = entries
        .iter()
        .filter(|e| done.contains(e.title.as_str()))
        .cloned()
        .collect();
    remember(conn, &kept, Some(&everything))?;
    for e in everything.iter().filter(|e| e.kind == "quran") {
        match import_entry(conn, e, None, None, say) {
            Ok((report, _)) => imported.push((e.title.clone(), report)),
            Err(x) => failed.push((e.title.clone(), x.to_string())),
        }
    }
    Ok((imported, failed, narrators))
}

/// Import what `updates` found, in one go (`catalog.update`).
pub fn update(
    conn: &Connection,
    items: &[Update],
    say: &dyn Fn(&str),
) -> Result<(Imported, Failed, Narrators), String> {
    let requests: Vec<(String, ImportRequest)> = items
        .iter()
        .map(|i| {
            let langs = (!i.languages.is_empty()).then_some(i.languages.as_slice());
            (
                i.entry.title.clone(),
                request_for(&i.entry, langs, None, false),
            )
        })
        .collect();
    if requests.is_empty() {
        return Ok((vec![], vec![], None));
    }
    let (imported, failed, narrators) = run_many(conn, &requests, say)?;
    let done: BTreeSet<&str> = imported.iter().map(|(l, _)| l.as_str()).collect();
    let entries: Vec<Entry> = items.iter().map(|i| i.entry.clone()).collect();
    for e in &entries {
        set_tradition(conn, e).map_err(|x| x.to_string())?;
    }
    let kept: Vec<Entry> = entries
        .iter()
        .filter(|e| done.contains(e.title.as_str()))
        .cloned()
        .collect();
    remember(conn, &kept, Some(&entries))?;
    Ok((imported, failed, narrators))
}

/// Import these languages in every imported book of the repository that offers them (`catalog.add_languages`).
pub fn add_languages(
    conn: &Connection,
    codes: &[String],
    say: &dyn Fn(&str),
) -> Result<(Imported, Failed, Narrators), String> {
    let entries = builtin_entries(None).map_err(|e| e.0)?;
    let mut requests = vec![];
    for e in &entries {
        if e.kind != "hadith" || e.book.is_none() || e.languages.is_empty() {
            continue;
        }
        if !status(conn, e).map_err(|x| x.to_string())?.imported {
            continue;
        }
        let wanted: Vec<String> = codes
            .iter()
            .filter(|c| e.languages.contains(c))
            .cloned()
            .collect();
        if !wanted.is_empty() {
            requests.push((e.title.clone(), request_for(e, Some(&wanted), None, false)));
        }
    }
    if requests.is_empty() {
        return Ok((vec![], vec![], None));
    }
    let result = run_many(conn, &requests, say)?;
    for e in &entries {
        set_tradition(conn, e).map_err(|x| x.to_string())?;
    }
    Ok(result)
}
