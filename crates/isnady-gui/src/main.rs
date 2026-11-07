//! isnady-rs — isnady's window in Rust (Tauri 2). The pages are drawn by the system's web view (ui/), so Arabic,
//! right-to-left text and the Amiri font render exactly; everything they show comes from isnady-core, the same core as
//! the command line iy-rs, reading the same data folder and database as the Python isnady.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use isnady_core::narrators::Person;
use isnady_core::{
    bookmarks, books, config, db, i18n, isnad, lives, names, narrators, paths, people, py, quran,
    readmarks, scholars, search, tables, works, VERSION,
};
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
    json!({
        "ui.family": config::get_str("ui.family", ""),
        "ui.size": config::get_f64("ui.size", 0.0),
        "text": text,
    })
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
    let window: Value = serde_json::from_str(i18n::window_json()).map_err(err)?;
    let of_language: HashMap<String, String> =
        serde_json::from_value(window["script_of_language"].clone()).unwrap_or_default();
    let compilers = window["compilers"].clone();
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
        let page = search::search(conn, &q).map_err(err)?;
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
        let mut results = vec![];
        for r in &page.results {
            // the content languages (L1): the texts chosen and any the search matched; the others folded or left out
            let kept: Vec<&search::TextHit> = r
                .texts
                .iter()
                .filter(|t| shown.is_empty() || shown.contains(&t.language) || !t.spans.is_empty())
                .collect();
            let (texts, folded): (Vec<&search::TextHit>, Vec<&search::TextHit>) =
                if !kept.is_empty() && kept.len() != r.texts.len() {
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
            let name = works::book_title(&r.collection_key, &r.collection_name, &lang);
            let first = r
                .texts
                .iter()
                .find(|x| x.language != "Arabic")
                .or(r.texts.first())
                .map(|x| x.text.split_whitespace().collect::<Vec<_>>().join(" "))
                .unwrap_or_default();
            let reference = format!("{}:{}", r.collection_key, r.number);
            results.push(json!({
                "hadith_id": r.hadith_id,
                "key": r.collection_key,
                "name": name,
                "number": r.number,
                "texts": texts.iter().map(|t| text_json(t, &of_language)).collect::<Vec<_>>(),
                "folded": folded.iter().map(|t| text_json(t, &of_language)).collect::<Vec<_>>(),
                "grades": r.grades.iter().map(|(g, v)| json!([g, v, isnady_core::grades::group(v)])).collect::<Vec<_>>(),
                "related": isnady_core::takhrij::related(conn, r.hadith_id, &lang).iter().map(|x| json!({
                    "hadith_id": x.hadith_id, "kind": x.kind, "book": x.book, "book_name": x.book_name,
                    "number": x.number,
                })).collect::<Vec<_>>(),
                "verses": verses_of.get(&r.hadith_id).map(|v| v.iter().map(|&(s, a, b)| json!([s, a, b,
                    quran::label(s, a, b, &lang)])).collect::<Vec<_>>()).unwrap_or_default(),
                "chains": drawn,
                "compiler": compilers.get(&r.collection_key).cloned().unwrap_or(Value::Null),
                "codes": codes,
                "bookmarked": bookmarks::is_marked("hadith", &reference),
                "subtitle": first.chars().take(160).collect::<String>(),
            }));
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

/// Where a hadith stands in its book (In its book): the book and its chapter's place in the reading order.
#[tauri::command]
fn hadith_place(state: tauri::State<Data>, hadith_id: i64) -> Result<Value, String> {
    let lang = i18n::ui_language();
    with_db(&state, |conn| {
        let (key, number): (String, String) = conn
            .query_row(
                "SELECT c.key, h.number FROM hadiths h JOIN collections c ON c.id = h.collection_id WHERE h.id = ?",
                [hadith_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(err)?;
        let index = works::find_work(conn, &key, &lang)
            .map_err(err)?
            .and_then(|w| {
                let chapter = works::chapter_of_hadith(conn, hadith_id)?;
                let order = works::reading_order(conn, w.id).ok()?;
                order.iter().position(|c| *c == chapter).map(|i| i + 1)
            });
        Ok(json!({"key": key, "number": number, "index": index}))
    })
}

fn person_json(p: &Person, lang: &str) -> Value {
    let t = tables::get();
    let v = p.verdicts.first();
    let reading = if p.known_arabic.is_empty() {
        String::new()
    } else {
        isnad::name_reading(&p.known_arabic, lang).0
    };
    let full = names::latin(&p.display_name, lang, 8).unwrap_or_default();
    json!({
        "id": p.id,
        "arabic": p.display_name,
        "known_arabic": p.known_arabic,
        "reading": reading,
        "full_reading": full,
        "tabaqa": p.tabaqa,
        "tabaqa_label": p.tabaqa.and_then(|n| t.rijal.tabaqat.get(&n.to_string()).cloned()),
        "death": p.death_year_ah,
        "verdict": v.map(|v| json!({
            "critic": v.critic_name,
            "phrase": v.phrase,
            "rank": v.rank,
            "shia": v.rank_scheme.as_deref() == Some("shia"),
            "label": if v.rank_scheme.as_deref() == Some("shia") {
                v.rank_label.clone()
            } else {
                v.rank.and_then(|r| t.rijal.ranks.get(&r.to_string()).cloned()).unwrap_or_default()
            },
        })),
    })
}

/// One hadith's chains, as the Isnad Chains page draws them.
#[tauri::command]
fn chain(state: tauri::State<Data>, book: String, number: String) -> Result<Value, String> {
    let lang = if i18n::ui_language() == "tr" {
        "tr"
    } else {
        "en"
    };
    with_db(&state, |conn| {
        let rng = isnad::number_range(conn, &book).map_err(err)?;
        let Some(rng) = rng else {
            return Ok(json!({"error": "no_book"}));
        };
        let number = if number.trim().is_empty() {
            rng.first.clone()
        } else {
            number.trim().to_string()
        };
        let base = json!({
            "book": book, "number": number,
            "first": rng.first, "last": rng.last, "count": rng.count,
            "prev": isnad::step_number(conn, &book, &number, -1),
            "next": isnad::step_number(conn, &book, &number, 1),
        });
        if !isnad::isnads_ready(conn) || !narrators::links_ready(conn) {
            let mut o = base;
            o["error"] = json!("not_read");
            return Ok(o);
        }
        let Some((hid, name)) = isnad::find_hadith(conn, &book, &number).map_err(err)? else {
            let mut o = base;
            o["error"] = json!("no_number");
            o["nearest"] = json!(isnad::nearest_number(conn, &book, &number));
            return Ok(o);
        };
        let chains = isnad::chain(conn, hid).map_err(err)?;
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
        let codes = isnad::chain_codes(
            &config::get_str("view.content_languages", "")
                .split(',')
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect::<Vec<_>>(),
        );
        let borrowed = &tables::get().isnad.borrowed;
        let tradition = chains
            .first()
            .map(|c| c.tradition.clone())
            .unwrap_or_else(|| "sunni".into());
        let out_chains: Vec<Value> = chains
            .iter()
            .map(|ch| {
                let links: Vec<Value> = ch
                    .links
                    .iter()
                    .enumerate()
                    .map(|(i, l)| {
                        let (ar, meaning) = isnad::term_label(&l.transmission);
                        let nxt = ch.links.get(i + 1).map(|x| x.raw_name.as_str()).unwrap_or("");
                        json!({
                            "position": l.position,
                            "term": l.transmission,
                            "term_ar": ar,
                            "term_meaning": meaning,
                            "raw": l.raw_name,
                            "reading": isnad::name_reading(&isnad::short_name(&l.raw_name), lang).0,
                            "kind": isnad::link_kind(&l.raw_name),
                            "members": isnad::group_members(&l.raw_name, nxt),
                            "person": l.person_id.and_then(|p| people.get(&p)).map(|p| person_json(p, lang)),
                            "confidence": l.match_confidence,
                            "candidates": l.candidates,
                            "meeting": ch.meeting.as_ref().filter(|m| m.first_position == l.position).map(|m| json!({
                                "said": m.said, "attested": m.attested,
                            })),
                        })
                    })
                    .collect();
                let words: Vec<Value> = codes
                    .iter()
                    .map(|c| json!({"code": c, "text": isnad::chain_in_words(ch, c, &people)}))
                    .filter(|w| w["text"].as_str().is_some_and(|s| !s.is_empty()))
                    .collect();
                json!({
                    "links": links,
                    "problem": ch.problem,
                    "borrowed": ch.problem.as_deref() == Some(borrowed.as_str()),
                    "raised": ch.raised,
                    "reaches_prophet": ch.reaches_prophet,
                    "end": isnad::end_label(ch),
                    "jamian": isnady_core::normalize::normalize(ch.raw.as_deref().unwrap_or("")).contains("جميعا"),
                    "raw": ch.raw,
                    "words": words,
                })
            })
            .collect();
        let mut o = base;
        o["name"] = json!(name);
        o["tradition"] = json!(tradition);
        o["chains"] = Value::Array(out_chains);
        Ok(o)
    })
}

/// The shelf: every book, each with its kind and tradition, in the order of the Python Books page.
#[tauri::command]
fn shelf(state: tauri::State<Data>) -> Result<Value, String> {
    let lang = i18n::ui_language();
    with_db(&state, |conn| {
        if !works::ready(conn) {
            return Ok(json!({"error": "not_ready", "works": []}));
        }
        let list = works::list_works(conn, &lang).map_err(err)?;
        let labels = &works::library().kind_labels;
        Ok(json!({"works": list.iter().map(|w| json!({
            "key": w.key, "title": w.title, "title_ar": w.title_ar, "kind": w.kind,
            "kind_label": labels.get(&w.kind).cloned().unwrap_or_else(|| w.kind.clone()),
            "leaves": w.leaves, "chapters": w.chapters, "tradition": w.tradition,
            "last": readmarks::last(&w.key).map(|p| readmarks::describe(&p, &w.title, &lang)),
        })).collect::<Vec<_>>()}))
    })
}

/// A book's chapters in reading order, and where it was left.
#[tauri::command]
fn contents(state: tauri::State<Data>, key: String) -> Result<Value, String> {
    let lang = i18n::ui_language();
    with_db(&state, |conn| {
        let Some(work) = works::find_work(conn, &key, &lang).map_err(err)? else {
            return Ok(json!({"error": "no_book"}));
        };
        let order = works::reading_order(conn, work.id).map_err(err)?;
        let chapters: Vec<Value> = order
            .iter()
            .enumerate()
            .map(|(i, cid)| {
                let trail: Vec<String> = works::path(conn, *cid, &lang)
                    .into_iter()
                    .map(|p| p.title)
                    .collect();
                let count = works::leaves(conn, *cid, 0, 0).map(|x| x.1).unwrap_or(0);
                json!({"index": i + 1, "trail": trail, "count": count})
            })
            .collect();
        let last = readmarks::last(&work.key).and_then(|p| {
            let (at, _h) = readmarks::resolve(conn, &p);
            at.and_then(|a| order.iter().position(|c| *c == a)).map(
                |i| json!({"index": i + 1, "where": readmarks::describe(&p, &work.title, &lang)}),
            )
        });
        Ok(
            json!({"key": work.key, "title": work.title, "kind": work.kind, "tradition": work.tradition,
                  "chapters": chapters, "last": last}),
        )
    })
}

/// One chapter to read: its items (a hadith with its texts, an entry, a section of the sira), and the place kept.
#[tauri::command]
fn read_chapter(
    state: tauri::State<Data>,
    key: String,
    index: usize,
    offset: i64,
    limit: i64,
) -> Result<Value, String> {
    let lang = i18n::ui_language();
    with_db(&state, |conn| {
        let Some(work) = works::find_work(conn, &key, &lang).map_err(err)? else {
            return Ok(json!({"error": "no_book"}));
        };
        let order = works::reading_order(conn, work.id).map_err(err)?;
        if index == 0 || index > order.len() {
            return Ok(json!({"error": "no_chapter", "count": order.len()}));
        }
        let cid = order[index - 1];
        let (items, total) = works::leaves(conn, cid, offset, limit).map_err(err)?;
        if offset == 0 {
            let first = items.iter().find_map(|it| it.hadith_id);
            let place = readmarks::place_of(conn, &work.key, Some(cid), first);
            readmarks::remember(
                &place.book,
                &place.chapter,
                &place.number,
                &place.title,
                place.place,
            );
        }
        let shown: Vec<String> = config::get_str("view.content_languages", "")
            .split(',')
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();
        let q = search::SearchQuery::new("");
        let out: Vec<Value> = items
            .iter()
            .map(|it| {
                if let (true, Some(hid)) = (it.kind == "hadith", it.hadith_id) {
                    match search::load_result(conn, hid, &[], &q) {
                        Ok(r) => json!({
                            "kind": "hadith", "key": r.collection_key, "number": r.number, "grades": r.grades,
                            "texts": r.texts.iter()
                                .filter(|t| shown.is_empty() || shown.contains(&t.language) || t.language == "Arabic")
                                .map(|t| json!({"language": t.language, "rtl": t.direction == "rtl", "text": t.text}))
                                .collect::<Vec<_>>(),
                        }),
                        Err(_) => json!({"kind": "missing"}),
                    }
                } else {
                    json!({"kind": it.kind, "label": it.label, "title": it.title, "text": it.text})
                }
            })
            .collect();
        let mut trail = vec![work.title.clone()];
        trail.extend(works::path(conn, cid, &lang).into_iter().map(|p| p.title));
        Ok(
            json!({"key": work.key, "kind": work.kind, "index": index, "count": order.len(), "trail": trail,
                  "total": total, "offset": offset, "items": out}),
        )
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

/// Narrators → "Open his entry in the book" (books_page.show_person): the rijal work and the chapter (in reading
/// order) that hold his entry.
#[tauri::command]
fn person_place(state: tauri::State<Data>, id: i64) -> Result<Value, String> {
    let lang = i18n::ui_language();
    with_db(&state, |conn| {
        let chapter: Option<i64> = conn
            .query_row(
                "SELECT parent_id FROM work_nodes WHERE person_id = ? AND kind = 'entry' LIMIT 1",
                [id],
                |r| r.get(0),
            )
            .ok()
            .flatten();
        let Some(chapter) = chapter else {
            return Ok(Value::Null);
        };
        let Some(node) = works::node(conn, chapter, &lang) else {
            return Ok(Value::Null);
        };
        let work = works::list_works(conn, &lang)
            .map_err(err)?
            .into_iter()
            .find(|w| w.id == node.work_id);
        Ok(match work {
            Some(w) => {
                let order = works::reading_order(conn, w.id).map_err(err)?;
                json!({"key": w.key, "index": order.iter().position(|c| *c == chapter).map(|i| i + 1)})
            }
            None => Value::Null,
        })
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

/// A number of the book chosen at random, of a hadith whose chain was read.
#[tauri::command]
fn random_number(state: tauri::State<Data>, book: String) -> Result<Option<String>, String> {
    with_db(&state, |conn| Ok(isnad::random_number(conn, &book)))
}

fn main() {
    tauri::Builder::default()
        .manage(Data(Mutex::new(None)))
        .manage(Cites(Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![
            start,
            find,
            chain,
            random_number,
            shelf,
            contents,
            read_chapter,
            narrators_overview,
            narrators_list,
            narrators_famous,
            narrator_names,
            narrator_page,
            source_entry,
            person_place,
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
            is_bookmarked,
            hadith_place
        ])
        .run(tauri::generate_context!())
        .expect("isnady-rs could not open its window");
}
