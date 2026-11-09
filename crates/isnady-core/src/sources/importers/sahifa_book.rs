//! A written source as a book (`sahifa_book.py`, SHF1b): isnady-data's sahifa/<road>/<book>.json (isnady-sahifa/1)
//! — the hadith of a sahifa as [book, number] of the books they stand in. Importing it makes a book of Books whose
//! leaves point at those hadith in the books imported here (`sahifa.write_work`): nothing is copied.

use rusqlite::{params, params_from_iter, Connection};
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};

use super::{py_str, read_json, ImportError, ImportReport, SourceInfo};
use crate::py::thousands;
use crate::sources::{now_iso, tier_for_license, ResourceError};

pub const SCHEMA: &str = "isnady-sahifa/1";

pub fn run(
    conn: &Connection,
    source: &SourceInfo,
    say: &dyn Fn(&str),
) -> Result<ImportReport, ImportError> {
    let data = read_json(&source.location, &source.auth, Some(300)).map_err(|e| {
        if e.0.starts_with("Not valid JSON") {
            ResourceError(format!("Not a sahifa file: {}", source.location))
        } else {
            e
        }
    })?;
    let id = data
        .get("id")
        .map(py_str)
        .filter(|i| !i.is_empty() && i != "None");
    let (true, Some(key)) = (
        data.get("schema").and_then(Value::as_str) == Some(SCHEMA),
        id,
    ) else {
        return Err(ResourceError(format!(
            "Not a sahifa file (isnady-sahifa/1): {}",
            source.location
        ))
        .into());
    };
    let title = data
        .get("title")
        .and_then(|t| t.get("en"))
        .and_then(Value::as_str)
        .filter(|t| !t.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| key.clone());
    let lang = crate::i18n::ui_language();
    say(&crate::i18n::t(
        &lang,
        "Reading {title}",
        &[("title", title.clone())],
    ));
    let mut report = ImportReport {
        source_key: key.clone(),
        ..Default::default()
    };
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO sources (key, name, format, location, license, tier, origin, auth_type, imported_at)
         VALUES (?, ?, 'isnady-sahifa', ?, ?, ?, ?, ?, ?)
         ON CONFLICT(key) DO UPDATE SET name = excluded.name, location = excluded.location,
             license = excluded.license, tier = excluded.tier, origin = excluded.origin,
             auth_type = excluded.auth_type, imported_at = excluded.imported_at",
        params![
            key,
            source.name.clone().filter(|n| !n.is_empty()).unwrap_or_else(|| title.clone()),
            source.location,
            source.license,
            source
                .tier
                .clone()
                .unwrap_or_else(|| tier_for_license(source.license.as_deref())),
            source.origin,
            source.auth.describe(),
            now_iso()
        ],
    )?;
    let source_id: i64 =
        tx.query_row("SELECT id FROM sources WHERE key = ?", [&key], |r| r.get(0))?;
    let (found, missing) = write_work(&tx, &data, source_id)?;
    tx.commit()?;
    report.editions.push(key);
    report.texts = found;
    if missing > 0 {
        report.warnings.push(format!(
            "{} hadith stand in books not imported here",
            thousands(missing)
        ));
    }
    say(&format!(
        "{} hadith{}",
        thousands(found),
        if missing > 0 {
            format!("; {} in books not imported here", thousands(missing))
        } else {
            String::new()
        }
    ));
    Ok(report)
}

fn pairs(v: Option<&Value>) -> Vec<(String, String)> {
    v.and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|p| Some((py_str(p.get(0)?), py_str(p.get(1)?))))
                .collect()
        })
        .unwrap_or_default()
}

/// A written source as a book of Books, its hadith those of the books imported here (`sahifa.write_work`):
/// (found, missing).
pub fn write_work(conn: &Connection, data: &Value, source_id: i64) -> rusqlite::Result<(i64, i64)> {
    let key = py_str(&data["id"]);
    let road = data
        .get("road")
        .map(py_str)
        .unwrap_or_else(|| "sunni".into());
    conn.execute("DELETE FROM works WHERE key = ?", [&key])?;
    let title = match data.get("title") {
        Some(Value::Object(t)) => t.get("en").map(py_str).unwrap_or_else(|| "None".into()),
        Some(v) if super::truthy(v) => py_str(v),
        _ => key.clone(),
    };
    let title_ar = data
        .get("title")
        .and_then(|t| t.get("ar"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let hadith = pairs(data.get("hadith"));
    let parallels: Vec<Vec<(String, String)>> = data
        .get("parallels")
        .and_then(Value::as_object)
        .map(|o| o.values().map(|v| pairs(Some(v))).collect())
        .unwrap_or_default();
    conn.execute(
        "INSERT INTO works (key, kind, title, title_ar, source_id, stamp) VALUES (?, 'sahifa', ?, ?, ?, ?)",
        params![key, title, title_ar, source_id, format!("sahifa|{road}|{}", hadith.len())],
    )?;
    let wid = conn.last_insert_rowid();
    let wanted: BTreeSet<String> = hadith
        .iter()
        .map(|(k, _)| k.clone())
        .chain(parallels.iter().flatten().map(|(k, _)| k.clone()))
        .collect();
    let mut lookup: HashMap<(String, String), i64> = HashMap::new();
    if !wanted.is_empty() {
        let marks = vec!["?"; wanted.len()].join(",");
        let mut st = conn.prepare(&format!(
            "SELECT c.key, h.number, h.id FROM hadiths h JOIN collections c ON c.id = h.collection_id
             WHERE c.key IN ({marks})"
        ))?;
        let rows = st.query_map(params_from_iter(wanted.iter()), |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
            ))
        })?;
        for r in rows {
            let (k, n, id) = r?;
            lookup.insert((k, n), id);
        }
    }
    let names: HashMap<String, String> = {
        let mut st = conn.prepare("SELECT key, name FROM collections")?;
        let v = st
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?;
        v
    };
    let mut chapters: HashMap<String, i64> = HashMap::new();
    let mut top = 0i64;
    let mut count: HashMap<i64, i64> = HashMap::new();
    let mut chapter = |label: String, book: &str| -> rusqlite::Result<i64> {
        if let Some(id) = chapters.get(&label) {
            return Ok(*id);
        }
        top += 1;
        let name = names.get(book).cloned().unwrap_or_else(|| book.to_string());
        conn.execute(
            "INSERT INTO work_nodes (work_id, parent_id, ordinal, kind, label, title) VALUES (?, NULL, ?, 'kitab', ?, ?)",
            params![wid, top, label, name],
        )?;
        let id = conn.last_insert_rowid();
        chapters.insert(label, id);
        Ok(id)
    };
    let mut leaf = conn.prepare(
        "INSERT INTO work_nodes (work_id, parent_id, ordinal, kind, label, hadith_id) VALUES (?, ?, ?, 'hadith', ?, ?)",
    )?;
    let (mut found, mut missing) = (0i64, 0i64);
    for (book, num) in &hadith {
        let Some(hid) = lookup.get(&(book.clone(), num.clone())) else {
            missing += 1;
            continue;
        };
        let parent = chapter(format!("book:{book}"), book)?;
        let c = count.entry(parent).or_insert(0);
        *c += 1;
        leaf.execute(params![wid, parent, *c, num, hid])?;
        found += 1;
    }
    for pars in &parallels {
        for (book, n) in pars {
            let Some(hid) = lookup.get(&(book.clone(), n.clone())) else {
                continue;
            };
            let parent = chapter(format!("par:{book}"), book)?;
            let c = count.entry(parent).or_insert(0);
            *c += 1;
            leaf.execute(params![wid, parent, *c, n, hid])?;
        }
    }
    Ok((found, missing))
}
