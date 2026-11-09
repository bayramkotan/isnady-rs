//! Works: every book isnady can open and read, as a tree of chapters and items — the Python isnady's
//! `isnady/core/works.py`, with the book and chapter titles of `core/titles.py` and the tables of contents of
//! `core/chapters.py` (read from `data/books.json`, written from the Python isnady by `tools/gen_tables.py`).
//!
//! The Python isnady builds the tree of a hadith collection when it is imported (or changed); isnady-rs reads it.
//! `ready` says whether every tree is up to date.

use rusqlite::{params, params_from_iter, Connection, OptionalExtension};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::{books, i18n};

pub const CONTAINERS: [&str; 4] = ["volume", "kitab", "bab", "section"];
pub const LEAVES: [&str; 4] = ["hadith", "entry", "reference", "paragraph"];

#[derive(Deserialize)]
pub struct CatalogEntry {
    pub id: String,
    pub title: String,
    pub title_ar: String,
}

#[derive(Deserialize)]
pub struct Toc {
    pub version: i64,
    pub parts: Vec<HashMap<String, String>>,
    /// [first hadith number, part, Arabic title, (Turkish title, English title)]
    pub chapters: Vec<Vec<Value>>,
}

#[derive(Deserialize, Default)]
pub struct Titles {
    pub chapters: HashMap<String, String>,
    pub books: HashMap<String, String>,
}

#[derive(Deserialize)]
pub struct Library {
    pub catalog: HashMap<String, CatalogEntry>,
    /// the short names of the library's works, in the Python table's order: [(short name, key)]
    pub aliases: Vec<(String, String)>,
    pub kind_labels: HashMap<String, String>,
    pub sahifa_roads: HashMap<String, String>,
    pub chapters: HashMap<String, Toc>,
    pub titles: HashMap<String, Titles>,
    pub edition_language: HashMap<String, String>,
}

pub fn library() -> &'static Library {
    static LIB: OnceLock<Library> = OnceLock::new();
    LIB.get_or_init(|| {
        serde_json::from_str(include_str!("../data/books.json")).expect("data/books.json")
    })
}

// ------------------------------------------------------------------ titles (core/titles.py, core/chapters.py)

fn clean(title: &str) -> String {
    title
        .replace('ـ', "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// A book's title in the language ("Sahîh-i Buhârî", "صحيح البخاري", "Sahih al-Bukhari").
pub fn book_title(key: &str, fallback: &str, lang: &str) -> String {
    let lib = library();
    if lang != "en" {
        if let Some(own) = lib.titles.get(lang).and_then(|t| t.books.get(key)) {
            return own.clone();
        }
        if lang == "ar" {
            if let Some(e) = lib.catalog.get(key).filter(|e| !e.title_ar.is_empty()) {
                return e.title_ar.clone();
            }
        }
    }
    if let Some(e) = lib.catalog.get(key).filter(|e| !e.title.is_empty()) {
        return e.title.split(" — ").next().unwrap_or("").to_string();
    }
    if fallback.is_empty() {
        key.to_string()
    } else {
        fallback.to_string()
    }
}

/// The name a book is written as in files and folders: its full title with its compiler (`books.full_name`).
pub fn full_name(key: &str) -> String {
    library()
        .catalog
        .get(key)
        .map(|e| e.id.clone())
        .unwrap_or_else(|| key.to_string())
}

fn fold_title(name: &str) -> String {
    name.to_lowercase()
        .replace("al-", "al")
        .replace(" at ", " al")
        .replace(" an ", " al")
        .chars()
        .filter(|c| c.is_ascii_lowercase())
        .collect()
}

/// A title given by its English name (as the statistics keep them) in the language (`titles.book_named`).
pub fn book_named(conn: Option<&Connection>, name: &str, lang: &str) -> String {
    let lib = library();
    let mut key: Option<String> = None;
    for (k, e) in &lib.catalog {
        let short = e.title.split(" — ").next().unwrap_or("");
        for n in [e.title.as_str(), short] {
            if !n.is_empty() && n == name {
                key = Some(k.clone());
            }
        }
    }
    if key.is_none() {
        let folded = fold_title(name);
        for (k, e) in &lib.catalog {
            let short = e.title.split(" — ").next().unwrap_or("");
            if [e.title.as_str(), short]
                .iter()
                .any(|n| !n.is_empty() && fold_title(n) == folded)
            {
                key = Some(k.clone());
            }
        }
    }
    if key.is_none() {
        if let Some(c) = conn {
            key = c
                .query_row("SELECT key FROM collections WHERE name = ?", [name], |r| {
                    r.get(0)
                })
                .ok();
        }
    }
    match key {
        Some(k) => book_title(&k, name, lang),
        None => name.to_string(),
    }
}

/// The database key of a book given by its full name, its key, or the id of an earlier version (`books.resolve`).
pub fn resolve(name: &str) -> String {
    let name = name.trim();
    let name = name.strip_prefix("fawaz-").unwrap_or(name);
    library()
        .catalog
        .iter()
        .find(|(_, e)| e.id == name)
        .map(|(k, _)| k.clone())
        .unwrap_or_else(|| name.to_string())
}

/// A chapter title in the language: the edition's own, else isnady's table, else as it is (`titles.chapter`).
fn chapter_in(title: &str, lang: &str, own: Option<&str>) -> String {
    if lang == "en" || title.is_empty() {
        return title.to_string();
    }
    if let Some(o) = own {
        let o = clean(o);
        if !o.is_empty() && o != clean(title) {
            return o;
        }
    }
    library()
        .titles
        .get(lang)
        .and_then(|t| t.chapters.get(&clean(title)))
        .cloned()
        .unwrap_or_else(|| title.to_string())
}

/// {chapter number: title} of one collection in the language (`titles.edition_titles`).
fn edition_titles(conn: &Connection, collection_id: i64, lang: &str) -> HashMap<String, String> {
    let wanted = library().edition_language.get(lang).cloned();
    let mut english: HashMap<i64, String> = HashMap::new();
    let mut first: Vec<(i64, String)> = Vec::new();
    let mut own: HashMap<i64, String> = HashMap::new();
    if let Ok(mut st) = conn.prepare(
        "SELECT s.number, t.title, e.language FROM sections s JOIN section_titles t ON t.section_id = s.id
               JOIN editions e ON e.id = t.edition_id WHERE s.collection_id = ?",
    ) {
        if let Ok(rows) = st.query_map(params![collection_id], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, Option<String>>(2)?,
            ))
        }) {
            for (number, title, language) in rows.flatten() {
                let Some(title) = title.filter(|t| !t.trim().is_empty()) else {
                    continue;
                };
                let title = title.trim().to_string();
                if language.as_deref() == Some("English") {
                    english.insert(number, title.clone());
                }
                if !first.iter().any(|(n, _)| *n == number) {
                    first.push((number, title.clone()));
                }
                if wanted.is_some() && language == wanted {
                    own.insert(number, title);
                }
            }
        }
    }
    first
        .into_iter()
        .map(|(n, f)| {
            let base = english.get(&n).cloned().unwrap_or(f);
            (
                n.to_string(),
                chapter_in(&base, lang, own.get(&n).map(String::as_str)),
            )
        })
        .collect()
}

/// A part of a table isnady keeps (Musnad Ahmad, al-Darimi) in the language, or None.
fn part_title(title: &str, lang: &str) -> Option<String> {
    for toc in library().chapters.values() {
        for p in &toc.parts {
            if p.get("en").map(String::as_str) == Some(title) {
                return Some(
                    p.get(lang)
                        .or_else(|| p.get("en"))
                        .cloned()
                        .unwrap_or_default(),
                );
            }
        }
    }
    None
}

/// A chapter's title from a table isnady keeps, in the language.
fn toc_chapter_title(key: &str, number: usize, lang: &str) -> Option<String> {
    let toc = library().chapters.get(key)?;
    if !(1..=toc.chapters.len()).contains(&number) {
        return None;
    }
    let row = &toc.chapters[number - 1];
    let at = |i: usize| {
        row.get(i)
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(String::from)
    };
    if lang == "tr" {
        if let Some(t) = at(3) {
            return Some(t);
        }
    }
    if !["tr", "ar", "fa", "ur", "prs", "ps", "ug"].contains(&lang) {
        if let Some(t) = at(4) {
            return Some(t);
        }
    }
    at(2)
}

/// "toc{version}:{chapters}" of a table isnady keeps, "" without one (`chapters.stamp`).
fn toc_stamp(key: &str) -> String {
    library()
        .chapters
        .get(key)
        .map(|t| format!("toc{}:{}", t.version, t.chapters.len()))
        .unwrap_or_default()
}

// ------------------------------------------------------------------ the shelf

/// What a hadith collection's tree is built from: its hadith, its chapters, its name and the table isnady keeps
/// for it; a change builds it again (`ensure_collection_works`' stamp).
fn collection_stamp(
    conn: &Connection,
    cid: i64,
    key: &str,
    name: &str,
) -> rusqlite::Result<String> {
    let (n, top): (i64, i64) = conn.query_row(
        "SELECT COUNT(*), COALESCE(MAX(id), 0) FROM hadiths WHERE collection_id = ?",
        params![cid],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let chapters: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sections WHERE collection_id = ?",
        params![cid],
        |r| r.get(0),
    )?;
    let toc = toc_stamp(key);
    Ok(format!(
        "collection|{n}|{top}|{chapters}|{name}{}",
        if toc.is_empty() {
            String::new()
        } else {
            format!("|{toc}")
        }
    ))
}

fn collections(conn: &Connection) -> rusqlite::Result<Vec<(i64, String, String)>> {
    let mut st = conn.prepare("SELECT id, key, name FROM collections")?;
    let rows = st
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

/// Every hadith collection's tree is the one `ensure_collection_works` builds for it now.
pub fn ready(conn: &Connection) -> bool {
    let run = || -> rusqlite::Result<bool> {
        for (cid, key, name) in collections(conn)? {
            let stamp = collection_stamp(conn, cid, &key, &name)?;
            let have: Option<Option<String>> = conn
                .query_row("SELECT stamp FROM works WHERE key = ?", params![key], |r| {
                    r.get(0)
                })
                .optional()?;
            if have.flatten().as_deref() != Some(stamp.as_str()) {
                return Ok(false);
            }
        }
        Ok(true)
    };
    run().unwrap_or(false)
}

/// The scholar who compiled a book (`works._author_of`).
fn author_of(key: &str) -> Option<String> {
    crate::scholars::all()
        .iter()
        .find(|s| s.compiler_of.iter().any(|b| b == key))
        .map(|s| s.id.clone())
}

/// The book's hadith (id, number) laid out by the table isnady keeps for it (`chapters.plan`):
/// [(part, [(chapter number, Arabic title, [(hadith id, number)])])]; None without a table.
type Plan = Vec<(usize, Vec<(usize, String, Vec<(i64, String)>)>)>;

fn plan(key: &str, hadith: &[(i64, String)]) -> Option<Plan> {
    let toc = library().chapters.get(key)?;
    if hadith.is_empty() {
        return None;
    }
    let mut index: HashMap<&str, usize> = HashMap::new();
    for (i, (_, number)) in hadith.iter().enumerate() {
        index.insert(number.as_str(), i); // the last of equal numbers, as a dict comprehension keeps it
    }
    let mut starts: Vec<(usize, usize, usize, String)> = vec![];
    for (n, row) in toc.chapters.iter().enumerate() {
        let first = match row.first() {
            Some(Value::String(s)) => s.clone(),
            Some(v) => v.to_string(),
            None => continue,
        };
        let part = row.get(1).and_then(Value::as_u64).unwrap_or(0) as usize;
        let title = row.get(2).and_then(Value::as_str).unwrap_or("").to_string();
        if let Some(&i) = index.get(first.as_str()) {
            starts.push((i, n + 1, part, title));
        }
    }
    if starts.is_empty() {
        return None;
    }
    starts.sort();
    starts[0].0 = 0; // a hadith before the first heading goes to the first chapter
    let mut out: Plan = vec![];
    for k in 0..starts.len() {
        let (begin, n, part, ref title) = starts[k];
        let end = if k + 1 < starts.len() {
            starts[k + 1].0
        } else {
            hadith.len()
        };
        if begin >= end {
            continue;
        }
        let rows: Vec<(i64, String)> = hadith[begin..end].to_vec();
        if out.last().map(|(p, _)| *p) != Some(part) {
            out.push((part, vec![]));
        }
        out.last_mut().unwrap().1.push((n, title.clone(), rows));
    }
    Some(out)
}

/// The tree of a book whose data carry no chapters, from the table isnady keeps: part → chapter → hadith
/// (`works._build_from_table`). False when isnady keeps no table for it.
fn build_from_table(conn: &Connection, wid: i64, key: &str, cid: i64) -> rusqlite::Result<bool> {
    let mut st = conn
        .prepare("SELECT id, number FROM hadiths WHERE collection_id = ? ORDER BY number_sort")?;
    let hadith: Vec<(i64, String)> = st
        .query_map(params![cid], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    let Some(layout) = plan(key, &hadith) else {
        return Ok(false);
    };
    let toc = &library().chapters[key];
    let mut leaf = conn.prepare(
        "INSERT INTO work_nodes (work_id, parent_id, ordinal, kind, label, hadith_id) VALUES (?, ?, ?, 'hadith', ?, ?)",
    )?;
    let mut rows = vec![];
    for (p_ordinal, (part, chapters)) in layout.iter().enumerate() {
        let en = toc
            .parts
            .get(*part)
            .and_then(|p| p.get("en"))
            .cloned()
            .unwrap_or_default();
        conn.execute(
            "INSERT INTO work_nodes (work_id, parent_id, ordinal, kind, title) VALUES (?, NULL, ?, 'kitab', ?)",
            params![wid, p_ordinal as i64, en],
        )?;
        let part_id = conn.last_insert_rowid();
        for (c_ordinal, (number, title, items)) in chapters.iter().enumerate() {
            conn.execute(
                "INSERT INTO work_nodes (work_id, parent_id, ordinal, kind, label, title) VALUES (?, ?, ?, 'bab', ?, ?)",
                params![wid, part_id, c_ordinal as i64, number.to_string(), title],
            )?;
            let chapter_id = conn.last_insert_rowid();
            for (i, (hid, num)) in items.iter().enumerate() {
                rows.push((chapter_id, i as i64 + 1, num.clone(), *hid));
            }
        }
    }
    for (parent, ordinal, num, hid) in rows {
        leaf.execute(params![wid, parent, ordinal, num, hid])?;
    }
    Ok(true)
}

/// Build (or build again, when the hadith or chapters changed) the tree of every imported hadith collection:
/// kitab → hadith, from `sections` and `hadiths.section_number` (`works.ensure_collection_works`). How many were
/// built.
pub fn ensure_collection_works(conn: &Connection) -> rusqlite::Result<usize> {
    let mut rebuilt = 0;
    for (cid, key, name) in collections(conn)? {
        let stamp = collection_stamp(conn, cid, &key, &name)?;
        let row: Option<(i64, Option<String>)> = conn
            .query_row(
                "SELECT id, stamp FROM works WHERE key = ?",
                params![key],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        if let Some((_, Some(have))) = &row {
            if *have == stamp {
                continue;
            }
        }
        let tx = conn.unchecked_transaction()?;
        if let Some((id, _)) = row {
            tx.execute("DELETE FROM works WHERE id = ?", params![id])?;
        }
        tx.execute(
            "INSERT INTO works (key, kind, title, author, collection_id, stamp) VALUES (?, 'hadith', ?, ?, ?, ?)",
            params![key, name, author_of(&key), cid, stamp],
        )?;
        let wid = tx.last_insert_rowid();
        let mut titles: HashMap<i64, String> = HashMap::new();
        {
            let mut st = tx.prepare(
                "SELECT s.number, t.title FROM sections s LEFT JOIN section_titles t ON t.section_id = s.id
                 WHERE s.collection_id = ? ORDER BY s.number, t.edition_id",
            )?;
            let rows = st.query_map(params![cid], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, Option<String>>(1)?))
            })?;
            for r in rows {
                let (number, title) = r?;
                if let Some(t) = title {
                    let t = crate::py::trim(&t).to_string();
                    if !t.is_empty() && !titles.contains_key(&number) {
                        titles.insert(number, t);
                    }
                }
            }
        }
        if titles.is_empty() && build_from_table(&tx, wid, &key, cid)? {
            tx.commit()?;
            rebuilt += 1;
            continue;
        }
        // chapter 0 without a title is how the source marks "no chapter": not a first chapter, but the last
        let mut numbers: Vec<i64> = {
            let mut st =
                tx.prepare("SELECT DISTINCT section_number FROM hadiths WHERE collection_id = ?")?;
            let v: Vec<Option<i64>> = st
                .query_map(params![cid], |r| r.get(0))?
                .collect::<Result<_, _>>()?;
            v.into_iter()
                .flatten()
                .filter(|n| !(*n == 0 && !titles.contains_key(&0)))
                .collect()
        };
        numbers.sort_unstable();
        numbers.dedup();
        let mut node_of: HashMap<i64, i64> = HashMap::new();
        for (ordinal, number) in numbers.iter().enumerate() {
            let title = titles
                .get(number)
                .cloned()
                .unwrap_or_else(|| format!("Chapter {number}"));
            tx.execute(
                "INSERT INTO work_nodes (work_id, parent_id, ordinal, kind, label, title) VALUES (?, NULL, ?, 'kitab', ?, ?)",
                params![wid, ordinal as i64, number.to_string(), title],
            )?;
            node_of.insert(*number, tx.last_insert_rowid());
        }
        let hadith: Vec<(i64, String, Option<i64>)> = {
            let mut st = tx.prepare(
                "SELECT id, number, section_number FROM hadiths WHERE collection_id = ? ORDER BY number_sort",
            )?;
            let v = st
                .query_map(params![cid], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
                .collect::<Result<_, _>>()?;
            v
        };
        let mut other: Option<i64> = None;
        let mut counters: HashMap<i64, i64> = HashMap::new();
        let mut rows = vec![];
        for (hid, num, section) in hadith {
            let parent = match section.and_then(|s| node_of.get(&s).copied()) {
                Some(p) => p,
                None => match other {
                    Some(o) => o,
                    None => {
                        tx.execute(
                            "INSERT INTO work_nodes (work_id, parent_id, ordinal, kind, title) VALUES \
                             (?, NULL, ?, 'kitab', 'Without a chapter in the source')",
                            params![wid, numbers.len() as i64],
                        )?;
                        let o = tx.last_insert_rowid();
                        other = Some(o);
                        o
                    }
                },
            };
            let c = counters.entry(parent).or_insert(0);
            *c += 1;
            rows.push((parent, *c, num, hid));
        }
        {
            let mut leaf = tx.prepare(
                "INSERT INTO work_nodes (work_id, parent_id, ordinal, kind, label, hadith_id) \
                 VALUES (?, ?, ?, 'hadith', ?, ?)",
            )?;
            for (parent, ordinal, num, hid) in rows {
                leaf.execute(params![wid, parent, ordinal, num, hid])?;
            }
        }
        tx.commit()?;
        rebuilt += 1;
    }
    if rebuilt > 0 {
        if let Ok(mut c) = title_cache().lock() {
            c.clear();
        }
    }
    Ok(rebuilt)
}

#[derive(Debug, Clone)]
pub struct Work {
    pub id: i64,
    pub key: String,
    pub kind: String,
    pub title: String,
    pub title_ar: Option<String>,
    pub author: Option<String>,
    pub leaves: i64,
    pub chapters: i64,
    pub tradition: String,
}

/// Every book on the shelf, each with its tradition: the Sunni ones first (hadith collections in reading order,
/// then the others), the Shia ones after them in the same way (`works.list_works`).
pub fn list_works(conn: &Connection, lang: &str) -> rusqlite::Result<Vec<Work>> {
    let mut st = conn.prepare(
        "SELECT w.id, w.key, w.kind, w.title, w.title_ar, w.author,
                  (SELECT COUNT(*) FROM work_nodes n WHERE n.work_id = w.id AND n.kind IN ('hadith', 'entry', 'paragraph')
                     AND NOT EXISTS (SELECT 1 FROM work_nodes p WHERE p.id = n.parent_id AND p.label LIKE 'par:%')),
                  (SELECT COUNT(*) FROM work_nodes n WHERE n.work_id = w.id AND n.parent_id IS NULL),
                  COALESCE((SELECT c.tradition FROM collections c WHERE c.id = w.collection_id),
                           (SELECT p.tradition FROM persons p WHERE p.source_id = w.source_id LIMIT 1), 'sunni')
           FROM works w",
    )?;
    let rows: Vec<Work> = st
        .query_map([], |r| {
            Ok(Work {
                id: r.get(0)?,
                key: r.get(1)?,
                kind: r.get(2)?,
                title: r.get::<_, Option<String>>(3)?.unwrap_or_default(),
                title_ar: r.get(4)?,
                author: r.get(5)?,
                leaves: r.get(6)?,
                chapters: r.get(7)?,
                tradition: r.get::<_, Option<String>>(8)?.unwrap_or_default(),
            })
        })?
        .collect::<Result<_, _>>()?;
    let order: HashMap<String, usize> = books::all_books(conn, true)?
        .into_iter()
        .enumerate()
        .map(|(i, b)| (b.key, i))
        .collect();
    let off = books::disabled();
    let roads = &library().sahifa_roads;
    let mut works: Vec<Work> = rows
        .into_iter()
        .filter(|w| !(w.kind == "hadith" && off.contains(&w.key)))
        .map(|mut w| {
            if w.kind == "sahifa" {
                w.tradition = if roads.get(&w.key).map(String::as_str) == Some("shia") {
                    "shia".into()
                } else {
                    "sunni".into()
                };
            }
            if ["hadith", "sahifa", "sira", "tabaqat"].contains(&w.kind.as_str()) {
                w.title = book_title(&w.key, &w.title, lang);
            }
            if w.tradition != "sunni" && w.tradition != "shia" {
                w.tradition = "other".into();
            }
            w
        })
        .collect();
    let rank = |t: &str| match t {
        "sunni" => 0,
        "shia" => 1,
        _ => 2,
    };
    let n = order.len();
    works.sort_by_key(|w| {
        (
            rank(&w.tradition),
            w.kind != "hadith",
            order.get(&w.key).copied().unwrap_or(n),
            books::sort_name(&w.title),
        )
    });
    Ok(works)
}

pub fn find_work(conn: &Connection, key: &str, lang: &str) -> rusqlite::Result<Option<Work>> {
    let key = library()
        .aliases
        .iter()
        .find(|(a, _)| a == key)
        .map(|(_, k)| k.clone())
        .unwrap_or_else(|| key.to_string());
    Ok(list_works(conn, lang)?.into_iter().find(|w| w.key == key))
}

/// The short name a work is called by on the command line ("sira" for Ibn Hisham), or its key.
pub fn short_name(key: &str) -> String {
    library()
        .aliases
        .iter()
        .find(|(_, k)| k == key)
        .map(|(a, _)| a.clone())
        .unwrap_or_else(|| key.to_string())
}

// ------------------------------------------------------------------ chapters

fn work_meta(conn: &Connection, work_id: i64) -> (Option<String>, Option<String>, Option<i64>) {
    conn.query_row(
        "SELECT key, kind, collection_id FROM works WHERE id = ?",
        params![work_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )
    .optional()
    .ok()
    .flatten()
    .unwrap_or((None, None, None))
}

type TitleCache = HashMap<(i64, String), HashMap<String, String>>;

fn title_cache() -> &'static Mutex<TitleCache> {
    static CACHE: OnceLock<Mutex<TitleCache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn chapter_titles(conn: &Connection, work_id: i64, lang: &str) -> HashMap<String, String> {
    let cache = title_cache();
    let key = (work_id, lang.to_string());
    if let Some(v) = cache.lock().ok().and_then(|c| c.get(&key).cloned()) {
        return v;
    }
    let (_k, kind, cid) = work_meta(conn, work_id);
    let v = match (kind.as_deref(), cid) {
        (Some("hadith"), Some(cid)) if lang != "en" => edition_titles(conn, cid, lang),
        _ => HashMap::new(),
    };
    if let Ok(mut c) = cache.lock() {
        c.insert(key, v.clone());
    }
    v
}

/// A chapter's title as shown, in the language (`works._shown`).
pub fn shown(
    conn: &Connection,
    work_id: i64,
    kind: &str,
    label: Option<&str>,
    title: Option<&str>,
    lang: &str,
) -> String {
    let title_s = title.unwrap_or("");
    if kind == "kitab" {
        if let Some(l) = label {
            if let Some(k) = l.strip_prefix("book:") {
                return book_title(k, title_s, lang);
            }
            if let Some(k) = l.strip_prefix("par:") {
                let name = book_title(k, title_s, lang);
                return i18n::t(lang, "Its parallels in {book}", &[("book", name)]);
            }
            if let Some(own) = chapter_titles(conn, work_id, lang).get(l) {
                return own.clone();
            }
        }
    }
    if title_s == "Without a chapter in the source" {
        return i18n::t(lang, title_s, &[]);
    }
    if kind == "kitab" && label.is_none() {
        if let Some(p) = part_title(title_s, lang) {
            return p;
        }
    }
    if kind == "bab" {
        if let Some(l) = label.filter(|l| !l.is_empty() && l.chars().all(|c| c.is_ascii_digit())) {
            if let (Some(key), _, _) = work_meta(conn, work_id) {
                if library().chapters.contains_key(&key) {
                    if let Ok(n) = l.parse::<usize>() {
                        return toc_chapter_title(&key, n, lang)
                            .unwrap_or_else(|| title_s.to_string());
                    }
                }
            }
        }
    }
    if let Some(n) = title_s
        .strip_prefix("Chapter ")
        .filter(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
    {
        return i18n::t(lang, "Chapter {n}", &[("n", n.to_string())]);
    }
    title_s.to_string()
}

#[derive(Debug, Clone)]
pub struct Container {
    pub id: i64,
    pub kind: String,
    pub label: Option<String>,
    pub title: String,
    pub leaves: i64,
    pub subchapters: i64,
}

type ContainerRow = (i64, String, Option<String>, Option<String>, i64, i64);

fn marks(n: usize) -> String {
    vec!["?"; n].join(",")
}

/// The chapters under a parent, with how many items each holds directly.
pub fn containers(
    conn: &Connection,
    work_id: i64,
    parent_id: Option<i64>,
    lang: &str,
) -> rusqlite::Result<Vec<Container>> {
    let sql = format!(
        "SELECT n.id, n.kind, n.label, n.title,
                   (SELECT COUNT(*) FROM work_nodes c WHERE c.parent_id = n.id AND c.kind IN ({l})),
                   (SELECT COUNT(*) FROM work_nodes c WHERE c.parent_id = n.id AND c.kind IN ({c}))
            FROM work_nodes n WHERE n.work_id = ? AND n.parent_id {p}
              AND n.kind IN ({c}) ORDER BY n.ordinal",
        l = marks(LEAVES.len()),
        c = marks(CONTAINERS.len()),
        p = if parent_id.is_none() {
            "IS NULL"
        } else {
            "= ?"
        }
    );
    let mut args: Vec<Value> = LEAVES.iter().map(|s| Value::from(*s)).collect();
    args.extend(CONTAINERS.iter().map(|s| Value::from(*s)));
    args.push(Value::from(work_id));
    if let Some(p) = parent_id {
        args.push(Value::from(p));
    }
    args.extend(CONTAINERS.iter().map(|s| Value::from(*s)));
    let mut st = conn.prepare(&sql)?;
    let rows: Vec<ContainerRow> = st
        .query_map(params_from_iter(args.iter().map(json_sql)), |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
            ))
        })?
        .collect::<Result<_, _>>()?;
    Ok(rows
        .into_iter()
        .map(|(id, kind, label, title, leaves, sub)| Container {
            title: shown(
                conn,
                work_id,
                &kind,
                label.as_deref(),
                title.as_deref(),
                lang,
            ),
            id,
            kind,
            label,
            leaves,
            subchapters: sub,
        })
        .collect())
}

fn json_sql(v: &Value) -> rusqlite::types::Value {
    match v {
        Value::Number(n) => rusqlite::types::Value::Integer(n.as_i64().unwrap_or(0)),
        Value::String(s) => rusqlite::types::Value::Text(s.clone()),
        _ => rusqlite::types::Value::Null,
    }
}

#[derive(Debug, Clone)]
pub struct Node {
    pub id: i64,
    pub work_id: i64,
    pub parent_id: Option<i64>,
    pub kind: String,
    pub label: Option<String>,
    pub title: String,
}

pub fn node(conn: &Connection, node_id: i64, lang: &str) -> Option<Node> {
    let r: (
        i64,
        i64,
        Option<i64>,
        String,
        Option<String>,
        Option<String>,
    ) = conn
        .query_row(
            "SELECT id, work_id, parent_id, kind, label, title FROM work_nodes WHERE id = ?",
            params![node_id],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                ))
            },
        )
        .optional()
        .ok()
        .flatten()?;
    Some(Node {
        title: shown(conn, r.1, &r.3, r.4.as_deref(), r.5.as_deref(), lang),
        id: r.0,
        work_id: r.1,
        parent_id: r.2,
        kind: r.3,
        label: r.4,
    })
}

/// The chapter and the chapters above it, from the top.
pub fn path(conn: &Connection, node_id: i64, lang: &str) -> Vec<Node> {
    let mut out = Vec::new();
    let mut current = node(conn, node_id, lang);
    while let Some(n) = current {
        let parent = n.parent_id;
        out.insert(0, n);
        current = parent.and_then(|p| node(conn, p, lang));
    }
    out
}

#[derive(Debug, Clone)]
pub struct Leaf {
    pub id: i64,
    pub kind: String,
    pub label: Option<String>,
    pub title: Option<String>,
    pub hadith_id: Option<i64>,
    pub person_id: Option<i64>,
    pub text: Option<String>,
}

/// The readable items directly under a chapter, in order: (items, total).
pub fn leaves(
    conn: &Connection,
    node_id: i64,
    offset: i64,
    limit: i64,
) -> rusqlite::Result<(Vec<Leaf>, i64)> {
    let ks = marks(LEAVES.len());
    let mut args: Vec<rusqlite::types::Value> = vec![rusqlite::types::Value::Integer(node_id)];
    args.extend(
        LEAVES
            .iter()
            .map(|s| rusqlite::types::Value::Text(s.to_string())),
    );
    let total: i64 = conn.query_row(
        &format!("SELECT COUNT(*) FROM work_nodes WHERE parent_id = ? AND kind IN ({ks})"),
        params_from_iter(args.iter()),
        |r| r.get(0),
    )?;
    args.push(rusqlite::types::Value::Integer(limit));
    args.push(rusqlite::types::Value::Integer(offset));
    let mut st = conn.prepare(&format!(
        "SELECT id, kind, label, title, hadith_id, person_id, text FROM work_nodes
            WHERE parent_id = ? AND kind IN ({ks}) ORDER BY ordinal LIMIT ? OFFSET ?"
    ))?;
    let rows = st
        .query_map(params_from_iter(args.iter()), |r| {
            Ok(Leaf {
                id: r.get(0)?,
                kind: r.get(1)?,
                label: r.get(2)?,
                title: r.get(3)?,
                hadith_id: r.get(4)?,
                person_id: r.get(5)?,
                text: r.get(6)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok((rows, total))
}

/// Every chapter that holds readable items, in reading order.
pub fn reading_order(conn: &Connection, work_id: i64) -> rusqlite::Result<Vec<i64>> {
    fn walk(
        conn: &Connection,
        work_id: i64,
        parent: Option<i64>,
        out: &mut Vec<i64>,
    ) -> rusqlite::Result<()> {
        for c in containers(conn, work_id, parent, "en")? {
            if c.leaves > 0 {
                out.push(c.id);
            }
            if c.subchapters > 0 {
                walk(conn, work_id, Some(c.id), out)?;
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    walk(conn, work_id, None, &mut out)?;
    Ok(out)
}

/// Every text of a hadith, one after the other (what "Find in this chapter" looks in).
pub fn texts_of(conn: &Connection, hadith_id: i64) -> String {
    let run = || -> rusqlite::Result<Vec<Option<String>>> {
        let mut st = conn.prepare("SELECT text FROM texts WHERE hadith_id = ?")?;
        let rows = st.query_map(params![hadith_id], |r| r.get(0))?;
        rows.collect()
    };
    run()
        .unwrap_or_default()
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ")
}

/// The chapter of a rijal work that holds a narrator's entry (`works.chapter_of_person`).
pub fn chapter_of_person(conn: &Connection, person_id: i64) -> Option<i64> {
    conn.query_row(
        "SELECT parent_id FROM work_nodes WHERE person_id = ? AND kind = 'entry' LIMIT 1",
        params![person_id],
        |r| r.get(0),
    )
    .optional()
    .ok()
    .flatten()
    .flatten()
}

pub fn chapter_of_hadith(conn: &Connection, hadith_id: i64) -> Option<i64> {
    conn.query_row(
        "SELECT parent_id FROM work_nodes WHERE hadith_id = ? LIMIT 1",
        params![hadith_id],
        |r| r.get(0),
    )
    .optional()
    .ok()
    .flatten()
    .flatten()
}

/// A chapter of a book by its label as the book numbers it ("3", "1:2") or by the start of its title.
pub fn chapter_named(
    conn: &Connection,
    book: &str,
    label: &str,
    lang: &str,
) -> rusqlite::Result<Option<Node>> {
    let Some(work) = find_work(conn, book, lang)? else {
        return Ok(None);
    };
    let mut row: Option<i64> = conn
        .query_row(
            "SELECT id FROM work_nodes WHERE work_id = ? AND label = ? AND hadith_id IS NULL ORDER BY id LIMIT 1",
            params![work.id, label],
            |r| r.get(0),
        )
        .optional()?;
    if row.is_none() {
        row = conn
            .query_row(
                "SELECT id FROM work_nodes WHERE work_id = ? AND hadith_id IS NULL AND title LIKE ? ORDER BY id LIMIT 1",
                params![work.id, format!("{label}%")],
                |r| r.get(0),
            )
            .optional()?;
    }
    Ok(row.and_then(|id| node(conn, id, lang)))
}

/// Every hadith in these chapters and in the chapters under them (`works.hadith_under`; search in a chapter).
pub fn hadith_under(
    conn: &Connection,
    node_ids: &[i64],
) -> rusqlite::Result<std::collections::HashSet<i64>> {
    if node_ids.is_empty() {
        return Ok(Default::default());
    }
    let marks = vec!["?"; node_ids.len()].join(",");
    let sql = format!(
        "WITH RECURSIVE sub(id) AS (
            SELECT id FROM work_nodes WHERE id IN ({marks})
            UNION SELECT w.id FROM work_nodes w JOIN sub ON w.parent_id = sub.id)
         SELECT w.hadith_id FROM work_nodes w JOIN sub ON sub.id = w.id WHERE w.hadith_id IS NOT NULL"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(node_ids.iter()), |r| {
        r.get::<_, i64>(0)
    })?;
    rows.collect()
}

pub struct ChapterHadith {
    pub hadith_id: i64,
    pub number: String,
    pub links: i64,
    pub words: String,
}

/// The hadith of one chapter, to choose one without knowing its number: the start of its text in the language asked
/// for (else Arabic) and how many narrators its chain was split into (`works.chapter_hadith`).
pub fn chapter_hadith(
    conn: &Connection,
    node_id: i64,
    language: Option<&str>,
    width: usize,
) -> rusqlite::Result<Vec<ChapterHadith>> {
    let mut st = conn.prepare(
        "SELECT w.hadith_id, h.number FROM work_nodes w JOIN hadiths h ON h.id = w.hadith_id
           WHERE w.parent_id = ? AND w.hadith_id IS NOT NULL ORDER BY w.ordinal",
    )?;
    let rows: Vec<(i64, String)> = st
        .query_map(params![node_id], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    if rows.is_empty() {
        return Ok(Vec::new());
    }
    let ids: Vec<rusqlite::types::Value> = rows
        .iter()
        .map(|r| rusqlite::types::Value::Integer(r.0))
        .collect();
    let m = marks(ids.len());
    let mut words: HashMap<i64, (u8, String)> = HashMap::new();
    let mut st = conn.prepare(&format!(
        "SELECT t.hadith_id, e.language, t.text FROM texts t JOIN editions e ON e.id = t.edition_id
                WHERE t.hadith_id IN ({m})"
    ))?;
    for r in st.query_map(params_from_iter(ids.iter()), |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, Option<String>>(1)?,
            r.get::<_, Option<String>>(2)?,
        ))
    })? {
        let (hid, lang, text) = r?;
        let rank = if language.is_some() && lang.as_deref() == language {
            0
        } else if lang.as_deref() == Some("Arabic") {
            1
        } else {
            2
        };
        let better = words.get(&hid).map(|w| rank < w.0).unwrap_or(true);
        if better {
            let joined = text
                .unwrap_or_default()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            words.insert(hid, (rank, joined));
        }
    }
    let mut links: HashMap<i64, i64> = HashMap::new();
    let mut st = conn.prepare(&format!(
        "SELECT i.hadith_id, COUNT(l.position) FROM isnads i LEFT JOIN isnad_links l ON l.isnad_id = i.id
            WHERE i.hadith_id IN ({m}) GROUP BY i.hadith_id"
    ))?;
    for r in st.query_map(params_from_iter(ids.iter()), |r| {
        Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?))
    })? {
        let (h, n) = r?;
        links.insert(h, n);
    }
    Ok(rows
        .into_iter()
        .map(|(hid, number)| {
            let text = words.get(&hid).map(|w| w.1.clone()).unwrap_or_default();
            let shown = if text.chars().count() <= width {
                text
            } else {
                let cut: String = text.chars().take(width).collect();
                let head = match cut.rfind(' ') {
                    Some(i) => cut[..i].to_string(),
                    None => cut,
                };
                head + " …"
            };
            ChapterHadith {
                hadith_id: hid,
                number,
                links: links.get(&hid).copied().unwrap_or(0),
                words: shown,
            }
        })
        .collect())
}
