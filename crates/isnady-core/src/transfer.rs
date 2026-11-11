//! Export and import, of everything, each in the formats that fit it (`core/transfer.py`, EX1): `iy export`,
//! `iy import-data`, File → Export… / Import….
//!
//!   hadith, book, search   json jsonl csv tsv xlsx md html txt epub docx sqlite (a book also as isnady-book)
//!   bookmarks, narrators   the table formats · network: graphml gexf csv json · settings: json csv tsv md
//!   profile: isnady-profile json · backup · statistics: json
//!
//! PDF is drawn from the HTML by the window's library in the Python isnady; here it comes with the window's PDF.

use rusqlite::{params, params_from_iter, Connection, OptionalExtension};
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::formats::{self as F, Entry, Row, Section, Table};
use crate::pre;
use crate::py::thousands;
use crate::stats::pyjson;
use crate::{config, isnad, narrators, works};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// (kind, label, export formats, import formats) (`KINDS`).
pub const KINDS: [(&str, &str, &[&str], &[&str]); 10] = [
    (
        "hadith",
        "Hadith",
        &[
            "json", "jsonl", "csv", "tsv", "xlsx", "md", "html", "txt", "epub", "docx", "pdf",
            "sqlite",
        ],
        &[],
    ),
    (
        "book",
        "Book or chapter",
        &[
            "isnady-book",
            "json",
            "jsonl",
            "csv",
            "tsv",
            "xlsx",
            "md",
            "html",
            "txt",
            "epub",
            "docx",
            "pdf",
            "sqlite",
        ],
        &["isnady-book", "json", "jsonl", "csv", "tsv"],
    ),
    (
        "search",
        "Search results",
        &[
            "json", "jsonl", "csv", "tsv", "xlsx", "md", "html", "txt", "epub", "docx", "pdf",
            "sqlite",
        ],
        &[],
    ),
    (
        "bookmarks",
        "Bookmarks",
        &[
            "json", "jsonl", "csv", "tsv", "xlsx", "md", "html", "txt", "pdf",
        ],
        &["json", "jsonl", "csv", "tsv"],
    ),
    (
        "narrators",
        "Narrators",
        &[
            "json", "jsonl", "csv", "tsv", "xlsx", "md", "html", "txt", "pdf", "sqlite",
        ],
        &[],
    ),
    (
        "network",
        "Isnad network",
        &["graphml", "gexf", "csv", "json"],
        &[],
    ),
    (
        "settings",
        "Settings",
        &["json", "csv", "tsv", "md"],
        &["json", "csv", "tsv"],
    ),
    (
        "profile",
        "Profile",
        &["isnady-profile", "json"],
        &["isnady-profile", "json"],
    ),
    (
        "backup",
        "Everything of mine (backup)",
        &["backup"],
        &["backup"],
    ),
    ("statistics", "Statistics", &["json"], &[]),
];
const DOCUMENT_KINDS: [&str; 3] = ["hadith", "book", "search"];
/// What a profile holds (`profiles.FILES`).
pub const FILES: [&str; 3] = ["settings.json", "state.json", "bookmarks.json"];

pub fn kind(
    k: &str,
) -> Option<&'static (
    &'static str,
    &'static str,
    &'static [&'static str],
    &'static [&'static str],
)> {
    KINDS.iter().find(|x| x.0 == k)
}

/// Stopped by the reader: the half-written file is removed.
pub const CANCELLED: &str = "Stopped.";

type Res<T> = Result<T, String>;
pub type Say<'a> = &'a dyn Fn(&str);
/// (done, total): how far a long export is; false stops it.
pub type Tick<'a> = Option<&'a dyn Fn(usize, usize) -> bool>;

fn db<T>(r: rusqlite::Result<T>) -> Res<T> {
    r.map_err(|e| e.to_string())
}

fn io<T>(r: std::io::Result<T>) -> Res<T> {
    r.map_err(|e| e.to_string())
}

fn meta(kind: &str, more: Vec<(&str, Value)>) -> Row {
    let mut m = Map::new();
    m.insert("kind".into(), json!(kind));
    m.insert(
        "exported".into(),
        json!(crate::py::now_local("%Y-%m-%dT%H:%M:%S")),
    );
    m.insert("isnady".into(), json!(VERSION));
    for (k, v) in more {
        m.insert(k.into(), v);
    }
    m
}

pub fn suffix(fmt: &str) -> String {
    F::extension(fmt)
}

fn zip_names(p: &Path) -> Option<Vec<String>> {
    let z = zip::ZipArchive::new(std::fs::File::open(p).ok()?).ok()?;
    Some(z.file_names().map(str::to_string).collect())
}

/// The format of a file to import, from its name (and, for a zip, its contents) (`guess_format`).
pub fn guess_format(path: &Path, kind: &str) -> String {
    let name = path.to_string_lossy().to_lowercase();
    for fmt in ["isnady-book", "isnady-profile", "backup"] {
        if name.ends_with(&F::extension(fmt)) {
            return fmt.into();
        }
    }
    if path.is_dir() || name.ends_with("book.json") {
        return "isnady-book".into();
    }
    if name.ends_with(".zip") {
        let Some(names) = zip_names(path) else {
            return "zip".into();
        };
        if names.iter().any(|n| n == "profile.json") {
            return "isnady-profile".into();
        }
        if names.iter().any(|n| n == "backup.json") {
            return "backup".into();
        }
        if names.iter().any(|n| n.ends_with("book.json")) {
            return "isnady-book".into();
        }
        return "zip".into();
    }
    match path.extension().and_then(|e| e.to_str()) {
        Some(e) if !e.is_empty() => e.to_string(),
        _ => kind.to_string(),
    }
}

// ================================================================== what is exported
/// `search.list_languages`.
pub fn list_languages(conn: &Connection) -> Res<Vec<String>> {
    let mut st = db(conn.prepare(
        "SELECT DISTINCT language FROM editions WHERE language IS NOT NULL AND text_count > 0 ORDER BY language",
    ))?;
    let mut v: Vec<String> = db(db(st.query_map([], |r| r.get(0)))?.collect())?;
    let chosen = crate::books::language_order();
    v.sort_by_key(|l| crate::books::language_rank(l, &chosen));
    Ok(v)
}

fn languages_of(conn: &Connection, wanted: &[String]) -> Res<Vec<String>> {
    let have = list_languages(conn)?;
    let mut order = vec!["Arabic".to_string()];
    order.extend(have.iter().filter(|l| *l != "Arabic").cloned());
    Ok(order
        .into_iter()
        .filter(|l| have.contains(l) && (wanted.is_empty() || wanted.contains(l)))
        .collect())
}

fn languages_present(conn: &Connection, ids: &[i64], langs: &[String]) -> Res<Vec<String>> {
    let mut have = std::collections::HashSet::new();
    for chunk in ids.chunks(900) {
        let mut st = db(conn.prepare(&format!(
            "SELECT DISTINCT e.language FROM editions e WHERE e.collection_id IN
             (SELECT DISTINCT collection_id FROM hadiths WHERE id IN ({}))",
            vec!["?"; chunk.len()].join(",")
        )))?;
        for l in db(st.query_map(params_from_iter(chunk.iter()), |r| {
            r.get::<_, Option<String>>(0)
        }))? {
            if let Some(l) = db(l)? {
                have.insert(l);
            }
        }
    }
    Ok(langs
        .iter()
        .filter(|l| have.contains(*l))
        .cloned()
        .collect())
}

/// Each hadith as a record, one after another (`iter_records`): book, number, chapter, texts by language, grades,
/// chains. `each` gets every record; false from `tick` stops with CANCELLED.
#[allow(clippy::too_many_arguments)]
pub fn for_records(
    conn: &Connection,
    ids: &[i64],
    languages: &[String],
    chains: bool,
    say: Say,
    tick: Tick,
    each: &mut dyn FnMut(Value) -> Res<()>,
) -> Res<()> {
    let langs = languages_of(conn, languages)?;
    let total = ids.len();
    let ui = crate::i18n::ui_language();
    let mut titles: HashMap<(i64, i64, i64), Option<String>> = HashMap::new();
    let people: HashMap<i64, narrators::Person> = HashMap::new();
    let mut book_now = String::new();
    let mut first = true;
    for (i, hid) in ids.iter().enumerate() {
        if let Some(t) = tick {
            if i % 25 == 0 && !t(i, total) {
                return Err(CANCELLED.into());
            }
        }
        let head: Option<(String, Option<i64>, String, String, String, i64)> = db(conn
            .query_row(
                "SELECT h.number, h.section_number, c.key, c.name, COALESCE(c.tradition, 'sunni'), h.collection_id
                 FROM hadiths h JOIN collections c ON c.id = h.collection_id WHERE h.id = ?",
                [hid],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
            )
            .optional())?;
        let Some((number, section, key, name, road, cid)) = head else {
            continue;
        };
        if first || key != book_now {
            first = false;
            book_now = key.clone();
            say(&format!(
                "{} · {} / {}",
                works::book_title(&key, &name, &ui),
                thousands(i as i64),
                thousands(total as i64)
            ));
        }
        let mut texts: Vec<(String, String)> = vec![];
        let mut chapter: Vec<(String, String)> = vec![];
        {
            let mut st = db(conn.prepare_cached(
                "SELECT e.language, t.text, e.id FROM texts t JOIN editions e ON e.id = t.edition_id
                 WHERE t.hadith_id = ? ORDER BY e.id",
            ))?;
            let rows: Vec<(Option<String>, String, i64)> =
                db(db(st.query_map([hid], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))))?.collect())?;
            for (lang, text, eid) in rows {
                let Some(lang) = lang else { continue };
                if !langs.contains(&lang) || texts.iter().any(|(l, _)| *l == lang) {
                    continue;
                }
                texts.push((lang.clone(), text));
                if let Some(sec) = section {
                    let k = (cid, sec, eid);
                    #[allow(clippy::map_entry)]
                    if !titles.contains_key(&k) {
                        let t: Option<Option<String>> = db(conn
                            .query_row(
                                "SELECT st.title FROM section_titles st JOIN sections s ON s.id = st.section_id
                                 WHERE s.collection_id = ? AND s.number = ? AND st.edition_id = ?",
                                params![cid, sec, eid],
                                |r| r.get(0),
                            )
                            .optional())?;
                        titles.insert(k, t.flatten());
                    }
                    if let Some(t) = titles[&k].as_ref().filter(|t| !t.is_empty()) {
                        chapter.push((lang, t.clone()));
                    }
                }
            }
        }
        let grades: Vec<Value> = {
            let mut st = db(conn.prepare_cached(
                "SELECT grader_name, grade FROM grades WHERE hadith_id = ? ORDER BY id",
            ))?;
            let v = db(db(st.query_map([hid], |r| {
                Ok(json!([
                    r.get::<_, Option<String>>(0)?,
                    r.get::<_, Option<String>>(1)?
                ]))
            }))?
            .collect())?;
            v
        };
        let by_lang = |pairs: &[(String, String)]| {
            let mut m = Map::new();
            for l in &langs {
                if let Some((_, v)) = pairs.iter().find(|(x, _)| x == l) {
                    m.insert(l.clone(), json!(v));
                }
            }
            Value::Object(m)
        };
        let mut record = Map::new();
        record.insert("book".into(), json!(key));
        record.insert("book_title".into(), json!(name));
        record.insert("tradition".into(), json!(road));
        record.insert("number".into(), json!(number));
        record.insert("chapter_number".into(), json!(section));
        record.insert("chapter".into(), by_lang(&chapter));
        record.insert("texts".into(), by_lang(&texts));
        record.insert("grades".into(), Value::Array(grades));
        if chains {
            let mut out = vec![];
            for ch in db(isnad::chain(conn, *hid))? {
                let raw = ch.raw.clone().unwrap_or_default();
                if ch.links.is_empty() && raw.is_empty() {
                    continue;
                }
                out.push(json!({
                    "arabic": raw,
                    "turkish": isnad::chain_in_words(&ch, "tr", &people),
                    "english": isnad::chain_in_words(&ch, "en", &people),
                    "narrators": ch.links.iter().map(|l| json!({"name": l.raw_name, "term": l.transmission,
                        "identified": l.person_id.is_some()})).collect::<Vec<_>>(),
                    "reaches": if ch.reaches_prophet || !ch.ends_with.is_empty() { isnad::end_label(&ch) } else { String::new() },
                }));
            }
            record.insert("chains".into(), Value::Array(out));
        }
        each(Value::Object(record))?;
    }
    if let Some(t) = tick {
        if !t(total, total) {
            return Err(CANCELLED.into());
        }
    }
    Ok(())
}

pub fn book_ids(conn: &Connection, book: &str, chapter: Option<i64>) -> Res<Vec<i64>> {
    let key = works::resolve(book);
    let mut sql =
        "SELECT h.id FROM hadiths h JOIN collections c ON c.id = h.collection_id WHERE c.key = ?1"
            .to_string();
    if chapter.is_some() {
        sql.push_str(" AND h.section_number = ?2");
    } else {
        sql.push_str(" AND ?2 IS NULL");
    }
    sql.push_str(" ORDER BY h.number_sort, h.id");
    let mut st = db(conn.prepare(&sql))?;
    let ids: Vec<i64> = db(db(st.query_map(params![key, chapter], |r| r.get(0)))?.collect())?;
    if ids.is_empty() {
        return Err(format!(
            "No hadith in '{book}'{}.",
            chapter
                .map(|c| format!(", chapter {c}"))
                .unwrap_or_default()
        ));
    }
    Ok(ids)
}

/// Books named for an export: keys or full names, or "all", "sunni", "shia" (`book_keys`).
pub fn book_keys(conn: &Connection, names: &[String]) -> Res<Vec<String>> {
    let every = db(crate::books::all_books(conn, false))?;
    let mut out: Vec<String> = vec![];
    for name in names {
        let name = crate::py::trim(name);
        let add: Vec<String> = if ["all", "sunni", "shia"].contains(&name) {
            every
                .iter()
                .filter(|b| name == "all" || b.tradition == name)
                .map(|b| b.key.clone())
                .collect()
        } else if !name.is_empty() {
            vec![works::resolve(name)]
        } else {
            vec![]
        };
        for k in add {
            if !out.contains(&k) {
                out.push(k);
            }
        }
    }
    Ok(out)
}

/// "bukhari:1", "bukhari:1-10" → hadith ids, in the order asked (`ref_ids`).
pub fn ref_ids(conn: &Connection, refs: &[String]) -> Res<Vec<i64>> {
    let mut out: Vec<i64> = vec![];
    let joined = refs.join(" ").replace(',', " ");
    for r in crate::reader::pyre::split(&joined) {
        let (book, nums) = r.split_once(':').unwrap_or((r, ""));
        if nums.is_empty() {
            return Err(format!(
                "'{r}': write it as book:number, e.g. bukhari:1 or bukhari:1-10."
            ));
        }
        let key = works::resolve(book);
        let found: Vec<i64> = if let Some((a, b)) = nums.split_once('-') {
            let f = crate::sources::importers::table::py_float;
            let (Some(lo), Some(hi)) = (f(a), f(b)) else {
                return Err(format!("'{r}': a range is two numbers, e.g. bukhari:1-10."));
            };
            let mut st = db(conn.prepare(
                "SELECT h.id FROM hadiths h JOIN collections c ON c.id = h.collection_id
                 WHERE c.key = ? AND h.number_sort BETWEEN ? AND ? ORDER BY h.number_sort, h.id",
            ))?;
            let v = db(db(st.query_map(params![key, lo, hi], |x| x.get(0)))?.collect())?;
            v
        } else {
            let row: Option<i64> = db(conn
                .query_row(
                    "SELECT h.id FROM hadiths h JOIN collections c ON c.id = h.collection_id WHERE c.key = ? AND h.number = ?",
                    params![key, nums],
                    |x| x.get(0),
                )
                .optional())?;
            match row {
                Some(id) => vec![id],
                None => return Err(format!("No hadith {r}.")),
            }
        };
        for id in found {
            if !out.contains(&id) {
                out.push(id);
            }
        }
    }
    if out.is_empty() {
        return Err("No hadith named.".into());
    }
    Ok(out)
}

/// The Search page's choices kept with a search export (`SEARCH_FILTERS`).
#[derive(Debug, Clone, Default)]
pub struct SearchFilters {
    pub mode: Option<String>,
    pub whole_words: bool,
    pub collections: Vec<String>,
    pub languages: Vec<String>,
    pub grades: Vec<String>,
    pub every_grader: bool,
    pub sahihayn: bool,
    pub chapters: Vec<i64>,
}

fn search_ids(conn: &Connection, text: &str, f: Option<&SearchFilters>) -> Res<Vec<i64>> {
    let mut q = crate::search::SearchQuery::new(text);
    q.limit = 1000;
    if let Some(f) = f {
        if let Some(m) = f.mode.as_deref().filter(|m| !m.is_empty()) {
            q.mode = if m == "meaning" {
                "all".into()
            } else {
                m.into()
            };
        }
        q.whole_words = f.whole_words;
        q.collections = f.collections.clone();
        q.languages = f.languages.clone();
        q.grades = f.grades.clone();
        q.every_grader = f.every_grader;
        q.sahihayn = f.sahihayn;
        q.chapters = f.chapters.clone();
    }
    let page = crate::search::search(conn, &q).map_err(|e| e.to_string())?;
    if page.results.is_empty() {
        return Err(format!("Nothing found for '{text}'."));
    }
    Ok(page.results.iter().map(|r| r.hadith_id).collect())
}

fn all_or(v: &[String]) -> Value {
    if v.is_empty() {
        json!("all")
    } else {
        json!(v)
    }
}

/// How the hadith of a search were found, said plainly (`search_method`).
pub fn search_method(conn: &Connection, query: &str, f: Option<&SearchFilters>) -> Value {
    let d = SearchFilters::default();
    let f = f.unwrap_or(&d);
    let mut out = Map::new();
    out.insert("words".into(), json!(query));
    out.insert(
        "match".into(),
        json!(f
            .mode
            .clone()
            .filter(|m| !m.is_empty())
            .unwrap_or_else(|| "all".into())),
    );
    out.insert("whole_words".into(), json!(f.whole_words));
    out.insert("books".into(), all_or(&f.collections));
    out.insert("languages".into(), all_or(&f.languages));
    if !f.grades.is_empty() {
        out.insert(
            "grades".into(),
            json!(crate::grades::describe_choice(
                &f.grades,
                f.every_grader,
                &|s| s.to_string()
            )),
        );
        out.insert(
            "sahihayn_counted_sahih".into(),
            json!(f.sahihayn && f.grades.iter().any(|g| g == "sahih")),
        );
    }
    if !f.chapters.is_empty() {
        let names: Vec<String> = f
            .chapters
            .iter()
            .map(|c| {
                works::path(conn, *c, "en")
                    .iter()
                    .map(|n| {
                        format!(
                            "{}{}",
                            n.label
                                .as_deref()
                                .filter(|l| !l.is_empty())
                                .map(|l| format!("{l}. "))
                                .unwrap_or_default(),
                            n.title
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(" › ")
            })
            .collect();
        out.insert("chapters".into(), json!(names));
    }
    Value::Object(out)
}

fn hadith_columns(langs: &[String], with_chains: bool) -> Vec<String> {
    let mut cols: Vec<String> = [
        "book",
        "book_title",
        "tradition",
        "number",
        "chapter_number",
        "chapter",
    ]
    .map(String::from)
    .to_vec();
    cols.extend(langs.iter().map(|l| format!("text_{l}")));
    cols.push("grades".into());
    if with_chains {
        cols.extend(
            ["chain_arabic", "chain_turkish", "chain_english", "reaches"].map(String::from),
        );
    }
    cols
}

fn first_value(v: &Value) -> String {
    v.as_object()
        .and_then(|o| o.values().next())
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn grades_note(r: &Value) -> String {
    r["grades"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|g| format!("{}: {}", F::cell(g.get(0)), F::cell(g.get(1))))
                .collect::<Vec<_>>()
                .join("; ")
        })
        .unwrap_or_default()
}

fn hadith_row(r: &Value, langs: &[String], with_chains: bool) -> Row {
    let mut row = Map::new();
    for k in [
        "book",
        "book_title",
        "tradition",
        "number",
        "chapter_number",
    ] {
        row.insert(k.into(), r[k].clone());
    }
    row.insert("chapter".into(), json!(first_value(&r["chapter"])));
    row.insert("grades".into(), json!(grades_note(r)));
    for l in langs {
        row.insert(
            format!("text_{l}"),
            json!(r["texts"].get(l).and_then(Value::as_str).unwrap_or("")),
        );
    }
    if with_chains {
        let empty = vec![];
        let chains = r.get("chains").and_then(Value::as_array).unwrap_or(&empty);
        let join = |k: &str, skip_empty: bool| {
            chains
                .iter()
                .map(|c| c[k].as_str().unwrap_or(""))
                .filter(|v| !skip_empty || !v.is_empty())
                .collect::<Vec<_>>()
                .join(" ‖ ")
        };
        row.insert("chain_arabic".into(), json!(join("arabic", false)));
        row.insert("chain_turkish".into(), json!(join("turkish", false)));
        row.insert("chain_english".into(), json!(join("english", false)));
        row.insert("reaches".into(), json!(join("reaches", true)));
    }
    row
}

fn entry(r: &Value, lang: &str) -> Entry {
    let tr = |s: &str| crate::i18n::t(lang, s, &[]);
    let mut paragraphs = vec![];
    for c in r
        .get("chains")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        paragraphs.push((tr("Chain"), c["arabic"].as_str().unwrap_or("").to_string()));
        paragraphs.push((
            tr("Turkish"),
            c["turkish"].as_str().unwrap_or("").to_string(),
        ));
        paragraphs.push((
            tr("English"),
            c["english"].as_str().unwrap_or("").to_string(),
        ));
    }
    for (l, text) in r["texts"].as_object().into_iter().flatten() {
        paragraphs.push((tr(l), text.as_str().unwrap_or("").to_string()));
    }
    Entry {
        heading: format!(
            "{} {}",
            r["book_title"].as_str().unwrap_or(""),
            F::cell(r.get("number"))
        ),
        paragraphs,
        note: grades_note(r),
    }
}

/// The records grouped by book and chapter, one section after another as they come (`hadith_sections`).
struct Sections {
    section: Option<Section>,
    current: Option<(String, Option<i64>)>,
    lang: String,
}

impl Sections {
    fn push(&mut self, r: &Value) -> Option<Section> {
        let key = (
            r["book"].as_str().unwrap_or("").to_string(),
            r["chapter_number"].as_i64(),
        );
        let mut done = None;
        if self.current.as_ref() != Some(&key) {
            done = self.section.take();
            let ch = first_value(&r["chapter"]);
            let has = r["chapter"].as_object().is_some_and(|o| !o.is_empty());
            let heading = format!(
                "{}{}",
                r["book_title"].as_str().unwrap_or(""),
                if has {
                    format!(" — {ch}")
                } else {
                    String::new()
                }
            );
            self.section = Some(Section {
                heading,
                entries: vec![],
            });
            self.current = Some(key);
        }
        if let Some(s) = self.section.as_mut() {
            s.entries.push(entry(r, &self.lang));
        }
        done
    }
}

fn bookmark_table() -> Table {
    let store = crate::bookmarks::Store::load();
    let rows = store
        .items
        .iter()
        .map(|i| {
            let mut m = Map::new();
            m.insert("id".into(), json!(i.id));
            m.insert("kind".into(), json!(i.kind));
            m.insert("ref".into(), json!(i.reference));
            m.insert("title".into(), json!(i.title));
            m.insert("subtitle".into(), json!(i.subtitle));
            m.insert("note".into(), json!(i.note));
            m.insert(
                "folders".into(),
                json!(i
                    .folders
                    .iter()
                    .map(|f| store.path_of(f))
                    .collect::<Vec<_>>()),
            );
            let added = if i.added > 0.0 {
                crate::py::local_time(
                    std::time::UNIX_EPOCH + std::time::Duration::from_secs_f64(i.added),
                    "%Y-%m-%d %H:%M",
                )
            } else {
                String::new()
            };
            m.insert("added".into(), json!(added));
            m
        })
        .collect();
    Table {
        title: "isnady bookmarks".into(),
        columns: [
            "kind", "ref", "title", "subtitle", "note", "folders", "added", "id",
        ]
        .map(String::from)
        .to_vec(),
        rows,
        meta: meta("bookmarks", vec![]),
    }
}

fn narrator_table(conn: &Connection, text: &str, tradition: Option<&str>) -> Res<Table> {
    let ui = crate::i18n::ui_language();
    let langs = narrators::Langs {
        reading: &ui,
        words: &ui,
    };
    let (rows, _total) = db(narrators::browse(
        conn, text, tradition, None, None, 100000, 0, langs,
    ))?;
    let opt = |s: &str| if s.is_empty() { Value::Null } else { json!(s) };
    let out = rows
        .iter()
        .map(|r| {
            let mut m = Map::new();
            m.insert("id".into(), json!(r.id));
            m.insert("tradition".into(), json!(r.tradition));
            m.insert("name_ar".into(), json!(r.name));
            m.insert("known_as".into(), json!(r.view.arabic));
            m.insert("reading".into(), json!(r.view.reading));
            m.insert("full_reading".into(), json!(r.view.full_reading));
            m.insert("tabaqa".into(), json!(r.tabaqa));
            m.insert("death_ah".into(), json!(r.death));
            m.insert("rank".into(), json!(r.rank));
            m.insert("verdict".into(), json!(r.verdict));
            m.insert("in_chains".into(), json!(r.in_chains));
            m.insert("source".into(), opt(&r.source));
            m
        })
        .collect();
    Ok(Table {
        title: format!(
            "isnady narrators{}",
            if text.is_empty() {
                String::new()
            } else {
                format!(": {text}")
            }
        ),
        columns: [
            "id",
            "tradition",
            "known_as",
            "reading",
            "full_reading",
            "name_ar",
            "tabaqa",
            "death_ah",
            "rank",
            "verdict",
            "in_chains",
            "source",
        ]
        .map(String::from)
        .to_vec(),
        rows: out,
        meta: meta(
            "narrators",
            vec![
                ("tradition", json!(tradition.unwrap_or("all"))),
                ("query", json!(text)),
            ],
        ),
    })
}

/// Who narrated from whom in the chains: nodes and weighted edges teacher → student (`network`).
pub fn network(
    conn: &Connection,
    books: &[String],
    tradition: Option<&str>,
) -> Res<(Vec<Value>, Vec<Value>)> {
    let keys = if books.is_empty() {
        vec![]
    } else {
        book_keys(conn, books)?
    };
    let mut wh = String::new();
    let mut args: Vec<rusqlite::types::Value> = vec![];
    if !keys.is_empty() {
        wh = format!("AND c.key IN ({})", vec!["?"; keys.len()].join(","));
        args.extend(keys.iter().map(|k| rusqlite::types::Value::Text(k.clone())));
    }
    if let Some(t) = tradition {
        wh.push_str(" AND COALESCE(c.tradition, 'sunni') = ?");
        args.push(rusqlite::types::Value::Text(t.into()));
    }
    args.push(rusqlite::types::Value::Integer(1));
    let mut st = db(conn.prepare(&format!(
        "SELECT b.person_id AS teacher, a.person_id AS student, COUNT(*) FROM isnad_links a
         JOIN isnad_links b ON b.isnad_id = a.isnad_id AND b.position = a.position + 1
         JOIN isnads i ON i.id = a.isnad_id JOIN hadiths h ON h.id = i.hadith_id JOIN collections c ON c.id = h.collection_id
         WHERE a.person_id IS NOT NULL AND b.person_id IS NOT NULL AND a.person_id != b.person_id {wh}
         GROUP BY teacher, student HAVING COUNT(*) >= ?"
    )))?;
    let pairs: Vec<(i64, i64, i64)> = db(db(st.query_map(params_from_iter(args.iter()), |r| {
        Ok((r.get(0)?, r.get(1)?, r.get(2)?))
    }))?
    .collect())?;
    if pairs.is_empty() {
        return Err("No identified teacher and student in these chains.".into());
    }
    let mut ids: Vec<i64> = pairs.iter().flat_map(|(t, s, _)| [*t, *s]).collect();
    ids.sort_unstable();
    ids.dedup();
    let mut nodes = vec![];
    for chunk in ids.chunks(900) {
        let mut st = db(conn.prepare(&format!(
            "SELECT id, name_ar, tabaqa, COALESCE(tradition, 'sunni') FROM persons WHERE id IN ({}) ORDER BY id",
            chunk.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(",")
        )))?;
        let rows: Vec<(i64, Option<String>, Option<i64>, String)> = db(db(
            st.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
        )?
        .collect())?;
        for (pid, name, tabaqa, road) in rows {
            let shown = narrators::display_name(&name.unwrap_or_default());
            let head = crate::reader::pyre::split(&shown)
                .into_iter()
                .take(4)
                .collect::<Vec<_>>()
                .join(" ");
            let (label, _) = isnad::name_reading(&head, "en");
            nodes.push(json!({"id": pid, "label": if label.is_empty() { shown.clone() } else { label }, "arabic": shown, "tabaqa": tabaqa, "tradition": road}));
        }
    }
    let edges = pairs
        .iter()
        .map(|(t, s, n)| json!({"source": t, "target": s, "weight": n}))
        .collect();
    Ok((nodes, edges))
}

fn settings_table() -> Table {
    let rows = config::all_values()
        .into_iter()
        .map(|(k, v, d)| {
            let mut m = Map::new();
            let changed = v != d;
            m.insert("key".into(), json!(k));
            m.insert("value".into(), v);
            m.insert("default".into(), d);
            m.insert("changed".into(), json!(changed));
            m
        })
        .collect();
    Table {
        title: "isnady settings".into(),
        columns: ["key", "value", "default", "changed"]
            .map(String::from)
            .to_vec(),
        rows,
        meta: meta("settings", vec![]),
    }
}

// ================================================================== export
/// What `export` is asked (the arguments of `transfer.export`).
#[derive(Default)]
pub struct Export<'a> {
    pub kind: String,
    pub fmt: String,
    pub target: PathBuf,
    pub book: String,
    pub chapter: Option<i64>,
    pub refs: Vec<String>,
    pub query: String,
    pub languages: Vec<String>,
    pub chains: bool,
    pub tradition: Option<String>,
    pub profile: Option<String>,
    pub books: Vec<String>,
    pub profiles: Vec<String>,
    pub separate: bool,
    pub search: Option<SearchFilters>,
    pub tick: Tick<'a>,
}

fn collection_name(conn: &Connection, key: &str) -> Res<String> {
    db(conn
        .query_row("SELECT name FROM collections WHERE key = ?", [key], |r| {
            r.get::<_, String>(0)
        })
        .optional())?
    .ok_or_else(|| format!("No book '{key}'."))
}

fn each_file(
    keys: &[String],
    folder: &Path,
    fmt: &str,
    say: Say,
    names: Option<&HashMap<String, String>>,
    write: &mut dyn FnMut(&str, &Path) -> Res<()>,
) -> Res<PathBuf> {
    let mut folder = folder.to_path_buf();
    if folder.extension().is_some() && !folder.is_dir() {
        folder = folder.with_extension("");
    }
    io(std::fs::create_dir_all(&folder))?;
    for (n, key) in keys.iter().enumerate() {
        let base = names
            .and_then(|m| m.get(key))
            .filter(|b| !b.is_empty())
            .cloned()
            .unwrap_or_else(|| works::full_name(key));
        let base: String = base
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || "-_. ".contains(c) {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let base = crate::py::trim(&base).to_string();
        let base = if base.is_empty() { key.clone() } else { base };
        say(&format!("{}/{}: {base}", n + 1, keys.len()));
        write(key, &folder.join(format!("{base}{}", suffix(fmt))))?;
    }
    Ok(folder)
}

/// Write what is asked; the file (or folder) written (`transfer.export`).
pub fn export(conn: Option<&Connection>, a: &Export, say: Say) -> Res<PathBuf> {
    let (kind_, fmt) = (a.kind.as_str(), a.fmt.as_str());
    let mut book = a.book.clone();
    let mut books = a.books.clone();
    let need = || conn.ok_or_else(|| "No database.".to_string());
    if kind_ == "book" && !books.is_empty() {
        let conn = need()?;
        let keys = book_keys(conn, &books)?;
        if keys.is_empty() {
            return Err("No book chosen.".into());
        }
        if keys.len() == 1 {
            book = keys[0].clone();
            books = vec![];
        } else if a.separate || fmt == "isnady-book" {
            let mut counts: HashMap<String, usize> = HashMap::new();
            for k in &keys {
                let n: i64 = db(conn.query_row("SELECT COUNT(*) FROM hadiths h JOIN collections c ON c.id = h.collection_id WHERE c.key = ?", [k], |r| r.get(0)))?;
                counts.insert(k.clone(), n as usize);
            }
            let whole: usize = counts.values().sum();
            let mut before: HashMap<String, usize> = HashMap::new();
            let mut acc = 0;
            for k in &keys {
                before.insert(k.clone(), acc);
                acc += counts[k];
            }
            return each_file(&keys, &a.target, fmt, say, None, &mut |key, path| {
                let b = before[key];
                let inner =
                    |done: usize, _t: usize| a.tick.map(|t| t(b + done, whole)).unwrap_or(true);
                let one = Export {
                    kind: "book".into(),
                    fmt: fmt.into(),
                    target: path.to_path_buf(),
                    book: key.into(),
                    languages: a.languages.clone(),
                    chains: a.chains,
                    tick: if a.tick.is_some() { Some(&inner) } else { None },
                    ..Default::default()
                };
                export(Some(conn), &one, say).map(|_| ())
            });
        }
    }
    if kind_ == "profile" && a.profiles.len() > 1 {
        let names: HashMap<String, String> = config::all_profiles()
            .into_iter()
            .map(|p| (p.id.clone(), if p.name.is_empty() { p.id } else { p.name }))
            .collect();
        let mut keys: Vec<String> = vec![];
        for p in &a.profiles {
            if !keys.contains(p) {
                keys.push(p.clone());
            }
        }
        return each_file(
            &keys,
            &a.target,
            fmt,
            say,
            Some(&names),
            &mut |pid, path| export_profile(fmt, path, Some(pid)).map(|_| ()),
        );
    }
    let profile = a.profiles.first().cloned().or_else(|| a.profile.clone());
    let Some(spec) = kind(kind_) else {
        return Err(format!(
            "Nothing called '{kind_}' is exported. One of: {}.",
            KINDS.map(|k| k.0).join(", ")
        ));
    };
    if !spec.2.contains(&fmt) {
        return Err(format!(
            "{} is exported as {}; not {fmt}.",
            spec.1,
            spec.2.join(", ")
        ));
    }
    let target = crate::paths::expand_user(&a.target.to_string_lossy());
    if DOCUMENT_KINDS.contains(&kind_) {
        let conn = need()?;
        let (ids, title) = match kind_ {
            "book" => {
                if book.is_empty() && books.is_empty() {
                    return Err("Which book? e.g. bukhari.".into());
                }
                if fmt == "isnady-book" {
                    return export_isnady_book(conn, &book, &target, &a.languages, say);
                }
                if !books.is_empty() {
                    let keys = book_keys(conn, &books)?;
                    let mut ids = vec![];
                    for k in &keys {
                        ids.extend(book_ids(conn, k, None)?);
                    }
                    let mut names = vec![];
                    for k in keys.iter().take(6) {
                        names.push(collection_name(conn, k)?);
                    }
                    (
                        ids,
                        format!(
                            "isnady: {}{}",
                            names.join(", "),
                            if keys.len() > 6 {
                                format!(" … ({})", keys.len())
                            } else {
                                String::new()
                            }
                        ),
                    )
                } else {
                    let ids = book_ids(conn, &book, a.chapter)?;
                    let mut title = collection_name(conn, &works::resolve(&book))?;
                    if let Some(c) = a.chapter {
                        title.push_str(&format!(" — {c}"));
                    }
                    (ids, title)
                }
            }
            "hadith" => (
                ref_ids(conn, &a.refs)?,
                format!("isnady: {}", a.refs.join(" ")),
            ),
            _ => (
                search_ids(conn, &a.query, a.search.as_ref())?,
                format!("isnady: “{}”", a.query),
            ),
        };
        say(&format!("{} hadith", thousands(ids.len() as i64)));
        if let Some(t) = a.tick {
            t(0, ids.len());
        }
        let mut more = vec![
            ("book", json!(book)),
            ("chapter", json!(a.chapter)),
            ("refs", json!(a.refs)),
            ("query", json!(a.query)),
            ("languages", json!(a.languages)),
        ];
        if kind_ == "search" {
            more.push(("search", search_method(conn, &a.query, a.search.as_ref())));
        }
        if kind_ == "book" && !books.is_empty() {
            more.push(("books", json!(book_keys(conn, &books)?)));
        }
        let meta_ = meta(kind_, more);
        let subtitle = format!("isnady {VERSION} · {}", crate::py::now_local("%Y-%m-%d"));
        if let Some(d) = target.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        if ["html", "md", "txt", "epub", "docx", "pdf"].contains(&fmt) {
            let mut w = Some(F::DocumentWriter::new(fmt, &target, &title, &subtitle)?);
            let mut sections = Sections {
                section: None,
                current: None,
                lang: crate::i18n::ui_language(),
            };
            let run = for_records(conn, &ids, &a.languages, a.chains, say, a.tick, &mut |r| {
                if let Some(s) = sections.push(&r) {
                    w.as_mut().map(|w| w.section(&s)).unwrap_or(Ok(()))?;
                }
                Ok(())
            })
            .and_then(|_| match sections.section.take() {
                Some(s) => w.as_mut().map(|w| w.section(&s)).unwrap_or(Ok(())),
                None => Ok(()),
            });
            return match (run, w.take()) {
                (Ok(()), Some(w)) => w.close(),
                (Err(e), Some(w)) => {
                    w.abort();
                    Err(e)
                }
                (r, None) => r.map(|_| target.clone()),
            };
        }
        let written = (|| -> Res<PathBuf> {
            if fmt == "json" || fmt == "jsonl" {
                let mut f = std::io::BufWriter::new(io(std::fs::File::create(&target))?);
                if fmt == "json" {
                    let mut head = Map::new();
                    head.insert("schema".into(), json!("isnady-export/1"));
                    for (k, v) in &meta_ {
                        head.insert(k.clone(), v.clone());
                    }
                    head.insert("title".into(), json!(title));
                    let head = pyjson::dumps_indent(&Value::Object(head), 1);
                    io(f.write_all(
                        format!("{},\n \"hadith\": [", &head[..head.len() - 2]).as_bytes(),
                    ))?;
                }
                let mut n = 0usize;
                for_records(conn, &ids, &a.languages, a.chains, say, a.tick, &mut |r| {
                    let line = pyjson::dumps(&r);
                    let text = if fmt == "json" {
                        format!("{}\n  {line}", if n > 0 { "," } else { "" })
                    } else {
                        format!("{line}\n")
                    };
                    n += 1;
                    io(f.write_all(text.as_bytes()))
                })?;
                if fmt == "json" {
                    io(f.write_all(b"\n ]\n}\n"))?;
                }
                io(f.flush())?;
                return Ok(target.clone());
            }
            let langs = languages_present(conn, &ids, &languages_of(conn, &a.languages)?)?;
            let cols = hadith_columns(&langs, a.chains);
            if fmt == "csv" || fmt == "tsv" {
                let d = if fmt == "csv" { ',' } else { '\t' };
                let mut f = std::io::BufWriter::new(io(std::fs::File::create(&target))?);
                io(f.write_all(format!("\u{feff}{}", F::csv_line(&cols, d)).as_bytes()))?;
                for_records(conn, &ids, &a.languages, a.chains, say, a.tick, &mut |r| {
                    let row = hadith_row(&r, &langs, a.chains);
                    io(f.write_all(
                        F::csv_line(
                            &cols.iter().map(|c| F::cell(row.get(c))).collect::<Vec<_>>(),
                            d,
                        )
                        .as_bytes(),
                    ))
                })?;
                io(f.flush())?;
                return Ok(target.clone());
            }
            let mut rows = vec![];
            for_records(conn, &ids, &a.languages, a.chains, say, a.tick, &mut |r| {
                rows.push(hadith_row(&r, &langs, a.chains));
                Ok(())
            })?;
            F::write_table(
                &Table {
                    title: title.clone(),
                    columns: cols,
                    rows,
                    meta: meta_.clone(),
                },
                fmt,
                &target,
            )
        })();
        if written.is_err() {
            let _ = std::fs::remove_file(&target);
        }
        return written;
    }
    let write_json = |v: &Value| -> Res<PathBuf> {
        if let Some(d) = target.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        io(std::fs::write(&target, pyjson::dumps_indent(v, 1)))?;
        Ok(target.clone())
    };
    let table = match kind_ {
        "network" => {
            let chosen: Vec<String> = if !books.is_empty() {
                books.clone()
            } else if !book.is_empty() {
                vec![book.clone()]
            } else {
                vec![]
            };
            let (nodes, edges) = network(need()?, &chosen, a.tradition.as_deref())?;
            return F::write_graph(
                &nodes,
                &edges,
                fmt,
                &target,
                &format!(
                    "isnady: {}",
                    if chosen.is_empty() {
                        "all books".to_string()
                    } else {
                        chosen.join(", ")
                    }
                ),
            );
        }
        "narrators" => narrator_table(need()?, &a.query, a.tradition.as_deref())?,
        "bookmarks" => {
            if fmt == "json" {
                let store = crate::bookmarks::Store::load();
                let (folders, items) = store.to_values();
                return write_json(
                    &json!({"schema": "isnady-bookmarks/1", "folders": folders, "items": items}),
                );
            }
            bookmark_table()
        }
        "settings" => {
            if fmt == "json" {
                let mut changed = Map::new();
                for (k, v, d) in config::all_values() {
                    if v != d {
                        changed.insert(k, v);
                    }
                }
                let mut out = Map::new();
                out.insert("schema".into(), json!("isnady-settings/1"));
                for (k, v) in meta("settings", vec![]) {
                    out.insert(k, v);
                }
                out.insert("settings".into(), Value::Object(changed));
                return write_json(&Value::Object(out));
            }
            settings_table()
        }
        "profile" => return export_profile(fmt, &target, profile.as_deref()),
        "backup" => return export_backup(conn, &target),
        "statistics" => {
            return export_statistics(
                need()?,
                &target,
                if book.is_empty() { None } else { Some(&book) },
                a.tradition.as_deref(),
                say,
            )
        }
        other => return Err(other.to_string()),
    };
    F::write_table(&table, fmt, &target)
}

fn export_statistics(
    conn: &Connection,
    target: &Path,
    book: Option<&str>,
    tradition: Option<&str>,
    say: Say,
) -> Res<PathBuf> {
    use crate::stats;
    let scope: Option<String> = match book {
        Some(b) => Some(works::resolve(b)),
        None if tradition == Some("shia") => Some(stats::SHIA_SCOPE.to_string()),
        None => None,
    };
    let sc = scope.as_deref();
    say(&format!(
        "statistics of {}",
        sc.unwrap_or("every Sunni book")
    ));
    let label = sc.unwrap_or("all sunni books").to_string();
    let e = |x: stats::corpus::CachedError| x.to_string();
    let mut out = Map::new();
    out.insert("scope".into(), json!(label));
    out.insert(
        "corpus".into(),
        stats::corpus::cached(conn, sc, say, false, false).map_err(e)?,
    );
    out.insert(
        "models".into(),
        stats::models::cached(conn, sc, say, false, false).map_err(e)?,
    );
    out.insert(
        "lab".into(),
        stats::lab::cached(conn, sc, say, false, false).map_err(e)?,
    );
    if let (Some(_), Some(key)) = (book, sc) {
        if db(stats::graders::books_with_grades(conn))?
            .iter()
            .any(|b| b.0 == key)
        {
            out.insert(
                "graders".into(),
                stats::graders::cached(conn, key, say, false, false).map_err(e)?,
            );
        }
    }
    let mut body = Map::new();
    body.insert("schema".into(), json!("isnady-statistics/1"));
    for (k, v) in meta("statistics", vec![("scope", json!(label))]) {
        body.insert(k, v);
    }
    body.insert("results".into(), Value::Object(out));
    if let Some(d) = target.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    io(std::fs::write(
        target,
        pyjson::dumps_indent(&Value::Object(body), 1),
    ))?;
    Ok(target.to_path_buf())
}

fn zput(z: &mut zip::ZipWriter<std::fs::File>, name: &str, data: &[u8]) -> Res<()> {
    z.start_file(
        name,
        zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .large_file(data.len() > 0x7fff_0000),
    )
    .map_err(|e| e.to_string())?;
    io(z.write_all(data))
}

/// The book in isnady-data's own format (isnady-book/1), zipped: imported again as a book (`export_isnady_book`).
fn export_isnady_book(
    conn: &Connection,
    book: &str,
    target: &Path,
    languages: &[String],
    say: Say,
) -> Res<PathBuf> {
    let key = works::resolve(book);
    let row: Option<(i64, String, String)> = db(conn
        .query_row(
            "SELECT id, name, COALESCE(tradition, 'sunni') FROM collections WHERE key = ?",
            [&key],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional())?;
    let Some((cid, title, road)) = row else {
        return Err(format!("No book '{book}'."));
    };
    #[allow(clippy::type_complexity)]
    let editions: Vec<(i64, Option<String>, Option<String>, Option<String>)> = {
        let mut st = db(conn.prepare("SELECT e.id, e.language, e.direction, e.author FROM editions e WHERE e.collection_id = ? AND e.text_count > 0 ORDER BY e.id"))?;
        let v = db(
            db(st.query_map([cid], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))))?
                .collect(),
        )?;
        v
    };
    let mut chosen: Vec<(String, i64, Option<String>, Option<String>)> = vec![];
    for (eid, lang, direction, author) in editions {
        if let Some(lang) = lang.filter(|l| !l.is_empty()) {
            if !chosen.iter().any(|c| c.0 == lang)
                && (languages.is_empty() || languages.contains(&lang))
            {
                chosen.push((lang, eid, direction, author));
            }
        }
    }
    if chosen.is_empty() {
        return Err(format!("'{book}' has no texts in the languages chosen."));
    }
    let mut hadiths: Vec<(i64, String, Value)> = vec![];
    {
        let mut st = db(conn.prepare("SELECT id, number, number_sort, section_number FROM hadiths WHERE collection_id = ? ORDER BY number_sort, id"))?;
        let rows: Vec<(i64, String, Option<f64>, Option<i64>)> = db(db(
            st.query_map([cid], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
        )?
        .collect())?;
        let mut refs_st =
            db(conn.prepare("SELECT scheme, value FROM hadith_refs WHERE hadith_id = ?"))?;
        let mut grades_st =
            db(conn.prepare("SELECT grader_name, grade FROM grades WHERE hadith_id = ?"))?;
        for (hid, number, sort, section) in rows {
            let mut refs = Map::new();
            for r in db(refs_st.query_map([hid], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            }))? {
                let (k, v) = db(r)?;
                refs.insert(k, json!(v));
            }
            let grades: Vec<Value> = db(db(grades_st.query_map([hid], |r| {
                Ok(json!([
                    r.get::<_, Option<String>>(0)?,
                    r.get::<_, Option<String>>(1)?
                ]))
            }))?
            .collect())?;
            let mut h = Map::new();
            h.insert("n".into(), json!(number));
            h.insert("sort".into(), json!(sort));
            h.insert("s".into(), json!(section));
            h.insert("refs".into(), Value::Object(refs));
            h.insert("grades".into(), Value::Array(grades));
            hadiths.push((hid, number, Value::Object(h)));
        }
    }
    let mut langs_meta = Map::new();
    let mut files: Vec<(String, Value)> = vec![];
    for (lang, eid, direction, author) in &chosen {
        let code = crate::sources::catalog::code_for(lang);
        say(&format!("{title}: {lang}"));
        let mut texts = Map::new();
        {
            let mut st =
                db(conn.prepare("SELECT text FROM texts WHERE hadith_id = ? AND edition_id = ?"))?;
            for (hid, number, _) in &hadiths {
                if let Some(t) = db(st
                    .query_row(params![hid, eid], |r| r.get::<_, String>(0))
                    .optional())?
                {
                    texts.insert(number.clone(), json!(t));
                }
            }
        }
        let mut sections = Map::new();
        {
            let mut st = db(conn.prepare("SELECT s.number, st.title FROM section_titles st JOIN sections s ON s.id = st.section_id WHERE s.collection_id = ? AND st.edition_id = ?"))?;
            for r in db(st.query_map(params![cid, eid], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, Option<String>>(1)?))
            }))? {
                let (n, t) = db(r)?;
                sections.insert(n.to_string(), json!(t));
            }
        }
        let mut f = Map::new();
        f.insert("language".into(), json!(code));
        f.insert("sections".into(), Value::Object(sections));
        f.insert("texts".into(), Value::Object(texts));
        files.push((format!("{code}.json"), Value::Object(f)));
        let mut m = Map::new();
        m.insert("file".into(), json!(format!("{code}.json")));
        m.insert("name".into(), json!(lang));
        m.insert(
            "direction".into(),
            json!(direction
                .clone()
                .filter(|d| !d.is_empty())
                .unwrap_or_else(|| "ltr".into())),
        );
        m.insert("author".into(), json!(author.clone().unwrap_or_default()));
        langs_meta.insert(code, Value::Object(m));
    }
    let license: Option<Option<String>> = db(conn
        .query_row("SELECT s.license FROM sources s JOIN editions e ON e.source_id = s.id WHERE e.collection_id = ? LIMIT 1", [cid], |r| r.get(0))
        .optional())?;
    let mut b = Map::new();
    b.insert("schema".into(), json!("isnady-book/1"));
    b.insert("key".into(), json!(key));
    b.insert("title".into(), json!(title));
    b.insert("tradition".into(), json!(road));
    b.insert("license".into(), json!(license.flatten()));
    b.insert(
        "credit".into(),
        json!(format!("Exported by isnady {VERSION}")),
    );
    b.insert("languages".into(), Value::Object(langs_meta));
    b.insert(
        "hadiths".into(),
        Value::Array(hadiths.into_iter().map(|h| h.2).collect()),
    );
    if let Some(d) = target.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let mut z = zip::ZipWriter::new(io(std::fs::File::create(target))?);
    zput(
        &mut z,
        &format!("{key}/book.json"),
        pyjson::dumps(&Value::Object(b)).as_bytes(),
    )?;
    for (name, data) in &files {
        zput(
            &mut z,
            &format!("{key}/{name}"),
            pyjson::dumps(data).as_bytes(),
        )?;
    }
    z.finish().map_err(|e| e.to_string())?;
    Ok(target.to_path_buf())
}

fn name_or_default(pid: &str) -> String {
    let n = config::profile_name(pid);
    if n.is_empty() {
        "default".into()
    } else {
        n
    }
}

fn export_profile(fmt: &str, target: &Path, profile: Option<&str>) -> Res<PathBuf> {
    let pid = match profile {
        Some(p) => config::find_profile(p).ok_or_else(|| format!("No profile '{p}'."))?,
        None => config::active_profile(),
    };
    if fmt == "isnady-profile" {
        return config::export_profile(&pid, &target.to_string_lossy());
    }
    let folder = config::folder_of(&pid);
    let mut bundle = Map::new();
    bundle.insert("schema".into(), json!("isnady-profile-json/1"));
    for (k, v) in meta("profile", vec![]) {
        bundle.insert(k, v);
    }
    bundle.insert("name".into(), json!(name_or_default(&pid)));
    for name in FILES {
        let v = std::fs::read_to_string(folder.join(name))
            .ok()
            .and_then(|t| serde_json::from_str::<Value>(&t).ok())
            .unwrap_or_else(|| json!({}));
        bundle.insert(name.split('.').next().unwrap_or(name).into(), v);
    }
    if let Some(d) = target.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    io(std::fs::write(
        target,
        pyjson::dumps_indent(&Value::Object(bundle), 1),
    ))?;
    Ok(target.to_path_buf())
}

/// Everything of mine: this profile's settings, place and bookmarks, and the list of the books imported
/// (`export_backup`).
fn export_backup(conn: Option<&Connection>, target: &Path) -> Res<PathBuf> {
    let folder = config::profile_folder();
    let mut sources = vec![];
    if let Some(conn) = conn {
        let mut st = db(conn.prepare(
            "SELECT key, name, format, location, license, origin FROM sources ORDER BY id",
        ))?;
        let rows = db(st.query_map([], |r| {
            Ok(json!({"key": r.get::<_, Option<String>>(0)?, "name": r.get::<_, Option<String>>(1)?, "format": r.get::<_, Option<String>>(2)?,
                      "location": r.get::<_, Option<String>>(3)?, "license": r.get::<_, Option<String>>(4)?, "origin": r.get::<_, Option<String>>(5)?}))
        }))?;
        for r in rows {
            sources.push(db(r)?);
        }
    }
    if let Some(d) = target.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let mut body = Map::new();
    body.insert("schema".into(), json!("isnady-backup/1"));
    for (k, v) in meta("backup", vec![]) {
        body.insert(k, v);
    }
    body.insert(
        "profile".into(),
        json!(name_or_default(&config::active_profile())),
    );
    body.insert("sources".into(), Value::Array(sources));
    let mut z = zip::ZipWriter::new(io(std::fs::File::create(target))?);
    zput(
        &mut z,
        "backup.json",
        pyjson::dumps_indent(&Value::Object(body), 1).as_bytes(),
    )?;
    for name in FILES {
        if let Ok(data) = std::fs::read(folder.join(name)) {
            zput(&mut z, name, &data)?;
        }
    }
    z.finish().map_err(|e| e.to_string())?;
    Ok(target.to_path_buf())
}

// ================================================================== import
fn file_name(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default()
}

fn read_text(path: &Path) -> Res<String> {
    let bytes = io(std::fs::read(path))?;
    let text = String::from_utf8(bytes).map_err(|e| e.to_string())?;
    Ok(text.strip_prefix('\u{feff}').unwrap_or(&text).to_string())
}

fn bad(path: &Path, fmt: &str, why: impl std::fmt::Display) -> String {
    format!("{}: not readable as {fmt} ({why}).", file_name(path))
}

/// The records of a JSON (an export's "rows"/"hadith" or a plain list), JSON Lines, CSV or TSV file (`_rows`).
fn rows_of(path: &Path, fmt: &str) -> Res<Vec<Value>> {
    let text = read_text(path)?;
    match fmt {
        "jsonl" => text
            .lines()
            .filter(|l| !crate::py::trim(l).is_empty())
            .map(|l| serde_json::from_str(l).map_err(|e| bad(path, fmt, e)))
            .collect(),
        "json" => {
            let data: Value = serde_json::from_str(&text).map_err(|e| bad(path, fmt, e))?;
            if let Value::Array(a) = data {
                return Ok(a);
            }
            for key in ["rows", "hadith", "items"] {
                if let Some(Value::Array(a)) = data.get(key) {
                    return Ok(a.clone());
                }
            }
            Err(format!("{}: no list of records in it.", file_name(path)))
        }
        "csv" | "tsv" => {
            let rows = crate::sources::importers::table::csv_rows_with(
                &text,
                if fmt == "tsv" { '\t' } else { ',' },
            );
            let mut it = rows.into_iter().filter(|r| !r.is_empty());
            let Some(head) = it.next() else {
                return Ok(vec![]);
            };
            Ok(it
                .map(|r| {
                    let mut m = Map::new();
                    for (i, h) in head.iter().enumerate() {
                        m.insert(h.clone(), r.get(i).map(|c| json!(c)).unwrap_or(Value::Null));
                    }
                    Value::Object(m)
                })
                .collect())
        }
        _ => Err(format!("{}: {fmt} cannot be read.", file_name(path))),
    }
}

/// What an import is asked (the arguments of `import_file`).
#[derive(Default)]
pub struct Import {
    pub kind: String,
    pub fmt: String,
    pub replace: bool,
    pub folder: String,
    pub name: String,
}

/// Bring a file in; a sentence saying what came in (`import_file`).
pub fn import_file(conn: Option<&Connection>, path: &str, a: &Import, say: Say) -> Res<String> {
    let path = crate::paths::expand_user(path);
    if !path.exists() {
        return Err(format!("No file {}.", path.display()));
    }
    let fmt = if a.fmt.is_empty() {
        guess_format(&path, &a.kind)
    } else {
        a.fmt.clone()
    };
    let importable: Vec<&str> = KINDS
        .iter()
        .filter(|k| !k.3.is_empty())
        .map(|k| k.0)
        .collect();
    let Some(spec) = kind(&a.kind) else {
        return Err(format!(
            "Nothing called '{}' is imported. One of: {}.",
            a.kind,
            importable.join(", ")
        ));
    };
    if !spec.3.contains(&fmt.as_str()) {
        return Err(format!(
            "{} is imported from {}; not {fmt}.",
            spec.1,
            if spec.3.is_empty() {
                "nothing".to_string()
            } else {
                spec.3.join(", ")
            }
        ));
    }
    match a.kind.as_str() {
        "bookmarks" => import_bookmarks(&path, &fmt, a.replace, &a.folder),
        "settings" => import_settings(&path, &fmt, a.replace),
        "profile" => import_profile(&path, &fmt, &a.name),
        "backup" => import_backup(&path),
        "book" => import_book(conn.ok_or("No database.")?, &path, &fmt, say),
        other => Err(other.to_string()),
    }
}

fn text_of(v: Option<&Value>) -> String {
    match v {
        None | Some(Value::Null) => String::new(),
        Some(Value::Bool(false)) => String::new(),
        Some(Value::Number(n)) if n.as_f64() == Some(0.0) => String::new(),
        Some(x) => config::py_str(x),
    }
}

fn import_bookmarks(path: &Path, fmt: &str, replace: bool, folder: &str) -> Res<String> {
    use crate::bookmarks::{Store, KINDS as BK};
    let mut store = if replace {
        Store::default()
    } else {
        Store::load()
    };
    if fmt == "json" {
        let data: Value = serde_json::from_str(&read_text(path)?).map_err(|e| bad(path, fmt, e))?;
        if data
            .get("schema")
            .map(config::py_str)
            .is_some_and(|s| s.starts_with("isnady-bookmarks"))
        {
            let mut incoming = Store::from_value(&data);
            if !folder.is_empty() {
                if let Some(top) = store.ensure_path(folder) {
                    for f in incoming.folders.iter_mut() {
                        if f.parent.is_none() {
                            f.parent = Some(top.id.clone());
                        }
                    }
                    for i in incoming.items.iter_mut() {
                        if i.folders.is_empty() {
                            i.folders = vec![top.id.clone()];
                        }
                    }
                }
            }
            let (added, folders) = store.merge(&incoming);
            io(store.save())?;
            return Ok(format!(
                "{added} bookmarks and {folders} folders added (the ones already there kept)."
            ));
        }
    }
    let mut added = 0;
    for r in rows_of(path, fmt)? {
        let kind_ = crate::py::trim(&text_of(r.get("kind"))).to_string();
        let reference = crate::py::trim(&text_of(r.get("ref"))).to_string();
        if !BK.contains(&kind_.as_str()) || reference.is_empty() {
            continue;
        }
        let id = match store.find(&kind_, &reference) {
            Some(i) => i.id.clone(),
            None => {
                let title = Some(text_of(r.get("title")))
                    .filter(|t| !t.is_empty())
                    .unwrap_or_else(|| reference.clone());
                store
                    .add(
                        &kind_,
                        &reference,
                        &title,
                        &text_of(r.get("subtitle")),
                        None,
                        "",
                    )
                    .id
            }
        };
        let note = text_of(r.get("note"));
        if !note.is_empty() {
            store.set_note(&id, &note);
        }
        let mut paths: Vec<String> = match r.get("folders") {
            Some(Value::Array(a)) => a.iter().map(config::py_str).collect(),
            other => text_of(other)
                .split('|')
                .map(|p| crate::py::trim(p).to_string())
                .filter(|p| !p.is_empty())
                .collect(),
        };
        if !folder.is_empty() {
            paths.push(folder.to_string());
        }
        for p in paths {
            if let Some(f) = store.ensure_path(&p) {
                store.file_in(&id, &f.id);
            }
        }
        added += 1;
    }
    io(store.save())?;
    Ok(format!("{added} bookmarks imported."))
}

fn import_settings(path: &Path, fmt: &str, replace: bool) -> Res<String> {
    let mut values: Vec<(String, Value)> = vec![];
    if fmt == "json" {
        let data: Value = serde_json::from_str(&read_text(path)?).map_err(|e| bad(path, fmt, e))?;
        let inner = match &data {
            Value::Object(o) => o.get("settings").cloned().unwrap_or(data.clone()),
            _ => json!({}),
        };
        if let Some(rows) = inner.get("rows").and_then(Value::as_array) {
            for r in rows {
                let (Some(k), Some(v)) = (r.get("key"), r.get("value")) else {
                    return Err(bad(path, fmt, "'key'"));
                };
                values.push((config::py_str(k), v.clone()));
            }
        } else if let Value::Object(o) = inner {
            values = o.into_iter().collect();
        }
    } else {
        for r in rows_of(path, fmt)? {
            let key = text_of(r.get("key"));
            let changed = r
                .get("changed")
                .map(config::py_str)
                .unwrap_or_else(|| "True".into())
                .to_lowercase();
            if !key.is_empty() && ["true", "1", "yes"].contains(&changed.as_str()) {
                values.push((key, r.get("value").cloned().unwrap_or(Value::Null)));
            }
        }
    }
    if replace {
        config::reset("")?;
    }
    let (mut done, mut skipped) = (0, vec![]);
    for (key, value) in values {
        if ["schema", "kind", "exported", "isnady", "columns", "title"].contains(&key.as_str()) {
            continue;
        }
        match config::set(&key, value) {
            Ok(()) => done += 1,
            Err(_) => skipped.push(key),
        }
    }
    Ok(format!(
        "{done} settings imported{}.",
        if skipped.is_empty() {
            String::new()
        } else {
            format!(
                "; not taken: {}",
                skipped
                    .iter()
                    .take(8)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    ))
}

fn import_profile(path: &Path, fmt: &str, name: &str) -> Res<String> {
    if fmt == "isnady-profile" {
        let pid = config::import_profile(
            &path.to_string_lossy(),
            if name.is_empty() { None } else { Some(name) },
        )?;
        return Ok(format!(
            "Imported as the profile {}.",
            config::profile_name(&pid)
        ));
    }
    let data: Value = serde_json::from_str(&read_text(path)?).map_err(|e| bad(path, fmt, e))?;
    if data.get("schema").and_then(Value::as_str) != Some("isnady-profile-json/1") {
        return Err(format!("{} is not an isnady profile.", file_name(path)));
    }
    let wanted = Some(name.to_string())
        .filter(|n| !n.is_empty())
        .or_else(|| {
            data.get("name")
                .and_then(Value::as_str)
                .filter(|n| !n.is_empty())
                .map(str::to_string)
        })
        .unwrap_or_else(|| "Imported".into());
    let wanted = crate::py::trim(&wanted).to_string();
    let (mut fin, mut n) = (wanted.clone(), 2);
    while config::find_profile(&fin).is_some() {
        fin = format!("{wanted} ({n})");
        n += 1;
    }
    let pid = config::create_profile(&fin, None)?;
    let folder = config::folder_of(&pid);
    for file in FILES {
        if let Some(part) = data
            .get(file.split('.').next().unwrap_or(file))
            .filter(|p| crate::sources::importers::truthy(p))
        {
            io(std::fs::write(
                folder.join(file),
                pyjson::dumps_indent(part, 1),
            ))?;
        }
    }
    Ok(format!("Imported as the profile {fin}."))
}

fn import_backup(path: &Path) -> Res<String> {
    let folder = config::profile_folder();
    let mut z =
        zip::ZipArchive::new(io(std::fs::File::open(path))?).map_err(|e| bad(path, "backup", e))?;
    let mut read = |name: &str| -> Option<Vec<u8>> {
        let mut f = z.by_name(name).ok()?;
        let mut out = vec![];
        f.read_to_end(&mut out).ok()?;
        Some(out)
    };
    let raw = read("backup.json").ok_or_else(|| {
        bad(
            path,
            "backup",
            "\"There is no item named 'backup.json' in the archive\"",
        )
    })?;
    let manifest: Value = serde_json::from_slice(&raw).map_err(|e| bad(path, "backup", e))?;
    if manifest.get("schema").and_then(Value::as_str) != Some("isnady-backup/1") {
        return Err(format!("{} is not an isnady backup.", file_name(path)));
    }
    let mut restored = vec![];
    for name in FILES {
        if let Some(raw) = read(name) {
            serde_json::from_slice::<Value>(&raw).map_err(|e| bad(path, "backup", e))?;
            let _ = std::fs::create_dir_all(&folder);
            io(std::fs::write(folder.join(name), raw))?;
            restored.push(name.split('.').next().unwrap_or(name));
        }
    }
    let books = manifest
        .get("sources")
        .and_then(Value::as_array)
        .map(|a| a.len())
        .unwrap_or(0);
    Ok(format!(
        "Restored: {}. The backup lists {books} sources; import the books again with File → Data Sources (or iy catalog import --all).",
        if restored.is_empty() { "nothing".to_string() } else { restored.join(", ") }
    ))
}

type Files = Vec<(String, Value)>;

/// An export (json, jsonl, csv, tsv) turned back into an isnady book (`_book_from_records`).
fn book_from_records(rows: &[Value], path: &Path) -> Res<(Value, Files)> {
    let Some(first) = rows.first() else {
        return Err(format!("{} has no hadith.", file_name(path)));
    };
    let stem = file_name(path);
    let stem = stem
        .rsplit_once('.')
        .map(|x| x.0)
        .unwrap_or(&stem)
        .split('.')
        .next()
        .unwrap_or("")
        .to_string();
    let named = Some(text_of(first.get("book")))
        .filter(|b| !b.is_empty())
        .unwrap_or(stem)
        .to_lowercase();
    let key = pre!(r"\W+").sub(&named, "_");
    let key = key.trim_matches('_');
    let key = if key.is_empty() {
        "imported".to_string()
    } else {
        key.to_string()
    };
    let title = Some(text_of(first.get("book_title")))
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| key.clone());
    // (code, name, texts, sections)
    #[allow(clippy::type_complexity)]
    let mut langs: Vec<(String, String, Map<String, Value>, Map<String, Value>)> = vec![];
    let mut hadiths = vec![];
    for r in rows {
        let number = crate::py::trim(&text_of(r.get("number"))).to_string();
        if number.is_empty() {
            continue;
        }
        let section: Option<i64> = match r.get("chapter_number") {
            Some(Value::Number(n)) => n.as_i64().or_else(|| n.as_f64().map(|f| f.trunc() as i64)),
            Some(Value::String(s)) if !s.is_empty() => crate::py::trim(s).parse().ok(),
            _ => None,
        };
        let sort = crate::sources::importers::table::py_float(&number);
        let grades: Vec<Value> = match r.get("grades") {
            Some(Value::String(s)) => s
                .split(';')
                .filter_map(|g| g.split_once(':'))
                .map(|(g, v)| json!([crate::py::trim(g), crate::py::trim(v)]))
                .collect(),
            Some(Value::Array(a)) => a
                .iter()
                .map(|g| {
                    json!([
                        crate::py::trim(&F::cell(g.get(0))),
                        crate::py::trim(&F::cell(g.get(1)))
                    ])
                })
                .collect(),
            _ => vec![],
        };
        hadiths.push(json!({"n": number, "sort": sort, "s": section, "grades": grades}));
        let texts: Vec<(String, Value)> = match r.get("texts") {
            Some(Value::Object(o)) => o.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            _ => r
                .as_object()
                .into_iter()
                .flatten()
                .filter(|(k, v)| k.starts_with("text_") && crate::sources::importers::truthy(v))
                .map(|(k, v)| (k[5..].to_string(), v.clone()))
                .collect(),
        };
        let chapters = r.get("chapter").filter(|c| c.is_object());
        for (lang, text) in texts {
            let code = crate::sources::catalog::code_for(&lang);
            let i = match langs.iter().position(|l| l.0 == code) {
                Some(i) => i,
                None => {
                    langs.push((code.clone(), lang.clone(), Map::new(), Map::new()));
                    langs.len() - 1
                }
            };
            langs[i].2.insert(number.clone(), text);
            if let (Some(sec), Some(t)) = (
                section,
                chapters
                    .and_then(|c| c.get(&lang))
                    .filter(|t| crate::sources::importers::truthy(t)),
            ) {
                langs[i].3.insert(sec.to_string(), t.clone());
            }
        }
    }
    let mut lm = Map::new();
    for (code, name, texts, _) in &langs {
        let first_text = texts.values().next().and_then(Value::as_str).unwrap_or("");
        let mut m = Map::new();
        m.insert("file".into(), json!(format!("{code}.json")));
        m.insert("name".into(), json!(name));
        m.insert(
            "direction".into(),
            json!(if F::rtl(first_text) { "rtl" } else { "ltr" }),
        );
        lm.insert(code.clone(), Value::Object(m));
    }
    let mut book = Map::new();
    book.insert("schema".into(), json!("isnady-book/1"));
    book.insert("key".into(), json!(key));
    book.insert("title".into(), json!(title));
    book.insert(
        "tradition".into(),
        json!(Some(text_of(first.get("tradition")))
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| "sunni".into())),
    );
    book.insert(
        "credit".into(),
        json!(format!("Imported from {}", file_name(path))),
    );
    book.insert("languages".into(), Value::Object(lm));
    book.insert("hadiths".into(), Value::Array(hadiths));
    let files = langs
        .into_iter()
        .map(|(code, _n, texts, sections)| {
            let mut f = Map::new();
            f.insert("language".into(), json!(code));
            f.insert("sections".into(), Value::Object(sections));
            f.insert("texts".into(), Value::Object(texts));
            (format!("{code}.json"), Value::Object(f))
        })
        .collect();
    Ok((Value::Object(book), files))
}

/// A book back into isnady, as a source of your own (`import_book`).
fn import_book(conn: &Connection, path: &Path, fmt: &str, say: Say) -> Res<String> {
    let dir = crate::paths::data_dir().join("imported-books");
    let book_json: PathBuf;
    if fmt == "isnady-book" {
        if path.is_dir() {
            book_json = path.join("book.json");
        } else if file_name(path).ends_with("book.json") {
            book_json = path.to_path_buf();
        } else {
            let mut z = zip::ZipArchive::new(io(std::fs::File::open(path))?)
                .map_err(|e| bad(path, fmt, e))?;
            let names: Vec<String> = z.file_names().map(str::to_string).collect();
            let mut inside: Vec<&String> =
                names.iter().filter(|n| n.ends_with("book.json")).collect();
            inside.sort_by_key(|n| names.iter().position(|x| x == *n));
            let Some(first) = (0..z.len())
                .filter_map(|i| z.by_index(i).ok().map(|f| f.name().to_string()))
                .find(|n| n.ends_with("book.json"))
            else {
                return Err(format!("{} holds no book.json.", file_name(path)));
            };
            let mut raw = String::new();
            io(z.by_name(&first)
                .map_err(|e| bad(path, fmt, e))?
                .read_to_string(&mut raw))?;
            let meta_: Value = serde_json::from_str(&raw).map_err(|e| bad(path, fmt, e))?;
            let stem = file_name(path);
            let stem = stem
                .rsplit_once('.')
                .map(|x| x.0.to_string())
                .unwrap_or(stem);
            let key = meta_
                .get("key")
                .and_then(Value::as_str)
                .filter(|k| !k.is_empty())
                .map(str::to_string)
                .unwrap_or(stem);
            let target = dir.join(pre!(r"\W+").sub(&key, "_"));
            io(std::fs::create_dir_all(&target))?;
            let base = match first.rsplit_once('/') {
                Some((b, _)) => format!("{b}/"),
                None => String::new(),
            };
            for i in 0..z.len() {
                let mut f = z.by_index(i).map_err(|e| bad(path, fmt, e))?;
                let n = f.name().to_string();
                if n.starts_with(&base) && !n.ends_with('/') && !n.contains("..") {
                    let out = target.join(&n[base.len()..]);
                    if let Some(d) = out.parent() {
                        io(std::fs::create_dir_all(d))?;
                    }
                    let mut data = vec![];
                    io(f.read_to_end(&mut data))?;
                    io(std::fs::write(out, data))?;
                }
            }
            book_json = target.join("book.json");
        }
    } else {
        let mut groups: Vec<(String, Vec<Value>)> = vec![];
        for r in rows_of(path, fmt)? {
            let b = text_of(r.get("book"));
            match groups.iter_mut().find(|g| g.0 == b) {
                Some(g) => g.1.push(r),
                None => groups.push((b, vec![r])),
            }
        }
        let mut said = vec![];
        for (_b, rows) in &groups {
            let (book, files) = book_from_records(rows, path)?;
            let target = dir.join(book["key"].as_str().unwrap_or("imported"));
            io(std::fs::create_dir_all(&target))?;
            io(std::fs::write(
                target.join("book.json"),
                pyjson::dumps(&book),
            ))?;
            for (name, data) in &files {
                io(std::fs::write(target.join(name), pyjson::dumps(data)))?;
            }
            said.push(import_book_json(conn, &target.join("book.json"), say)?);
        }
        return Ok(said.join(" "));
    }
    import_book_json(conn, &book_json, say)
}

fn import_book_json(conn: &Connection, book_json: &Path, say: Say) -> Res<String> {
    use crate::sources::imports::{run_import, ImportRequest};
    let text = io(std::fs::read_to_string(book_json))
        .map_err(|e| format!("{}: {e}", book_json.display()))?;
    let meta_: Value = serde_json::from_str(&text).map_err(|e| bad(book_json, "isnady-book", e))?;
    let mut options = Map::new();
    options.insert(
        "languages".into(),
        json!(meta_
            .get("languages")
            .and_then(Value::as_object)
            .map(|o| o.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default()),
    );
    let request = ImportRequest {
        format_id: "isnady".into(),
        location: book_json.to_string_lossy().to_string(),
        name: meta_
            .get("title")
            .and_then(Value::as_str)
            .map(str::to_string),
        license: meta_
            .get("license")
            .and_then(Value::as_str)
            .map(str::to_string),
        tier: None,
        origin: "user".into(),
        auth: Default::default(),
        options,
    };
    let (report, _n) = run_import(conn, &request, say).map_err(|e| e.to_string())?;
    Ok(format!(
        "{}: {} hadith, {} texts imported.",
        meta_
            .get("title")
            .map(config::py_str)
            .unwrap_or_else(|| "None".into()),
        thousands(report.new_hadiths),
        thousands(report.texts)
    ))
}
