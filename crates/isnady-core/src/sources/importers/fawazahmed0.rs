//! The fawazahmed0/hadith-api edition format (`json_fawazahmed0.py`): one edition file, or the editions index from
//! which the chosen editions are downloaded through their own links. Every edition adds texts to shared hadith rows;
//! diacritics-free Arabic copies are skipped unless asked for.

use rusqlite::{params, Connection};
use serde_json::{Map, Value};

use super::{py_str, sql, ImportError, ImportReport, Options, SourceInfo};
use crate::pre;
use crate::sources::{now_iso, read_bytes_with, resolve, slugify, tier_for_license, ResourceError};

/// The interface's words, filled in.
type Tr<'a> = dyn Fn(&str, &[(&str, String)]) -> String + 'a;

fn language_of(code: &str) -> Option<&'static str> {
    Some(match code {
        "ara" => "Arabic",
        "ben" => "Bengali",
        "eng" => "English",
        "fra" => "French",
        "ind" => "Indonesian",
        "rus" => "Russian",
        "tam" => "Tamil",
        "tur" => "Turkish",
        "urd" => "Urdu",
        _ => return None,
    })
}

fn load_json(data: &[u8], location: &str) -> Result<Value, ResourceError> {
    let text = std::str::from_utf8(data)
        .map_err(|e| ResourceError(format!("Not valid UTF-8 JSON: {location} ({e})")))?;
    serde_json::from_str(text.trim_start_matches('\u{feff}'))
        .map_err(|e| ResourceError(format!("Not valid UTF-8 JSON: {location} ({e})")))
}

fn is_edition(v: &Value) -> bool {
    v.get("metadata").is_some_and(Value::is_object) && v.get("hadiths").is_some_and(Value::is_array)
}

fn is_index(v: &Value) -> bool {
    v.as_object().is_some_and(|o| {
        !o.is_empty()
            && o.values()
                .all(|b| b.get("collection").is_some_and(Value::is_array))
    })
}

fn st<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k).and_then(Value::as_str).unwrap_or("")
}

fn is_plain_copy(entry: &Value) -> bool {
    st(entry, "comments")
        .to_lowercase()
        .contains("diacritics removed")
        || pre!(r"^ara-.*\d$").match_(st(entry, "name")).is_some()
}

fn edition_key_from_location(location: &str) -> String {
    let path = if location.contains("://") {
        url::Url::parse(location)
            .map(|u| u.path().to_string())
            .unwrap_or_default()
    } else {
        location.replace('\\', "/")
    };
    let name = path.rsplit('/').next().unwrap_or("").to_string();
    for suffix in [".min.json", ".json"] {
        if let Some(n) = name.strip_suffix(suffix) {
            return n.to_string();
        }
    }
    name
}

/// `_format_number`: (the number as text, as a number).
fn format_number(v: Option<&Value>) -> (String, Option<f64>) {
    match v {
        None | Some(Value::Null) | Some(Value::Bool(_)) => (String::new(), None),
        Some(Value::Number(n)) => match n.as_i64() {
            Some(i) => (i.to_string(), Some(i as f64)),
            None => {
                let f = n.as_f64().unwrap_or(0.0);
                let t = if f.fract() == 0.0 {
                    (f as i64).to_string()
                } else {
                    crate::py::float_repr(f)
                };
                (t, Some(f))
            }
        },
        Some(other) => {
            let t = crate::py::trim(&py_str(other)).to_string();
            let f = super::table::py_float(&t);
            (t, f)
        }
    }
}

/// `str.title()`.
fn title_case(s: &str) -> String {
    let mut out = String::new();
    let mut prev_letter = false;
    for c in s.chars() {
        if c.is_alphabetic() {
            if prev_letter {
                out.extend(c.to_lowercase());
            } else {
                out.extend(c.to_uppercase());
            }
            prev_letter = true;
        } else {
            out.push(c);
            prev_letter = false;
        }
    }
    out
}

fn list(options: &Options, k: &str) -> Vec<String> {
    options
        .get(k)
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn select_from_index(root: &Value, options: &Options) -> Vec<Value> {
    let editions: Vec<String> = list(options, "editions")
        .iter()
        .map(|e| e.to_lowercase())
        .collect();
    let books: Vec<String> = list(options, "books")
        .iter()
        .map(|e| e.to_lowercase())
        .collect();
    let languages: Vec<String> = list(options, "languages")
        .iter()
        .map(|l| {
            let l = l.to_lowercase();
            language_of(&l).map(str::to_lowercase).unwrap_or(l)
        })
        .collect();
    let all = options.get("all").and_then(Value::as_bool).unwrap_or(false);
    if editions.is_empty() && books.is_empty() && languages.is_empty() && !all {
        return vec![];
    }
    let matches =
        |v: &str, wanted: &[String]| wanted.is_empty() || wanted.contains(&v.to_lowercase());
    let mut chosen = vec![];
    for (book_key, book) in root.as_object().into_iter().flatten() {
        for entry in book["collection"].as_array().into_iter().flatten() {
            let Some(obj) = entry.as_object() else {
                continue;
            };
            if !editions.is_empty() && !editions.contains(&st(entry, "name").to_lowercase()) {
                continue;
            }
            let b = Some(st(entry, "book"))
                .filter(|b| !b.is_empty())
                .unwrap_or(book_key);
            if !matches(b, &books) {
                continue;
            }
            if !matches(st(entry, "language"), &languages) {
                continue;
            }
            let mut e = obj.clone();
            e.insert("book".into(), Value::String(b.to_string()));
            e.insert(
                "book_name".into(),
                book.get("name").cloned().unwrap_or(Value::Null),
            );
            chosen.push(Value::Object(e));
        }
    }
    chosen
}

pub fn run(
    conn: &Connection,
    source: &SourceInfo,
    options: &Options,
    say: &dyn Fn(&str),
) -> Result<ImportReport, ImportError> {
    let lang = crate::i18n::ui_language();
    let tr = |t: &str, v: &[(&str, String)]| crate::i18n::t(&lang, t, v);
    say(&format!("Reading {}", source.location));
    let root = load_json(
        &read_bytes_with(&source.location, &source.auth, None)?,
        &source.location,
    )?;
    let given = source.name.clone().filter(|n| !n.is_empty());
    let source_name = given
        .clone()
        .unwrap_or_else(|| edition_key_from_location(&source.location));
    let source_key = slugify(given.as_deref().unwrap_or(&source.location));
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
         VALUES (?, ?, 'fawazahmed0', ?, ?, ?, ?, ?, ?)
         ON CONFLICT(key) DO UPDATE SET
             name = excluded.name, location = excluded.location,
             license = COALESCE(excluded.license, sources.license),
             tier = excluded.tier, origin = excluded.origin, auth_type = excluded.auth_type",
        params![source_key, source_name, source.location, source.license, tier, source.origin, source.auth.describe(), now_iso()],
    )?;
    let source_id: i64 =
        conn.query_row("SELECT id FROM sources WHERE key = ?", [&source_key], |r| {
            r.get(0)
        })?;
    if is_edition(&root) {
        let key = options
            .get("edition_key")
            .and_then(Value::as_str)
            .filter(|k| !k.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| edition_key_from_location(&source.location));
        let languages = list(options, "languages");
        let books = list(options, "books");
        let language = languages.first().map(|l| {
            language_of(&l.to_lowercase())
                .map(str::to_string)
                .unwrap_or_else(|| title_case(l))
        });
        let mut entry = Map::new();
        entry.insert("name".into(), Value::String(key));
        entry.insert(
            "book".into(),
            books
                .first()
                .map(|b| Value::String(b.clone()))
                .unwrap_or(Value::Null),
        );
        entry.insert(
            "language".into(),
            language.map(Value::String).unwrap_or(Value::Null),
        );
        import_edition(
            conn,
            source_id,
            &Value::Object(entry),
            &root,
            &source.location,
            &mut report,
            say,
            &tr,
        )?;
    } else if is_index(&root) {
        let mut entries = select_from_index(&root, options);
        if entries.is_empty() {
            return Err(ResourceError(
                "No editions matched. Choose with --edition, --book or --language, or pass --all to import every \
                 edition in the index."
                    .into(),
            )
            .into());
        }
        if !options
            .get("include_plain")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            let explicit: Vec<String> = list(options, "editions")
                .iter()
                .map(|e| e.to_lowercase())
                .collect();
            let mut kept = vec![];
            for e in entries {
                if is_plain_copy(&e) && !explicit.contains(&st(&e, "name").to_lowercase()) {
                    report.skipped_editions.push(st(&e, "name").to_string());
                } else {
                    kept.push(e);
                }
            }
            entries = kept;
        }
        let skipped = report.skipped_editions.len();
        let mut line = tr(
            "{entries} edition(s) selected from the index",
            &[("entries", entries.len().to_string())],
        );
        if skipped > 0 {
            line.push_str(&tr(
                ", {skipped_editions} diacritics-free cop{value} skipped",
                &[
                    ("skipped_editions", skipped.to_string()),
                    ("value", if skipped == 1 { "y" } else { "ies" }.into()),
                ],
            ));
        }
        say(&line);
        for entry in &entries {
            let link = Some(st(entry, "linkmin"))
                .filter(|l| !l.is_empty())
                .or(Some(st(entry, "link")).filter(|l| !l.is_empty()));
            let name = entry
                .get("name")
                .map(py_str)
                .unwrap_or_else(|| "None".into());
            let Some(link) = link else {
                report
                    .warnings
                    .push(format!("{name}: no link in the index, skipped"));
                continue;
            };
            let url = resolve(&source.location, link);
            say(&format!("Downloading {name}"));
            let data = load_json(&read_bytes_with(&url, &source.auth, None)?, &url)?;
            if !is_edition(&data) {
                report
                    .warnings
                    .push(format!("{name}: not an edition file, skipped"));
                continue;
            }
            import_edition(conn, source_id, entry, &data, &url, &mut report, say, &tr)?;
        }
    } else {
        return Err(ResourceError(format!(
            "{} is JSON but not in the fawazahmed0 format (expected 'metadata' + 'hadiths', or an editions index).",
            source.location
        ))
        .into());
    }
    conn.execute(
        "UPDATE sources SET imported_at = ? WHERE id = ?",
        params![now_iso(), source_id],
    )?;
    Ok(report)
}

#[allow(clippy::too_many_arguments)]
fn import_edition(
    conn: &Connection,
    source_id: i64,
    entry: &Value,
    data: &Value,
    location: &str,
    report: &mut ImportReport,
    say: &dyn Fn(&str),
    tr: &Tr<'_>,
) -> Result<(), ImportError> {
    let meta = &data["metadata"];
    let key = Some(st(entry, "name"))
        .filter(|n| !n.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| edition_key_from_location(location));
    let (prefix, rest) = key.split_once('-').unwrap_or((key.as_str(), ""));
    let language = Some(st(entry, "language"))
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .or_else(|| language_of(prefix).map(str::to_string))
        .unwrap_or_else(|| "Unknown".into());
    let stripped = pre!(r"\d+$").sub(rest, "");
    let book_key = Some(st(entry, "book"))
        .filter(|b| !b.is_empty())
        .map(str::to_string)
        .or_else(|| Some(stripped).filter(|b| !b.is_empty()))
        .unwrap_or_else(|| key.clone());
    let book_name = Some(st(entry, "book_name"))
        .filter(|b| !b.is_empty())
        .or_else(|| Some(st(meta, "name")).filter(|b| !b.is_empty()))
        .unwrap_or(&book_key)
        .to_string();
    let direction = Some(st(entry, "direction"))
        .filter(|d| !d.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            if ["Arabic", "Urdu"].contains(&language.as_str()) {
                "rtl"
            } else {
                "ltr"
            }
            .into()
        });
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO collections (key, name) VALUES (?, ?) ON CONFLICT(key) DO NOTHING",
        params![book_key, book_name],
    )?;
    let cid: i64 = tx.query_row(
        "SELECT id FROM collections WHERE key = ?",
        [&book_key],
        |r| r.get(0),
    )?;
    tx.execute(
        "DELETE FROM editions WHERE source_id = ? AND key = ?",
        params![source_id, key],
    )?;
    let comments = Some(st(entry, "comments")).filter(|c| !c.is_empty());
    tx.execute(
        "INSERT INTO editions (source_id, collection_id, key, language, direction, author, comments, location, imported_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        params![source_id, cid, key, language, direction, sql(entry.get("author").unwrap_or(&Value::Null)), comments, location, now_iso()],
    )?;
    let eid = tx.last_insert_rowid();
    let empty_map = Map::new();
    let details = meta
        .get("section_details")
        .and_then(Value::as_object)
        .unwrap_or(&empty_map);
    for (number, title) in meta
        .get("sections")
        .and_then(Value::as_object)
        .unwrap_or(&empty_map)
    {
        let Ok(num) = crate::py::trim(number).parse::<i64>() else {
            continue;
        };
        let d = details.get(number).cloned().unwrap_or(Value::Null);
        tx.execute(
            "INSERT INTO sections (collection_id, number, first_number, last_number) VALUES (?, ?, ?, ?)
             ON CONFLICT(collection_id, number) DO NOTHING",
            params![cid, num, sql(d.get("hadithnumber_first").unwrap_or(&Value::Null)), sql(d.get("hadithnumber_last").unwrap_or(&Value::Null))],
        )?;
        if super::truthy(title) {
            let sid: i64 = tx.query_row(
                "SELECT id FROM sections WHERE collection_id = ? AND number = ?",
                params![cid, num],
                |r| r.get(0),
            )?;
            tx.execute("INSERT OR REPLACE INTO section_titles (section_id, edition_id, title) VALUES (?, ?, ?)", params![sid, eid, sql(title)])?;
        }
    }
    let (mut texts, mut empty, mut new_hadiths, mut new_grades) = (0i64, 0i64, 0i64, 0i64);
    for item in data["hadiths"].as_array().into_iter().flatten() {
        if !item.is_object() {
            continue;
        }
        let text = crate::py::trim(st(item, "text")).to_string();
        let (number, number_sort) = format_number(item.get("hadithnumber"));
        if number.is_empty() {
            continue;
        }
        let r = item.get("reference").cloned().unwrap_or(Value::Null);
        let section = r.get("book").and_then(|b| {
            if b.is_boolean() {
                b.as_bool().map(|x| x as i64)
            } else {
                b.as_i64()
            }
        });
        new_hadiths += tx.execute(
            "INSERT INTO hadiths (collection_id, number, number_sort, section_number) VALUES (?, ?, ?, ?)
             ON CONFLICT(collection_id, number) DO NOTHING",
            params![cid, number, number_sort, section],
        )? as i64;
        let hid: i64 = tx.query_row(
            "SELECT id FROM hadiths WHERE collection_id = ? AND number = ?",
            params![cid, number],
            |r| r.get(0),
        )?;
        let (arabic, _) = format_number(item.get("arabicnumber"));
        if !arabic.is_empty() {
            tx.execute(
                "INSERT OR IGNORE INTO hadith_refs VALUES (?, 'arabic', ?)",
                params![hid, arabic],
            )?;
        }
        if r.get("book").is_some_and(|b| !b.is_null())
            && r.get("hadith").is_some_and(|b| !b.is_null())
        {
            tx.execute(
                "INSERT OR IGNORE INTO hadith_refs VALUES (?, 'in-book', ?)",
                params![
                    hid,
                    format!("{}:{}", py_str(&r["book"]), py_str(&r["hadith"]))
                ],
            )?;
        }
        for g in item
            .get("grades")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let grade = st(g, "grade");
            if g.is_object() && !grade.is_empty() {
                let name = Some(st(g, "name"))
                    .filter(|n| !n.is_empty())
                    .unwrap_or("Unknown");
                new_grades += tx.execute(
                    "INSERT OR IGNORE INTO grades (hadith_id, grader_name, grade, source_id) VALUES (?, ?, ?, ?)",
                    params![hid, crate::py::trim(name), crate::py::trim(grade), source_id],
                )? as i64;
            }
        }
        if text.is_empty() {
            empty += 1;
            continue;
        }
        tx.execute(
            "INSERT INTO texts (hadith_id, edition_id, text) VALUES (?, ?, ?)",
            params![hid, eid, text],
        )?;
        texts += 1;
    }
    tx.execute(
        "UPDATE editions SET text_count = ? WHERE id = ?",
        params![texts, eid],
    )?;
    tx.commit()?;
    report.editions.push(key.clone());
    report.texts += texts;
    report.new_hadiths += new_hadiths;
    report.grades += new_grades;
    if empty > 0 {
        report.warnings.push(format!(
            "{key}: {empty} hadith without text in this edition"
        ));
    }
    say(&tr(
        "{key}: {texts} texts ({language}), {new_hadiths} new hadith, {new_grades} new grades",
        &[
            ("key", key),
            ("texts", texts.to_string()),
            ("language", language),
            ("new_hadiths", new_hadiths.to_string()),
            ("new_grades", new_grades.to_string()),
        ],
    ));
    Ok(())
}
