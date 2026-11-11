//! Hadith published as a table: CSV files, or JSON lists of records (`table.py`). The repository's catalog says
//! which column is which (the mapping); the location is one file or several (a book in parts or volumes).

use rusqlite::{params, Connection};
use serde_json::{Map, Value};
use std::collections::HashMap;

use super::{py_str, ImportError, ImportReport, Options, SourceInfo};
use crate::pre;
use crate::reader::pyre;
use crate::sources::{now_iso, read_bytes_with, slugify, tier_for_license, ResourceError};

/// `float(text)` of Python: the digits of any script, an exponent, "inf"/"nan".
pub fn py_float(text: &str) -> Option<f64> {
    let ascii: String = text
        .chars()
        .map(|c| {
            let u = c as u32;
            for b in [0x660u32, 0x6F0, 0x7C0, 0x966, 0x9E6, 0xFF10] {
                if (b..b + 10).contains(&u) {
                    return char::from_digit(u - b, 10).unwrap_or(c);
                }
            }
            c
        })
        .collect();
    let t = crate::py::trim(&ascii).replace('_', "");
    if t.is_empty() {
        return None;
    }
    t.parse::<f64>().ok()
}

/// A column by name, by position ("#N"), or a dotted path into nested JSON (`_get`).
fn get<'a>(record: &'a Value, field: &str) -> Option<&'a Value> {
    if field.is_empty() {
        return None;
    }
    if let Some(row) = record.as_array() {
        let i: usize = field.strip_prefix('#')?.parse().ok()?;
        return row.get(i);
    }
    let obj = record.as_object()?;
    if let Some(v) = obj.get(field) {
        return Some(v);
    }
    let mut value = record;
    for part in field.split('.') {
        value = value.as_object()?.get(part)?;
    }
    Some(value)
}

/// `_clean`.
fn clean(v: Option<&Value>) -> String {
    let text = match v {
        None | Some(Value::Null) => return String::new(),
        Some(Value::Number(n))
            if n.as_i64().is_none() && n.as_f64().is_some_and(|f| f.fract() == 0.0) =>
        {
            (n.as_f64().unwrap_or(0.0) as i64).to_string()
        }
        Some(x) => py_str(x),
    };
    let text = text.replace(['\u{200f}', '\u{200e}'], "");
    crate::py::trim(&pre!(r"[ \t]+").sub(&text, " ")).to_string()
}

/// `_number`: (the number as text, as a number).
fn number(v: Option<&Value>) -> (String, Option<f64>) {
    let mut text = clean(v);
    if pre!(r"\d+\.0+").fullmatch(&text) {
        text = text.split('.').next().unwrap_or("").to_string();
    }
    if let Some(f) = py_float(&text) {
        return (text, Some(f));
    }
    let f = pre!(r"(\d+(?:\.\d+)?)")
        .match_(&text)
        .and_then(|m| py_float(m.g(1)));
    (text, f)
}

/// Python's csv reader (the excel dialect, not strict): the rows of a text.
pub fn csv_rows(text: &str) -> Vec<Vec<String>> {
    csv_rows_with(text, ',')
}

/// The same with another delimiter (a tab: the excel-tab dialect).
pub fn csv_rows_with(text: &str, delim: char) -> Vec<Vec<String>> {
    #[derive(PartialEq)]
    enum S {
        StartRecord,
        StartField,
        InField,
        InQuoted,
        QuoteInQuoted,
        EatCrnl,
    }
    let mut rows = vec![];
    let mut row: Vec<String> = vec![];
    let mut field = String::new();
    let mut state = S::StartRecord;
    for c in text.chars() {
        loop {
            match state {
                S::StartRecord => {
                    if c == '\n' || c == '\r' {
                        state = if c == '\r' {
                            S::EatCrnl
                        } else {
                            S::StartRecord
                        };
                        rows.push(std::mem::take(&mut row));
                        break;
                    }
                    state = S::StartField;
                    continue;
                }
                S::StartField => {
                    if c == '\n' || c == '\r' {
                        row.push(std::mem::take(&mut field));
                        rows.push(std::mem::take(&mut row));
                        state = if c == '\r' {
                            S::EatCrnl
                        } else {
                            S::StartRecord
                        };
                    } else if c == '"' {
                        state = S::InQuoted;
                    } else if c == delim {
                        row.push(std::mem::take(&mut field));
                    } else {
                        field.push(c);
                        state = S::InField;
                    }
                }
                S::InField => {
                    if c == '\n' || c == '\r' {
                        row.push(std::mem::take(&mut field));
                        rows.push(std::mem::take(&mut row));
                        state = if c == '\r' {
                            S::EatCrnl
                        } else {
                            S::StartRecord
                        };
                    } else if c == delim {
                        row.push(std::mem::take(&mut field));
                        state = S::StartField;
                    } else {
                        field.push(c);
                    }
                }
                S::InQuoted => {
                    if c == '"' {
                        state = S::QuoteInQuoted;
                    } else {
                        field.push(c);
                    }
                }
                S::QuoteInQuoted => {
                    if c == '"' {
                        field.push('"');
                        state = S::InQuoted;
                    } else if c == delim {
                        row.push(std::mem::take(&mut field));
                        state = S::StartField;
                    } else if c == '\n' || c == '\r' {
                        row.push(std::mem::take(&mut field));
                        rows.push(std::mem::take(&mut row));
                        state = if c == '\r' {
                            S::EatCrnl
                        } else {
                            S::StartRecord
                        };
                    } else {
                        field.push(c);
                        state = S::InField;
                    }
                }
                S::EatCrnl => {
                    state = S::StartRecord;
                    if c == '\n' {
                        break;
                    }
                    continue;
                }
            }
            break;
        }
    }
    if state != S::StartRecord && state != S::EatCrnl {
        row.push(field);
        rows.push(row);
    }
    rows
}

fn records(data: &[u8], location: &str, mapping: &Value) -> Result<Vec<Value>, ResourceError> {
    let kind = mapping
        .get("type")
        .and_then(Value::as_str)
        .filter(|k| !k.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            if location
                .to_lowercase()
                .split('?')
                .next()
                .unwrap_or("")
                .ends_with(".json")
            {
                "json"
            } else {
                "csv"
            }
            .into()
        });
    let text = String::from_utf8_lossy(data);
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    if kind == "json" {
        let mut root: Value = serde_json::from_str(text)
            .map_err(|e| ResourceError(format!("Not valid JSON: {location} ({e})")))?;
        if let Some(path) = mapping
            .get("records")
            .and_then(Value::as_str)
            .filter(|p| !p.is_empty())
        {
            root = get(&root, path).cloned().unwrap_or(Value::Null);
        }
        if let Value::Object(o) = root {
            root = Value::Array(o.into_iter().map(|(_, v)| v).collect());
        }
        return match root {
            Value::Array(a) => Ok(a),
            _ => Err(ResourceError(format!(
                "{location}: no list of hadith found (records: {})",
                match mapping.get("records") {
                    Some(Value::String(s)) => format!("'{s}'"),
                    _ => "None".into(),
                }
            ))),
        };
    }
    let skip = mapping
        .get("skip_rows")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    let rows: Vec<Vec<String>> = csv_rows(text).into_iter().skip(skip).collect();
    let filled = |r: &Vec<String>| r.iter().any(|c| !crate::py::trim(c).is_empty());
    if mapping
        .get("header")
        .and_then(Value::as_bool)
        .unwrap_or(true)
    {
        let Some(first) = rows.first() else {
            return Ok(vec![]);
        };
        let head: Vec<String> = first
            .iter()
            .map(|h| crate::py::trim(h).to_string())
            .collect();
        return Ok(rows[1..]
            .iter()
            .filter(|r| filled(r))
            .map(|r| {
                let mut o = Map::new();
                for (h, c) in head.iter().zip(r) {
                    o.insert(h.clone(), Value::String(c.clone()));
                }
                Value::Object(o)
            })
            .collect());
    }
    Ok(rows
        .into_iter()
        .filter(|r| filled(r))
        .map(|r| Value::Array(r.into_iter().map(Value::String).collect()))
        .collect())
}

fn grade(v: Option<&Value>, rule: &Value) -> String {
    let mut text = clean(v);
    if text.is_empty() {
        return String::new();
    }
    if let Some(p) = rule
        .get("pattern")
        .and_then(Value::as_str)
        .filter(|p| !p.is_empty())
    {
        let re = pyre::Re::new(&format!("(?s){p}"));
        text = crate::py::trim(
            re.search(&text)
                .map(|m| m.g(1).to_string())
                .unwrap_or_default()
                .as_str(),
        )
        .to_string();
    }
    let skips = rule
        .get("skip")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if skips
        .iter()
        .filter_map(Value::as_str)
        .any(|s| text.contains(s))
    {
        return String::new();
    }
    text
}

fn s<'a>(v: &'a Value, k: &str) -> Option<&'a str> {
    v.get(k).and_then(Value::as_str).filter(|x| !x.is_empty())
}

fn lang_of(spec: &Value) -> String {
    spec.get("language")
        .and_then(Value::as_str)
        .unwrap_or("Arabic")
        .to_string()
}

pub fn run(
    conn: &Connection,
    source: &SourceInfo,
    options: &Options,
    say: &dyn Fn(&str),
) -> Result<ImportReport, ImportError> {
    let lang = crate::i18n::ui_language();
    let tr = |t: &str, v: &[(&str, String)]| crate::i18n::t(&lang, t, v);
    let mapping = options
        .get("mapping")
        .cloned()
        .unwrap_or(Value::Object(Map::new()));
    let specs: Vec<Value> = mapping
        .get("texts")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if specs.is_empty() {
        return Err(ResourceError(
            "This source has no column mapping (which column holds the text).".into(),
        )
        .into());
    }
    let given = source.name.clone().filter(|n| !n.is_empty());
    let opt = |k: &str| {
        options
            .get(k)
            .and_then(Value::as_str)
            .filter(|x| !x.is_empty())
            .map(str::to_string)
    };
    let book = opt("book").unwrap_or_else(|| slugify(given.as_deref().unwrap_or(&source.location)));
    let title = opt("title")
        .or_else(|| given.clone())
        .unwrap_or_else(|| book.clone());
    let tradition = opt("tradition").unwrap_or_else(|| "sunni".into());
    let parts: Vec<Value> = options
        .get("parts")
        .and_then(Value::as_array)
        .filter(|a| !a.is_empty())
        .cloned()
        .unwrap_or_else(|| vec![Value::String(source.location.clone())]);
    let source_key =
        opt("source_key").unwrap_or_else(|| slugify(given.as_deref().unwrap_or(&source.location)));
    let mut report = ImportReport {
        source_key: source_key.clone(),
        ..Default::default()
    };
    let tier = source
        .tier
        .clone()
        .unwrap_or_else(|| tier_for_license(source.license.as_deref()));
    conn.execute(
        "INSERT INTO sources (key, name, format, location, license, tier, origin, auth_type, imported_at)
         VALUES (?, ?, 'table', ?, ?, ?, ?, ?, ?)
         ON CONFLICT(key) DO UPDATE SET name = excluded.name, location = excluded.location,
             license = COALESCE(excluded.license, sources.license), tier = excluded.tier,
             origin = excluded.origin, auth_type = excluded.auth_type",
        params![source_key, given.clone().unwrap_or_else(|| source_key.clone()), source.location, source.license, tier, source.origin, source.auth.describe(), now_iso()],
    )?;
    let source_id: i64 =
        conn.query_row("SELECT id FROM sources WHERE key = ?", [&source_key], |r| {
            r.get(0)
        })?;
    let mut rows: Vec<(Value, Option<i64>, usize)> = vec![];
    for (n, part) in parts.iter().enumerate() {
        let (where_, volume) = match part {
            Value::Object(_) => (
                s(part, "location").unwrap_or("").to_string(),
                part.get("volume").and_then(Value::as_i64),
            ),
            other => (py_str(other), None),
        };
        let mut line = format!("Reading {where_}");
        if parts.len() > 1 {
            line.push_str(&tr(
                " ({n} of {parts})",
                &[
                    ("n", (n + 1).to_string()),
                    ("parts", parts.len().to_string()),
                ],
            ));
        }
        say(&line);
        let recs = records(
            &read_bytes_with(&where_, &source.auth, None)?,
            &where_,
            &mapping,
        )?;
        rows.extend(
            recs.into_iter()
                .enumerate()
                .map(|(i, r)| (r, volume, i + 1)),
        );
    }
    if rows.is_empty() {
        return Err(ResourceError(format!("No hadith found in {}.", source.location)).into());
    }
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO collections (key, name, tradition) VALUES (?, ?, ?) ON CONFLICT(key) DO UPDATE SET tradition = excluded.tradition",
        params![book, title, tradition],
    )?;
    let cid: i64 = tx.query_row("SELECT id FROM collections WHERE key = ?", [&book], |r| {
        r.get(0)
    })?;
    let mut editions: Vec<(String, i64)> = vec![];
    for spec in &specs {
        let lang_ = lang_of(spec);
        let name = s(spec, "name")
            .map(str::to_string)
            .unwrap_or_else(|| lang_.clone());
        let key = format!("{source_key}:{book}:{}", name.to_lowercase());
        tx.execute(
            "DELETE FROM editions WHERE source_id = ? AND key = ?",
            params![source_id, key],
        )?;
        tx.execute(
            "INSERT INTO editions (source_id, collection_id, key, language, direction, author, comments, location, imported_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                source_id,
                cid,
                key,
                lang_,
                if ["Arabic", "Urdu", "Persian"].contains(&lang_.as_str()) { "rtl" } else { "ltr" },
                super::sql(spec.get("author").unwrap_or(&Value::Null)),
                super::sql(spec.get("comments").unwrap_or(&Value::Null)),
                source.location,
                now_iso()
            ],
        )?;
        let id = tx.last_insert_rowid();
        match editions.iter_mut().find(|(n, _)| *n == name) {
            Some(e) => e.1 = id,
            None => editions.push((name, id)),
        }
    }
    let edition_of: HashMap<String, i64> = editions.iter().cloned().collect();
    let mut counts: Vec<(String, i64)> = editions.iter().map(|(n, _)| (n.clone(), 0)).collect();
    let (mut new_hadiths, mut new_grades) = (0i64, 0i64);
    let mut seen: Vec<String> = vec![];
    let mut seen_n: HashMap<String, i64> = HashMap::new();
    let field = s(&mapping, "number").unwrap_or("@row").to_string();
    for (record, volume, row_no) in &rows {
        let row_value = Value::from(*row_no as i64);
        let mut raw: Option<Value> = if field == "@row" {
            Some(row_value.clone())
        } else {
            get(record, &field).cloned()
        };
        if let (Some(p), Some(r)) = (s(&mapping, "number_pattern"), raw.as_ref()) {
            raw = pyre::Re::new(p)
                .search(&clean(Some(r)))
                .map(|m| Value::String(m.g(1).to_string()));
        }
        let (mut num, mut sort) = number(Some(raw.as_ref().unwrap_or(&row_value)));
        if num.is_empty() {
            continue;
        }
        if let Some(v) = volume {
            num = format!("{v}:{num}");
            sort = Some(*v as f64 * 100000.0 + sort.unwrap_or(0.0));
        }
        match seen_n.get_mut(&num) {
            Some(c) => {
                *c += 1;
                num = format!("{num}.{c}");
            }
            None => {
                seen_n.insert(num.clone(), 1);
                seen.push(num.clone());
            }
        }
        let mut section: Option<i64> = None;
        if let Some(f) = s(&mapping, "section") {
            let (_t, so) = number(get(record, f));
            section = so.map(|x| x.trunc() as i64);
            if let (Some(v), Some(sec)) = (volume, section) {
                section = Some(v * 1000 + sec);
            }
        }
        new_hadiths += tx.execute(
            "INSERT INTO hadiths (collection_id, number, number_sort, section_number) VALUES (?, ?, ?, ?)
             ON CONFLICT(collection_id, number) DO NOTHING",
            params![cid, num, sort, section],
        )? as i64;
        let hid: i64 = tx.query_row(
            "SELECT id FROM hadiths WHERE collection_id = ? AND number = ?",
            params![cid, num],
            |r| r.get(0),
        )?;
        if let Some(sec) = section {
            tx.execute("INSERT INTO sections (collection_id, number) VALUES (?, ?) ON CONFLICT(collection_id, number) DO NOTHING", params![cid, sec])?;
            for spec in mapping
                .get("section_titles")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let t = clean(get(record, s(spec, "field").unwrap_or("")));
                let want = lang_of(spec);
                let target = specs.iter().find(|x| lang_of(x) == want).map(|x| {
                    s(x, "name")
                        .map(str::to_string)
                        .unwrap_or_else(|| lang_of(x))
                });
                if let (false, Some(eid)) = (
                    t.is_empty(),
                    target.and_then(|n| edition_of.get(&n).copied()),
                ) {
                    let sid: i64 = tx.query_row(
                        "SELECT id FROM sections WHERE collection_id = ? AND number = ?",
                        params![cid, sec],
                        |r| r.get(0),
                    )?;
                    tx.execute(
                        "INSERT OR IGNORE INTO section_titles VALUES (?, ?, ?)",
                        params![sid, eid, t],
                    )?;
                }
            }
        }
        if let Some(f) = s(&mapping, "url") {
            let url = clean(get(record, f));
            if !url.is_empty() {
                tx.execute(
                    "INSERT OR IGNORE INTO hadith_refs VALUES (?, 'url', ?)",
                    params![hid, url],
                )?;
            }
        }
        for spec in &specs {
            let mut text = clean(get(record, s(spec, "field").unwrap_or("")));
            let lead = s(spec, "lead")
                .map(|l| clean(get(record, l)))
                .unwrap_or_default();
            if !lead.is_empty() && !text.starts_with(&lead) {
                text = crate::py::trim(&format!("{lead} {text}")).to_string();
            }
            if !text.is_empty() {
                let name = s(spec, "name")
                    .map(str::to_string)
                    .unwrap_or_else(|| lang_of(spec));
                tx.execute(
                    "INSERT OR REPLACE INTO texts (hadith_id, edition_id, text) VALUES (?, ?, ?)",
                    params![hid, edition_of[&name], text],
                )?;
                if let Some(c) = counts.iter_mut().find(|(n, _)| *n == name) {
                    c.1 += 1;
                }
            }
        }
        for rule in mapping
            .get("grades")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let g = grade(get(record, s(rule, "field").unwrap_or("")), rule);
            if !g.is_empty() {
                new_grades += tx.execute(
                    "INSERT OR IGNORE INTO grades (hadith_id, grader_name, grade, source_id) VALUES (?, ?, ?, ?)",
                    params![hid, rule.get("grader").and_then(Value::as_str).unwrap_or("Unknown"), g, source_id],
                )? as i64;
            }
        }
    }
    for (name, eid) in &editions {
        let c = counts
            .iter()
            .find(|(n, _)| n == name)
            .map(|x| x.1)
            .unwrap_or(0);
        tx.execute(
            "UPDATE editions SET text_count = ? WHERE id = ?",
            params![c, eid],
        )?;
        if c > 0 {
            report.editions.push(format!("{book} ({name})"));
        } else {
            tx.execute("DELETE FROM editions WHERE id = ?", [eid])?;
        }
    }
    tx.execute(
        "UPDATE sources SET imported_at = ? WHERE id = ?",
        params![now_iso(), source_id],
    )?;
    tx.commit()?;
    report.texts = counts.iter().map(|c| c.1).sum();
    report.new_hadiths = new_hadiths;
    report.grades = new_grades;
    let mut line = tr(
        "{title}: {seen:,} hadith, ",
        &[
            ("title", title.clone()),
            ("seen:,", crate::py::thousands(seen.len() as i64)),
        ],
    );
    line.push_str(
        &counts
            .iter()
            .filter(|c| c.1 > 0)
            .map(|(l, c)| format!("{} {l}", crate::py::thousands(*c)))
            .collect::<Vec<_>>()
            .join(", "),
    );
    if new_grades > 0 {
        line.push_str(&tr(
            ", {new_grades:,} grades",
            &[("new_grades:,", crate::py::thousands(new_grades))],
        ));
    }
    say(&line);
    Ok(report)
}
