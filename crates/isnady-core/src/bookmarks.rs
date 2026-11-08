//! Bookmarks (BM1): the Python isnady's `core/bookmarks.py` — bookmarks.json in the profile's folder, in the same
//! form, so a ☆ set by one program is a ★ in the other. What the window needs today: is a thing marked, and mark or
//! unmark it (the ☆ of a card); the Bookmarks page with its folders comes with that page.

use serde_json::{json, Map, Value};
use std::path::PathBuf;

use crate::config;

/// The kinds a bookmark can be (`bookmarks.KINDS`).
pub const KINDS: [&str; 6] = ["hadith", "narrator", "scholar", "term", "chapter", "tree"];

pub fn path() -> PathBuf {
    config::profile_folder().join("bookmarks.json")
}

/// bookmarks.json as `Store.from_dict` reads it: folders with an id, items of a known kind with a ref, an item's
/// folders only those that exist.
fn load() -> (Vec<Value>, Vec<Value>) {
    let data: Value = std::fs::read_to_string(path())
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or(Value::Null);
    let folders: Vec<Value> = data
        .get("folders")
        .and_then(|f| f.as_array())
        .map(|a| {
            a.iter()
                .filter(|f| {
                    f.get("id")
                        .and_then(|i| i.as_str())
                        .is_some_and(|i| !i.is_empty())
                })
                .map(|f| {
                    json!({"id": f["id"], "name": f.get("name").cloned().unwrap_or(Value::Null),
                           "parent": f.get("parent").cloned().unwrap_or(Value::Null)})
                })
                .collect()
        })
        .unwrap_or_default();
    let known: Vec<&str> = folders.iter().filter_map(|f| f["id"].as_str()).collect();
    let items: Vec<Value> = data
        .get("items")
        .and_then(|i| i.as_array())
        .map(|a| {
            a.iter()
                .filter(|d| {
                    d.get("kind")
                        .and_then(|k| k.as_str())
                        .is_some_and(|k| KINDS.contains(&k))
                        && d.get("ref")
                            .is_some_and(|r| !r.is_null() && r != "" && r != false)
                })
                .map(|d| {
                    let reference = match &d["ref"] {
                        Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    let title = d
                        .get("title")
                        .and_then(|t| t.as_str())
                        .filter(|t| !t.is_empty())
                        .map(String::from)
                        .unwrap_or_else(|| reference.clone());
                    let id = d
                        .get("id")
                        .and_then(|i| i.as_str())
                        .filter(|i| !i.is_empty())
                        .map(String::from)
                        .unwrap_or_else(new_id);
                    let in_folders: Vec<Value> = d
                        .get("folders")
                        .and_then(|f| f.as_array())
                        .map(|a| {
                            a.iter()
                                .filter(|f| f.as_str().is_some_and(|f| known.contains(&f)))
                                .cloned()
                                .collect()
                        })
                        .unwrap_or_default();
                    let added = d.get("added").and_then(|a| a.as_f64()).unwrap_or(0.0);
                    json!({"id": id, "kind": d["kind"], "ref": reference, "title": title,
                           "subtitle": d.get("subtitle").cloned().unwrap_or(json!("")),
                           "note": d.get("note").cloned().unwrap_or(json!("")),
                           "folders": in_folders, "added": added})
                })
                .collect()
        })
        .unwrap_or_default();
    (folders, items)
}

fn save(folders: Vec<Value>, items: Vec<Value>) -> std::io::Result<()> {
    let mut out = Map::new();
    out.insert("schema".into(), json!("isnady-bookmarks/1"));
    out.insert("folders".into(), Value::Array(folders));
    out.insert("items".into(), Value::Array(items));
    let file = path();
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = file.with_extension("tmp");
    std::fs::write(&tmp, config::to_python_json(&Value::Object(out)))?;
    std::fs::rename(tmp, file)
}

/// A new item's id: twelve hex digits, as `uuid.uuid4().hex[:12]`.
fn new_id() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    let mut h = RandomState::new().build_hasher();
    h.write_u128(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0),
    );
    format!("{:012x}", h.finish() & 0xffff_ffff_ffff)
}

/// Is this thing marked?
pub fn is_marked(kind: &str, reference: &str) -> bool {
    load()
        .1
        .iter()
        .any(|i| i["kind"] == kind && i["ref"] == reference)
}

/// Mark or unmark one thing (the ☆'s click, `bookmarks_page.toggle`); true when it is marked now. A new item goes
/// first, as `Store.add` puts it.
pub fn toggle(kind: &str, reference: &str, title: &str, subtitle: &str) -> std::io::Result<bool> {
    let (folders, mut items) = load();
    let before = items.len();
    items.retain(|i| !(i["kind"] == kind && i["ref"] == reference));
    let marked = items.len() == before;
    if marked {
        let added = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);
        items.insert(
            0,
            json!({"id": new_id(), "kind": kind, "ref": reference, "title": title, "subtitle": subtitle,
                   "note": "", "folders": [], "added": added}),
        );
    }
    save(folders, items)?;
    Ok(marked)
}

/// (reference, title, subtitle) of a chapter (`bookmarks.chapter_ref`): "bukhari:1", "Sahih al-Bukhari › 1. Revelation".
pub fn chapter_ref(
    conn: &rusqlite::Connection,
    node_id: i64,
    lang: &str,
) -> Option<(String, String, String)> {
    let node = crate::works::node(conn, node_id, lang)?;
    let (key, title): (String, Option<String>) = conn
        .query_row(
            "SELECT key, title FROM works WHERE id = ?",
            [node.work_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .ok()?;
    let book = crate::works::book_title(&key, title.as_deref().unwrap_or(""), lang);
    let label = node
        .label
        .as_deref()
        .filter(|l| !l.is_empty())
        .map(|l| format!("{l}. "))
        .unwrap_or_default();
    let reference = format!(
        "{key}:{}",
        node.label
            .clone()
            .filter(|l| !l.is_empty())
            .unwrap_or_else(|| node_id.to_string())
    );
    Some((
        reference,
        format!("{book} › {label}{}", node.title),
        String::new(),
    ))
}
