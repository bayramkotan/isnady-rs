//! isnady-rs — isnady's window in Rust (Tauri 2). The pages are drawn by the system's web view (ui/), so Arabic,
//! right-to-left text and the Amiri font render exactly; everything they show comes from isnady-core, the same core as
//! the command line iy-rs, reading the same data folder and database as the Python isnady.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use isnady_core::narrators::Person;
use isnady_core::{
    books, config, db, i18n, isnad, names, narrators, paths, people, readmarks, search, tables,
    works, VERSION,
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

/// A search, as the Search page shows it.
#[tauri::command]
fn find(
    state: tauri::State<Data>,
    text: String,
    mode: String,
    whole_words: bool,
    books: Vec<String>,
    offset: usize,
    limit: usize,
) -> Result<Value, String> {
    if text.trim().is_empty() {
        return Ok(json!({"total": 0, "results": [], "elapsed_ms": 0, "spellings": []}));
    }
    search::add_recent(text.trim(), "hadith");
    with_db(&state, |conn| {
        let mut q = search::SearchQuery::new(text.trim());
        q.mode = mode;
        q.whole_words = whole_words;
        q.collections = books;
        q.offset = offset;
        q.limit = limit;
        let page = search::search(conn, &q).map_err(err)?;
        let shown: Vec<String> = config::get_str("view.content_languages", "")
            .split(',')
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();
        let chosen = books::language_order();
        let results: Vec<Value> = page
            .results
            .iter()
            .map(|r| {
                let mut texts: Vec<&search::TextHit> = r
                    .texts
                    .iter()
                    .filter(|t| t.matched || shown.is_empty() || shown.contains(&t.language))
                    .collect();
                texts.sort_by_key(|t| books::language_rank(&t.language, &chosen));
                json!({
                    "key": r.collection_key,
                    "name": r.collection_name,
                    "number": r.number,
                    "grades": r.grades,
                    "texts": texts.iter().map(|t| json!({
                        "language": t.language,
                        "rtl": t.direction == "rtl",
                        "text": t.text,
                        "spans": t.spans,
                        "matched": t.matched,
                    })).collect::<Vec<_>>(),
                })
            })
            .collect();
        Ok(json!({
            "total": page.total,
            "elapsed_ms": page.elapsed_ms as u64,
            "spellings": page.spellings,
            "results": results,
        }))
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

fn ui_reading_lang() -> &'static str {
    if i18n::ui_language() == "tr" {
        "tr"
    } else {
        "en"
    }
}

fn person_row(conn: &db::Connection, pid: i64, titles: &HashMap<i64, String>) -> Option<Value> {
    let p = narrators::describe(conn, pid)?;
    let v = p.verdicts.first();
    let n: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM isnad_links WHERE person_id = ?",
            [pid],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let reading = if p.known_arabic.is_empty() {
        String::new()
    } else {
        isnad::name_reading(&p.known_arabic, ui_reading_lang()).0
    };
    Some(json!({
        "id": pid, "title": titles.get(&pid), "arabic": p.display_name, "reading": reading,
        "tabaqa": p.tabaqa, "death": p.death_year_ah, "in_chains": n,
        "rank": v.and_then(|v| v.rank), "shia": v.map(|v| v.rank_scheme.as_deref() == Some("shia")),
    }))
}

/// The famous ones, group by group, as the Narrators page opens.
#[tauri::command]
fn famous(state: tauri::State<Data>, tradition: String) -> Result<Value, String> {
    let lang = i18n::ui_language();
    with_db(&state, |conn| {
        let titles = people::famous_titles(conn, &lang);
        let road = if tradition == "all" {
            None
        } else {
            Some(tradition.as_str())
        };
        let groups: Vec<Value> = people::famous_groups(road)
            .into_iter()
            .map(|g| {
                let ids = people::famous_ids(conn, &g.keys);
                json!({"id": g.id, "title": g.title, "tradition": g.tradition,
                       "people": ids.iter().filter_map(|pid| person_row(conn, *pid, &titles)).collect::<Vec<_>>()})
            })
            .filter(|g| g["people"].as_array().is_some_and(|a| !a.is_empty()))
            .collect();
        Ok(json!({"groups": groups}))
    })
}

/// Narrators whose names contain the words typed (Arabic or Latin letters), the famous ones first.
#[tauri::command]
fn find_narrators(
    state: tauri::State<Data>,
    text: String,
    tradition: String,
) -> Result<Value, String> {
    let lang = i18n::ui_language();
    with_db(&state, |conn| {
        let road = if tradition == "all" {
            None
        } else {
            Some(tradition.as_str())
        };
        let mut found: Vec<i64> = people::search_persons(conn, &text, 60)
            .map_err(err)?
            .into_iter()
            .map(|f| f.0)
            .collect();
        let first = people::famous_ids(conn, &people::famous_named(&text, road));
        if !first.is_empty() {
            let mut merged = first.clone();
            merged.extend(found.into_iter().filter(|f| !first.contains(f)));
            found = merged;
        }
        if let Some(r) = road {
            found.retain(|pid| {
                conn.query_row(
                    "SELECT COALESCE(tradition, 'sunni') FROM persons WHERE id = ?",
                    [*pid],
                    |x| x.get::<_, String>(0),
                )
                .map(|t| t == r)
                .unwrap_or(true)
            });
        }
        let titles = people::famous_titles(conn, &lang);
        Ok(
            json!({"people": found.iter().filter_map(|pid| person_row(conn, *pid, &titles)).collect::<Vec<_>>()}),
        )
    })
}

/// One narrator: his names, every verdict, his generation, his books, his teachers and students, his hadith.
#[tauri::command]
fn narrator(state: tauri::State<Data>, id: i64) -> Result<Value, String> {
    let lang = i18n::ui_language();
    with_db(&state, |conn| {
        let Some(d) = people::describe(conn, id) else {
            return Ok(json!({"error": "no_person"}));
        };
        let titles = people::famous_titles(conn, &lang);
        let rel = people::relations(conn, id, 12).map_err(err)?;
        let (hadith, total) = people::hadiths_of(conn, id, 60, &lang).map_err(err)?;
        let reading = if d.person.known_arabic.is_empty() {
            String::new()
        } else {
            isnad::name_reading(&d.person.known_arabic, ui_reading_lang()).0
        };
        let rl = |v: &Vec<people::Related>| -> Vec<Value> {
            v.iter()
                .map(|r| json!({"id": r.id, "name": r.name, "count": r.count}))
                .collect()
        };
        Ok(json!({
            "id": id, "title": titles.get(&id), "arabic": d.person.display_name, "known_arabic": d.person.known_arabic,
            "reading": reading, "full_reading": names::latin(&d.person.display_name, ui_reading_lang(), 8),
            "tradition": d.tradition, "source": d.source,
            "tabaqa": d.person.tabaqa, "tabaqa_label": d.tabaqa_label, "death": d.person.death_year_ah,
            "death_note": d.death_year_note, "other_names": d.other_names,
            "verdicts": d.verdicts.iter().map(|v| json!({
                "critic": v.critic_name, "work": v.work, "phrase": v.phrase, "rank": v.rank,
                "shia": v.rank_scheme.as_deref() == Some("shia"), "label": v.rank_label, "madhhab": v.madhhab_label,
            })).collect::<Vec<_>>(),
            "marks": d.marks, "in_chains": d.in_chains,
            "teachers": rl(&rel.teachers), "students": rl(&rel.students),
            "compilers": rel.compilers.iter().map(|(b, k, n)| json!({"book": works::book_title(k, b, &lang), "count": n})).collect::<Vec<_>>(),
            "hadith": hadith.iter().map(|h| json!({"key": h.key, "book": h.book, "number": h.number})).collect::<Vec<_>>(),
            "hadith_total": total,
        }))
    })
}

/// A number of the book chosen at random, of a hadith whose chain was read.
#[tauri::command]
fn random_number(state: tauri::State<Data>, book: String) -> Result<Option<String>, String> {
    with_db(&state, |conn| Ok(isnad::random_number(conn, &book)))
}

fn main() {
    tauri::Builder::default()
        .manage(Data(Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![
            start,
            find,
            chain,
            random_number,
            shelf,
            contents,
            read_chapter,
            famous,
            find_narrators,
            narrator,
            set_setting,
            profiles,
            use_profile,
            content_languages,
            open_data_folder,
            open_url,
            quit,
            full_screen,
            about,
            licences
        ])
        .run(tauri::generate_context!())
        .expect("isnady-rs could not open its window");
}
