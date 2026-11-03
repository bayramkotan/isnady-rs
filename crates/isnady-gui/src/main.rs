//! isnady-rs — isnady's window in Rust (Tauri 2). The pages are drawn by the system's web view (ui/), so Arabic,
//! right-to-left text and the Amiri font render exactly; everything they show comes from isnady-core, the same core as
//! the command line iy-rs, reading the same data folder and database as the Python isnady.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use isnady_core::narrators::Person;
use isnady_core::{
    books, config, db, i18n, isnad, names, narrators, paths, readmarks, search, tables, works,
    VERSION,
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

/// What the window needs to start: the version, the interface language and its catalog, the theme, the books.
#[tauri::command]
fn start(state: tauri::State<Data>) -> Value {
    let lang = i18n::ui_language();
    let mut out = json!({
        "version": VERSION,
        "data_dir": paths::shown(&paths::data_dir()),
        "ui_language": lang,
        "rtl": i18n::rtl(&lang),
        "catalog": serde_json::from_str::<Value>(i18n::catalog_json(&lang)).unwrap_or(json!({})),
        "theme": config::get_str("view.theme", "system"),
        "content_languages": config::get_str("view.content_languages", ""),
        "books": [],
        "hadith": 0,
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
    out
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
            read_chapter
        ])
        .run(tauri::generate_context!())
        .expect("isnady-rs could not open its window");
}
