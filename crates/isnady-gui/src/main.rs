//! isnady-rs — isnady's window in Rust (Tauri 2). The pages are drawn by the system's web view (ui/), so Arabic,
//! right-to-left text and the Amiri font render exactly; everything they show comes from isnady-core, the same core as
//! the command line iy-rs, reading the same data folder and database as the Python isnady.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use isnady_core::narrators::Person;
use isnady_core::{
    bookmarks, books, config, db, genealogy, huffaz, i18n, isnad, learn, lives, name_parts, names,
    narrators, paths, people, py, quran, readmarks, sahaba, sahifa, scholars, search, sira, tables,
    works, VERSION,
};
mod help_page;
mod sources_page;
mod stats_page;
mod transfer_page;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Mutex;

struct Data(Mutex<Option<db::Connection>>);

fn with_db<T>(
    state: &tauri::State<Data>,
    f: impl FnOnce(&db::Connection) -> Result<T, String>,
) -> Result<T, String> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    if guard.is_none() {
        *guard = Some(db::connect(None).map_err(|e| e.to_string())?);
    }
    f(guard.as_ref().expect("connection"))
}

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// What the window needs to start: the version, the interface language and its catalog, the theme, the text size,
/// the order of the sections, what the frame needs (data/window.json), the books and how many hadith there are.
#[tauri::command]
fn start(state: tauri::State<Data>, window: tauri::WebviewWindow) -> Value {
    let lang = i18n::ui_language();
    // the profile in use in the title, as the Python window has it (PRF1)
    let person = config::profile_name(&config::active_profile());
    let _ = window.set_title(&if person.is_empty() {
        "isnady".to_string()
    } else {
        format!("isnady — {person}")
    });
    let mut out = json!({
        "version": VERSION,
        "data_dir": paths::shown(&paths::data_dir()),
        "ui_language": lang,
        "rtl": i18n::rtl(&lang),
        "catalog": serde_json::from_str::<Value>(i18n::catalog_json(&lang)).unwrap_or(json!({})),
        "theme": config::get_str("view.theme", "system"),
        "text_scale": config::get_f64("view.text_scale", 1.0),
        "sidebar": config::get_str("view.sidebar", ""),
        "content_languages": config::get_str("view.content_languages", ""),
        "window": serde_json::from_str::<Value>(i18n::window_json()).unwrap_or(json!({})),
        "settings": look_settings(),
        "books": [],
        "hadith": 0,
        "hadith_all": 0,
        "collections": 0,
        "error": Value::Null,
        // a page to open at once, for screenshots and tests: "search:WORDS" or "chain:BOOK:NUMBER"
        "open": std::env::var("ISNADY_RS_OPEN").unwrap_or_default(),
    });
    // a new version reads the chains or identifies the narrators again, once: in the background (search_page._ready)
    if let Ok(behind) = with_db(&state, |conn| {
        Ok(!isnady_core::search::index_ready(conn)
            || !isnady_core::reader::store::isnads_ready(conn)
            || (narrators::loaded(conn) && !narrators::links_ready(conn)))
    }) {
        out["catch_up"] = json!(behind);
    }
    // an import cut short the last time (ROB1): said on the Search page (check_interrupted_import)
    if let Some((titles, started)) = isnady_core::sources::imports::interrupted() {
        let mut names = titles
            .iter()
            .take(6)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        if titles.len() > 6 {
            names.push_str(" …");
        }
        out["interrupted"] = json!(i18n::t(
            &lang,
            "An import did not finish ({when}): {names}. The books imported before it are kept; import the rest again \
             in File → Data Sources.",
            &[("when", started.replace('T', " ")), ("names", names)]
        ));
    }
    match with_db(&state, |conn| books::all_books(conn, false).map_err(err)) {
        Ok(list) => {
            let total: i64 = list.iter().map(|b| b.count).sum();
            out["hadith"] = json!(total);
            out["books"] = Value::Array(
                list.into_iter()
                    .map(|b| json!({"key": b.key, "name": b.name, "tradition": b.tradition, "count": b.count}))
                    .collect(),
            );
        }
        Err(e) => out["error"] = json!(e),
    }
    // the footer: every hadith in every book, as the Python window counts them
    if let Ok((hadith, books)) = with_db(&state, |conn| {
        let count = |sql: &str| conn.query_row(sql, [], |r| r.get::<_, i64>(0)).map_err(err);
        Ok((
            count("SELECT COUNT(*) FROM hadiths")?,
            count("SELECT COUNT(*) FROM collections")?,
        ))
    }) {
        out["hadith_all"] = json!(hadith);
        out["collections"] = json!(books);
    }
    out
}

/// The settings of the look (config.DEFAULTS): the interface's typeface and size, each script's typeface, size,
/// colour and line height; those not set are left out (the window takes the defaults of data/window.json).
fn look_settings() -> Value {
    let text: serde_json::Map<String, Value> = ["arabic", "latin", "cyrillic", "bengali", "tamil"]
        .iter()
        .map(|script| {
            let mut one = serde_json::Map::new();
            for (key, number) in [
                ("family", false),
                ("size", true),
                ("color", false),
                ("line_height", true),
            ] {
                let name = format!("text.{script}.{key}");
                if let Some(v) = config::get(&name) {
                    let v = if number {
                        json!(v
                            .as_f64()
                            .or_else(|| v.as_str().and_then(|s| s.parse().ok())))
                    } else {
                        v
                    };
                    one.insert(key.to_string(), v);
                }
            }
            (script.to_string(), Value::Object(one))
        })
        .collect();
    // the user's own colours of the Lapis pair (theme.apply: colors.light.*, colors.dark.*)
    let mut colors = serde_json::Map::new();
    for mode in ["light", "dark"] {
        let mut one = serde_json::Map::new();
        for (token, _) in &config::known().color_tokens {
            let v = config::py_str(&config::effective(&format!("colors.{mode}.{token}")));
            if !v.is_empty() {
                one.insert(token.clone(), json!(v));
            }
        }
        colors.insert(mode.to_string(), Value::Object(one));
    }
    json!({
        "ui.family": config::get_str("ui.family", ""),
        "ui.size": config::get_f64("ui.size", 0.0),
        "text": text,
        "colors": colors,
    })
}

// ------------------------------------------------------------------ Edit → Preferences (gui/preferences.py)

/// Preferences: every setting in effect, what the tabs list, the books and the languages of the texts.
#[tauri::command]
fn prefs_state(state: tauri::State<Data>) -> Result<Value, String> {
    let k = config::known();
    let values: serde_json::Map<String, Value> = config::all_values()
        .into_iter()
        .map(|(key, v, _)| (key, v))
        .collect();
    let (books, languages) = with_db(&state, |conn| {
        let books = books::all_books(conn, true).map_err(err)?;
        Ok((books, content_languages_of(conn)?))
    })
    .unwrap_or_default();
    let canon = |t: &str| {
        if t == "sunni" || t == "shia" {
            t.to_string()
        } else {
            "other".to_string()
        }
    };
    let groups: Vec<Value> = [("sunni", "Sunni"), ("shia", "Shia"), ("other", "Other")]
        .iter()
        .filter_map(|(t, label)| {
            let mine: Vec<Value> = books
                .iter()
                .filter(|b| canon(&b.tradition) == *t)
                .map(|b| json!({"key": b.key, "name": works::book_title(&b.key, &b.name, &i18n::ui_language()), "count": b.count, "active": b.active}))
                .collect();
            (!mine.is_empty()).then(|| json!({"tradition": t, "label": label, "books": mine}))
        })
        .collect();
    Ok(json!({
        "values": values,
        "scripts": k.scripts.iter().map(|(key, (label, sample, family, size, line))| json!({"key": key, "label": label,
            "sample": sample, "family": family, "size": size, "line": line})).collect::<Vec<_>>(),
        "color_tokens": k.color_tokens,
        "sidebar": config::sidebar_order(),
        "settings_path": paths::shown(&config::settings_path()),
        "books": groups,
        "languages": languages,
        "content": config::py_str(&config::effective("view.content_languages")),
        "folds": config::py_str(&config::effective("view.other_languages")) != "omit",
        "look": look_settings(),
    }))
}

/// Preferences: one setting stored (config.set, validated); the look as it now is.
#[tauri::command]
fn prefs_set(key: String, value: Value) -> Result<Value, String> {
    config::set(&key, value)?;
    if key == "view.content_languages" {
        config::state_forget("reader.languages.");
    }
    Ok(look_settings())
}

/// Preferences: the settings under a prefix back to their defaults ("" = all: Reset appearance).
#[tauri::command]
fn prefs_reset(prefix: String) -> Result<Value, String> {
    config::reset(&prefix)?;
    Ok(look_settings())
}

/// Preferences → Reset: back to zero (core.reset): the settings, the books, or everything.
#[tauri::command]
fn reset_data(state: tauri::State<Data>, kind: String) -> Result<Value, String> {
    if kind == "settings" {
        let n = config::reset_settings()?;
        isnady_core::log::warning("reset", &format!("⟲ every setting reset ({n} stored)"));
        return Ok(json!({"n": n, "look": look_settings(), "sidebar": config::sidebar_order()}));
    }
    // the books (core.reset): the window's own connection is let go first, so VACUUM can make the file small
    if let Ok(mut guard) = state.0.lock() {
        *guard = None;
    }
    let conn = isnady_core::sources::store::connect_rw(None)?;
    let n = if kind == "books" {
        isnady_core::sources::store::reset_books(&conn).map_err(err)?
    } else {
        isnady_core::sources::store::reset_everything(&conn)
            .map_err(err)?
            .books
    };
    Ok(json!({"n": n, "look": look_settings(), "sidebar": config::sidebar_order()}))
}

/// Preferences → Books: a book switched on or off (books.set_active).
#[tauri::command]
fn book_active(keys: Vec<String>, on: bool) -> Result<(), String> {
    let mut off = books::disabled();
    for k in keys {
        if on {
            off.remove(&k);
        } else {
            off.insert(k);
        }
    }
    let mut list: Vec<String> = off.into_iter().collect();
    list.sort();
    config::set("books.disabled", json!(list.join(",")))
}

/// A setting the menus change, stored as the Python isnady stores it (config.set), with what goes with it: the
/// glossary follows the interface language where it has one; every book follows new content languages.
#[tauri::command]
fn set_setting(key: String, value: Value) -> Result<(), String> {
    let value = match (key.as_str(), value) {
        ("view.text_scale", Value::Number(n)) => {
            let v = (n.as_f64().unwrap_or(1.0) * 100.0).round() / 100.0;
            json!(v)
        }
        ("view.theme" | "view.ui_language" | "view.content_languages", Value::String(s)) => {
            Value::String(s)
        }
        _ => return Err(format!("not a setting the window changes: {key}")),
    };
    if key == "view.theme" {
        let known: Value = serde_json::from_str(i18n::window_json()).map_err(err)?;
        let ok = value == "system"
            || known["themes"]
                .as_array()
                .is_some_and(|a| a.iter().any(|t| t[0] == value));
        if !ok {
            return Err(format!("no theme {value}"));
        }
    }
    config::set(&key, value.clone())?;
    if key == "view.ui_language" {
        let known: Value = serde_json::from_str(i18n::window_json()).map_err(err)?;
        if known["learn_languages"]
            .as_array()
            .is_some_and(|a| a.contains(&value))
        {
            config::state_set("learn.language", value);
        }
    }
    if key == "view.content_languages" {
        config::state_forget("reader.languages.");
    }
    Ok(())
}

/// File → Profiles: every profile, the one in use marked.
#[tauri::command]
fn profiles() -> Vec<config::Profile> {
    config::all_profiles()
}

#[tauri::command]
fn use_profile(id: String) -> Result<(), String> {
    config::switch_profile(&id)
}

/// File → Profiles → New Profile… (main_window._new_profile): its id, to use it at once.
#[tauri::command]
fn profile_new(name: String) -> Result<String, String> {
    config::create_profile(name.trim(), None)
}

/// File → Profiles → Rename This Profile….
#[tauri::command]
fn profile_rename(name: String) -> Result<(), String> {
    config::rename_profile(&config::active_profile(), name.trim())
}

/// File → Profiles → Delete a Profile… (not the first one, not the one in use).
#[tauri::command]
fn profile_delete(id: String) -> Result<(), String> {
    config::delete_profile(&id)
}

/// File → Profiles → Export This Profile…: the file written.
#[tauri::command]
fn profile_export(path: String) -> Result<String, String> {
    let mut path = path.trim().to_string();
    if !path.ends_with(config::PROFILE_EXTENSION) {
        path.push_str(config::PROFILE_EXTENSION);
    }
    config::export_profile(&config::active_profile(), &path).map(|p| p.display().to_string())
}

/// File → Profiles → Import a Profile…: (id, name) of the new profile.
#[tauri::command]
fn profile_import(path: String) -> Result<Value, String> {
    let id = config::import_profile(path.trim(), None)?;
    Ok(json!({"id": id, "name": config::profile_name(&id)}))
}

/// The system's own file dialog (QFileDialog in the Python window): "save" a file, "open" one or several, or a
/// "folder"; the paths chosen, [] when it was closed. `suggested` is the file or folder offered first; `filters`
/// (name, [extensions]) the kinds of file shown.
#[tauri::command]
async fn choose_path(
    app: tauri::AppHandle,
    mode: String,
    title: String,
    suggested: String,
    filters: Vec<(String, Vec<String>)>,
    multiple: bool,
) -> Vec<String> {
    use tauri_plugin_dialog::DialogExt;
    let mut d = app.dialog().file().set_title(&title);
    let start = paths::expand_user(&suggested);
    if !suggested.is_empty() {
        if mode == "folder" || start.is_dir() {
            d = d.set_directory(&start);
        } else {
            if let Some(dir) = start.parent().filter(|p| p.is_dir()) {
                d = d.set_directory(dir);
            }
            if let Some(name) = start.file_name() {
                d = d.set_file_name(name.to_string_lossy());
            }
        }
    }
    for (name, exts) in &filters {
        let exts: Vec<&str> = exts.iter().map(String::as_str).collect();
        d = d.add_filter(name, &exts);
    }
    let shown =
        |p: tauri_plugin_dialog::FilePath| p.into_path().ok().map(|p| p.display().to_string());
    tauri::async_runtime::spawn_blocking(move || match mode.as_str() {
        "save" => d.blocking_save_file().and_then(shown).into_iter().collect(),
        "folder" => d
            .blocking_pick_folder()
            .and_then(shown)
            .into_iter()
            .collect(),
        _ if multiple => d
            .blocking_pick_files()
            .unwrap_or_default()
            .into_iter()
            .filter_map(shown)
            .collect(),
        _ => d.blocking_pick_file().and_then(shown).into_iter().collect(),
    })
    .await
    .unwrap_or_default()
}

/// Where a file is suggested: the home folder.
#[tauri::command]
fn home_folder() -> String {
    paths::expand_user("~").display().to_string()
}

/// The languages of the imported texts, Arabic first, then the order chosen (search.list_languages).
#[tauri::command]
fn content_languages(state: tauri::State<Data>) -> Result<Vec<String>, String> {
    with_db(&state, |conn| {
        let mut stmt = conn
            .prepare("SELECT DISTINCT language FROM editions WHERE language IS NOT NULL AND text_count > 0 ORDER BY language")
            .map_err(err)?;
        let mut list: Vec<String> = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(err)?
            .filter_map(Result::ok)
            .collect();
        let chosen = books::language_order();
        list.sort_by_key(|l| books::language_rank(l, &chosen));
        Ok(list)
    })
}

/// A folder or a web page opened by the system, as QDesktopServices.openUrl does.
fn open_external(target: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    let mut cmd = std::process::Command::new("explorer");
    #[cfg(target_os = "macos")]
    let mut cmd = std::process::Command::new("open");
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let mut cmd = std::process::Command::new("xdg-open");
    cmd.arg(target).spawn().map(|_| ()).map_err(err)
}

/// File → Open Data Folder.
#[tauri::command]
fn open_data_folder() -> Result<(), String> {
    open_external(&paths::shown(&paths::data_dir()))
}

// ------------------------------------------------------------------ Tools and Help → Log (RS18)

/// Tools → Clear the Cache: the kept files deleted and what is in memory forgotten (MainWindow._clear_cache).
#[tauri::command]
fn cache_clear() -> Value {
    let folder = paths::data_dir().join("cache");
    let files: Vec<std::path::PathBuf> = std::fs::read_dir(&folder)
        .map(|d| {
            d.flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x == "json"))
                .collect()
        })
        .unwrap_or_default();
    let size: u64 = files
        .iter()
        .filter_map(|p| p.metadata().ok())
        .map(|m| m.len())
        .sum();
    isnady_core::log::info("cache", "⟲ kept files cleared");
    isnady_core::cache::clear();
    let n = files
        .iter()
        .filter(|p| std::fs::remove_file(p).is_ok())
        .count();
    json!({"n": n, "size": size as f64 / 1024.0})
}

/// Tools → Create Desktop Shortcut… (MainWindow._create_shortcut).
#[tauri::command]
fn shortcut_create() -> Result<Value, String> {
    let (program, how) = isnady_core::shortcut::launch_command();
    let made = isnady_core::shortcut::create(true, true)?;
    Ok(
        json!({"made": made.iter().map(|(w, p)| json!([w, paths::shown(p)])).collect::<Vec<_>>(),
        "program": paths::shown(&program), "how": how}),
    )
}

/// Help → Log: the files, newest first, each with its kind, time and size (LogViewer._fill_files).
#[tauri::command]
fn log_files() -> Vec<Value> {
    use isnady_core::log;
    let mut files = log::files();
    if files.is_empty() {
        files.push(log::path());
    }
    files
        .iter()
        .map(|f| {
            let name = f.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let (stamp, size) = match f.metadata() {
                Ok(m) => (
                    m.modified().map(|t| py::local_time(t, "%Y-%m-%d %H:%M")).unwrap_or_default(),
                    format!("{} KB", py::thousands(py::round_even(m.len() as f64 / 1024.0) as i64)),
                ),
                Err(_) => (String::new(), String::new()),
            };
            json!({"path": paths::shown(f), "name": name, "crash": name.starts_with("crash_"), "stamp": stamp, "size": size})
        })
        .collect()
}

/// Help → Log: a file read, only a level and above or the lines holding a text (log.read).
#[tauri::command]
fn log_read(path: String, level: String, text: String) -> Value {
    let p = std::path::PathBuf::from(&path);
    let size = p.metadata().map(|m| m.len()).unwrap_or(0);
    json!({"body": isnady_core::log::read(&p, 0, &level, &text), "size": size})
}

/// Help → Log: Delete what is older, Clear the log, Open the folder.
#[tauri::command]
fn log_change(action: String, day: Option<String>) -> Result<Value, String> {
    use isnady_core::log;
    match action.as_str() {
        "before" => Ok(json!(log::delete_before(day.as_deref().unwrap_or("")))),
        "clear" => Ok(json!(log::clear())),
        "folder" => {
            let _ = std::fs::create_dir_all(log::folder());
            open_external(&paths::shown(&log::folder())).map(|_| Value::Null)
        }
        _ => Ok(json!(format!(
            "isnady {VERSION} (Rust) · {} · {}",
            std::env::consts::OS,
            std::env::consts::ARCH
        ))),
    }
}

/// Help → isnady on GitHub, Report an Issue; the links of the dialogs. Web pages only.
#[tauri::command]
fn open_url(url: String) -> Result<(), String> {
    if !url.starts_with("https://") {
        return Err(format!("not a web page: {url}"));
    }
    open_external(&url)
}

/// File → Quit.
#[tauri::command]
fn quit(app: tauri::AppHandle) {
    app.exit(0);
}

/// View → Full Screen (F11): on, or off again.
#[tauri::command]
fn full_screen(window: tauri::WebviewWindow) -> Result<(), String> {
    let on = window.is_fullscreen().map_err(err)?;
    window.set_fullscreen(!on).map_err(err)
}

/// Help → About isnady: the versions and the places, as the Python dialog shows them.
#[tauri::command]
fn about(state: tauri::State<Data>) -> Value {
    let sqlite = with_db(&state, |conn| {
        conn.query_row("SELECT sqlite_version()", [], |r| r.get::<_, String>(0))
            .map_err(err)
    })
    .unwrap_or_default();
    let fts5 = db::Connection::open_in_memory()
        .and_then(|c| c.execute_batch("CREATE VIRTUAL TABLE t USING fts5(x, tokenize='trigram')"))
        .is_ok();
    json!({
        "rust": env!("ISNADY_RUSTC"),
        "os": std::env::consts::OS,
        "tauri": tauri::VERSION,
        "webview": tauri::webview_version().unwrap_or_default(),
        "sqlite": sqlite,
        "fts5": fts5,
        "data_dir": paths::shown(&paths::data_dir()),
        "db_path": paths::shown(&paths::db_path()),
    })
}

/// Help → Licences: the data sources in this database.
#[tauri::command]
fn licences(state: tauri::State<Data>) -> Value {
    let sources = with_db(&state, |conn| {
        let mut stmt = conn
            .prepare("SELECT name, license, tier, origin, location FROM sources ORDER BY origin, name")
            .map_err(err)?;
        let rows = stmt
            .query_map([], |r| {
                Ok(json!({
                    "name": r.get::<_, Option<String>>(0)?, "license": r.get::<_, Option<String>>(1)?,
                    "tier": r.get::<_, Option<String>>(2)?, "origin": r.get::<_, Option<String>>(3)?,
                    "location": r.get::<_, Option<String>>(4)?,
                }))
            })
            .map_err(err)?
            .filter_map(Result::ok)
            .collect::<Vec<_>>();
        Ok(rows)
    })
    .unwrap_or_default();
    json!({"sources": sources})
}

/// What the Search page needs before a search (gui/search_page.py _fill_filters, _show_start): the books grouped
/// Sunni, Shia and other (core.books.grouped), the languages of the texts and those the data repository has more
/// of, the recent searches, how many hadith there are, whether any narrators are imported.
#[tauri::command]
fn search_start(state: tauri::State<Data>) -> Result<Value, String> {
    let lang = i18n::ui_language();
    let names: HashMap<String, String> = serde_json::from_str::<Value>(i18n::window_json())
        .ok()
        .and_then(|w| serde_json::from_value(w["catalog_languages"].clone()).ok())
        .unwrap_or_default();
    with_db(&state, |conn| {
        let all = books::all_books(conn, false).map_err(err)?;
        let mut groups = vec![];
        for (t, label) in [("sunni", "Sunni"), ("shia", "Shia"), ("other", "Other")] {
            let list: Vec<Value> = all
                .iter()
                .filter(|b| {
                    let road = if ["sunni", "shia"].contains(&b.tradition.as_str()) {
                        b.tradition.as_str()
                    } else {
                        "other"
                    };
                    road == t
                })
                .map(|b| json!({"key": b.key, "name": works::book_title(&b.key, &b.name, &lang), "count": b.count}))
                .collect();
            if !list.is_empty() {
                groups.push(json!({"tradition": t, "label": label, "books": list}));
            }
        }
        let languages = content_languages_of(conn)?;
        let more = books::more_languages(conn, &languages, &names);
        let hadith: i64 = conn
            .query_row("SELECT COUNT(*) FROM hadiths", [], |r| r.get(0))
            .map_err(err)?;
        Ok(json!({
            "groups": groups,
            "languages": languages,
            "more": more,
            "recent": search::recent_items().into_iter().map(|(text, kind)| json!({"text": text, "kind": kind})).collect::<Vec<_>>(),
            "hadith": hadith,
            "persons": narrators::loaded(conn),
        }))
    })
}

/// The recent searches (S4), the newest first; and forgetting them.
#[tauri::command]
fn recent_searches() -> Vec<Value> {
    search::recent_items()
        .into_iter()
        .map(|(text, kind)| json!({"text": text, "kind": kind}))
        .collect()
}

#[tauri::command]
fn clear_recent_searches() -> usize {
    search::clear_recent()
}

/// A search not of the hadith (the Narrators and Scholars of the drop-down) is remembered too.
#[tauri::command]
fn remember_search(text: String, kind: String) {
    search::add_recent(text.trim(), &kind);
}

/// Search → Narrators or Scholars (S3, search_page._search_people): the narrators of both traditions, or one, the
/// closest first (narrators.find), or the scholars (scholars.search); with how many there are and how long it took.
#[tauri::command]
fn find_people(
    state: tauri::State<Data>,
    kind: String,
    text: String,
    tradition: String,
) -> Result<Value, String> {
    let lang = i18n::ui_language();
    let started = std::time::Instant::now();
    with_db(&state, |conn| {
        if kind == "scholars" {
            let found = scholars::search(conn, text.trim());
            let rows: Vec<Value> = found.iter().map(|f| scholar_json(f, &lang)).collect();
            return Ok(json!({"rows": rows, "total": rows.len(),
                             "ms": started.elapsed().as_millis() as i64}));
        }
        let langs = narrators::Langs {
            reading: &lang,
            words: &lang,
        };
        let road = Some(tradition.as_str()).filter(|t| !t.is_empty());
        let (rows, total) = narrators::find(conn, text.trim(), road, langs).map_err(err)?;
        Ok(
            json!({"rows": rows, "total": total, "read": narrators::SEARCH_RERANK,
                  "ms": started.elapsed().as_millis() as i64}),
        )
    })
}

/// The next narrators found, after those read (NR-B1, search_page._read_more_people).
#[tauri::command]
fn more_people(
    state: tauri::State<Data>,
    text: String,
    tradition: String,
    offset: usize,
    limit: usize,
) -> Result<Value, String> {
    let lang = i18n::ui_language();
    with_db(&state, |conn| {
        let langs = narrators::Langs {
            reading: &lang,
            words: &lang,
        };
        let road = Some(tradition.as_str()).filter(|t| !t.is_empty());
        let rows =
            narrators::find_more(conn, text.trim(), road, offset, limit, langs).map_err(err)?;
        Ok(json!({"rows": rows}))
    })
}

/// A scholar found, as his card shows him (people_results.scholar_card).
fn scholar_json(f: &scholars::Found, lang: &str) -> Value {
    let s = f.scholar;
    let shown = scholars::short_name(s, lang);
    let view = scholars::name_view(s, lang, lang);
    let fuller = if lang == "tr" {
        s.tr.clone()
    } else {
        s.full.clone()
    };
    json!({
        "id": s.id, "short": shown, "fuller": fuller, "dates": s.dates, "works": s.works, "roles": f.roles,
        "close": f.close, "arabic": s.arabic,
        "head": if s.known_ar.is_empty() { view.arabic } else { s.known_ar.clone() },
    })
}

fn content_languages_of(conn: &db::Connection) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare("SELECT DISTINCT language FROM editions WHERE language IS NOT NULL AND text_count > 0 ORDER BY language")
        .map_err(err)?;
    let mut list: Vec<String> = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(err)?
        .filter_map(Result::ok)
        .collect();
    let chosen = books::language_order();
    list.sort_by_key(|l| books::language_rank(l, &chosen));
    Ok(list)
}

/// The verses the hadith name, counted once per change of the database (quran.cites, core.cache).
struct Cites(Mutex<Option<(i64, quran::Cites)>>);

/// The script of a language for its typeface (config.script_for_language).
fn script_of(language: &str, direction: &str, of_language: &HashMap<String, String>) -> String {
    if let Some(s) = of_language.get(&language.to_lowercase()) {
        return s.clone();
    }
    if direction == "rtl" {
        "arabic".into()
    } else {
        "latin".into()
    }
}

fn text_json(t: &search::TextHit, of_language: &HashMap<String, String>) -> Value {
    json!({
        "language": t.language,
        "direction": t.direction,
        "text": t.text,
        "spans": t.spans,
        "edition_key": t.edition_key,
        "script": script_of(&t.language, &t.direction, of_language),
    })
}

/// One chain as the strip of a card draws it (chain_widgets.ChainStrip): each link with its short name, its term,
/// the narrator identified (his verdict, tabaqa, death) or why none; whom it reaches; the words of each language under
/// each name.
fn strip_json(ch: &isnad::Chain, people: &HashMap<i64, Person>, codes: &[String]) -> Value {
    let links: Vec<Value> = ch
        .links
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let (ar, meaning) = isnad::term_label(&l.transmission);
            let nxt = ch
                .links
                .get(i + 1)
                .map(|x| x.raw_name.as_str())
                .unwrap_or("");
            let person = l.person_id.and_then(|p| people.get(&p)).map(|p| {
                let v = p.verdicts.first();
                json!({
                    "id": p.id,
                    "display_name": p.display_name,
                    "tabaqa": p.tabaqa,
                    "death": p.death_year_ah,
                    "verdict": v.map(|v| json!({
                        "scheme": v.rank_scheme, "critic_name": v.critic_name, "phrase": v.phrase,
                        "rank": v.rank, "rank_label": v.rank_label,
                    })),
                })
            });
            json!({
                "raw": l.raw_name,
                "short": isnad::short_name(&l.raw_name),
                "transmission": l.transmission,
                "term_ar": ar,
                "term_meaning": meaning,
                "person": person,
                "candidates": l.candidates,
                "note": isnad::link_note(&l.raw_name, nxt),
            })
        })
        .collect();
    let words: Vec<Vec<String>> = codes
        .iter()
        .map(|c| isnad::chain_word_parts(ch, c, people))
        .filter(|w| !w.is_empty())
        .collect();
    json!({
        "links": links,
        "problem": ch.problem,
        "reaches_prophet": ch.reaches_prophet,
        "ends_with": ch.ends_with,
        "raised": ch.raised,
        "end_label": isnad::end_label(ch),
        "masum": isnad::masum_label(if ch.ends_with.is_empty() { "prophet" } else { &ch.ends_with }),
        "words": words,
    })
}

/// A search of the hadith with every filter of the page (gui/search_page.py _query), each hadith as its card
/// shows it (make_card): the texts in the content languages and any text the search matched, the others folded;
/// its chains with their narrators; its grades; the other narrations; the verses it names; its ☆.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
fn find(
    state: tauri::State<Data>,
    cites: tauri::State<Cites>,
    text: String,
    mode: String,
    whole_words: bool,
    books: Vec<String>,
    languages: Vec<String>,
    grades: Vec<String>,
    every: bool,
    sahihayn: bool,
    chapters: Vec<i64>,
    offset: usize,
    limit: usize,
    remember: bool,
) -> Result<Value, String> {
    if text.trim().is_empty() {
        return Ok(json!({"total": 0, "results": [], "elapsed_ms": 0, "spellings": ""}));
    }
    if remember {
        search::add_recent(text.trim(), "hadith");
    }
    let lang = i18n::ui_language();
    with_db(&state, |conn| {
        let mut q = search::SearchQuery::new(text.trim());
        q.mode = mode;
        q.whole_words = whole_words;
        q.collections = books;
        q.languages = languages;
        q.grades = grades;
        q.every_grader = every;
        q.sahihayn = sahihayn;
        q.chapters = chapters;
        q.offset = offset;
        q.limit = limit;
        if q.mode == "meaning" {
            // by meaning (YZ1a): the cards with their closeness; why not, when it cannot run yet
            let page = match isnady_core::semantic::search(conn, &q, 0.10) {
                Ok(p) => p,
                Err(e) => {
                    return Ok(
                        json!({"meaning_error": e, "not_built": e == isnady_core::semantic::NOT_BUILT}),
                    )
                }
            };
            let cx = CardCtx::new(conn, &cites, &lang)?;
            let mut results = vec![];
            for (r, score) in page.results.iter().zip(&page.scores) {
                let mut card = card_json(conn, r, &cx, false)?;
                card["score"] = json!(format!("{score:.2}"));
                results.push(card);
            }
            let stale = isnady_core::semantic::status(conn)["stale"] == json!(true);
            return Ok(json!({
                "total": page.total, "elapsed_ms": page.elapsed_ms as u64, "used_index": true, "spellings": "",
                "before_grades": 0, "terms": 0, "results": results, "meaning": true, "stale": stale,
            }));
        }
        let page = search::search(conn, &q).map_err(err)?;
        let cx = CardCtx::new(conn, &cites, &lang)?;
        let mut results = vec![];
        for r in &page.results {
            results.push(card_json(conn, r, &cx, false)?);
        }
        let spellings: Vec<String> = page
            .spellings
            .iter()
            .map(|(w, alts)| {
                let shown: Vec<&str> = alts.iter().take(4).map(String::as_str).collect();
                format!(
                    "{w} → {}{}",
                    shown.join(", "),
                    if alts.len() > 4 { " …" } else { "" }
                )
            })
            .collect();
        Ok(json!({
            "total": page.total,
            "elapsed_ms": page.elapsed_ms as u64,
            "used_index": page.used_index,
            "spellings": spellings.join("; "),
            "before_grades": page.before_grades,
            "terms": page.terms.len(),
            "results": results,
        }))
    })
}

/// What every hadith card needs besides the hadith (search_page.make_card): the interface language, the scripts of
/// the languages, the compilers, the content languages, whether the others fold, the verses each hadith names.
struct CardCtx {
    lang: String,
    of_language: HashMap<String, String>,
    compilers: Value,
    shown: Vec<String>,
    folds: bool,
    verses_of: quran::Cites,
}

impl CardCtx {
    fn new(
        conn: &db::Connection,
        cites: &tauri::State<Cites>,
        lang: &str,
    ) -> Result<CardCtx, String> {
        let window: Value = serde_json::from_str(i18n::window_json()).map_err(err)?;
        let shown: Vec<String> = config::get_str("view.content_languages", "")
            .split(',')
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();
        let folds = config::get_str("view.other_languages", "fold") != "omit";
        let verses_of = if quran::available() {
            let version: i64 = conn
                .query_row("PRAGMA data_version", [], |r| r.get(0))
                .map_err(err)?;
            let mut kept = cites.0.lock().map_err(err)?;
            if kept.as_ref().map_or(true, |(v, _)| *v != version) {
                *kept = Some((version, quran::cites(conn).map_err(err)?));
            }
            kept.as_ref().map(|(_, c)| c.clone()).unwrap_or_default()
        } else {
            HashMap::new()
        };
        Ok(CardCtx {
            lang: lang.to_string(),
            of_language: serde_json::from_value(window["script_of_language"].clone())
                .unwrap_or_default(),
            compilers: window["compilers"].clone(),
            shown,
            folds,
            verses_of,
        })
    }
}

/// One hadith as its card draws it (ResultCard); in_book: in the reader of its book — every text of the languages
/// asked for, none folded (search_page.make_card(in_book=True)).
fn card_json(
    conn: &db::Connection,
    r: &search::SearchResult,
    cx: &CardCtx,
    in_book: bool,
) -> Result<Value, String> {
    let (lang, of_language, compilers, shown, folds, verses_of) = (
        &cx.lang,
        &cx.of_language,
        &cx.compilers,
        &cx.shown,
        cx.folds,
        &cx.verses_of,
    );
    // the content languages (L1): the texts chosen and any the search matched; the others folded or left out
    let kept: Vec<&search::TextHit> = r
        .texts
        .iter()
        .filter(|t| shown.is_empty() || shown.contains(&t.language) || !t.spans.is_empty())
        .collect();
    let (texts, folded): (Vec<&search::TextHit>, Vec<&search::TextHit>) = if in_book {
        (r.texts.iter().collect(), vec![])
    } else if !kept.is_empty() && kept.len() != r.texts.len() {
        let others = if folds {
            r.texts
                .iter()
                .filter(|t| !kept.iter().any(|k| std::ptr::eq(*k, *t)))
                .collect()
        } else {
            vec![]
        };
        (kept, others)
    } else {
        (r.texts.iter().collect(), vec![])
    };
    let chains = isnad::chain(conn, r.hadith_id).map_err(err)?;
    let mut people: HashMap<i64, Person> = HashMap::new();
    for ch in &chains {
        for l in &ch.links {
            if let Some(pid) = l.person_id {
                if let std::collections::hash_map::Entry::Vacant(e) = people.entry(pid) {
                    if let Some(p) = narrators::describe(conn, pid) {
                        e.insert(p);
                    }
                }
            }
        }
    }
    let mut codes: Vec<String> = vec![];
    for t in &texts {
        let code = match t.language.as_str() {
            "Turkish" => "tr",
            "English" => "en",
            _ => continue,
        };
        if !codes.iter().any(|c| c == code) {
            codes.push(code.into());
        }
    }
    let direct = "no chain: the compiler gives the words without the narrators between";
    let drawn: Vec<Value> = chains
        .iter()
        .filter(|c| !c.links.is_empty() || c.problem.as_deref() == Some(direct))
        .map(|c| strip_json(c, &people, &codes))
        .collect();
    let name = works::book_title(&r.collection_key, &r.collection_name, lang);
    let first = r
        .texts
        .iter()
        .find(|x| x.language != "Arabic")
        .or(r.texts.first())
        .map(|x| x.text.split_whitespace().collect::<Vec<_>>().join(" "))
        .unwrap_or_default();
    let reference = format!("{}:{}", r.collection_key, r.number);
    Ok(json!({
        "hadith_id": r.hadith_id,
        "key": r.collection_key,
        "name": name,
        "number": r.number,
        "texts": texts.iter().map(|t| text_json(t, of_language)).collect::<Vec<_>>(),
        "folded": folded.iter().map(|t| text_json(t, of_language)).collect::<Vec<_>>(),
        "grades": r.grades.iter().map(|(g, v)| json!([g, v, isnady_core::grades::group(v)])).collect::<Vec<_>>(),
        "related": isnady_core::takhrij::related(conn, r.hadith_id, lang).iter().map(|x| json!({
            "hadith_id": x.hadith_id, "kind": x.kind, "book": x.book, "book_name": x.book_name,
            "number": x.number,
        })).collect::<Vec<_>>(),
        "verses": verses_of.get(&r.hadith_id).map(|v| v.iter().map(|&(s, a, b)| json!([s, a, b,
            quran::label(s, a, b, lang)])).collect::<Vec<_>>()).unwrap_or_default(),
        "chains": drawn,
        "compiler": compilers.get(&r.collection_key).cloned().unwrap_or(Value::Null),
        "codes": codes,
        "bookmarked": bookmarks::is_marked("hadith", &reference),
        "subtitle": first.chars().take(160).collect::<String>(),
        "in_book": in_book,
    }))
}

/// ☆ / ★ of a card (BookmarkButton): marked now?
#[tauri::command]
fn toggle_bookmark(
    kind: String,
    reference: String,
    title: String,
    subtitle: String,
) -> Result<bool, String> {
    bookmarks::toggle(&kind, &reference, &title, &subtitle).map_err(err)
}

#[tauri::command]
fn is_bookmarked(kind: String, reference: String) -> bool {
    bookmarks::is_marked(&kind, &reference)
}

// ------------------------------------------------------------------ the chains brought up to date (search_page._CatchUp)

struct CatchUp {
    running: bool,
    text: String,
    count: i64,
    error: String,
    /// the words said as they are (the AI indexes say them translated already)
    raw: bool,
    /// what the job says when it is done ("ai")
    done: String,
}

static CATCH: std::sync::Mutex<CatchUp> = std::sync::Mutex::new(CatchUp {
    running: false,
    text: String::new(),
    count: 0,
    error: String::new(),
    raw: false,
    done: String::new(),
});

/// What building the AI indexes says, translated already (search_page.build_meaning_index).
fn ai_say(m: &str) {
    if let Ok(mut c) = CATCH.lock() {
        c.text = m.to_string();
        c.count = 0;
        c.raw = true;
    }
}

fn catch_say(m: &str) {
    if let Ok(mut c) = CATCH.lock() {
        // "Reading isnads of 99263 hadith": the words to translate (as the catalog has them) and the number apart
        let digits: String = m.chars().filter(|c| c.is_ascii_digit()).collect();
        c.count = digits.parse().unwrap_or(0);
        c.text = if digits.is_empty() {
            m.to_string()
        } else {
            let name = if m.starts_with("Indexing") {
                "{missing}"
            } else {
                "{todo}"
            };
            m.replacen(&digits, name, 1)
        };
    }
}

/// On a connection of its own, the window staying usable: after a new version (job "start", search_page._CatchUp)
/// ensure_index → ensure_isnads → link_narrators; from the Tools menu the search index built again ("index",
/// rebuild_index), every chain read again ("chains", rebuild_chains) or every narrator identified again
/// ("narrators", identify_narrators).
#[tauri::command]
fn catch_up(job: Option<String>) {
    let job = job.unwrap_or_else(|| "start".into());
    {
        let Ok(mut c) = CATCH.lock() else {
            return;
        };
        if c.running {
            return;
        }
        c.running = true;
        c.error = String::new();
        c.text = String::new();
        c.raw = false;
        c.done = String::new();
    }
    std::thread::spawn(move || {
        let started = std::time::Instant::now();
        let run = || -> Result<(), String> {
            let conn = isnady_core::sources::store::connect_rw(None)?;
            match job.as_str() {
                "index" => {
                    isnady_core::search::rebuild_index(&conn, &catch_say).map_err(err)?;
                }
                "chains" => {
                    isnady_core::reader::store::ensure_isnads(&conn, &catch_say, true)
                        .map_err(err)?;
                }
                "narrators" => {
                    isnady_core::reader::matcher::link_narrators(&conn, &catch_say, true)?;
                }
                // Tools → Build AI Indexes (Meaning, Takhrij)
                "ai" => {
                    let lang = i18n::ui_language();
                    let meta = isnady_core::semantic::build(
                        &conn,
                        isnady_core::semantic::DEFAULT_DIMS,
                        &ai_say,
                    )
                    .map_err(|e| {
                        i18n::t(&lang, "Meaning index failed: {error}", &[("error", e)])
                    })?;
                    let found = isnady_core::takhrij::build(&conn, &ai_say).map_err(|e| {
                        i18n::t(&lang, "Meaning index failed: {error}", &[("error", e)])
                    })?;
                    let n = |v: &Value| isnady_core::py::thousands(v.as_i64().unwrap_or(0));
                    let done = i18n::t(
                        &lang,
                        "AI indexes ready: meaning ({documents:,} hadith, {concepts} concepts); takhrij ({same:,} + {report:,} pairs)",
                        &[
                            ("documents:,", n(&meta["documents"])),
                            ("concepts", meta["dims"].to_string()),
                            ("same:,", n(&found["same"])),
                            ("report:,", n(&found["same_report"])),
                        ],
                    );
                    if let Ok(mut c) = CATCH.lock() {
                        c.done = done;
                    }
                }
                _ => {
                    isnady_core::search::ensure_index(&conn, &catch_say).map_err(err)?;
                    isnady_core::reader::store::ensure_isnads(&conn, &catch_say, false)
                        .map_err(err)?;
                    isnady_core::reader::matcher::link_narrators(&conn, &catch_say, false)?;
                }
            }
            Ok(())
        };
        let result = run();
        if let Err(e) = &result {
            isnady_core::log::error("window", &format!("bringing the chains up to date: {e}"));
        } else {
            isnady_core::log::info(
                "window",
                &format!(
                    "chains brought up to date in {}",
                    isnady_core::log::since(started)
                ),
            );
        }
        isnady_core::cache::clear();
        if let Ok(mut c) = CATCH.lock() {
            c.running = false;
            c.error = result.err().unwrap_or_default();
        }
    });
}

#[tauri::command]
fn catch_up_status() -> Value {
    match CATCH.lock() {
        Ok(c) => {
            json!({"running": c.running, "text": c.text, "count": c.count, "error": c.error, "raw": c.raw,
            "done": c.done})
        }
        Err(_) => json!({"running": false, "text": "", "count": 0, "error": ""}),
    }
}

// ------------------------------------------------------------------ Isnad Chains (gui/isnad_page.py)

/// The overview of the page: per book, the chains read, split, reaching the Prophet (or a Ma'sum), narrators per chain,
/// identified, and why the rest were kept whole (isnad_page._show_overview, _stats_card).
#[tauri::command]
fn chains_overview(state: tauri::State<Data>) -> Result<Value, String> {
    let lang = i18n::ui_language();
    with_db(&state, |conn| {
        if !isnad::isnads_ready(conn) {
            return Ok(json!({"books": [], "not_read": true}));
        }
        let known = isnad::known_by_book(conn).map_err(err)?;
        let stats = isnad::book_stats(conn).map_err(err)?;
        let books: Vec<Value> = stats
            .iter()
            .map(|b| {
                let k = known.get(&b.key);
                json!({
                    "key": b.key, "name": works::book_title(&b.key, &b.stored_name, &lang),
                    "tradition": b.tradition, "chains": b.chains, "split": b.split, "marfu": b.marfu,
                    "links": b.links, "problems": b.problems,
                    "known": k.filter(|k| k.2 != 0).map(|k| json!([k.0, k.1, k.2])),
                })
            })
            .collect();
        Ok(json!({"books": books}))
    })
}

/// The numbers of a book (HadithNavigator.set_book): first, last and how many; None for a book without hadith.
#[tauri::command]
fn book_numbers(state: tauri::State<Data>, book: String) -> Result<Value, String> {
    with_db(&state, |conn| {
        Ok(match isnad::number_range(conn, &book).map_err(err)? {
            Some(r) => json!({"first": r.first, "last": r.last, "count": r.count}),
            None => Value::Null,
        })
    })
}

/// The book's numbers that begin with what is typed (the number field's completer).
#[tauri::command]
fn numbers_starting(
    state: tauri::State<Data>,
    book: String,
    prefix: String,
) -> Result<Vec<String>, String> {
    with_db(&state, |conn| {
        Ok(isnad::numbers_starting(conn, &book, &prefix, 12))
    })
}

/// Where a number stands among the book's hadith: [place, count], or None.
#[tauri::command]
fn number_place(
    state: tauri::State<Data>,
    book: String,
    number: String,
) -> Result<Option<(i64, i64)>, String> {
    with_db(&state, |conn| Ok(isnad::position(conn, &book, &number)))
}

/// The number before or after one in the book's order.
#[tauri::command]
fn step_number(
    state: tauri::State<Data>,
    book: String,
    number: String,
    delta: i32,
) -> Result<Option<String>, String> {
    with_db(&state, |conn| {
        Ok(isnad::step_number(conn, &book, &number, delta))
    })
}

/// A number typed on the page (isnad_page._go): the hadith, or why there is none — the nearest number, how the
/// book writes its numbers, the range.
#[tauri::command]
fn hadith_by_number(
    state: tauri::State<Data>,
    book: String,
    number: String,
) -> Result<Value, String> {
    with_db(&state, |conn| {
        if let Some(id) = isnad::hadith_id(conn, &book, &number) {
            return Ok(json!({"id": id}));
        }
        let sample: Vec<String> = conn
            .prepare(
                "SELECT h.number FROM hadiths h JOIN collections c ON c.id = h.collection_id WHERE c.key = ? \
                 ORDER BY h.number_sort LIMIT 3",
            )
            .and_then(|mut st| st.query_map([&book], |r| r.get(0))?.collect())
            .map_err(err)?;
        let range = isnad::number_range(conn, &book)
            .map_err(err)?
            .map(|r| json!({"first": r.first, "last": r.last}));
        Ok(json!({
            "id": null, "near": isnad::nearest_number(conn, &book, &number), "sample": sample, "range": range,
        }))
    })
}

/// A link of a chain that names no one person, in words (`isnad.link_note`), in the interface language.
fn link_note_in(raw: &str, next_raw: &str, lang: &str) -> String {
    match isnad::link_kind(raw) {
        "group" => {
            let members = isnad::group_members(raw, next_raw);
            if members.is_empty() {
                i18n::t(lang, "A group of al-Kulayni's teachers", &[])
            } else {
                i18n::t(
                    lang,
                    "A group of al-Kulayni's teachers: {members} (as al-Kulayni explained it)",
                    &[("members", members.join("، "))],
                )
            }
        }
        "previous" => i18n::t(
            lang,
            "\"From him\": the narrator who heads the previous chain in the book",
            &[],
        ),
        "vague" => i18n::t(
            lang,
            "A narrator not named (mubham): the chain does not say who",
            &[],
        ),
        _ => String::new(),
    }
}

/// Whom a chain reaches, in words (`isnad.end_label`), in the interface language.
fn end_label_in(ch: &isnad::Chain, lang: &str) -> String {
    let key = if !ch.ends_with.is_empty() {
        ch.ends_with.clone()
    } else if ch.reaches_prophet {
        "prophet".into()
    } else {
        return String::new();
    };
    let label = if key == "prophet" {
        i18n::t(lang, "the Prophet ﷺ", &[])
    } else if key == "imam" {
        i18n::t(lang, "an Imam", &[])
    } else {
        let m = isnad::masum_label(&key);
        if m.is_empty() {
            key.clone()
        } else {
            m
        }
    };
    if ch.raised {
        i18n::t(lang, "raised (رفعه) to {whom}", &[("whom", label)])
    } else {
        label
    }
}

/// An identified narrator of a chain, as its node shows him (chain_widgets.ChainNode).
fn node_person(conn: &db::Connection, id: i64, lang: &str) -> Option<Value> {
    let d = people::describe(conn, id)?;
    let view = narrators::name_view(conn, id, &d.person.name_ar, page_langs(lang));
    let v = d.verdicts.first();
    Some(json!({
        "id": id, "display_name": d.person.display_name, "name_ar": d.person.name_ar,
        "other_names": d.other_names.iter().take(8).collect::<Vec<_>>(),
        "reading": view.reading, "full_reading": view.full_reading, "how": view.how,
        "latin": names::latin(&d.person.display_name, "en", 8).unwrap_or_default(),
        "tabaqa": d.person.tabaqa, "tabaqa_label": i18n::t(lang, &d.tabaqa_label, &[]),
        "death": d.person.death_year_ah, "death_note": d.death_year_note,
        "verdict": v.map(|v| json!({
            "critic_name": v.critic_name, "work": v.work, "phrase": v.phrase, "rank": v.rank,
            "shia": v.rank_scheme.as_deref() == Some("shia"),
            "label": if v.rank_label.is_empty() { String::new() } else { i18n::t(lang, &v.rank_label, &[]) },
            "madhhab": if v.madhhab_label.is_empty() { String::new() } else { i18n::t(lang, &v.madhhab_label, &[]) },
        })),
    }))
}

/// One hadith's page (isnad_page.show_hadith): the book and number, the road, each chain with its nodes, the Arabic
/// text, and the chain in words in the languages the texts are shown in.
#[tauri::command]
fn chain_page(state: tauri::State<Data>, id: i64) -> Result<Value, String> {
    let lang = i18n::ui_language();
    let code = if lang == "tr" { "tr" } else { "en" };
    with_db(&state, |conn| {
        let Some(info) = isnad::describe(conn, id, &lang) else {
            return Ok(Value::Null);
        };
        let tradition: String = conn
            .query_row(
                "SELECT COALESCE(tradition, 'sunni') FROM collections WHERE key = ?",
                [&info.book],
                |r| r.get(0),
            )
            .map_err(err)?;
        let chains = isnad::chain(conn, id).map_err(err)?;
        let mut people: HashMap<i64, Person> = HashMap::new();
        let mut shown: HashMap<i64, Value> = HashMap::new();
        for ch in &chains {
            for l in &ch.links {
                if let Some(pid) = l.person_id {
                    if let std::collections::hash_map::Entry::Vacant(e) = people.entry(pid) {
                        if let Some(p) = narrators::describe(conn, pid) {
                            e.insert(p);
                        }
                        if let Some(v) = node_person(conn, pid, &lang) {
                            shown.insert(pid, v);
                        }
                    }
                }
            }
        }
        let t = tables::get();
        let out: Vec<Value> = chains
            .iter()
            .map(|ch| {
                let names = isnad::chain_link_names(ch, code, &people);
                let links: Vec<Value> = ch
                    .links
                    .iter()
                    .enumerate()
                    .map(|(i, l)| {
                        let (ar, meaning) = isnad::term_label(&l.transmission);
                        let nxt = ch
                            .links
                            .get(i + 1)
                            .map(|x| x.raw_name.as_str())
                            .unwrap_or("");
                        let (reading, spelled) = isnad::name_reading(&l.raw_name, &lang);
                        json!({
                            "position": l.position, "raw": l.raw_name, "short": isnad::short_name(&l.raw_name),
                            "term": l.transmission, "term_ar": ar, "term_meaning": i18n::t(&lang, &meaning, &[]),
                            "term_why": i18n::t(&lang, &isnad::term_why(&l.transmission), &[]),
                            "note": link_note_in(&l.raw_name, nxt, &lang),
                            "reading": reading, "spelled": spelled,
                            "name": names.get(i).cloned().unwrap_or_default(),
                            "person": l.person_id.and_then(|p| shown.get(&p).cloned()),
                            "confidence": l.match_confidence, "candidates": l.candidates,
                        })
                    })
                    .collect();
                json!({
                    "problem": ch.problem, "links": links, "raw": ch.raw,
                    "reaches_prophet": ch.reaches_prophet, "ends_with": ch.ends_with, "raised": ch.raised,
                    "end": end_label_in(ch, &lang),
                    "jamian": isnady_core::normalize::normalize(ch.raw.as_deref().unwrap_or("")).contains("جميعا"),
                    "meeting": ch.meeting.as_ref().map(|m| json!({
                        "first_position": m.first_position, "said": m.said, "attested": m.attested,
                    })),
                })
            })
            .collect();
        // under the Arabic, each road in the languages the texts are shown in (ISN-TR1)
        let codes = isnad::chain_codes(
            &config::get_str("view.content_languages", "")
                .split(',')
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect::<Vec<_>>(),
        );
        let words: Vec<Value> = codes
            .iter()
            .map(|c| {
                let roads: Vec<String> = chains
                    .iter()
                    .filter(|ch| {
                        (ch.problem.is_none() || !ch.links.is_empty())
                            && (!ch.links.is_empty()
                                || ch.problem.as_deref() == Some(t.isnad.direct.as_str()))
                    })
                    .map(|ch| isnad::chain_in_words(ch, c, &people))
                    .collect();
                json!({"code": c, "roads": roads})
            })
            .collect();
        Ok(json!({
            "info": {"id": info.id, "number": info.number, "book": info.book, "book_name": info.book_name,
                     "prev": info.prev, "next": info.next},
            "tradition": tradition, "chains": out, "text": isnad::arabic_text(conn, id), "words": words,
            "problems": {"borrowed": t.isnad.borrowed, "direct": t.isnad.direct, "note": t.isnad.note,
                         "continued": t.isnad.continued, "no_chain": t.isnad.no_chain, "no_term": t.isnad.no_term},
        }))
    })
}

/// "From the contents…" (chapter_picker.ChapterPicker): a book's chapters as a tree, the one holding a hadith.
#[tauri::command]
fn contents_tree(
    state: tauri::State<Data>,
    book: String,
    hadith: Option<i64>,
) -> Result<Value, String> {
    let lang = i18n::ui_language();
    with_db(&state, |conn| {
        let Some(work) = works::find_work(conn, &book, &lang).map_err(err)? else {
            return Ok(Value::Null);
        };
        fn walk(conn: &db::Connection, work: i64, parent: Option<i64>, lang: &str) -> Vec<Value> {
            works::containers(conn, work, parent, lang)
                .unwrap_or_default()
                .into_iter()
                .map(|c| {
                    let label = c.label.as_deref().filter(|l| !l.is_empty()).map(|l| format!("{l}. ")).unwrap_or_default()
                        + &c.title;
                    json!({
                        "id": c.id, "label": label, "leaves": c.leaves,
                        "children": if c.subchapters > 0 { walk(conn, work, Some(c.id), lang) } else { vec![] },
                    })
                })
                .collect()
        }
        let here = hadith.and_then(|h| works::chapter_of_hadith(conn, h));
        Ok(json!({"title": work.title, "tree": walk(conn, work.id, None, &lang), "here": here}))
    })
}

/// One chapter of the picker: where it stands, and its hadith with the start of their text and how far each chain
/// was read.
#[tauri::command]
fn contents_chapter(state: tauri::State<Data>, chapter: i64) -> Result<Value, String> {
    let lang = i18n::ui_language();
    with_db(&state, |conn| {
        let trail: Vec<String> = works::path(conn, chapter, &lang)
            .into_iter()
            .map(|n| {
                n.label
                    .as_deref()
                    .filter(|l| !l.is_empty())
                    .map(|l| format!("{l}. "))
                    .unwrap_or_default()
                    + &n.title
            })
            .collect();
        // the start of each text in the first language chosen for the texts after Arabic, else the interface's
        let chosen: Vec<String> = config::get_str("view.content_languages", "")
            .split(',')
            .filter(|s| !s.is_empty() && *s != "Arabic")
            .map(String::from)
            .collect();
        let language = chosen.first().cloned().or_else(|| match lang.as_str() {
            "tr" => Some("Turkish".into()),
            "en" => Some("English".into()),
            _ => None,
        });
        let rows = works::chapter_hadith(conn, chapter, language.as_deref(), 90).map_err(err)?;
        Ok(json!({
            "trail": trail,
            "hadith": rows.iter().map(|h| json!({"id": h.hadith_id, "number": h.number, "links": h.links, "words": h.words})).collect::<Vec<_>>(),
        }))
    })
}

// ------------------------------------------------------------------ Books (gui/books_page.py)

/// The shelf (BooksPage.refresh): every book with its kind, size, compiler and where it was left; the Qur'an's row;
/// the book read last.
#[tauri::command]
fn books_shelf(state: tauri::State<Data>) -> Result<Value, String> {
    let lang = i18n::ui_language();
    // the tree of every hadith collection built, or built again when its hadith or chapters changed
    if !with_db(&state, |conn| Ok(works::ready(conn)))? {
        let conn = isnady_core::sources::store::connect_rw(None)?;
        works::ensure_collection_works(&conn).map_err(err)?;
    }
    with_db(&state, |conn| {
        let list = works::list_works(conn, &lang).map_err(err)?;
        let labels = &works::library().kind_labels;
        let out: Vec<Value> = list
            .iter()
            .map(|w| {
                let author = w.author.as_deref().and_then(scholars::find).map(|s| {
                    if s.tr.is_empty() {
                        s.name.clone()
                    } else {
                        s.tr.clone()
                    }
                });
                json!({
                    "key": w.key, "title": w.title, "kind": w.kind, "tradition": w.tradition,
                    "kind_label": labels.get(&w.kind).cloned().unwrap_or_else(|| "Book".into()),
                    "leaves": w.leaves, "chapters": w.chapters, "author": author,
                    "last": readmarks::last(&w.key).map(|p| json!({"number": p.number, "place": p.place})),
                })
            })
            .collect();
        Ok(json!({
            "works": out,
            "quran": {"available": quran::available(), "translations": quran::translations()},
            "last_work": config::state_get("reader.last_work"),
        }))
    })
}

/// A book opened (BooksPage.open_work): its chapters in reading order, its contents as a tree, the languages of its
/// texts (the book's own choice, else the content languages), and the chapter to open — where it was left (with the
/// hadith), else the one read last, else the first.
#[tauri::command]
fn book_open(state: tauri::State<Data>, key: String) -> Result<Value, String> {
    let lang = i18n::ui_language();
    with_db(&state, |conn| {
        let Some(work) = works::find_work(conn, &key, &lang).map_err(err)? else {
            return Ok(Value::Null);
        };
        config::state_set("reader.last_work", json!(work.key));
        let order = works::reading_order(conn, work.id).map_err(err)?;
        let hadith = work.kind == "hadith";
        fn walk(
            conn: &db::Connection,
            work: i64,
            parent: Option<i64>,
            lang: &str,
            hadith: bool,
        ) -> Vec<Value> {
            works::containers(conn, work, parent, lang)
                .unwrap_or_default()
                .into_iter()
                .map(|c| {
                    let label = c
                        .label
                        .as_deref()
                        .filter(|l| !l.is_empty() && hadith)
                        .map(|l| format!("{l}. "))
                        .unwrap_or_default()
                        + &c.title;
                    json!({
                        "id": c.id, "label": label, "leaves": c.leaves,
                        "children": if c.subchapters > 0 { walk(conn, work, Some(c.id), lang, hadith) } else { vec![] },
                    })
                })
                .collect()
        }
        let tree = walk(conn, work.id, None, &lang, hadith);
        let mut languages: Vec<Value> = vec![];
        if hadith {
            let mut langs: Vec<String> = conn
                .prepare(
                    "SELECT DISTINCT e.language FROM editions e JOIN collections c ON c.id = e.collection_id \
                     WHERE c.key = ?",
                )
                .and_then(|mut st| st.query_map([&work.key], |r| r.get(0))?.collect())
                .map_err(err)?;
            let chosen = books::language_order();
            langs.sort_by_key(|l| books::language_rank(l, &chosen));
            let content: Vec<String> = config::get_str("view.content_languages", "")
                .split(',')
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect();
            let shows = |l: &String| content.is_empty() || content.contains(l);
            let saved: Vec<String> = config::state_get(&format!("reader.languages.{}", work.key))
                .and_then(|v| serde_json::from_value(v).ok())
                .filter(|v: &Vec<String>| !v.is_empty())
                .unwrap_or_else(|| {
                    let shown: Vec<String> = langs.iter().filter(|l| shows(l)).cloned().collect();
                    if shown.is_empty() {
                        langs.clone()
                    } else {
                        shown
                    }
                });
            languages = langs
                .iter()
                .map(|l| json!({"name": l, "checked": saved.contains(l)}))
                .collect();
        }
        let mut start: (Option<i64>, Option<i64>) = (None, None);
        if let Some(p) = readmarks::last(&work.key) {
            let (at, hid) = readmarks::resolve(conn, &p);
            if at.is_some_and(|a| order.contains(&a)) {
                start = (at, hid);
            }
        }
        if start.0.is_none() {
            let remembered = config::state_get(&format!("reader.position.{}", work.key))
                .and_then(|v| v.as_i64())
                .filter(|c| order.contains(c));
            start.0 = remembered.or(order.first().copied());
        }
        Ok(json!({
            "key": work.key, "title": work.title, "kind": work.kind, "tradition": work.tradition,
            "order": order, "tree": tree, "languages": languages,
            "start": {"chapter": start.0, "hadith": start.1},
        }))
    })
}

/// A chapter opened (BooksPage.open_chapter): where it stands, its ☆, the hadith number to show, and its items —
/// those with every word looked for in their texts (Find in this chapter). The place is remembered.
#[tauri::command]
fn book_chapter(
    state: tauri::State<Data>,
    key: String,
    chapter: i64,
    find: String,
    focus_hadith: Option<i64>,
) -> Result<Value, String> {
    let lang = i18n::ui_language();
    with_db(&state, |conn| {
        let Some(work) = works::find_work(conn, &key, &lang).map_err(err)? else {
            return Ok(Value::Null);
        };
        let hadith = work.kind == "hadith";
        config::state_set(&format!("reader.position.{}", work.key), json!(chapter));
        let order = works::reading_order(conn, work.id).map_err(err)?;
        let mut crumbs = vec![work.title.clone()];
        crumbs.extend(works::path(conn, chapter, &lang).into_iter().map(|p| {
            p.label
                .as_deref()
                .filter(|l| !l.is_empty() && hadith)
                .map(|l| format!("{l}. "))
                .unwrap_or_default()
                + &p.title
        }));
        let position = order.iter().position(|c| *c == chapter).map(|i| i + 1);
        let place = readmarks::place_of(conn, &work.key, Some(chapter), focus_hadith);
        readmarks::remember(
            &place.book,
            &place.chapter,
            &place.number,
            &place.title,
            place.place,
        );
        let (items, total) = works::leaves(conn, chapter, 0, -1).map_err(err)?;
        let number = if hadith {
            focus_hadith
                .or_else(|| items.iter().find_map(|i| i.hadith_id))
                .and_then(|h| {
                    conn.query_row("SELECT number FROM hadiths WHERE id = ?", [h], |r| {
                        r.get::<_, String>(0)
                    })
                    .ok()
                })
        } else {
            None
        };
        let wanted: Vec<String> = isnady_core::normalize::normalize(&find)
            .split_whitespace()
            .map(String::from)
            .collect();
        let matches = |it: &works::Leaf| -> bool {
            if wanted.is_empty() {
                return true;
            }
            let text = if it.kind == "hadith" {
                works::texts_of(conn, it.hadith_id.unwrap_or(0))
            } else {
                it.text
                    .clone()
                    .filter(|t| !t.is_empty())
                    .or_else(|| it.title.clone())
                    .unwrap_or_default()
            };
            let folded = isnady_core::normalize::normalize(&text);
            wanted.iter().all(|w| folded.contains(w.as_str()))
        };
        let shown: Vec<Value> = items
            .iter()
            .filter(|it| matches(it))
            .map(|it| {
                let reading = it.person_id.map(|_| {
                    let title = it.title.as_deref().unwrap_or("").trim();
                    let view = name_parts::present(
                        &name_parts::parse(title, None, &[], None, None),
                        &lang,
                        &lang,
                    );
                    if view.full_reading.is_empty() {
                        names::latin(title, "en", 8).unwrap_or_default()
                    } else {
                        view.full_reading
                    }
                });
                json!({
                    "kind": it.kind, "hadith_id": it.hadith_id, "person_id": it.person_id, "label": it.label,
                    "title": it.title, "text": if it.kind == "hadith" { None } else { it.text.clone() },
                    "reading": reading,
                })
            })
            .collect();
        let star = bookmarks::chapter_ref(conn, chapter, &lang).map(|(r, t, s)| {
            json!({"ref": r.clone(), "title": t, "subtitle": s, "marked": bookmarks::is_marked("chapter", &r)})
        });
        Ok(json!({
            "crumbs": crumbs, "position": position, "count": order.len(), "total": total, "items": shown,
            "number": number, "star": star,
        }))
    })
}

/// The cards of some hadith of a chapter, in the languages chosen for the book.
#[tauri::command]
fn book_cards(
    state: tauri::State<Data>,
    cites: tauri::State<Cites>,
    ids: Vec<i64>,
    languages: Vec<String>,
) -> Result<Vec<Value>, String> {
    let lang = i18n::ui_language();
    with_db(&state, |conn| {
        let cx = CardCtx::new(conn, &cites, &lang)?;
        let mut q = search::SearchQuery::new("");
        q.languages = languages;
        let mut out = vec![];
        for id in ids {
            let r = search::load_result(conn, id, &[], &q).map_err(err)?;
            out.push(card_json(conn, &r, &cx, true)?);
        }
        Ok(out)
    })
}

/// The languages ticked for one book (reader.languages.<book>).
#[tauri::command]
fn book_languages(key: String, languages: Vec<String>) {
    config::state_set(&format!("reader.languages.{key}"), json!(languages));
}

/// Where the reader has come to: the hadith at the top of the reading pane.
#[tauri::command]
fn book_remember(
    state: tauri::State<Data>,
    key: String,
    chapter: i64,
    hadith: Option<i64>,
) -> Result<(), String> {
    with_db(&state, |conn| {
        let p = readmarks::place_of(conn, &key, Some(chapter), hadith);
        readmarks::remember(&p.book, &p.chapter, &p.number, &p.title, p.place);
        Ok(())
    })
}

/// The book and chapter of a hadith (BooksPage.show_hadith), of a narrator's entry (show_person), of a chapter
/// (show_chapter).
#[tauri::command]
fn book_place_of(
    state: tauri::State<Data>,
    hadith: Option<i64>,
    person: Option<i64>,
    chapter: Option<i64>,
) -> Result<Value, String> {
    let lang = i18n::ui_language();
    with_db(&state, |conn| {
        let at = match (hadith, person, chapter) {
            (Some(h), _, _) => works::chapter_of_hadith(conn, h),
            (_, Some(p), _) => works::chapter_of_person(conn, p),
            (_, _, c) => c,
        };
        let Some(at) = at else {
            return Ok(Value::Null);
        };
        let Some(node) = works::node(conn, at, &lang) else {
            return Ok(Value::Null);
        };
        let work = works::list_works(conn, &lang)
            .map_err(err)?
            .into_iter()
            .find(|w| w.id == node.work_id);
        Ok(work.map_or(
            Value::Null,
            |w| json!({"key": w.key, "tradition": w.tradition, "kind": w.kind, "chapter": at}),
        ))
    })
}

/// A chapter's ☆ (bookmarks.chapter_ref): its reference, title and subtitle.
#[tauri::command]
fn book_chapter_ref(state: tauri::State<Data>, chapter: i64) -> Result<Value, String> {
    let lang = i18n::ui_language();
    with_db(&state, |conn| {
        Ok(bookmarks::chapter_ref(conn, chapter, &lang).map_or(
            Value::Null,
            |(r, t, s)| json!({"ref": r, "title": t, "subtitle": s}),
        ))
    })
}

/// 🔖 Markers (BooksPage._fill_marks_menu): the reader's markers and where each book was left, in words.
#[tauri::command]
fn marks_list(state: tauri::State<Data>) -> Result<Value, String> {
    let lang = i18n::ui_language();
    with_db(&state, |conn| {
        let titles: HashMap<String, String> = works::list_works(conn, &lang)
            .map_err(err)?
            .into_iter()
            .map(|w| (w.key, w.title))
            .collect();
        let title = |b: &str| titles.get(b).cloned().unwrap_or_default();
        let mine: Vec<Value> = readmarks::marks()
            .iter()
            .map(|m| {
                json!({"id": m.id, "name": m.name, "book": m.book,
                            "where": readmarks::describe(m, &title(&m.book), &lang)})
            })
            .collect();
        let left: Vec<Value> = readmarks::recent(10)
            .iter()
            .filter(|p| titles.contains_key(&p.book))
            .map(|p| json!({"book": p.book, "where": readmarks::describe(p, &title(&p.book), &lang)}))
            .collect();
        Ok(json!({"marks": mine, "left": left}))
    })
}

/// What a marker put here would be called (the place in words).
#[tauri::command]
fn mark_suggest(
    state: tauri::State<Data>,
    key: String,
    chapter: i64,
    hadith: Option<i64>,
) -> Result<String, String> {
    let lang = i18n::ui_language();
    with_db(&state, |conn| {
        let title = works::find_work(conn, &key, &lang)
            .map_err(err)?
            .map(|w| w.title)
            .unwrap_or_default();
        let p = readmarks::place_of(conn, &key, Some(chapter), hadith);
        Ok(readmarks::describe(&p, &title, &lang))
    })
}

#[tauri::command]
fn mark_add(
    state: tauri::State<Data>,
    name: String,
    key: String,
    chapter: i64,
    hadith: Option<i64>,
) -> Result<(), String> {
    with_db(&state, |conn| {
        let p = readmarks::place_of(conn, &key, Some(chapter), hadith);
        readmarks::add(&name, &p.book, &p.chapter, &p.number, &p.title, p.place);
        Ok(())
    })
}

#[tauri::command]
fn mark_rename(id: String, name: String) -> bool {
    readmarks::rename(&id, &name)
}

#[tauri::command]
fn mark_remove(id: String) -> bool {
    readmarks::remove(&id)
}

/// Every marker removed; how many there were.
#[tauri::command]
fn marks_remove_all() -> usize {
    let mine = readmarks::marks();
    for m in &mine {
        if let Some(id) = &m.id {
            readmarks::remove(id);
        }
    }
    mine.len()
}

/// Forget where one book (or every book) was left; how many.
#[tauri::command]
fn place_forget(book: Option<String>) -> usize {
    readmarks::forget(book.as_deref())
}

/// A marker (by its id) or the place a book was left (by the book), as a book, a chapter and a hadith.
#[tauri::command]
fn mark_resolve(
    state: tauri::State<Data>,
    id: Option<String>,
    book: Option<String>,
) -> Result<Value, String> {
    with_db(&state, |conn| {
        let mark = match (id, book) {
            (Some(i), _) => readmarks::marks()
                .into_iter()
                .find(|m| m.id.as_deref() == Some(i.as_str())),
            (_, Some(b)) => readmarks::last(&b),
            _ => None,
        };
        let Some(mark) = mark else {
            return Ok(Value::Null);
        };
        let (chapter, hadith) = readmarks::resolve(conn, &mark);
        Ok(chapter.map_or(
            Value::Null,
            |c| json!({"book": mark.book, "chapter": c, "hadith": hadith}),
        ))
    })
}

/// The languages the Narrators page reads in: names and words in the interface's.
fn page_langs(lang: &str) -> narrators::Langs<'_> {
    narrators::Langs {
        reading: lang,
        words: lang,
    }
}

/// The page's filters as the window sends them: a generation, a rank (-1: no judgment), a book.
fn filters_of(tabaqa: Option<i64>, rank: Option<i64>, book: Option<String>) -> narrators::Filters {
    narrators::Filters {
        tabaqa,
        rank,
        book: book.filter(|b| !b.is_empty()),
    }
}

/// Narrators: the overview at the top of the list (narrators_page.refresh): per tradition shown, how many narrators
/// from which works, how many names of its chains are identified.
#[tauri::command]
fn narrators_overview(state: tauri::State<Data>, tradition: String) -> Result<Value, String> {
    with_db(&state, |conn| {
        let roads: Vec<&str> = if tradition.is_empty() {
            vec!["sunni", "shia"]
        } else {
            vec![tradition.as_str()]
        };
        let mut out = Vec::new();
        for road in roads {
            let (persons, links, identified, works) =
                narrators::overview(conn, road).map_err(err)?;
            out.push(
                json!({"road": road, "persons": persons, "links": links, "identified": identified,
                            "works": works}),
            );
        }
        Ok(json!({"roads": out}))
    })
}

/// Narrators: the list (narrators_page._populate / _near_end): the narrators of a tradition (or both) with the
/// filters, most often in the chains first, or the words typed with the famous ones named any way people write them
/// first; offset > 0: the next ones (NR-B1).
#[tauri::command]
#[allow(clippy::too_many_arguments)]
fn narrators_list(
    state: tauri::State<Data>,
    text: String,
    tradition: String,
    tabaqa: Option<i64>,
    rank: Option<i64>,
    book: Option<String>,
    offset: usize,
    limit: usize,
) -> Result<Value, String> {
    let lang = i18n::ui_language();
    with_db(&state, |conn| {
        let langs = page_langs(&lang);
        let road = Some(tradition.as_str()).filter(|t| !t.is_empty());
        let filters = filters_of(tabaqa, rank, book);
        let (mut rows, mut total) = narrators::browse_by(
            conn, &text, &filters, road, None, None, limit, offset, langs,
        )
        .map_err(err)?;
        if offset == 0 && !text.trim().is_empty() {
            let first = people::famous_ids(conn, &people::famous_named(&text, road));
            if !first.is_empty() {
                let (mut ahead, _n) = narrators::browse_by(
                    conn,
                    "",
                    &filters,
                    road,
                    Some(&first),
                    None,
                    first.len(),
                    0,
                    langs,
                )
                .map_err(err)?;
                let place: HashMap<i64, usize> =
                    first.iter().enumerate().map(|(i, p)| (*p, i)).collect();
                ahead.sort_by_key(|r| place.get(&r.id).copied().unwrap_or(first.len()));
                let listed: std::collections::HashSet<i64> = rows.iter().map(|r| r.id).collect();
                total += ahead.iter().filter(|r| !listed.contains(&r.id)).count();
                let known: std::collections::HashSet<i64> = ahead.iter().map(|r| r.id).collect();
                rows.retain(|r| !known.contains(&r.id));
                ahead.extend(rows);
                rows = ahead;
            }
        }
        let titles = people::famous_titles(conn, &lang);
        for r in rows.iter_mut() {
            r.famous = titles.get(&r.id).cloned();
        }
        Ok(json!({"rows": rows, "total": total}))
    })
}

/// Narrators: the famous ones of the tradition shown, group by group (narrators_page._show_famous, FS1).
#[tauri::command]
fn narrators_famous(state: tauri::State<Data>, tradition: String) -> Result<Value, String> {
    let lang = i18n::ui_language();
    with_db(&state, |conn| {
        let langs = page_langs(&lang);
        let titles = people::famous_titles(conn, &lang);
        let road = Some(tradition.as_str()).filter(|t| !t.is_empty());
        let mut groups = Vec::new();
        for g in people::famous_groups(road) {
            let ids = people::famous_ids(conn, &g.keys);
            if ids.is_empty() {
                continue;
            }
            let (mut rows, _n) = narrators::browse_by(
                conn,
                "",
                &narrators::Filters::default(),
                Some(g.tradition.as_str()),
                Some(&ids),
                None,
                ids.len(),
                0,
                langs,
            )
            .map_err(err)?;
            let place: HashMap<i64, usize> = ids.iter().enumerate().map(|(i, p)| (*p, i)).collect();
            rows.sort_by_key(|r| place.get(&r.id).copied().unwrap_or(ids.len()));
            for r in rows.iter_mut() {
                r.famous = titles.get(&r.id).cloned();
            }
            groups.push(json!({"title": g.title, "rows": rows}));
        }
        Ok(json!({"groups": groups}))
    })
}

/// A narrator's names for the list's menu and the bookmark (bookmarks.narrator_ref): his key, the name he is
/// known by (read, else Arabic), his Arabic, the entry's heading.
fn narrator_ref(conn: &db::Connection, id: i64, lang: &str) -> Option<Value> {
    let (key, name_ar, road): (String, String, Option<String>) = conn
        .query_row(
            "SELECT key, name_ar, tradition FROM persons WHERE id = ?",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .ok()?;
    let (reading, arabic) = lives::reading(conn, id, &name_ar, page_langs(lang));
    Some(
        json!({"key": key, "title": if reading.is_empty() { arabic.clone() } else { reading.clone() },
                "reading": reading, "arabic": arabic, "name_ar": name_ar,
                "tradition": road.unwrap_or_else(|| "sunni".into()),
                "marked": bookmarks::is_marked("narrator", &key)}),
    )
}

/// Narrators: the names of one narrator, for the list's right-click menu.
#[tauri::command]
fn narrator_names(state: tauri::State<Data>, id: i64) -> Result<Value, String> {
    let lang = i18n::ui_language();
    with_db(&state, |conn| {
        Ok(narrator_ref(conn, id, &lang).unwrap_or(Value::Null))
    })
}

/// Narrators: everything the right side shows of one narrator (narrators_page.show_person): his name laid out, his
/// life, what the critics say, Ibn Hajar's marks, teachers and students, the hadith.
#[tauri::command]
fn narrator_page(state: tauri::State<Data>, id: i64) -> Result<Value, String> {
    let lang = i18n::ui_language();
    with_db(&state, |conn| {
        let langs = page_langs(&lang);
        let Some(d) = people::describe(conn, id) else {
            return Ok(json!({"error": "no_person"}));
        };
        let view = narrators::name_view(conn, id, &d.person.name_ar, langs);
        let (life, written) = lives::life(conn, id, langs);
        let own = lives::of_person(conn, id).and_then(|l| l["id"].as_str().map(String::from));
        let have: Vec<String> = conn
            .prepare("SELECT key FROM collections")
            .and_then(|mut st| st.query_map([], |r| r.get::<_, String>(0))?.collect())
            .unwrap_or_default();
        let marks_of = &tables::get().narrators.collection_marks;
        let marks: Vec<Value> = d
            .marks
            .iter()
            .map(|(mark, meaning)| {
                let books: Vec<&String> = marks_of
                    .iter()
                    .filter(|(_k, m)| m.contains(mark) && mark != "ع" && mark != "4")
                    .map(|(k, _m)| k)
                    .collect();
                let book = (books.len() == 1 && have.contains(books[0])).then(|| books[0].clone());
                json!({"mark": mark, "meaning": i18n::t(&lang, meaning, &[]), "book": book})
            })
            .collect();
        let rel = people::relations(conn, id, 12).map_err(err)?;
        let side = |v: &Vec<people::Related>| -> Vec<Value> {
            v.iter()
                .map(|r| {
                    let (reading, _ar) = lives::reading(conn, r.id, &r.name, langs);
                    let short = if reading.is_empty() {
                        let mut parts: Vec<&str> = py::words(&r.name).into_iter().take(5).collect();
                        while parts
                            .last()
                            .is_some_and(|w| ["بن", "بنت", "ابن", "أبو", "أبي", "أم"].contains(w))
                        {
                            parts.pop();
                        }
                        parts.join(" ")
                    } else {
                        reading
                    };
                    json!({"id": r.id, "name": r.name, "short": short, "count": r.count})
                })
                .collect()
        };
        let (hadith, total) = people::hadiths_of(conn, id, 60, &lang).map_err(err)?;
        Ok(json!({
            "id": id, "view": view, "ref": narrator_ref(conn, id, &lang),
            "tradition": if d.tradition.is_empty() { "sunni".to_string() } else { d.tradition.clone() },
            "source": d.source, "name_ar": d.person.name_ar, "display_name": d.person.display_name,
            "tabaqa": d.person.tabaqa, "tabaqa_label": i18n::t(&lang, &d.tabaqa_label, &[]),
            "death": d.person.death_year_ah, "death_note": d.death_year_note,
            "verdicts": d.verdicts.iter().map(|v| json!({
                "critic": i18n::t(&lang, &v.critic_name, &[]), "work": v.work, "phrase": v.phrase, "rank": v.rank,
                "shia": v.rank_scheme.as_deref() == Some("shia"),
                "label": if v.rank_label.is_empty() { String::new() } else { i18n::t(&lang, &v.rank_label, &[]) },
                "madhhab": if v.madhhab_label.is_empty() { String::new() } else { i18n::t(&lang, &v.madhhab_label, &[]) },
            })).collect::<Vec<_>>(),
            "marks": marks,
            "life": life, "written": written,
            "life_id": own,
            "teachers": side(&rel.teachers), "students": side(&rel.students),
            "compilers": rel.compilers.iter().map(|(b, _k, n)| json!({"book": b, "count": n})).collect::<Vec<_>>(),
            "hadith": hadith.iter().map(|h| json!({"id": h.hadith_id, "key": h.key, "book": h.book, "number": h.number})).collect::<Vec<_>>(),
            "hadith_total": total,
        }))
    })
}

/// Narrators: a classical work's entry under a life (SourceEntry): its heading and text when the work is
/// downloaded, else whether it is and how big it is.
#[tauri::command]
fn source_entry(work: String, volume: i64, number: i64) -> Value {
    match lives::entry(&work, volume, number) {
        Some((heading, text)) => json!({"heading": heading, "text": text}),
        None => json!({"available": lives::available(&work), "size_mb": lives::work_size(&work)}),
    }
}

/// The classical work downloaded once and split into its entries (life_card: Download), in the background.
#[tauri::command]
async fn source_fetch(work: String) -> Result<usize, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let n = lives::fetch(&work, &|_| {});
        match &n {
            Ok(n) => isnady_core::log::info("sources", &format!("{work}: {n} entries downloaded")),
            Err(e) => isnady_core::log::error("sources", &format!("{work}: {e}")),
        }
        n
    })
    .await
    .map_err(|e| e.to_string())?
}

// ------------------------------------------------------------------ Hadith Scholars (gui/scholars_page.py)

/// Hadith Scholars: the scholars with something in this database, each with his roles, as the list shows him.
#[tauri::command]
fn scholars_list(state: tauri::State<Data>) -> Result<Value, String> {
    let lang = i18n::ui_language();
    with_db(&state, |conn| {
        let rows: Vec<Value> = scholars::present(conn)
            .into_iter()
            .map(|(s, roles)| {
                let short = scholars::short_name(s, &lang);
                let fuller = if lang == "tr" {
                    s.tr.clone()
                } else if i18n::rtl(&lang) {
                    String::new()
                } else {
                    s.full.clone()
                };
                json!({"id": s.id, "short": short, "fuller": fuller, "full": s.full, "arabic": s.arabic,
                       "tradition": scholars::tradition_of(s), "dates": i18n::dates(&lang, &s.dates),
                       "roles": roles})
            })
            .collect();
        Ok(json!({"people": rows}))
    })
}

fn scholar_books(conn: &db::Connection, s: &tables::Scholar, lang: &str) -> Vec<Value> {
    let have: HashMap<String, String> = conn
        .prepare("SELECT key, name FROM collections")
        .and_then(|mut st| {
            st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
                .collect()
        })
        .unwrap_or_default();
    s.compiler_of
        .iter()
        .filter_map(|k| {
            have.get(k)
                .map(|name| json!({"key": k, "title": works::book_title(k, name, lang)}))
        })
        .collect()
}

/// Hadith Scholars: the right-click menu of a scholar in the list — his books, his entry as a narrator.
#[tauri::command]
fn scholar_menu(state: tauri::State<Data>, id: String) -> Result<Value, String> {
    let lang = i18n::ui_language();
    with_db(&state, |conn| {
        let Some(s) = scholars::find(&id) else {
            return Ok(Value::Null);
        };
        Ok(
            json!({"books": scholar_books(conn, s, &lang), "narrator": scholars::as_narrator(conn, s),
                  "short": scholars::short_name(s, &lang), "short_en": scholars::short_name(s, "en"),
                  "arabic": s.arabic, "full": s.full,
                  "marked": bookmarks::is_marked("scholar", &s.id)}),
        )
    })
}

fn counted_json(
    conn: &db::Connection,
    rows: &[scholars::Counted],
    langs: narrators::Langs,
) -> Vec<Value> {
    rows.iter()
        .map(|r| {
            let (reading, arabic) = lives::reading(conn, r.id, &r.name, langs);
            let short = if reading.is_empty() {
                let mut parts: Vec<&str> = py::words(&r.name).into_iter().take(4).collect();
                while parts
                    .last()
                    .is_some_and(|w| ["بن", "بنت", "ابن", "أبو", "أبي", "أم"].contains(w))
                {
                    parts.pop();
                }
                parts.join(" ")
            } else {
                reading.clone()
            };
            json!({"id": r.id, "name": r.name, "reading": reading, "arabic": arabic, "label": short, "count": r.count})
        })
        .collect()
}

/// Hadith Scholars: everything the right side shows of one scholar (scholars_page.show_scholar): his name laid out,
/// his roles and books, his life, and what the data measures of his work.
#[tauri::command]
fn scholar_page(state: tauri::State<Data>, id: String) -> Result<Value, String> {
    let lang = i18n::ui_language();
    with_db(&state, |conn| {
        let Some(s) = scholars::find(&id) else {
            return Ok(json!({"error": "no_scholar"}));
        };
        let langs = page_langs(&lang);
        let roles = scholars::present(conn)
            .into_iter()
            .find(|(p, _)| p.id == id)
            .map(|(_, r)| r)
            .unwrap_or_default();
        let view = scholars::name_view(s, &lang, &lang);
        let pid = scholars::as_narrator(conn, s);
        let (life, written) = match scholars::written_life(&s.id) {
            Some(w) => (Some(lives::view(w, &lang)), true),
            None => (
                pid.and_then(|p| lives::read(conn, p, langs))
                    .filter(|l| !l.paragraphs.is_empty()),
                false,
            ),
        };
        let compiler: Vec<Value> = scholars::compiler_stats(conn, s)
            .map_err(err)?
            .iter()
            .map(|st| {
                json!({"key": st.key, "book": st.book, "hadith": st.hadith, "chains": st.chains, "split": st.split,
                       "reach": st.reach, "links": st.links, "identified": st.identified,
                       "teachers": counted_json(conn, &st.teachers, langs),
                       "companions": counted_json(conn, &st.companions, langs),
                       "ends": st.ends.iter().map(|(k, label, n)| json!({"key": k, "label": label, "count": n})).collect::<Vec<_>>(),
                       "tradition": st.tradition})
            })
            .collect();
        let grader = scholars::grader_stats(conn, s).map_err(err)?.map(|g| {
            let mut agreement: Vec<&(String, scholars::Agreement)> = g.agreement.iter().collect();
            agreement.sort_by(|a, b| b.1.agree.partial_cmp(&a.1.agree).unwrap_or(std::cmp::Ordering::Equal));
            json!({"graded": g.graded, "books": g.books, "tradition": g.tradition, "distribution": g.distribution,
                   "wordings": g.wordings, "strictness": g.strictness, "compared": g.compared, "lower": g.lower,
                   "higher": g.higher,
                   "agreement": agreement.iter().map(|(o, a)| json!({"other": o, "common": a.common, "agree": a.agree,
                       "within_one": a.within_one, "kappa": a.kappa, "lower": a.lower, "higher": a.higher})).collect::<Vec<_>>()})
        });
        let critic = scholars::critic_stats(conn, s).map_err(err)?.map(|c| {
            json!({"verdicts": c.verdicts, "works": c.works, "tradition": c.tradition, "ranks": c.ranks,
                   "creeds": c.creeds, "unranked": c.unranked})
        });
        Ok(json!({
            "id": s.id, "view": view, "short": scholars::short_name(s, &lang), "short_en": scholars::short_name(s, "en"),
            "arabic": s.arabic, "tradition": scholars::tradition_of(s), "dates": i18n::dates(&lang, &s.dates),
            "works": s.works, "roles": roles, "books": scholar_books(conn, s, &lang), "narrator": pid,
            "marked": bookmarks::is_marked("scholar", &s.id),
            "life": life, "written": written,
            "compiler": compiler, "grader": grader, "critic": critic,
        }))
    })
}

fn hafiz_json(r: &huffaz::Hafiz, lang: &str) -> Value {
    let (year, whose) = huffaz::death_of(r);
    json!({"n": r.entry.n, "tabaqa": r.entry.tabaqa, "name": r.entry.name, "head": r.entry.head,
           "rumuz": r.entry.rumuz, "reading": huffaz::reading(r, lang), "year": year, "whose": whose,
           "taqrib_death": r.taqrib_death, "person": r.person})
}

/// Hadith Scholars, Huffaz: the list — the huffaz whose name holds what is typed, in one generation or all — and
/// each generation's counts for its heading.
#[tauri::command]
fn huffaz_list(
    state: tauri::State<Data>,
    text: String,
    tabaqa: Option<i64>,
) -> Result<Value, String> {
    let lang = i18n::ui_language();
    with_db(&state, |conn| {
        let rows: Vec<Value> = huffaz::find(Some(conn), text.trim(), tabaqa)
            .iter()
            .map(|r| hafiz_json(r, &lang))
            .collect();
        Ok(json!({"rows": rows, "generations": huffaz::generations(Some(conn))}))
    })
}

/// Hadith Scholars, Huffaz: the book at a glance (scholars_page.show_huffaz_overview).
#[tauri::command]
fn huffaz_overview(state: tauri::State<Data>) -> Result<Value, String> {
    with_db(&state, |conn| {
        let rows = huffaz::people(Some(conn));
        Ok(json!({
            "count": rows.len(),
            "dated": rows.iter().filter(|r| huffaz::death_of(r).0.is_some()).count(),
            "linked": rows.iter().filter(|r| r.person.is_some()).count(),
            "generations": huffaz::generations(Some(conn)),
            "centuries": huffaz::by_century(Some(conn)),
        }))
    })
}

/// Hadith Scholars, Huffaz: one hafiz (scholars_page.show_hafiz): his entry in the index, the Taqrib's entry of the
/// same man and al-Dhahabi's entry to read.
#[tauri::command]
fn hafiz_page(state: tauri::State<Data>, n: i64) -> Result<Value, String> {
    let lang = i18n::ui_language();
    with_db(&state, |conn| {
        let Some(r) = huffaz::people(Some(conn))
            .into_iter()
            .find(|x| x.entry.n == n)
        else {
            return Ok(json!({"error": "no_hafiz"}));
        };
        let mut out = hafiz_json(&r, &lang);
        let gen = huffaz::generations(Some(conn))
            .into_iter()
            .find(|g| g.tabaqa == r.entry.tabaqa);
        out["generation"] = json!(gen);
        out["rumuz_text"] = json!(huffaz::rumuz_text(&r.entry.rumuz)
            .iter()
            .map(|x| i18n::t(&lang, x, &[]))
            .collect::<Vec<_>>());
        out["in_chains"] = json!(r.in_chains);
        if let Some(name) = &r.taqrib_name {
            let shown = narrators::display_name(name);
            let full = if lang == "ar" {
                None
            } else {
                names::latin(&shown, &lang, 8)
            };
            out["taqrib_name"] = json!(shown);
            out["taqrib_reading"] = json!(full);
        }
        if let Some(pid) = r.person {
            let verdicts: Vec<Value> = people::describe(conn, pid)
                .map(|d| {
                    d.verdicts
                        .iter()
                        .filter(|v| !v.rank_label.is_empty())
                        .map(|v| {
                            json!({"critic": i18n::t(&lang, &v.critic_name, &[]), "rank": v.rank,
                                        "label": i18n::t(&lang, &v.rank_label, &[])})
                        })
                        .collect()
                })
                .unwrap_or_default();
            out["verdicts"] = json!(verdicts);
        }
        out["entry"] = json!({"work": huffaz::work(), "volume": 0, "number": r.entry.n,
            "title": lives::work_title(huffaz::work(), &lang),
            "where": i18n::t(&lang, "generation {tabaqa}, no. {number}",
                &[("tabaqa", r.entry.tabaqa.to_string()), ("number", r.entry.n.to_string())])});
        Ok(out)
    })
}

// ------------------------------------------------------------------ Companions and the Household (gui/companions_page.py)

/// Companions: the groups of one road, for the list.
#[tauri::command]
fn companions_groups(road: String) -> Value {
    let lang = i18n::ui_language();
    json!(sahaba::groups(Some(&road))
        .iter()
        .map(|g| json!({"id": g.id, "title": sahaba::text(&g.title, &lang)}))
        .collect::<Vec<_>>())
}

/// Companions: one group (companions_page.show_group): its title and lead, its members with what the books imported
/// here hold of them, the hadith that name it.
#[tauri::command]
fn companion_group(state: tauri::State<Data>, id: String) -> Result<Value, String> {
    let lang = i18n::ui_language();
    with_db(&state, |conn| {
        let Some(g) = sahaba::find(&id) else {
            return Ok(json!({"error": "no_group"}));
        };
        let shia = g.road == "shia";
        let found = sahaba::hadith(conn, g);
        Ok(json!({
            "id": g.id, "road": g.road, "title": sahaba::text(&g.title, &lang), "lead": sahaba::text(&g.lead, &lang),
            "members": sahaba::members(Some(conn), g, &lang),
            "shared": if shia { sahaba::shared_ends(conn) } else { 0 },
            "refs": g.hadith.iter().map(|(b, n)| format!("{b} {n}")).collect::<Vec<_>>(),
            "found": found.iter().map(|(_, _, h)| *h).collect::<Vec<_>>(),
        }))
    })
}

/// The cards of some hadith as Search shows them (ResultCard): the Companions page's hadith that name a group.
#[tauri::command]
fn hadith_cards(
    state: tauri::State<Data>,
    cites: tauri::State<Cites>,
    ids: Vec<i64>,
) -> Result<Vec<Value>, String> {
    let lang = i18n::ui_language();
    with_db(&state, |conn| {
        let cx = CardCtx::new(conn, &cites, &lang)?;
        let q = search::SearchQuery::new("");
        let mut out = vec![];
        for id in ids {
            // one card never stops the page
            if let Ok(r) = search::load_result(conn, id, &[], &q) {
                if let Ok(c) = card_json(conn, &r, &cx, false) {
                    out.push(c);
                }
            }
        }
        Ok(out)
    })
}

// ------------------------------------------------------------------ Sira (gui/sira_page.py)

fn sira_event_json(e: &sira::Event, lang: &str) -> Value {
    json!({"id": e.id, "period": e.period, "year": e.year, "title": sira::text(&e.title, lang),
           "year_label": sira::year_label(e, lang)})
}

/// Sira: the periods and the events, for the timeline and the list.
#[tauri::command]
fn sira_events() -> Value {
    let lang = i18n::ui_language();
    json!({
        "periods": sira::periods().iter().map(|(k, v)| json!({"key": k, "name": sira::text(v, &lang)})).collect::<Vec<_>>(),
        "events": sira::all().iter().map(|e| sira_event_json(e, &lang)).collect::<Vec<_>>(),
    })
}

/// Sira: one event (sira_page.show_event) — its date, place, age, text, where reports differ, the verses, Ibn Hisham's
/// chapter (when imported) and how many hadith of the Sunni books say its phrase.
#[tauri::command]
fn sira_event(state: tauri::State<Data>, id: String) -> Result<Value, String> {
    let lang = i18n::ui_language();
    let Some(e) = sira::find(&id) else {
        return Ok(json!({"error": "no_event"}));
    };
    let (chapter, count) = with_db(&state, |conn| {
        let count = if e.phrase.is_empty() {
            None
        } else {
            sira::hadith_counts(conn)
                .ok()
                .and_then(|c| c.get(&e.id).copied())
        };
        Ok((sira::sira_chapter(conn, e), count))
    })
    .unwrap_or((None, None));
    let mut out = sira_event_json(e, &lang);
    out["arabic"] = json!(e.title.get("ar").cloned().unwrap_or_default());
    out["period_name"] = json!(sira::period_name(&e.period, &lang));
    out["when"] = json!(sira::text(&e.when, &lang));
    out["place"] = json!(sira::text(&e.place, &lang));
    out["age"] = json!(sira::age(e));
    out["text"] = json!(sira::text(&e.text, &lang));
    out["differ"] = json!(e.differ.as_ref().map(|d| sira::text(d, &lang)));
    out["quran"] = json!(e
        .quran
        .iter()
        .map(|(s, a, b)| json!({"sura": s, "aya": a, "label": quran::label(*s, *a, *b, &lang)}))
        .collect::<Vec<_>>());
    out["sira"] = json!(e.sira);
    out["chapter"] = json!(chapter);
    out["phrase"] = json!(e.phrase);
    out["count"] = json!(count);
    Ok(out)
}

// ------------------------------------------------------------------ Genealogies (gui/genealogy_page.py)

fn tree_json(tree: &genealogy::Tree, lang: &str) -> Value {
    let nodes: serde_json::Map<String, Value> = tree
        .order
        .iter()
        .map(|id| {
            let n = &tree.nodes[id];
            (id.clone(), json!({"id": n.id, "name": n.name(lang), "arabic": n.names.get("ar").cloned().unwrap_or_default(),
                "en": n.names.get("en"), "tr": n.names.get("tr"),
                "kind": n.kind, "parent": n.parent, "gap": n.gap, "disputed": n.disputed.is_some().then(|| n.text("disputed", lang)),
                "note": n.note.is_some().then(|| n.text("note", lang)), "person": n.person_id, "count": n.count,
                "children": n.children}))
        })
        .collect();
    json!({"key": tree.key, "title": tree.title, "root": tree.root, "up": tree.up, "nodes": nodes})
}

/// The narrator most often in the imported chains: the one the family and learning views open with.
fn first_narrator(conn: &db::Connection) -> Option<i64> {
    conn.query_row(
        "SELECT person_id FROM isnad_links WHERE person_id IS NOT NULL GROUP BY person_id ORDER BY COUNT(*) DESC LIMIT 1",
        [],
        |r| r.get(0),
    )
    .ok()
}

/// Genealogies: one tree (genealogy_page.show_view) — a written one, or a narrator's family or line of learning.
#[tauri::command]
fn genealogy_view(
    state: tauri::State<Data>,
    key: String,
    person: Option<i64>,
) -> Result<Value, String> {
    let lang = i18n::ui_language();
    if genealogy::is_written(&key) {
        let tree = genealogy::written(&key, &lang).ok_or("no tree")?;
        return Ok(json!({"key": key, "tree": tree_json(&tree, &lang)}));
    }
    with_db(&state, |conn| {
        let langs = page_langs(&lang);
        let Some(pid) = person.or_else(|| first_narrator(conn)) else {
            return Ok(json!({"key": key, "none": true}));
        };
        if key == "family" {
            let count = genealogy::family_count(conn);
            return match genealogy::family(conn, pid, langs).map_err(err)? {
                Some(tree) => Ok(
                    json!({"key": key, "person": pid, "tree": tree_json(&tree, &lang), "family_count": count}),
                ),
                None => Ok(
                    json!({"key": key, "person": pid, "no_kin": genealogy::person_name(conn, pid, langs),
                                  "family_count": count}),
                ),
            };
        }
        let tree = genealogy::learning(conn, pid, 2, 4, langs).map_err(err)?;
        Ok(json!({"key": key, "person": pid, "tree": tree_json(&tree, &lang)}))
    })
}

/// Genealogies: what the side panel shows of a box beyond the tree itself — his life (written for isnady), the hadith
/// that name a prophet, his entry as a narrator, the stars.
#[tauri::command]
fn genealogy_node(
    state: tauri::State<Data>,
    key: String,
    node: String,
    person: Option<i64>,
    current: Option<i64>,
) -> Result<Value, String> {
    let lang = i18n::ui_language();
    let written = genealogy::is_written(&key);
    let mut out = json!({});
    let life_id = if !written {
        None
    } else if node == "muhammad" {
        Some("muhammad")
    } else {
        genealogy::node_life(&key, &node)
    };
    let db_part = with_db(&state, |conn| {
        let mut extra = json!({});
        if let Some(l) = life_id.and_then(genealogy::life_by_id) {
            extra["life"] = json!(lives::view(l, &lang));
            if node != "muhammad" {
                extra["life_person"] = json!(lives::person_of(conn, l));
            }
        } else if written && genealogy::has_prophet_words(&node) {
            let found = genealogy::prophet_hadith(conn, &node, 40).map_err(err)?;
            let named = |items: &[genealogy::Named]| -> Vec<Value> {
                items
                    .iter()
                    .map(|i| {
                        let title = works::book_title(&i.book, &i.book, &lang);
                        json!({"hadith": i.hadith_id, "label": format!("{title} {}", i.number)})
                    })
                    .collect()
            };
            extra["prophet"] = json!({"total": found.total, "sunni": named(&found.sunni), "shia": named(&found.shia),
                                      "count_sunni": found.count_sunni, "count_shia": found.count_shia});
        }
        if let Some(pid) = person {
            extra["narrator"] = narrator_ref(conn, pid, &lang).unwrap_or(Value::Null);
        }
        // ☆ for this place in the genealogy (BM2): a box of a written tree, or the family / line of learning shown
        let tree_ref = if written {
            genealogy::written(&key, &lang).and_then(|t| {
                t.nodes.get(&node).map(|n| {
                    json!({"ref": format!("{key}:{}", n.id), "title": n.name(&lang),
                           "subtitle": n.names.get("ar").cloned().unwrap_or_default()})
                })
            })
        } else if person.is_some() && person == current {
            narrator_ref(conn, person.expect("person"), &lang).map(|r| {
                json!({"ref": format!("{key}:{}", r["key"].as_str().unwrap_or("")), "title": r["title"],
                       "subtitle": r["arabic"]})
            })
        } else {
            None
        };
        if let Some(mut r) = tree_ref {
            r["marked"] = json!(bookmarks::is_marked(
                "tree",
                r["ref"].as_str().unwrap_or("")
            ));
            extra["tree_star"] = r;
        }
        Ok(extra)
    });
    if let Ok(extra) = db_part {
        out = extra;
    } else if let Some(l) = life_id.and_then(genealogy::life_by_id) {
        out["life"] = json!(lives::view(l, &lang));
    }
    Ok(out)
}

/// Genealogies: where a narrator's written life stands in a written tree — (tree, box) — for "his place in the tree".
#[tauri::command]
fn genealogy_place_of(state: tauri::State<Data>, person: i64) -> Result<Value, String> {
    with_db(&state, |conn| {
        Ok(lives::of_person(conn, person)
            .and_then(|l| l["id"].as_str())
            .and_then(genealogy::life_node)
            .map(|(t, n)| json!([t, n]))
            .unwrap_or(Value::Null))
    })
}

/// Genealogies: the boxes of a written tree whose names hold every word, the closest name first.
#[tauri::command]
fn genealogy_find_in_tree(key: String, text: String) -> Vec<String> {
    genealogy::written(&key, "en")
        .map(|t| {
            genealogy::find(&t, &text)
                .iter()
                .map(|n| n.id.clone())
                .collect()
        })
        .unwrap_or_default()
}

/// Genealogies: the narrators found for the words typed (the family and learning views).
#[tauri::command]
fn genealogy_find_narrators(state: tauri::State<Data>, text: String) -> Result<Value, String> {
    let lang = i18n::ui_language();
    with_db(&state, |conn| {
        let (rows, _n) = narrators::find(conn, &text, None, page_langs(&lang)).map_err(err)?;
        Ok(json!(rows.into_iter().take(30).collect::<Vec<_>>()))
    })
}

// ------------------------------------------------------------------ Bookmarks (gui/bookmarks_page.py)

/// Bookmarks: the folders as a tree and the items of the folder chosen (bookmarks_page.refresh, _fill).
#[tauri::command]
fn bookmarks_page(state: tauri::State<Data>, folder: String) -> Result<Value, String> {
    let lang = i18n::ui_language();
    let store = bookmarks::Store::load();
    let folder =
        if folder != "__all__" && folder != "__unfiled__" && store.folder(&folder).is_none() {
            "__all__".to_string()
        } else {
            folder
        };
    fn tree(store: &bookmarks::Store, parent: Option<&str>) -> Vec<Value> {
        store
            .children(parent)
            .into_iter()
            .map(|f| {
                json!({"id": f.id, "name": f.name, "path": store.path_of(&f.id), "count": store.in_folder(Some(&f.id)).len(),
                       "children": tree(store, Some(&f.id))})
            })
            .collect()
    }
    let items: Vec<&bookmarks::Item> = match folder.as_str() {
        "__all__" => store.items.iter().collect(),
        "__unfiled__" => store.unfiled(),
        f => store.in_folder(Some(f)),
    };
    let conn = with_db(&state, |_| Ok(())).ok();
    let guard = state.0.lock().map_err(err)?;
    let db = if conn.is_some() { guard.as_ref() } else { None };
    let rows: Vec<Value> = items
        .iter()
        .map(|i| {
            json!({"id": i.id, "kind": i.kind, "ref": i.reference, "title": i.title, "subtitle": i.subtitle, "note": i.note,
                   "detail": bookmarks::detail(db, i, &lang),
                   "folders": i.folders.iter().filter(|f| store.folder(f).is_some()).map(|f| json!({"id": f, "path": store.path_of(f)})).collect::<Vec<_>>()})
        })
        .collect();
    Ok(json!({"folder": folder, "heading": match folder.as_str() {
                  "__all__" | "__unfiled__" => Value::Null, f => json!(store.path_of(f)) },
              "total": store.items.len(), "unfiled": store.unfiled().len(), "tree": tree(&store, None),
              "folders": store.folders.iter().map(|f| json!({"id": f.id, "path": store.path_of(&f.id)})).collect::<Vec<_>>(),
              "items": rows}))
}

/// Bookmarks: one change to the store (the page's buttons and menus); the page draws itself again after.
#[tauri::command]
fn bookmarks_change(
    action: String,
    item: Option<String>,
    folder: Option<String>,
    name: Option<String>,
    on: Option<bool>,
) -> Result<Value, String> {
    let mut store = bookmarks::Store::load();
    let item = item.unwrap_or_default();
    let name = name.unwrap_or_default();
    let mut made: Option<String> = None;
    match action.as_str() {
        "remove" => {
            store.remove(&item);
        }
        "note" => store.set_note(&item, &name),
        "file" => {
            let f = folder.unwrap_or_default();
            if on.unwrap_or(true) {
                store.file_in(&item, &f);
            } else {
                store.take_out(&item, &f);
            }
        }
        "new_folder" => {
            if name.trim().is_empty() {
                return Ok(Value::Null);
            }
            let f = match folder.as_deref() {
                Some(parent) => Some(store.new_folder(&name, Some(parent))),
                None => store.ensure_path(&name),
            };
            if let Some(f) = f {
                if !item.is_empty() {
                    store.file_in(&item, &f.id);
                }
                made = Some(f.id);
            }
        }
        "rename" => store.rename_folder(folder.as_deref().unwrap_or(""), &name),
        "move" => store.move_folder(
            folder.as_deref().unwrap_or(""),
            Some(name.as_str()).filter(|n| !n.is_empty()),
        ),
        "delete" => store.delete_folder(folder.as_deref().unwrap_or("")),
        _ => return Err(format!("unknown action {action}")),
    }
    store.save().map_err(err)?;
    Ok(json!(made))
}

/// Bookmarks: where a bookmark leads — (kind, target) to open, or null when it is not in the database now.
#[tauri::command]
fn bookmark_target(state: tauri::State<Data>, id: String) -> Result<Value, String> {
    let store = bookmarks::Store::load();
    let Some(item) = store.item(&id).cloned() else {
        return Ok(Value::Null);
    };
    if item.kind == "scholar" || item.kind == "term" {
        return Ok(json!({"kind": item.kind, "target": item.reference, "title": item.title}));
    }
    with_db(&state, |conn| {
        let target = match item.kind.as_str() {
            "tree" => {
                bookmarks::resolve_tree(Some(conn), &item.reference).map(|(w, t)| json!([w, t]))
            }
            "hadith" => bookmarks::resolve_hadith(conn, &item.reference).map(|x| json!(x)),
            "narrator" => bookmarks::resolve_narrator(conn, &item.reference).map(|x| json!(x)),
            _ => bookmarks::resolve_chapter(conn, &item.reference).map(|x| json!(x)),
        };
        Ok(json!({"kind": item.kind, "target": target, "title": item.title}))
    })
}

// ------------------------------------------------------------------ Learn (gui/learn_page.py)

fn learn_lang(lang: Option<String>) -> String {
    if let Some(l) = lang.filter(|l| learn::languages().iter().any(|(c, _)| c == l)) {
        return l;
    }
    let ui = i18n::ui_language();
    if learn::languages().iter().any(|(c, _)| *c == ui) {
        return ui;
    }
    config::state_get("learn.language")
        .and_then(|v| v.as_str().map(String::from))
        .filter(|l| learn::languages().iter().any(|(c, _)| c == l))
        .unwrap_or_else(|| "en".into())
}

/// Learn: the list — the articles of History and method and the terms by category, those that hold the words typed.
#[tauri::command]
fn learn_list(lang: Option<String>, query: String) -> Value {
    let lang = learn_lang(lang);
    let found: std::collections::HashSet<String> = learn::search(&query)
        .iter()
        .map(|t| learn::field(t, "id"))
        .collect();
    let cats: Vec<Value> = learn::categories()
        .keys()
        .map(|k| {
            let members: Vec<Value> = learn::terms()
                .iter()
                .filter(|t| learn::field(t, "cat") == *k && found.contains(&learn::field(t, "id")))
                .map(|t| {
                    json!({"id": learn::field(t, "id"), "name": learn::name(t, &lang), "ar": learn::field(t, "ar"),
                           "en": learn::field(t, "en"), "short": learn::text(t, "short", &lang)})
                })
                .collect();
            json!({"key": k, "name": learn::category(k, &lang), "terms": members})
        })
        .collect();
    let arts: Vec<Value> = learn::search_articles(&query, &lang)
        .iter()
        .map(|a| json!({"id": a["id"], "title": learn::say_text(&a["title"], &lang), "lead": learn::say_text(&a["lead"], &lang)}))
        .collect();
    json!({"lang": lang, "languages": learn::languages(), "count": learn::terms().len(),
           "history": learn::label("category", &lang), "articles": arts, "categories": cats,
           "category_names": learn::categories().keys().map(|k| json!([k, learn::category(k, &lang)])).collect::<Vec<_>>()})
}

/// Learn: the language chosen on the page (kept for the session and the next start, learn.language).
#[tauri::command]
fn learn_set_language(lang: String) {
    config::state_set("learn.language", json!(lang));
}

/// Learn: one term's card (learn_page.show_term).
#[tauri::command]
fn learn_term(lang: Option<String>, id: String) -> Value {
    let lang = learn_lang(lang);
    let Some(t) = learn::get(&id) else {
        return Value::Null;
    };
    let cat = learn::field(t, "cat");
    let others: Vec<Value> = learn::languages()
        .iter()
        .filter(|(c, _)| *c != lang && c != "ar")
        .map(|(c, n)| json!([n, learn::name(t, c)]))
        .collect();
    let related: Vec<Value> = t["related"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| learn::get(r.as_str().unwrap_or("")))
        .map(|r| {
            json!({"id": learn::field(r, "id"), "cat": learn::field(r, "cat"), "name": learn::name(r, &lang),
                   "ar": learn::field(r, "ar"), "category": learn::category(&learn::field(r, "cat"), &lang),
                   "short": learn::text(r, "short", &lang)})
        })
        .collect();
    let same: Vec<Value> = learn::terms()
        .iter()
        .filter(|x| learn::field(x, "cat") == cat && learn::field(x, "id") != id)
        .map(|x| json!({"id": learn::field(x, "id"), "name": learn::name(x, &lang), "short": learn::text(x, "short", &lang)}))
        .collect();
    json!({"id": id, "lang": lang, "cat": cat, "category": learn::category(&cat, &lang), "name": learn::name(t, &lang),
           "name_en": learn::name(t, "en"), "ar": learn::field(t, "ar"), "others": others,
           "short": learn::text(t, "short", &lang), "long": learn::text(t, "long", &lang),
           "source": learn::field(t, "source"), "related": related, "same": same,
           "marked": bookmarks::is_marked("term", &id)})
}

/// Learn: one article of History and method (learn_page.show_article): its texts; the blocks are counted apart.
#[tauri::command]
fn learn_article(lang: Option<String>, id: String) -> Value {
    let lang = learn_lang(lang);
    let Some(a) = learn::article(&id) else {
        return Value::Null;
    };
    let sections: Vec<Value> = a["sections"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|s| json!({"heading": learn::say_text(&s["heading"], &lang), "text": learn::say_list(&s["text"], &lang),
                        "block": s["block"]}))
        .collect();
    json!({"id": id, "lang": lang, "title": learn::say_text(&a["title"], &lang), "lead": learn::say_text(&a["lead"], &lang),
           "sections": sections, "sources": (!a["sources"].is_null()).then(|| learn::say_text(&a["sources"], &lang)),
           "labels": {"history": learn::label("category", &lang), "sources": learn::label("sources", &lang),
                      "more": learn::label("more", &lang), "counting": learn::label("counting", &lang),
                      "no_data": learn::label("no_data", &lang), "live": learn::label("live", &lang)},
           "others": learn::articles().iter().filter(|x| x["id"] != a["id"])
               .map(|x| json!({"id": x["id"], "title": learn::say_text(&x["title"], &lang)})).collect::<Vec<_>>()})
}

/// Learn: one block of live numbers (counted once per change of the database, in a thread of its own).
#[tauri::command]
async fn learn_block(kind: String, lang: Option<String>) -> Value {
    let lang = learn_lang(lang);
    tauri::async_runtime::spawn_blocking(move || {
        let conn = if kind == "timeline" {
            None
        } else {
            db::connect(None).ok()
        };
        learn::block(conn.as_ref(), &kind, &lang).unwrap_or_default()
    })
    .await
    .map(|rows| json!(rows))
    .unwrap_or(Value::Null)
}

// ------------------------------------------------------------------ the Qur'an in Books (gui/quran_view.py)

fn sura_title(s: &quran::Sura, lang: &str) -> String {
    match lang {
        "tr" if !s.tr.is_empty() => s.tr.clone(),
        "ar" if !s.arabic.is_empty() => s.arabic.clone(),
        _ => s.name.clone(),
    }
}

/// The Qur'an: whether it is downloaded, its suras for the contents, the sura read last, the languages the reader
/// reads (for the download card).
#[tauri::command]
fn quran_state() -> Value {
    let lang = i18n::ui_language();
    let suras = quran::suras();
    json!({
        "available": quran::available(),
        "suras": suras.iter().map(|s| json!({"n": s.n, "label": format!("{}. {}   ({})", s.n, sura_title(s, &lang), s.verses)}))
            .collect::<Vec<_>>(),
        "last": config::state_get("quran.sura").and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|x| x.parse().ok()))).unwrap_or(1),
        "languages": quran::wanted_languages(),
    })
}

/// The Qur'an: one sura (QuranView.show_sura) — its title and facts, the translations to tick, the languages with no
/// translation yet, its verses with the texts chosen and the hadith that name each (the two roads apart).
#[tauri::command]
fn quran_sura(state: tauri::State<Data>, n: u32) -> Result<Value, String> {
    let lang = i18n::ui_language();
    if !quran::available() {
        return Ok(
            json!({"download": true, "languages": quran::wanted_languages(),
                         "defaults": quran::default_editions()}),
        );
    }
    let Some(s) = quran::suras().into_iter().find(|x| x.n == n) else {
        return Ok(json!({"error": "no_sura"}));
    };
    config::state_set("quran.sura", json!(n));
    let named = with_db(&state, |conn| quran::named(conn).map_err(err)).ok();
    let chosen = quran::shown(quran::saved_choice().as_deref());
    let langs = quran::wanted_languages();
    let mut eds: Vec<quran::Edition> = quran::editions()
        .into_iter()
        .filter(|e| e.key != quran::ARABIC)
        .collect();
    eds.sort_by_key(|e| {
        (
            chosen
                .iter()
                .position(|k| *k == e.key)
                .unwrap_or(chosen.len()),
            langs
                .iter()
                .position(|l| *l == e.language)
                .unwrap_or(langs.len()),
            e.language.clone(),
            e.author.clone(),
        )
    });
    let all = quran::editions();
    let skipped: Vec<String> = config::state_get("quran.skip")
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default()
        .split(',')
        .filter(|x| !x.is_empty())
        .map(String::from)
        .collect();
    let missing: Vec<String> = quran::missing_languages()
        .into_iter()
        .filter(|l| !skipped.contains(l))
        .collect();
    let rows = quran::verses(n, &chosen, 1, None);
    let books: HashMap<i64, (String, String, String)> = match &named {
        Some(named) => {
            let ids: Vec<i64> = rows
                .iter()
                .flat_map(|v| quran::hadith_of(named, n, v.ayah))
                .collect();
            with_db(&state, |conn| hadith_refs(conn, &ids, &lang)).unwrap_or_default()
        }
        None => HashMap::new(),
    };
    let verses: Vec<Value> = rows
        .iter()
        .map(|v| {
            let texts: Vec<Value> = chosen
                .iter()
                .filter_map(|key| {
                    let text = v.texts.get(key).filter(|t| !t.is_empty())?;
                    let e = all.iter().find(|e| &e.key == key);
                    Some(json!({"text": text, "rtl": e.is_some_and(|e| e.direction == "rtl"),
                        "language": e.map(|e| e.language.clone()).unwrap_or_default(),
                        "author": e.map(|e| e.author.clone()).unwrap_or_default()}))
                })
                .collect();
            let hadith = named.as_ref().map(|nm| quran::hadith_of(nm, n, v.ayah)).unwrap_or_default();
            let road = |r: &str| -> Vec<Value> {
                hadith.iter().filter_map(|h| books.get(h).filter(|b| b.2 == r).map(|b| json!([h, b.0, b.1]))).collect()
            };
            json!({"ayah": v.ayah, "arabic": v.texts.get(quran::ARABIC).cloned().unwrap_or_default(), "texts": texts,
                "sunni": road("sunni"), "shia": road("shia")})
        })
        .collect();
    Ok(json!({
        "n": s.n, "title": format!("{}. {}", s.n, match lang.as_str() {
            "tr" | "ar" => sura_title(&s, &lang),
            _ => format!("{} — {}", s.name, s.english),
        }),
        "arabic": s.arabic, "verses_count": s.verses, "meccan": s.place == "Mecca",
        "named": named.as_ref().and_then(|nm| quran::counts_by_sura(nm).get(&n).copied()).unwrap_or(0),
        "name": quran::sura_name(n, &lang),
        "editions": eds.iter().map(|e| json!({"key": e.key, "language": e.language, "author": e.author,
            "source": if e.source.is_empty() { e.origin.clone() } else { e.source.clone() }, "origin": e.origin,
            "on": chosen.contains(&e.key)})).collect::<Vec<_>>(),
        "missing_keys": missing.iter().map(|l| quran::default_of(l).unwrap_or("")).collect::<Vec<_>>(),
        "missing": missing,
        "verses": verses,
    }))
}

/// (book title, number, road) of some hadith, for the links under a verse.
fn hadith_refs(
    conn: &db::Connection,
    ids: &[i64],
    lang: &str,
) -> Result<HashMap<i64, (String, String, String)>, String> {
    let mut out = HashMap::new();
    for part in ids.chunks(900) {
        let sql = format!(
            "SELECT h.id, c.key, c.name, h.number, COALESCE(c.tradition, 'sunni') FROM hadiths h \
             JOIN collections c ON c.id = h.collection_id WHERE h.id IN ({})",
            vec!["?"; part.len()].join(",")
        );
        let mut st = conn.prepare(&sql).map_err(err)?;
        let mut rows = st.query(db::params_from_iter(part.iter())).map_err(err)?;
        while let Some(r) = rows.next().map_err(err)? {
            let key: String = r.get(1).map_err(err)?;
            let name: String = r
                .get::<_, Option<String>>(2)
                .map_err(err)?
                .unwrap_or_default();
            out.insert(
                r.get(0).map_err(err)?,
                (
                    works::book_title(&key, &name, lang),
                    db::number_text(r, 3).map_err(err)?,
                    r.get(4).map_err(err)?,
                ),
            );
        }
    }
    Ok(out)
}

/// The Qur'an: a translation ticked or unticked (QuranView._toggle).
#[tauri::command]
fn quran_toggle(key: String, on: bool) {
    let mut chosen: Vec<String> = quran::shown(quran::saved_choice().as_deref())
        .into_iter()
        .filter(|k| *k != key)
        .collect();
    if on {
        chosen.push(key);
    }
    let joined = chosen.join(",");
    config::state_set(
        "quran.editions",
        json!(if joined.is_empty() {
            ",".to_string()
        } else {
            joined
        }),
    );
}

/// The Qur'an: the languages offered not offered again (✕).
#[tauri::command]
fn quran_skip(languages: Vec<String>) {
    let mut skipped: Vec<String> = config::state_get("quran.skip")
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default()
        .split(',')
        .filter(|x| !x.is_empty())
        .map(String::from)
        .collect();
    skipped.extend(languages);
    config::state_set("quran.skip", json!(skipped.join(",")));
}

/// The Qur'an: a search through the texts shown (QuranView._search).
#[tauri::command]
fn quran_search(text: String) -> Value {
    let lang = i18n::ui_language();
    let chosen = quran::shown(quran::saved_choice().as_deref());
    let found = quran::search(&text, &chosen, 200);
    json!(found
        .iter()
        .map(|r| json!({"sura": r.sura, "ayah": r.ayah, "rtl": r.edition == quran::ARABIC, "text": r.text,
            "label": format!("{} {}:{}", quran::sura_name(r.sura, &lang), r.sura, r.ayah)}))
        .collect::<Vec<_>>())
}

// ------------------------------------------------------------------ Written sources (gui/sahifa_page.py)

/// Written sources: count them in a thread of their own (sahifa_page._Count) — the window is never held while it runs.
#[tauri::command]
async fn sahifa_count() -> bool {
    tauri::async_runtime::spawn_blocking(|| {
        db::connect(None)
            .ok()
            .map(|c| sahifa::counted(&c).is_ok())
            .unwrap_or(false)
    })
    .await
    .unwrap_or(false)
}

/// Written sources: the sources of a road, with how many hadith each has (none while the count runs).
#[tauri::command]
fn sahifa_list(state: tauri::State<Data>, road: String) -> Result<Value, String> {
    let lang = i18n::ui_language();
    with_db(&state, |conn| {
        let counts = if sahifa::is_counted(conn) {
            Some(sahifa::summary(&sahifa::counted(conn).map_err(err)?))
        } else {
            None
        };
        Ok(json!(sahifa::sources(Some(&road))
            .iter()
            .map(|s| json!({"id": s.id, "title": sahifa::text(&s.title, &lang), "layer": s.layer,
                "count": counts.as_ref().and_then(|c| c.iter().find(|r| r.id == s.id)).map(|r| r.count)}))
            .collect::<Vec<_>>()))
    })
}

/// Written sources: one source (sahifa_page.show_source) — what it is, where it lives, its hadith; Hammam's with the
/// parallels of each.
#[tauri::command]
fn sahifa_source(state: tauri::State<Data>, id: String) -> Result<Value, String> {
    let lang = i18n::ui_language();
    with_db(&state, |conn| {
        let Some(s) = sahifa::find(&id) else {
            return Ok(json!({"error": "no_source"}));
        };
        if !sahifa::is_counted(conn) {
            return Ok(json!({"waiting": true, "road": s.road}));
        }
        let c = sahifa::counted(conn).map_err(err)?;
        let as_book = conn
            .query_row("SELECT 1 FROM works WHERE key = ?", [&s.book], |_| Ok(()))
            .is_ok();
        let ids = sahifa::hadith(&c, &s.id);
        let books = |ids: &[i64]| -> Result<Vec<Value>, String> {
            Ok(sahifa::by_book(conn, ids)
                .map_err(err)?
                .iter()
                .map(|(k, name, n)| json!([works::book_title(k, name, &lang), n]))
                .collect())
        };
        let mut out = json!({
            "id": s.id, "road": s.road, "layer": s.layer, "book": s.book, "as_book": as_book,
            "title": sahifa::text(&s.title, &lang), "lead": sahifa::text(&s.lead, &lang),
            "how": sahifa::text(&s.how, &lang), "ids": ids, "books": books(&ids)?,
        });
        if s.id == "hammam" {
            let others: Vec<i64> = c
                .hammam
                .parallels
                .iter()
                .flat_map(|(_, v)| v.iter().copied())
                .collect();
            let mut pars = serde_json::Map::new();
            for (at, v) in &c.hammam.parallels {
                let found = sahifa::refs(conn, v).map_err(err)?;
                pars.insert(
                    at.to_string(),
                    json!(found
                        .iter()
                        .map(|(h, k, n)| json!([h, works::book_title(k, k, &lang), n]))
                        .collect::<Vec<_>>()),
                );
            }
            out["hammam"] = json!({"with": c.hammam.parallels.len(), "count": others.len(),
                "books": books(&others)?, "parallels": pars});
        }
        Ok(out)
    })
}

/// A number of the book chosen at random, of a hadith whose chain was read.
#[tauri::command]
fn random_number(state: tauri::State<Data>, book: String) -> Result<Option<String>, String> {
    with_db(&state, |conn| Ok(isnad::random_number(conn, &book)))
}

fn main() {
    isnady_core::log::setup("window");
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Data(Mutex::new(None)))
        .manage(Cites(Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![
            start,
            find,
            chains_overview,
            book_numbers,
            numbers_starting,
            number_place,
            step_number,
            hadith_by_number,
            chain_page,
            contents_tree,
            contents_chapter,
            random_number,
            scholars_list,
            sira_events,
            bookmarks_page,
            bookmarks_change,
            bookmark_target,
            learn_list,
            learn_set_language,
            learn_term,
            learn_article,
            learn_block,
            genealogy_view,
            genealogy_node,
            genealogy_place_of,
            genealogy_find_narrators,
            genealogy_find_in_tree,
            sira_event,
            sahifa_count,
            prefs_state,
            prefs_set,
            prefs_reset,
            book_active,
            cache_clear,
            shortcut_create,
            log_files,
            log_read,
            log_change,
            reset_data,
            quran_state,
            quran_sura,
            quran_toggle,
            quran_skip,
            quran_search,
            sahifa_list,
            sahifa_source,
            catch_up,
            catch_up_status,
            sources_page::src_page,
            sources_page::src_use_repo,
            sources_page::src_refresh_repo,
            sources_page::src_add_repo,
            sources_page::src_remove_repo,
            sources_page::src_remove,
            sources_page::src_remove_language,
            sources_page::src_delete,
            sources_page::src_add,
            sources_page::src_stage,
            sources_page::ready_available,
            sources_page::first_start,
            sources_page::src_relocate,
            profile_new,
            profile_rename,
            profile_delete,
            profile_export,
            profile_import,
            home_folder,
            choose_path,
            source_fetch,
            help_page::help_check,
            help_page::help_update,
            help_page::help_update_status,
            transfer_page::xfer_page,
            transfer_page::xfer_chapters,
            transfer_page::xfer_chapter_number,
            transfer_page::xfer_method,
            transfer_page::xfer_preview,
            transfer_page::xfer_export,
            transfer_page::xfer_stop,
            transfer_page::xfer_status,
            transfer_page::xfer_show,
            transfer_page::xfer_guess,
            transfer_page::xfer_import,
            sources_page::src_job,
            sources_page::src_job_status,
            sources_page::quran_offered,
            stats_page::stats_progress,
            stats_page::stats_scope,
            stats_page::stats_corpus,
            stats_page::stats_graders,
            stats_page::stats_names,
            stats_page::stats_terms,
            stats_page::stats_home,
            stats_page::stats_write,
            stats_page::stats_csv,
            stats_page::stats_xlsx,
            stats_page::stats_workbook,
            stats_page::stats_picture,
            stats_page::stats_pdf,
            stats_page::stats_csv_folder,
            stats_page::stats_json,
            companions_groups,
            companion_group,
            hadith_cards,
            scholar_menu,
            scholar_page,
            huffaz_list,
            huffaz_overview,
            hafiz_page,
            books_shelf,
            book_open,
            book_chapter,
            book_cards,
            book_languages,
            book_remember,
            book_place_of,
            book_chapter_ref,
            marks_list,
            mark_suggest,
            mark_add,
            mark_rename,
            mark_remove,
            marks_remove_all,
            place_forget,
            mark_resolve,
            narrators_overview,
            narrators_list,
            narrators_famous,
            narrator_names,
            narrator_page,
            source_entry,
            set_setting,
            profiles,
            use_profile,
            content_languages,
            open_data_folder,
            open_url,
            quit,
            full_screen,
            about,
            licences,
            search_start,
            recent_searches,
            clear_recent_searches,
            remember_search,
            find_people,
            more_people,
            toggle_bookmark,
            is_bookmarked
        ])
        .run(tauri::generate_context!())
        .expect("isnady-rs could not open its window");
}
