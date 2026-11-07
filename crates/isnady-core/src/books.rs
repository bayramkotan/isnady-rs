//! The imported hadith collections in reading order, and which are active — as the Python isnady's
//! `isnady/core/books.py`: the six books, the Muwatta, the Musnad and al-Darimi first (in their classical order), then
//! the others by name read without its leading apostrophe or article; Shia collections after all Sunni ones, al-Kafi
//! first. A book switched off (`books.disabled`) stays in the database and is left out.

use regex::Regex;
use rusqlite::Connection;
use std::collections::HashSet;
use std::sync::OnceLock;

use crate::config;

const TRADITIONS: [&str; 3] = ["sunni", "shia", "other"];
const CANON_SUNNI: [&str; 9] = [
    "bukhari", "muslim", "abudawud", "tirmidhi", "nasai", "ibnmajah", "malik", "ahmad", "darimi",
];
const CANON_SHIA: [&str; 4] = ["kafi", "faqih", "tahdhib", "istibsar"];

#[derive(Debug, Clone)]
pub struct Book {
    pub key: String,
    pub name: String,
    pub tradition: String,
    pub count: i64,
    pub active: bool,
}

fn re(pattern: &str) -> Regex {
    Regex::new(pattern).expect("pattern")
}

/// A title as it is alphabetised: no leading quote marks or 'ayn signs, no article (al-, an-, as- …).
pub fn sort_name(name: &str) -> String {
    static LEAD: OnceLock<Regex> = OnceLock::new();
    static ART: OnceLock<Regex> = OnceLock::new();
    let lead = LEAD.get_or_init(|| re(r#"^[\s'‘’`ʿʾ"]+"#));
    let art = ART.get_or_init(|| re(r"^(al|an|as|ash|at|ad|adh|ar|az|az-z)[- ]"));
    let n = lead.replace(name, "").to_lowercase();
    let n = art.replace(&n, "").to_string();
    lead.replace(&n, "").to_string()
}

fn sort_key(b: &Book) -> (usize, usize, String, String) {
    let tradition = if b.tradition == "sunni" || b.tradition == "shia" {
        b.tradition.as_str()
    } else {
        "other"
    };
    let order = TRADITIONS.iter().position(|t| *t == tradition).unwrap_or(2);
    let canon: &[&str] = match tradition {
        "sunni" => &CANON_SUNNI,
        "shia" => &CANON_SHIA,
        _ => &[],
    };
    let base = b.key.split('-').next().unwrap_or("");
    let rank = canon.iter().position(|k| *k == base).unwrap_or(canon.len());
    (order, rank, sort_name(&b.name), b.key.clone())
}

pub fn disabled() -> HashSet<String> {
    config::get_str("books.disabled", "")
        .split(',')
        .filter(|k| !k.is_empty())
        .map(String::from)
        .collect()
}

/// Every collection that has texts, in reading order; the inactive ones too when asked.
pub fn all_books(conn: &Connection, include_inactive: bool) -> rusqlite::Result<Vec<Book>> {
    let off = disabled();
    let mut stmt = conn.prepare(
        "SELECT c.key, c.name, c.tradition, COUNT(DISTINCT h.id) FROM collections c
         JOIN hadiths h ON h.collection_id = c.id
         WHERE EXISTS (SELECT 1 FROM texts t WHERE t.hadith_id = h.id)
         GROUP BY c.id",
    )?;
    let mut rows: Vec<Book> = stmt
        .query_map([], |r| {
            let key: String = r.get(0)?;
            let tradition: Option<String> = r.get(2)?;
            Ok(Book {
                active: !off.contains(&key),
                key,
                name: r.get(1)?,
                tradition: tradition
                    .filter(|t| !t.is_empty())
                    .unwrap_or_else(|| "sunni".into()),
                count: r.get(3)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    rows.sort_by_key(sort_key);
    if !include_inactive {
        rows.retain(|b| b.active);
    }
    Ok(rows)
}

/// An SQL expression that sorts by the reading order of the books.
pub fn order_sql(conn: &Connection, column: &str) -> rusqlite::Result<String> {
    let keys: Vec<String> = all_books(conn, true)?.into_iter().map(|b| b.key).collect();
    if keys.is_empty() {
        return Ok(column.to_string());
    }
    let whens: Vec<String> = keys
        .iter()
        .enumerate()
        .map(|(i, k)| format!("WHEN '{}' THEN {}", k.replace('\'', "''"), i))
        .collect();
    Ok(format!(
        "CASE {} {} ELSE {} END",
        column,
        whens.join(" "),
        keys.len()
    ))
}

/// The keys to search when "all books" is asked for: None when every book is active.
pub fn active_keys(conn: &Connection) -> rusqlite::Result<Option<Vec<String>>> {
    if disabled().is_empty() {
        return Ok(None);
    }
    Ok(Some(
        all_books(conn, false)?.into_iter().map(|b| b.key).collect(),
    ))
}

/// The languages after Arabic in the order chosen (view.language_order); empty means A to Z.
pub fn language_order() -> Vec<String> {
    config::get_str("view.language_order", "")
        .split(',')
        .filter(|p| !p.is_empty() && *p != "Arabic")
        .map(String::from)
        .collect()
}

/// The sort key of a language: Arabic first, then those in the order chosen, then the rest A to Z.
pub fn language_rank(language: &str, chosen: &[String]) -> (u8, usize, String) {
    if language == "Arabic" {
        return (0, 0, String::new());
    }
    if let Some(i) = chosen.iter().position(|c| c == language) {
        return (1, i, String::new());
    }
    (2, 0, language.to_string())
}

/// The languages the imported books have in the data repository's catalog but not on this computer
/// (`search_page._more_languages`): the catalog the Python isnady saved (repos/<repository>.json), its entries and
/// its library, each book's languages by their names (`catalog.LANGUAGE_NAMES`, given here).
pub fn more_languages(
    conn: &Connection,
    imported: &[String],
    names: &std::collections::HashMap<String, String>,
) -> Vec<String> {
    let repo = config::get_str("sources.repository", "");
    let repo = if repo.is_empty() {
        "isnady".to_string()
    } else {
        repo
    };
    let file = crate::paths::data_dir()
        .join("repos")
        .join(format!("{repo}.json"));
    let Some(cat) = std::fs::read_to_string(file)
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
    else {
        return vec![];
    };
    let keys: HashSet<String> = conn
        .prepare("SELECT key FROM collections")
        .and_then(|mut s| s.query_map([], |r| r.get::<_, String>(0))?.collect())
        .unwrap_or_default();
    let mut out: Vec<String> = vec![];
    let entries = cat["entries"]
        .as_array()
        .into_iter()
        .flatten()
        .chain(cat["library"].as_array().into_iter().flatten());
    for e in entries {
        let Some(book) = e.get("book").and_then(|b| b.as_str()) else {
            continue;
        };
        if !keys.contains(book) {
            continue;
        }
        for code in e
            .get("languages")
            .and_then(|l| l.as_array())
            .into_iter()
            .flatten()
        {
            let Some(code) = code.as_str() else { continue };
            let name = names.get(code).cloned().unwrap_or_else(|| code.to_string());
            if !imported.contains(&name) && !out.contains(&name) {
                out.push(name);
            }
        }
    }
    out
}
