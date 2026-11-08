//! Bookmarks (BM1): the Python isnady's `core/bookmarks.py` — bookmarks.json in the profile's folder, in the same
//! form, so a ☆ set by one program is a ★ in the other. What the window needs today: is a thing marked, and mark or
//! unmark it (the ☆ of a card); the Bookmarks page with its folders comes with that page.

use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};
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
    parse(&data)
}

fn parse(data: &Value) -> (Vec<Value>, Vec<Value>) {
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
    write(&path(), folders, items)
}

fn write(file: &std::path::Path, folders: Vec<Value>, items: Vec<Value>) -> std::io::Result<()> {
    let mut out = Map::new();
    out.insert("schema".into(), json!("isnady-bookmarks/1"));
    out.insert("folders".into(), Value::Array(folders));
    out.insert("items".into(), Value::Array(items));
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

/// A folder of the user's own.
#[derive(Debug, Clone, PartialEq)]
pub struct Folder {
    pub id: String,
    pub name: String,
    pub parent: Option<String>,
}

/// One thing kept: its kind, its reference, the title it was marked with, a note, the folders it is in.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Item {
    pub id: String,
    pub kind: String,
    #[serde(rename = "ref")]
    pub reference: String,
    pub title: String,
    pub subtitle: String,
    pub note: String,
    pub folders: Vec<String>,
    pub added: f64,
}

/// bookmarks.json (`bookmarks.Store`).
#[derive(Debug, Clone, Default)]
pub struct Store {
    pub folders: Vec<Folder>,
    pub items: Vec<Item>,
}

fn now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

fn fold(s: &str) -> String {
    s.to_lowercase()
}

impl Store {
    pub fn load() -> Store {
        Store::load_from(&path())
    }

    pub fn load_from(file: &std::path::Path) -> Store {
        let data: Value = std::fs::read_to_string(file)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or(Value::Null);
        Store::from_value(&data)
    }

    pub fn from_value(data: &Value) -> Store {
        let (folders, items) = parse(data);
        Store {
            folders: folders
                .iter()
                .map(|f| Folder {
                    id: f["id"].as_str().unwrap_or("").to_string(),
                    name: f["name"].as_str().unwrap_or("").to_string(),
                    parent: f["parent"].as_str().map(String::from),
                })
                .collect(),
            items: items
                .iter()
                .map(|i| Item {
                    id: i["id"].as_str().unwrap_or("").to_string(),
                    kind: i["kind"].as_str().unwrap_or("").to_string(),
                    reference: i["ref"].as_str().unwrap_or("").to_string(),
                    title: i["title"].as_str().unwrap_or("").to_string(),
                    subtitle: i["subtitle"].as_str().unwrap_or("").to_string(),
                    note: i["note"].as_str().unwrap_or("").to_string(),
                    folders: i["folders"]
                        .as_array()
                        .map(|a| {
                            a.iter()
                                .filter_map(|f| f.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default(),
                    added: i["added"].as_f64().unwrap_or(0.0),
                })
                .collect(),
        }
    }

    pub fn to_values(&self) -> (Vec<Value>, Vec<Value>) {
        (
            self.folders
                .iter()
                .map(|f| json!({"id": f.id, "name": f.name, "parent": f.parent}))
                .collect(),
            self.items
                .iter()
                .map(|i| {
                    json!({"id": i.id, "kind": i.kind, "ref": i.reference, "title": i.title, "subtitle": i.subtitle,
                           "note": i.note, "folders": i.folders, "added": i.added})
                })
                .collect(),
        )
    }

    pub fn save(&self) -> std::io::Result<()> {
        self.save_to(&path())
    }

    pub fn save_to(&self, file: &std::path::Path) -> std::io::Result<()> {
        let (folders, items) = self.to_values();
        write(file, folders, items)
    }

    // ------------------------------------------------------------------ items
    pub fn find(&self, kind: &str, reference: &str) -> Option<&Item> {
        self.items
            .iter()
            .find(|i| i.kind == kind && i.reference == reference)
    }

    pub fn item(&self, id: &str) -> Option<&Item> {
        self.items.iter().find(|i| i.id == id)
    }

    fn item_mut(&mut self, id: &str) -> Option<&mut Item> {
        self.items.iter_mut().find(|i| i.id == id)
    }

    /// Add a thing (kept where it is if marked already), in a folder when one is given; the new one goes first.
    pub fn add(
        &mut self,
        kind: &str,
        reference: &str,
        title: &str,
        subtitle: &str,
        folder: Option<&str>,
        note: &str,
    ) -> Item {
        if self.find(kind, reference).is_none() {
            self.items.insert(
                0,
                Item {
                    id: new_id(),
                    kind: kind.into(),
                    reference: reference.into(),
                    title: title.into(),
                    subtitle: subtitle.into(),
                    note: note.into(),
                    folders: vec![],
                    added: now(),
                },
            );
        }
        let has_folder = folder.is_some_and(|f| self.folder(f).is_some());
        let item = self
            .items
            .iter_mut()
            .find(|i| i.kind == kind && i.reference == reference)
            .expect("item");
        if let (Some(f), true) = (folder, has_folder) {
            if !item.folders.iter().any(|x| x == f) {
                item.folders.push(f.to_string());
            }
        }
        item.clone()
    }

    pub fn remove(&mut self, id: &str) -> bool {
        let before = self.items.len();
        self.items.retain(|i| i.id != id);
        self.items.len() != before
    }

    pub fn set_note(&mut self, id: &str, note: &str) {
        if let Some(i) = self.item_mut(id) {
            i.note = note.trim().to_string();
        }
    }

    /// Put an item in a folder (also in its other folders).
    pub fn file_in(&mut self, id: &str, folder: &str) {
        if self.folder(folder).is_none() {
            return;
        }
        if let Some(i) = self.item_mut(id) {
            if !i.folders.iter().any(|f| f == folder) {
                i.folders.push(folder.to_string());
            }
        }
    }

    pub fn take_out(&mut self, id: &str, folder: &str) {
        if let Some(i) = self.item_mut(id) {
            i.folders.retain(|f| f != folder);
        }
    }

    // ------------------------------------------------------------------ folders
    pub fn folder(&self, id: &str) -> Option<&Folder> {
        self.folders.iter().find(|f| f.id == id)
    }

    /// A folder by its name or its path ("Prayer/Friday"), case not minded.
    pub fn folder_named(&self, name: &str) -> Option<&Folder> {
        let mut parent: Option<String> = None;
        let mut found = None;
        for p in name
            .replace('\\', "/")
            .split('/')
            .map(str::trim)
            .filter(|p| !p.is_empty())
        {
            let f = self
                .folders
                .iter()
                .find(|f| f.parent == parent && fold(&f.name) == fold(p))?;
            parent = Some(f.id.clone());
            found = Some(f);
        }
        found
    }

    pub fn new_folder(&mut self, name: &str, parent: Option<&str>) -> Folder {
        let name = if name.trim().is_empty() {
            "…".to_string()
        } else {
            name.trim().to_string()
        };
        let parent = parent.map(String::from);
        if let Some(same) = self
            .folders
            .iter()
            .find(|f| f.parent == parent && f.name == name)
        {
            return same.clone();
        }
        let parent = parent.filter(|p| self.folder(p).is_some());
        let f = Folder {
            id: new_id(),
            name,
            parent,
        };
        self.folders.push(f.clone());
        f
    }

    /// The folder of a path ("Prayer/Friday"), made where it is missing.
    pub fn ensure_path(&mut self, name: &str) -> Option<Folder> {
        let mut parent: Option<String> = None;
        let mut folder = None;
        for p in name
            .replace('\\', "/")
            .split('/')
            .map(str::trim)
            .filter(|p| !p.is_empty())
        {
            let f = self.new_folder(p, parent.as_deref());
            parent = Some(f.id.clone());
            folder = Some(f);
        }
        folder
    }

    pub fn rename_folder(&mut self, id: &str, name: &str) {
        if name.trim().is_empty() {
            return;
        }
        if let Some(f) = self.folders.iter_mut().find(|f| f.id == id) {
            f.name = name.trim().to_string();
        }
    }

    /// A folder and the folders inside it go; their items stay among the bookmarks.
    pub fn delete_folder(&mut self, id: &str) {
        let mut gone: HashSet<String> = self.descendants(id).into_iter().collect();
        gone.insert(id.to_string());
        self.folders.retain(|f| !gone.contains(&f.id));
        for i in self.items.iter_mut() {
            i.folders.retain(|f| !gone.contains(f));
        }
    }

    pub fn move_folder(&mut self, id: &str, parent: Option<&str>) {
        if parent == Some(id) || parent.is_some_and(|p| self.descendants(id).iter().any(|d| d == p))
        {
            return;
        }
        let parent = parent
            .filter(|p| self.folder(p).is_some())
            .map(String::from);
        if let Some(f) = self.folders.iter_mut().find(|f| f.id == id) {
            f.parent = parent;
        }
    }

    pub fn children(&self, parent: Option<&str>) -> Vec<&Folder> {
        let mut out: Vec<&Folder> = self
            .folders
            .iter()
            .filter(|f| f.parent.as_deref() == parent)
            .collect();
        out.sort_by_key(|f| fold(&f.name));
        out
    }

    pub fn descendants(&self, id: &str) -> Vec<String> {
        let mut out = vec![];
        let mut todo = vec![id.to_string()];
        while let Some(current) = todo.pop() {
            for f in &self.folders {
                if f.parent.as_deref() == Some(current.as_str()) {
                    out.push(f.id.clone());
                    todo.push(f.id.clone());
                }
            }
        }
        out
    }

    pub fn path_of(&self, id: &str) -> String {
        let mut names = vec![];
        let mut current = self.folder(id);
        while let Some(f) = current {
            names.insert(0, f.name.clone());
            current = f.parent.as_deref().and_then(|p| self.folder(p));
        }
        names.join("/")
    }

    /// The items of a folder and of the folders inside it; None: every item.
    pub fn in_folder(&self, id: Option<&str>) -> Vec<&Item> {
        let Some(id) = id else {
            return self.items.iter().collect();
        };
        let mut wanted: HashSet<String> = self.descendants(id).into_iter().collect();
        wanted.insert(id.to_string());
        self.items
            .iter()
            .filter(|i| i.folders.iter().any(|f| wanted.contains(f)))
            .collect()
    }

    pub fn unfiled(&self) -> Vec<&Item> {
        self.items.iter().filter(|i| i.folders.is_empty()).collect()
    }

    /// Add another store's folders (by path) and items (by kind and reference): (items, folders) added.
    pub fn merge(&mut self, other: &Store) -> (usize, usize) {
        let (mut added_items, mut added_folders) = (0, 0);
        let mut mapping: HashMap<String, String> = HashMap::new();
        let mut order: Vec<&Folder> = other.folders.iter().collect();
        order.sort_by_key(|f| other.path_of(&f.id).matches('/').count());
        for f in order {
            let before = self.folders.len();
            if let Some(made) = self.ensure_path(&other.path_of(&f.id)) {
                mapping.insert(f.id.clone(), made.id);
            }
            added_folders += self.folders.len() - before;
        }
        for i in &other.items {
            if self.find(&i.kind, &i.reference).is_none() {
                self.items.push(Item {
                    id: new_id(),
                    folders: vec![],
                    added: if i.added != 0.0 { i.added } else { now() },
                    ..i.clone()
                });
                added_items += 1;
            } else if !i.note.is_empty() {
                let existing = self
                    .items
                    .iter_mut()
                    .find(|x| x.kind == i.kind && x.reference == i.reference)
                    .expect("item");
                if !existing.note.contains(&i.note) {
                    existing.note = format!("{}\n{}", existing.note, i.note).trim().to_string();
                }
            }
            let existing = self
                .items
                .iter_mut()
                .find(|x| x.kind == i.kind && x.reference == i.reference)
                .expect("item");
            for f in &i.folders {
                if let Some(m) = mapping.get(f) {
                    if !existing.folders.contains(m) {
                        existing.folders.push(m.clone());
                    }
                }
            }
        }
        (added_items, added_folders)
    }
}

/// Is this thing marked?/// Is this thing marked?
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

/// The names of each kind (`bookmarks.KINDS`), in English: the caller translates.
pub fn kind_name(kind: &str) -> &'static str {
    match kind {
        "hadith" => "Hadith",
        "narrator" => "Narrator",
        "scholar" => "Scholar",
        "term" => "Term",
        "chapter" => "Chapter",
        "tree" => "Genealogy",
        _ => "",
    }
}

/// The trees a "tree" bookmark points into (`bookmarks.TREES`).
pub fn tree_name(which: &str) -> Option<&'static str> {
    match which {
        "prophets" => Some("The prophets"),
        "quraysh" => Some("Quraysh and the Prophet's family"),
        "family" => Some("A narrator's family"),
        "learning" => Some("A narrator's line of learning"),
        _ => None,
    }
}

/// (reference, title, subtitle) of a hadith: "bukhari:1", "Sahih al-Bukhari 1", the start of its text.
pub fn hadith_ref(
    conn: &rusqlite::Connection,
    hadith_id: i64,
    lang: &str,
) -> Option<(String, String, String)> {
    let (key, name, number): (String, String, String) = conn
        .query_row(
            "SELECT c.key, c.name, h.number FROM hadiths h JOIN collections c ON c.id = h.collection_id WHERE h.id = ?",
            [hadith_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get::<_, Option<String>>(2)?.unwrap_or_default())),
        )
        .ok()?;
    let text: String = conn
        .query_row(
            "SELECT t.text FROM texts t JOIN editions e ON e.id = t.edition_id WHERE t.hadith_id = ? \
             ORDER BY CASE e.language WHEN 'Arabic' THEN 1 ELSE 0 END LIMIT 1",
            [hadith_id],
            |r| r.get::<_, Option<String>>(0),
        )
        .ok()
        .flatten()
        .unwrap_or_default();
    let snippet: String = text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(140)
        .collect();
    Some((
        format!("{key}:{number}"),
        format!("{} {number}", crate::works::book_title(&key, &name, lang)),
        snippet,
    ))
}

pub fn resolve_hadith(conn: &rusqlite::Connection, reference: &str) -> Option<i64> {
    let (key, number) = reference.split_once(':').unwrap_or((reference, ""));
    conn.query_row(
        "SELECT h.id FROM hadiths h JOIN collections c ON c.id = h.collection_id WHERE c.key = ? AND h.number = ?",
        [key, number],
        |r| r.get(0),
    )
    .ok()
}

/// (his key, the name he is known by, his Arabic) of a narrator (`bookmarks.narrator_ref`).
pub fn narrator_ref(
    conn: &rusqlite::Connection,
    person_id: i64,
    langs: crate::narrators::Langs,
) -> Option<(String, String, String)> {
    let (key, name): (String, String) = conn
        .query_row(
            "SELECT key, name_ar FROM persons WHERE id = ?",
            [person_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .ok()?;
    let (reading, arabic) = crate::lives::reading(conn, person_id, &name, langs);
    Some((
        key,
        if reading.is_empty() {
            arabic.clone()
        } else {
            reading
        },
        arabic,
    ))
}

pub fn resolve_narrator(conn: &rusqlite::Connection, reference: &str) -> Option<i64> {
    conn.query_row("SELECT id FROM persons WHERE key = ?", [reference], |r| {
        r.get(0)
    })
    .ok()
}

pub fn resolve_chapter(conn: &rusqlite::Connection, reference: &str) -> Option<i64> {
    let (key, label) = reference.split_once(':').unwrap_or((reference, ""));
    conn.query_row(
        "SELECT n.id FROM work_nodes n JOIN works w ON w.id = n.work_id \
         WHERE w.key = ? AND n.label = ? AND n.kind IN ('volume', 'kitab', 'bab', 'section')",
        [key, label],
        |r| r.get(0),
    )
    .ok()
}

/// (reference, title, subtitle) of a place in a genealogy: "prophets:musa", or "family:<narrator's key>" /
/// "learning:<narrator's key>" (`bookmarks.tree_ref`).
pub fn tree_ref(
    conn: Option<&rusqlite::Connection>,
    which: &str,
    node_id: Option<&str>,
    person_id: Option<i64>,
    langs: crate::narrators::Langs,
) -> Option<(String, String, String)> {
    if crate::genealogy::is_written(which) {
        let tree = crate::genealogy::written(which, langs.words)?;
        let node = tree.nodes.get(node_id.unwrap_or(""))?;
        return Some((
            format!("{which}:{}", node.id),
            node.name(langs.words),
            node.names.get("ar").cloned().unwrap_or_default(),
        ));
    }
    if which == "family" || which == "learning" {
        let (key, title, arabic) = narrator_ref(conn?, person_id?, langs)?;
        return Some((format!("{which}:{key}"), title, arabic));
    }
    None
}

/// Where a "tree" bookmark points: (which tree, the box's id) or (which tree, the narrator's id as text).
pub fn resolve_tree(
    conn: Option<&rusqlite::Connection>,
    reference: &str,
) -> Option<(String, String)> {
    let (which, rest) = reference.split_once(':').unwrap_or((reference, ""));
    if crate::genealogy::is_written(which) {
        let tree = crate::genealogy::written(which, "en")?;
        return tree
            .nodes
            .contains_key(rest)
            .then(|| (which.to_string(), rest.to_string()));
    }
    if which == "family" || which == "learning" {
        let pid = resolve_narrator(conn?, rest)?;
        return Some((which.to_string(), pid.to_string()));
    }
    None
}

/// One line on what a bookmark is, beyond its title (`bookmarks.detail`): a narrator's road, generation, death and
/// first verdict; which tree a genealogy bookmark is in. In the language given.
pub fn detail(conn: Option<&rusqlite::Connection>, item: &Item, lang: &str) -> String {
    let t = |s: &str, v: &[(&str, String)]| crate::i18n::t(lang, s, v);
    if item.kind == "tree" {
        let which = item.reference.split(':').next().unwrap_or("");
        return tree_name(which).map(|n| t(n, &[])).unwrap_or_default();
    }
    if item.kind == "narrator" {
        let Some(conn) = conn else {
            return String::new();
        };
        type Row = (i64, Option<String>, Option<i64>, Option<i64>);
        let row: Option<Row> = conn
            .query_row(
                "SELECT id, tradition, tabaqa, death_year_ah FROM persons WHERE key = ?",
                [&item.reference],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .ok();
        let Some((pid, tradition, tabaqa, death)) = row else {
            return String::new();
        };
        let mut parts = vec![if tradition.as_deref() == Some("shia") {
            t("Shia narrator", &[])
        } else {
            t("Sunni narrator", &[])
        }];
        if let Some(tb) = tabaqa {
            parts.push(t("tabaqa {n}", &[("n", tb.to_string())]));
        }
        if let Some(d) = death.filter(|d| *d != 0) {
            parts.push(t("died {year} AH", &[("year", d.to_string())]));
        }
        let phrase: Option<String> = conn
            .query_row(
                "SELECT phrase FROM verdicts WHERE person_id = ? ORDER BY id LIMIT 1",
                [pid],
                |r| r.get(0),
            )
            .ok()
            .flatten();
        if let Some(p) = phrase.filter(|p| !p.is_empty()) {
            parts.push(p);
        }
        return parts.join(" · ");
    }
    String::new()
}
