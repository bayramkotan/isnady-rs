//! isnady's own book format (isnady-book/1), the format of the isnady-data repository (`isnady_book.py`).
//!
//! A book is a folder — sunni/bukhari/, shia/kafi/ — with book.json (what does not depend on the language: numbers,
//! chapters, references, grades) and one file per language (its texts and chapter titles), so only the languages
//! chosen are downloaded.

use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{Map, Value};
use std::collections::{BTreeSet, HashMap};

use super::{py_str, read_json, sql, truthy, ImportError, ImportReport, Options, SourceInfo};
use crate::py::thousands;
use crate::sources::{now_iso, prefetch as fetch_ahead, resolve, tier_for_license, ResourceError};

pub const SCHEMA: &str = "isnady-book/1";
/// Books imported before isnady-data existed: importing the book from isnady-data replaces them.
const LEGACY_SOURCES: [&str; 1] = ["fawazahmed0-hadith-api"];

fn offered(book: &Value) -> Map<String, Value> {
    book.get("languages")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default()
}

/// The language files of a book that are imported: the ones chosen that the book has, or its first
/// (`wanted_languages`).
pub fn wanted_languages(book: &Value, options: &Options) -> Vec<String> {
    let offered = offered(book);
    let chosen: Vec<String> = options
        .get("languages")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .filter(|c| offered.contains_key(*c))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    if chosen.is_empty() {
        offered.keys().take(1).cloned().collect()
    } else {
        chosen
    }
}

/// Start downloading a book and, as soon as its book.json is in, the language files that will be imported
/// (`isnady_book.prefetch`).
pub fn prefetch(location: &str, options: &Options) {
    let base = location.to_string();
    let options = options.clone();
    fetch_ahead(
        location,
        Some(Box::new(move |data: &[u8]| {
            let text = String::from_utf8_lossy(data);
            let Ok(book) = serde_json::from_str::<Value>(text.trim_start_matches('\u{feff}'))
            else {
                return;
            };
            let offered = offered(&book);
            for code in wanted_languages(&book, &options) {
                if let Some(file) = offered[&code].get("file").and_then(Value::as_str) {
                    fetch_ahead(&resolve(&base, file), None);
                }
            }
        })),
    );
}

fn s<'a>(v: &'a Value, k: &str) -> Option<&'a str> {
    v.get(k).and_then(Value::as_str)
}

pub fn run(
    conn: &Connection,
    source: &SourceInfo,
    options: &Options,
    say: &dyn Fn(&str),
) -> Result<ImportReport, ImportError> {
    say(&format!("Reading {}", source.location));
    let book = read_json(&source.location, &source.auth, None)?;
    if s(&book, "schema") != Some(SCHEMA) {
        return Err(ResourceError(format!(
            "{} is not an isnady book (schema {SCHEMA}).",
            source.location
        ))
        .into());
    }
    let key = py_str(&book["key"]);
    let title = py_str(&book["title"]);
    let offered = offered(&book);
    let wanted = wanted_languages(&book, options);
    let source_key = format!("isnady-{key}");
    let mut report = ImportReport {
        source_key: source_key.clone(),
        ..Default::default()
    };
    let book_license = s(&book, "license").filter(|l| !l.is_empty());
    let tier = source
        .tier
        .clone()
        .unwrap_or_else(|| tier_for_license(book_license.or(source.license.as_deref())));
    conn.execute(
        "INSERT INTO sources (key, name, format, location, license, tier, origin, auth_type, imported_at)
         VALUES (?, ?, 'isnady', ?, ?, ?, ?, ?, ?)
         ON CONFLICT(key) DO UPDATE SET name = excluded.name, location = excluded.location,
             license = excluded.license, tier = excluded.tier, origin = excluded.origin",
        params![
            source_key,
            format!("isnady-data: {title}"),
            source.location,
            sql(book.get("license").unwrap_or(&Value::Null)),
            tier,
            source.origin,
            source.auth.describe(),
            now_iso()
        ],
    )?;
    let source_id: i64 =
        conn.query_row("SELECT id FROM sources WHERE key = ?", [&source_key], |r| {
            r.get(0)
        })?;

    let mut texts_by_lang: Vec<(String, Value)> = vec![];
    for code in &wanted {
        let meta = &offered[code];
        let where_ = resolve(&source.location, s(meta, "file").unwrap_or(""));
        say(&format!(
            "Downloading {title} ({})",
            s(meta, "name").unwrap_or(code)
        ));
        texts_by_lang.push((code.clone(), read_json(&where_, &source.auth, None)?));
    }

    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO collections (key, name, tradition) VALUES (?, ?, ?)
         ON CONFLICT(key) DO UPDATE SET name = excluded.name, tradition = excluded.tradition",
        params![key, title, s(&book, "tradition").unwrap_or("sunni")],
    )?;
    let cid: i64 = tx.query_row("SELECT id FROM collections WHERE key = ?", [&key], |r| {
        r.get(0)
    })?;
    for legacy in LEGACY_SOURCES {
        let old: Option<i64> = tx
            .query_row("SELECT id FROM sources WHERE key = ?", [legacy], |r| {
                r.get(0)
            })
            .optional()?;
        let Some(old) = old else { continue };
        let gone = tx.execute(
            "DELETE FROM editions WHERE source_id = ? AND collection_id = ?",
            params![old, cid],
        )?;
        tx.execute(
            "DELETE FROM grades WHERE source_id = ? AND hadith_id IN \
             (SELECT id FROM hadiths WHERE collection_id = ?)",
            params![old, cid],
        )?;
        if gone > 0 {
            say(&format!(
                "{title}: the copy imported before isnady-data is replaced"
            ));
        }
        let left: Option<i64> = tx
            .query_row("SELECT 1 FROM editions WHERE source_id = ?", [old], |r| {
                r.get(0)
            })
            .optional()?;
        if left.is_none() {
            tx.execute("DELETE FROM sources WHERE id = ?", [old])?;
        }
    }
    let empty = vec![];
    let hadiths = book
        .get("hadiths")
        .and_then(Value::as_array)
        .unwrap_or(&empty);
    let mut ids: HashMap<String, i64> = HashMap::new();
    let (mut new_hadiths, mut new_grades) = (0i64, 0i64);
    {
        let mut add = tx.prepare(
            "INSERT INTO hadiths (collection_id, number, number_sort, section_number)
             VALUES (?, ?, ?, ?) ON CONFLICT(collection_id, number) DO NOTHING",
        )?;
        let mut find =
            tx.prepare("SELECT id FROM hadiths WHERE collection_id = ? AND number = ?")?;
        let mut refs = tx.prepare("INSERT OR IGNORE INTO hadith_refs VALUES (?, ?, ?)")?;
        let mut grade = tx.prepare(
            "INSERT OR IGNORE INTO grades (hadith_id, grader_name, grade, source_id) VALUES (?, ?, ?, ?)",
        )?;
        let null = Value::Null;
        for h in hadiths {
            let n = &h["n"];
            new_hadiths += add.execute(params![
                cid,
                sql(n),
                sql(h.get("sort").unwrap_or(&null)),
                sql(h.get("s").unwrap_or(&null))
            ])? as i64;
            let hid: i64 = find.query_row(params![cid, sql(n)], |r| r.get(0))?;
            ids.insert(py_str(n), hid);
            if let Some(r) = h.get("refs").and_then(Value::as_object) {
                for (scheme, value) in r {
                    refs.execute(params![hid, scheme, py_str(value)])?;
                }
            }
            if let Some(gs) = h.get("grades").and_then(Value::as_array) {
                for g in gs {
                    let grader = g.get(0).unwrap_or(&null);
                    let value = g.get(1).unwrap_or(&null);
                    new_grades +=
                        grade.execute(params![hid, sql(grader), sql(value), source_id])? as i64;
                }
            }
        }
    }
    let sections: BTreeSet<i64> = hadiths
        .iter()
        .filter_map(|h| h.get("s").and_then(Value::as_i64))
        .collect();
    for sec in &sections {
        tx.execute(
            "INSERT INTO sections (collection_id, number) VALUES (?, ?) \
             ON CONFLICT(collection_id, number) DO NOTHING",
            params![cid, sec],
        )?;
    }
    for (code, data) in &texts_by_lang {
        let meta = &offered[code];
        let name = s(meta, "name").unwrap_or(code).to_string();
        let ekey = format!("isnady:{key}:{code}");
        tx.execute(
            "DELETE FROM editions WHERE source_id = ? AND key = ?",
            params![source_id, ekey],
        )?;
        tx.execute(
            "INSERT INTO editions (source_id, collection_id, key, language, direction, author, comments,
                                   location, imported_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                source_id,
                cid,
                ekey,
                name,
                s(meta, "direction").unwrap_or("ltr"),
                sql(meta.get("author").unwrap_or(&Value::Null)),
                sql(book.get("credit").unwrap_or(&Value::Null)),
                resolve(&source.location, s(meta, "file").unwrap_or("")),
                now_iso()
            ],
        )?;
        let eid = tx.last_insert_rowid();
        if let Some(secs) = data.get("sections").and_then(Value::as_object) {
            for (number, title) in secs {
                // a section without a number in the source ("None" in al-Shama'il) is left out
                let Ok(number) = crate::py::trim(number).parse::<i64>() else {
                    continue;
                };
                let row: Option<i64> = tx
                    .query_row(
                        "SELECT id FROM sections WHERE collection_id = ? AND number = ?",
                        params![cid, number],
                        |r| r.get(0),
                    )
                    .optional()?;
                if let Some(sid) = row {
                    if truthy(title) {
                        tx.execute(
                            "INSERT OR REPLACE INTO section_titles VALUES (?, ?, ?)",
                            params![sid, eid, sql(title)],
                        )?;
                    }
                }
            }
        }
        let mut count = 0i64;
        if let Some(texts) = data.get("texts").and_then(Value::as_object) {
            let mut put = tx.prepare(
                "INSERT OR REPLACE INTO texts (hadith_id, edition_id, text) VALUES (?, ?, ?)",
            )?;
            for (number, text) in texts {
                if let Some(hid) = ids.get(number) {
                    if truthy(text) {
                        put.execute(params![hid, eid, sql(text)])?;
                        count += 1;
                    }
                }
            }
        }
        tx.execute(
            "UPDATE editions SET text_count = ? WHERE id = ?",
            params![count, eid],
        )?;
        report.editions.push(format!("{key} ({name})"));
        report.texts += count;
    }
    tx.execute(
        "UPDATE sources SET imported_at = ? WHERE id = ?",
        params![now_iso(), source_id],
    )?;
    tx.commit()?;
    report.new_hadiths = new_hadiths;
    report.grades = new_grades;
    say(&format!(
        "{title}: {} hadith, {} texts, {} grades",
        thousands(ids.len() as i64),
        thousands(report.texts),
        thousands(new_grades)
    ));
    Ok(report)
}
