//! Statistics (gui/statistics_page.py, ST7): what the page asks of the core. The long computations run in a thread
//! of their own (stats_jobs, PERF2): the window is never held while they run, and their steps can be asked for.

use isnady_core::stats::{self, corpus, graders, lab, models, pyjson, words};
use isnady_core::{books, db, i18n, lives, log, paths, scholars, works};
use serde_json::{json, Map, Value};
use std::sync::Mutex;

use crate::{err, page_langs, with_db, Data};

static PROGRESS: Mutex<String> = Mutex::new(String::new());

fn say(text: &str) {
    if let Ok(mut p) = PROGRESS.lock() {
        *p = text.to_string();
    }
}

/// The step the computation is at ("Measuring the chains"), for the line under the page's head.
#[tauri::command]
pub fn stats_progress() -> String {
    PROGRESS.lock().map(|p| p.clone()).unwrap_or_default()
}

/// The choice of books (fill_book_combo with "All Sunni books", "All Shia books", "All books"), and the books with
/// two graders or more (Graders).
#[tauri::command]
pub fn stats_scope(state: tauri::State<Data>) -> Result<Value, String> {
    let lang = i18n::ui_language();
    with_db(&state, |conn| {
        let all = books::all_books(conn, false).map_err(err)?;
        let mut groups = vec![];
        for (t, label) in [("sunni", "Sunni"), ("shia", "Shia"), ("other", "Other")] {
            let list: Vec<Value> = all
                .iter()
                .filter(|b| {
                    let road = if ["sunni", "shia"].contains(&b.tradition.as_str()) { b.tradition.as_str() } else { "other" };
                    road == t
                })
                .map(|b| json!({"key": b.key, "name": works::book_title(&b.key, &b.name, &lang), "count": b.count}))
                .collect();
            if !list.is_empty() {
                groups.push(json!({"tradition": t, "label": label, "books": list}));
            }
        }
        let sunni = all.iter().filter(|b| b.tradition != "shia").count();
        let counts: std::collections::HashMap<String, i64> = graders::books_with_grades(conn)
            .map_err(err)?
            .into_iter()
            .map(|(k, _n, c)| (k, c))
            .collect();
        let grader_books: Vec<Value> = all
            .iter()
            .filter(|b| counts.get(&b.key).copied().unwrap_or(0) >= 2)
            .map(|b| {
                json!([
                    b.key,
                    works::book_title(&b.key, &b.name, &lang),
                    counts[&b.key],
                    stats::tradition_of_scope(conn, Some(&b.key))
                ])
            })
            .collect();
        Ok(
            json!({"groups": groups, "sunni": sunni, "shia": all.len() - sunni, "grader_books": grader_books}),
        )
    })
}

fn one_road(conn: &db::Connection, key: Option<&str>, recompute: bool) -> Result<Value, String> {
    let c = corpus::cached(conn, key, &say, recompute, false).map_err(err)?;
    let m = models::cached(conn, key, &say, recompute, false).map_err(err)?;
    let l = lab::cached(conn, key, &say, recompute, false).map_err(err)?;
    Ok(json!({"corpus": c, "models": m, "lab": l, "road": stats::tradition_of_scope(conn, key)}))
}

/// The corpus sections' results (stats_jobs.corpus_and_models): saved ones at once, else computed in a thread.
/// "All books": each road on its own, one after the other — never together.
#[tauri::command]
pub async fn stats_corpus(scope: Option<String>, recompute: bool) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        say("");
        let conn = db::connect(None).map_err(err)?;
        let out = match scope.as_deref() {
            Some(stats::ALL_SCOPE) => json!({"roads": [one_road(&conn, None, recompute)?,
                                                       one_road(&conn, Some(stats::SHIA_SCOPE), recompute)?]}),
            other => one_road(&conn, other.filter(|s| !s.is_empty()), recompute)?,
        };
        say("");
        Ok(out)
    })
    .await
    .map_err(err)?
}

/// The graders of one book (stats_jobs.graders).
#[tauri::command]
pub async fn stats_graders(key: String, recompute: bool) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        say("");
        let conn = db::connect(None).map_err(err)?;
        let out = graders::cached(&conn, &key, &say, recompute, false).map_err(err);
        say("");
        out
    })
    .await
    .map_err(err)?
}

/// The names the page shows in the interface language: the graders' short and full names, the books' titles (the
/// results keep them in English) and the narrators' names as read (`_short`, `_full`, `_bk`, `_name`).
#[tauri::command]
pub fn stats_names(
    state: tauri::State<Data>,
    graders: Vec<String>,
    books: Vec<String>,
    people: Vec<(i64, Option<String>)>,
) -> Result<Value, String> {
    let lang = i18n::ui_language();
    let mut short = Map::new();
    let mut full = Map::new();
    for g in &graders {
        let s = scholars::all()
            .iter()
            .find(|s| s.grader_names.iter().any(|x| x == g));
        short.insert(
            g.clone(),
            json!(s
                .map(|s| scholars::short_name(s, &lang))
                .unwrap_or_else(|| g.clone())),
        );
        full.insert(
            g.clone(),
            json!(s
                .map(|s| if s.full.is_empty() {
                    s.name.clone()
                } else {
                    s.full.clone()
                })
                .unwrap_or_else(|| g.clone())),
        );
    }
    with_db(&state, |conn| {
        let titles: Map<String, Value> = books
            .iter()
            .map(|b| (b.clone(), json!(works::book_named(Some(conn), b, &lang))))
            .collect();
        let mut names = Map::new();
        for (id, name) in &people {
            let name = name.clone().unwrap_or_default();
            let (reading, arabic) = lives::reading(conn, *id, &name, page_langs(&lang));
            names.insert(
                id.to_string(),
                json!(if reading.is_empty() { arabic } else { reading }),
            );
        }
        Ok(json!({"short": short, "full": full, "books": titles, "people": names}))
    })
}

/// Every measure in plain words (Terms explained, and "What do these words mean?").
#[tauri::command]
pub fn stats_terms() -> Value {
    json!(words::all_terms()
        .into_iter()
        .map(|t| json!({"key": t.key, "name": t.name, "what": t.what, "read": t.read, "hadith": t.hadith,
                        "example": t.example}))
        .collect::<Vec<_>>())
}

/// Where an export is offered first: the home folder.
#[tauri::command]
pub fn stats_home() -> String {
    paths::expand_user("~").display().to_string()
}

fn write(path: &str, text: &str) -> Result<(), String> {
    let p = paths::expand_user(path);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).map_err(err)?;
    }
    std::fs::write(&p, text).map_err(err)
}

fn logged(what: &str, path: &str) {
    log::info("export", &format!("✔ statistics ({what}) → {path}"));
}

/// A page (HTML) or anything else the page has written out.
#[tauri::command]
pub fn stats_write(path: String, text: String, what: String) -> Result<String, String> {
    write(&path, &text)?;
    logged(&what, &path);
    Ok(paths::expand_user(&path).display().to_string())
}

/// The figures behind one finding as CSV (`_save_csv`).
#[tauri::command]
pub fn stats_csv(
    path: String,
    header: Vec<Value>,
    rows: Vec<Vec<Value>>,
) -> Result<String, String> {
    write(&path, &stats::csv_text(&header, &rows))?;
    logged("csv", &path);
    Ok(paths::expand_user(&path).display().to_string())
}

fn safe(name: &str) -> String {
    let s: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    let s: String = s.trim_matches('-').chars().take(60).collect();
    if s.is_empty() {
        "isnady".into()
    } else {
        s
    }
}

/// Every table as CSV files in a folder (`save_csv_folder`).
#[tauri::command]
pub fn stats_csv_folder(
    folder: String,
    tables: Vec<(String, Vec<Value>, Vec<Vec<Value>>)>,
) -> Result<String, String> {
    let dir = paths::expand_user(&folder);
    std::fs::create_dir_all(&dir).map_err(err)?;
    let mut used = std::collections::HashSet::new();
    for (name, header, rows) in &tables {
        let base = safe(name);
        let (mut n, mut stem) = (1, base.clone());
        while used.contains(&stem) {
            n += 1;
            stem = format!("{base}-{n}");
        }
        used.insert(stem.clone());
        std::fs::write(
            dir.join(format!("{stem}.csv")),
            stats::csv_text(header, rows),
        )
        .map_err(err)?;
    }
    logged("all, csv", &folder);
    Ok(dir.display().to_string())
}

/// The computed results (`save_json`).
#[tauri::command]
pub fn stats_json(path: String, results: Value) -> Result<String, String> {
    let out = json!({"schema": "isnady-statistics/1", "isnady": isnady_core::VERSION,
                     "exported": chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(), "results": results});
    write(&path, &pyjson::dumps_indent(&out, 1))?;
    logged("all, json", &path);
    Ok(paths::expand_user(&path).display().to_string())
}

fn sheet(
    name: &str,
    header: &[Value],
    rows: &[Vec<Value>],
    meta: bool,
) -> isnady_core::formats::Table {
    let columns: Vec<String> = header.iter().map(isnady_core::config::py_str).collect();
    let rows = rows
        .iter()
        .map(|r| {
            let mut m = serde_json::Map::new();
            for (c, v) in columns.iter().zip(r) {
                m.insert(c.clone(), v.clone());
            }
            m
        })
        .collect();
    let mut table = isnady_core::formats::Table {
        title: name.into(),
        columns,
        rows,
        ..Default::default()
    };
    if meta {
        table
            .meta
            .insert("isnady".into(), json!(isnady_core::VERSION));
        table.meta.insert("kind".into(), json!("statistics"));
    }
    table
}

/// The figures behind one finding as Excel (`save_table`, xlsx).
#[tauri::command]
pub fn stats_xlsx(
    path: String,
    name: String,
    header: Vec<Value>,
    rows: Vec<Vec<Value>>,
) -> Result<String, String> {
    let p = paths::expand_user(&path);
    let written =
        isnady_core::formats::write_table(&sheet(&name, &header, &rows, true), "xlsx", &p)?;
    logged("finding, xlsx", &path);
    Ok(written.display().to_string())
}

/// Every table as Excel, a sheet each (`save_workbook`).
#[tauri::command]
pub fn stats_workbook(
    path: String,
    tables: Vec<(String, Vec<Value>, Vec<Vec<Value>>)>,
) -> Result<String, String> {
    let p = paths::expand_user(&path);
    let sheets: Vec<_> = tables
        .iter()
        .map(|(n, h, r)| sheet(n, h, r, false))
        .collect();
    isnady_core::formats::write_workbook(&sheets, &p)?;
    logged("all, xlsx", &path);
    Ok(p.display().to_string())
}

/// A picture the page drew (PNG as base64, or SVG text) written out (`save_png`, `save_svg`).
#[tauri::command]
pub fn stats_picture(
    path: String,
    png: Option<String>,
    svg: Option<String>,
    what: String,
) -> Result<String, String> {
    let p = paths::expand_user(&path);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).map_err(err)?;
    }
    match (png, svg) {
        (Some(b64), _) => {
            let bytes =
                unbase64(b64.split(',').next_back().unwrap_or("")).ok_or("not a picture")?;
            std::fs::write(&p, bytes).map_err(err)?;
        }
        (None, Some(text)) => std::fs::write(&p, text).map_err(err)?,
        _ => return Err("nothing to write".into()),
    }
    logged(&what, &path);
    Ok(p.display().to_string())
}

/// Standard base64 (what canvas.toDataURL gives) to bytes.
fn unbase64(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() * 3 / 4);
    let (mut acc, mut bits) = (0u32, 0u32);
    for c in text.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' | b'\n' | b'\r' | b' ' => continue,
            _ => return None,
        } as u32;
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Some(out)
}

/// Findings as PDF, a heading over each section and each finding as the page draws it (`save_pdf`).
#[tauri::command]
pub async fn stats_pdf(
    path: String,
    title: String,
    parts: Vec<(String, Vec<(String, f32)>)>,
    what: String,
) -> Result<String, String> {
    let p = paths::expand_user(&path);
    tauri::async_runtime::spawn_blocking(move || {
        let mut pdf = isnady_core::pdf::Pdf::new(&title);
        if !title.is_empty() {
            pdf.paragraph(&title, 16.0, true, false, 0.0);
            pdf.paragraph(
                &format!("isnady-rs {}", isnady_core::VERSION),
                8.5,
                false,
                true,
                0.0,
            );
            pdf.space(6.0);
        }
        for (heading, pictures) in parts {
            if !heading.is_empty() {
                pdf.space(6.0);
                pdf.paragraph(&heading, 13.0, true, false, 0.0);
                pdf.rule();
            }
            for (jpeg, width) in pictures {
                let bytes = isnady_core::sources::base64_decode(&jpeg).ok_or("not a picture")?;
                pdf.picture_jpeg(bytes, width)?;
                pdf.space(8.0);
            }
        }
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).map_err(err)?;
        }
        pdf.save(&p)?;
        logged(&what, &path);
        Ok(p.display().to_string())
    })
    .await
    .map_err(err)?
}
