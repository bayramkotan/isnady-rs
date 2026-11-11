//! File → Export… and File → Import… (gui/transfer_dialog.py, EX1): anything isnady holds, in the format chosen,
//! with a preview before it is written. Everything is decided in isnady_core::transfer; this only asks what, how
//! and where, and runs the long ones in the background.

use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use isnady_core::transfer::{self as T, Export, Import, SearchFilters};
use isnady_core::{cache, config, db, formats as F, i18n, paths, works};

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

struct Job {
    running: bool,
    title: String,
    text: String,
    done: usize,
    total: usize,
    started: Option<std::time::Instant>,
    result: Value,
}

static JOB: Mutex<Job> = Mutex::new(Job {
    running: false,
    title: String::new(),
    text: String::new(),
    done: 0,
    total: 0,
    started: None,
    result: Value::Null,
});
static STOP: AtomicBool = AtomicBool::new(false);

fn say(m: &str) {
    if let Ok(mut j) = JOB.lock() {
        j.text = m.to_string();
    }
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

fn text(v: &Value) -> String {
    v.as_str().unwrap_or("").to_string()
}

/// What the dialogs offer: the kinds and their formats, the books, the languages, the profiles.
#[tauri::command]
pub fn xfer_page() -> Result<Value, String> {
    let conn = db::connect(None).map_err(err)?;
    let ui = i18n::ui_language();
    let books: Vec<Value> = works::list_works(&conn, &ui)
        .map_err(err)?
        .into_iter()
        .filter(|w| w.kind == "hadith")
        .map(|w| json!({"key": w.key, "title": w.title, "tradition": w.tradition}))
        .collect();
    let have = T::list_languages(&conn)?;
    let chosen: Vec<String> = config::get_str("view.content_languages", "")
        .split(',')
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect();
    let mut labels = serde_json::Map::new();
    let mut suffix = serde_json::Map::new();
    for k in T::KINDS {
        for f in k.2.iter().chain(k.3.iter()) {
            labels.insert((*f).into(), json!(F::label(f)));
            suffix.insert((*f).into(), json!(T::suffix(f)));
        }
    }
    let home = paths::expand_user("~").display().to_string();
    Ok(json!({
        "kinds": T::KINDS.iter().map(|k| json!({"key": k.0, "export": k.2, "import": k.3})).collect::<Vec<_>>(),
        "labels": labels, "suffix": suffix, "books": books,
        "languages": have, "chosen": chosen,
        "profiles": config::all_profiles().into_iter().map(|p| json!({"id": p.id, "name": p.name})).collect::<Vec<_>>(),
        "active": config::active_profile(),
        "home": home,
    }))
}

/// The chapters of a book, by number (`_fill_chapters`).
#[tauri::command]
pub fn xfer_chapters(book: String) -> Result<Value, String> {
    let conn = db::connect(None).map_err(err)?;
    let mut st = conn
        .prepare(
            "SELECT s.number, (SELECT st.title FROM section_titles st WHERE st.section_id = s.id LIMIT 1)
             FROM sections s JOIN collections c ON c.id = s.collection_id WHERE c.key = ? ORDER BY s.number",
        )
        .map_err(err)?;
    let rows: Vec<Value> = st
        .query_map([&book], |r| {
            Ok(json!({"number": r.get::<_, i64>(0)?, "title": r.get::<_, Option<String>>(1)?.unwrap_or_default()}))
        })
        .map_err(err)?
        .collect::<Result<_, _>>()
        .map_err(err)?;
    Ok(Value::Array(rows))
}

/// The number of the chapter being read in Books, for "this chapter as a file" (`BooksPage._chapter_number`).
#[tauri::command]
pub fn xfer_chapter_number(node: i64) -> Option<i64> {
    let conn = db::connect(None).ok()?;
    // the book (kitab) the chapter is in: the export's unit
    let trail = works::path(&conn, node, "en");
    let label = trail.first()?.label.clone()?;
    if label.is_empty() || !label.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    label.parse().ok()
}

fn filters(v: &Value) -> Option<SearchFilters> {
    let f = v.as_object()?;
    let get = |k: &str| f.get(k).cloned().unwrap_or(Value::Null);
    Some(SearchFilters {
        mode: get("mode").as_str().map(String::from),
        whole_words: get("whole_words").as_bool().unwrap_or(false),
        collections: strings(&get("collections")),
        languages: strings(&get("languages")),
        grades: strings(&get("grades")),
        every_grader: get("every_grader").as_bool().unwrap_or(false),
        sahihayn: get("sahihayn").as_bool().unwrap_or(true),
        chapters: get("chapters")
            .as_array()
            .map(|a| a.iter().filter_map(Value::as_i64).collect())
            .unwrap_or_default(),
    })
}

fn request<'a>(a: &Value, fmt: &str, target: std::path::PathBuf) -> Export<'a> {
    Export {
        kind: text(&a["kind"]),
        fmt: fmt.to_string(),
        target,
        book: text(&a["book"]),
        chapter: a["chapter"].as_i64(),
        refs: strings(&a["refs"]),
        query: text(&a["query"]),
        languages: strings(&a["languages"]),
        chains: a["chains"].as_bool().unwrap_or(true),
        tradition: a["tradition"].as_str().map(String::from),
        profile: None,
        books: strings(&a["books"]),
        profiles: strings(&a["profiles"]),
        separate: a["separate"].as_bool().unwrap_or(false),
        search: filters(&a["search"]),
        tick: None,
    }
}

/// With the choices of the Search page, said in a line (`_filters_text`): what `search_method` says.
#[tauri::command]
pub fn xfer_method(query: String, search: Value) -> Result<Value, String> {
    let conn = db::connect(None).map_err(err)?;
    let ui = i18n::ui_language();
    let mut m = T::search_method(&conn, &query, filters(&search).as_ref());
    if let Some(books) = m["books"].as_array() {
        m["books"] = json!(books
            .iter()
            .map(|k| works::book_title(k.as_str().unwrap_or(""), k.as_str().unwrap_or(""), &ui))
            .collect::<Vec<_>>());
    }
    Ok(m)
}

/// The preview (`_preview`): the file as it will be, in a format that reads as text; its first 20,000 characters.
#[tauri::command]
pub async fn xfer_preview(args: Value) -> Result<Value, String> {
    let kind = text(&args["kind"]);
    let fmt = text(&args["fmt"]);
    let spec = T::kind(&kind).ok_or_else(|| kind.clone())?;
    let mut shown = match fmt.as_str() {
        "xlsx" | "sqlite" => "md",
        "docx" | "epub" | "pdf" => "txt",
        "isnady-book" | "isnady-profile" | "backup" => "json",
        other => other,
    }
    .to_string();
    if !spec.2.contains(&shown.as_str()) {
        shown = ["md", "json", "txt", "csv"]
            .into_iter()
            .find(|f| spec.2.contains(f))
            .unwrap_or(&fmt)
            .to_string();
    }
    let folder = std::env::temp_dir().join(format!("isnady-preview-{}", std::process::id()));
    std::fs::create_dir_all(&folder).map_err(err)?;
    let tmp = folder.join(format!("preview{}", T::suffix(&shown)));
    let conn = db::connect(None).map_err(err)?;
    let written = T::export(Some(&conn), &request(&args, &shown, tmp), &|_| {});
    let out = written.map(|path| {
        let body = if path.is_file() {
            String::from_utf8_lossy(&std::fs::read(&path).unwrap_or_default())
                .trim_start_matches('\u{feff}')
                .to_string()
        } else {
            String::new()
        };
        let long = body.chars().count() > 20000;
        let head: String = body.chars().take(20000).collect();
        json!({"shown": shown, "text": head, "more": long})
    });
    let _ = std::fs::remove_dir_all(&folder);
    out
}

/// Export…: started in the background; the dialog and the bar at the bottom ask how it goes.
#[tauri::command]
pub fn xfer_export(args: Value, path: String) -> Result<(), String> {
    {
        let mut j = JOB.lock().map_err(err)?;
        if j.running {
            return Err("An export or import is running already.".into());
        }
        *j = Job {
            running: true,
            title: paths::expand_user(&path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
            text: String::new(),
            done: 0,
            total: 0,
            started: Some(std::time::Instant::now()),
            result: Value::Null,
        };
    }
    STOP.store(false, Ordering::SeqCst);
    std::thread::spawn(move || {
        let started = std::time::Instant::now();
        let fmt = text(&args["fmt"]);
        let tick = |done: usize, total: usize| -> bool {
            if let Ok(mut j) = JOB.lock() {
                j.done = done;
                j.total = total;
            }
            !STOP.load(Ordering::SeqCst)
        };
        let run = || -> Result<String, String> {
            let conn = db::connect(None).map_err(err)?;
            let mut r = request(&args, &fmt, path.clone().into());
            r.tick = Some(&tick);
            T::export(Some(&conn), &r, &say).map(|p| p.display().to_string())
        };
        let out = run();
        let cancelled = STOP.load(Ordering::SeqCst);
        match &out {
            Ok(p) => isnady_core::log::info(
                "export",
                &format!(
                    "{} as {fmt}: {p} in {}",
                    text(&args["kind"]),
                    isnady_core::log::since(started)
                ),
            ),
            Err(e) => {
                isnady_core::log::error("export", &format!("{} as {fmt}: {e}", text(&args["kind"])))
            }
        }
        if let Ok(mut j) = JOB.lock() {
            j.running = false;
            j.result = match out {
                Ok(p) => json!({"ok": true, "message": p}),
                Err(e) => json!({"ok": false, "message": e, "cancelled": cancelled}),
            };
        }
    });
    Ok(())
}

#[tauri::command]
pub fn xfer_stop() {
    STOP.store(true, Ordering::SeqCst);
}

#[tauri::command]
pub fn xfer_status() -> Value {
    match JOB.lock() {
        Ok(j) => {
            json!({"running": j.running, "title": j.title, "text": j.text, "done": j.done, "total": j.total,
            "gone": j.started.map(|s| s.elapsed().as_secs()).unwrap_or(0), "result": j.result})
        }
        Err(_) => json!({"running": false, "result": null}),
    }
}

/// Show in folder: the folder of what was written.
#[tauri::command]
pub fn xfer_show(path: String) -> Result<(), String> {
    let p = std::path::PathBuf::from(&path);
    let folder = if p.is_dir() {
        p
    } else {
        p.parent().map(|d| d.to_path_buf()).unwrap_or(p)
    };
    crate::open_external(&folder.display().to_string())
}

/// What each file to import is read as (`_detect`).
#[tauri::command]
pub fn xfer_guess(kind: String, files: Vec<String>) -> Value {
    Value::Array(
        files
            .iter()
            .map(|f| {
                let p = paths::expand_user(f);
                let fmt = T::guess_format(&p, &kind);
                json!({"name": p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                       "format": fmt, "label": F::label(&fmt), "there": p.exists()})
            })
            .collect(),
    )
}

/// Import: each file in its turn, in the background (`ImportDialog._import`).
#[tauri::command]
pub fn xfer_import(
    kind: String,
    files: Vec<String>,
    replace: bool,
    folder: String,
    name: String,
) -> Result<(), String> {
    {
        let mut j = JOB.lock().map_err(err)?;
        if j.running {
            return Err("An export or import is running already.".into());
        }
        *j = Job {
            running: true,
            title: String::new(),
            text: String::new(),
            done: 0,
            total: 0,
            started: Some(std::time::Instant::now()),
            result: Value::Null,
        };
    }
    std::thread::spawn(move || {
        let lang = i18n::ui_language();
        let run = || -> Result<String, String> {
            let conn = if kind == "book" {
                Some(isnady_core::sources::store::connect_rw(None)?)
            } else {
                None
            };
            let request = Import {
                kind: kind.clone(),
                fmt: String::new(),
                replace,
                folder: folder.clone(),
                name: name.clone(),
            };
            let mut said = vec![];
            for (n, path) in files.iter().enumerate() {
                let file = paths::expand_user(path)
                    .file_name()
                    .map(|f| f.to_string_lossy().to_string())
                    .unwrap_or_default();
                let prefix = if files.len() > 1 {
                    format!("{}/{} · {file}: ", n + 1, files.len())
                } else {
                    String::new()
                };
                match T::import_file(conn.as_ref(), path, &request, &|m| {
                    say(&format!("{prefix}{m}"))
                }) {
                    Ok(m) => said.push(format!("{prefix}{m}")),
                    Err(e) if files.len() == 1 => return Err(e),
                    Err(e) => said.push(format!("{prefix}{}{e}", i18n::t(&lang, "Failed: ", &[]))),
                }
            }
            Ok(said.join("\n"))
        };
        let out = run();
        cache::clear();
        match &out {
            Ok(m) => {
                isnady_core::log::info("import", &format!("{kind}: {}", m.replace('\n', " | ")))
            }
            Err(e) => isnady_core::log::error("import", &format!("{kind}: {e}")),
        }
        if let Ok(mut j) = JOB.lock() {
            j.running = false;
            j.result = match out {
                Ok(m) => json!({"ok": true, "message": m}),
                Err(e) => json!({"ok": false, "message": e}),
            };
        }
    });
    Ok(())
}
