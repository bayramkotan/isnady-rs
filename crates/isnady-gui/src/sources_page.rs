//! Edit → Preferences → Data sources, also File → Data Sources… (gui/sources_page.py): the repository chosen, its
//! books by kind and road with what of each is imported, the languages for every book, the user's own sources and
//! the data folder. Importing, updating, adding a language to every book, removing, choosing and adding.

use serde_json::{json, Value};

use isnady_core::sources::{self, catalog, repos, store};
use isnady_core::{cache, i18n, paths, quran, sahifa, works};

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// `db.tier_for_license`: A redistributable, B with conditions, C not.
fn tier_for_license(license: Option<&str>) -> &'static str {
    let Some(l) = license.filter(|l| !l.trim().is_empty()) else {
        return "C";
    };
    let name = l.trim().to_lowercase();
    const A: [&str; 11] = [
        "unlicense",
        "cc0",
        "cc0-1.0",
        "public domain",
        "public-domain",
        "pd",
        "cc-by",
        "cc-by-4.0",
        "cc by",
        "cc by 4.0",
        "mit",
    ];
    const B: [&str; 10] = [
        "odbl",
        "odbl-1.0",
        "cc-by-sa",
        "cc-by-sa-4.0",
        "cc by-sa",
        "cc-by-nc",
        "cc-by-nc-4.0",
        "cc-by-nc-sa",
        "cc-by-nc-sa-4.0",
        "cc by-nc-sa",
    ];
    if A.contains(&name.as_str()) {
        "A"
    } else if B.contains(&name.as_str()) || name.contains("-nc") || name.contains("-sa") {
        "B"
    } else {
        "C"
    }
}

/// `titles.about`: the description in the interface language when isnady wrote one for this very English text.
fn about(entry_id: &str, english: &str, lang: &str) -> String {
    if lang == "en" || english.is_empty() {
        return english.to_string();
    }
    let item = sources::tables()
        .about
        .get(lang)
        .and_then(|m| m.get(entry_id));
    match item {
        Some(i)
            if i.get("en").and_then(Value::as_str) == Some(english)
                && i.get("text")
                    .and_then(Value::as_str)
                    .is_some_and(|t| !t.is_empty()) =>
        {
            i["text"].as_str().unwrap_or(english).to_string()
        }
        _ => english.to_string(),
    }
}

/// The key Books opens this book by, once it is imported (`SourcesPage._work_key`).
fn work_key(conn: &isnady_core::db::Connection, e: &catalog::Entry) -> Option<String> {
    if e.kind == "quran" {
        return Some("quran".into());
    }
    let names: Vec<&String> = e
        .book
        .iter()
        .chain(std::iter::once(&e.id))
        .chain(e.aliases.iter())
        .filter(|n| !n.is_empty())
        .collect();
    if names.is_empty() {
        return None;
    }
    let sql = format!(
        "SELECT key FROM works WHERE key IN ({}) LIMIT 1",
        vec!["?"; names.len()].join(",")
    );
    let mut row: Option<String> = conn
        .query_row(&sql, isnady_core::db::params_from_iter(names.iter()), |r| {
            r.get(0)
        })
        .ok();
    if row.is_none() && e.kind != "hadith" {
        row = conn
            .query_row(
                "SELECT w.key FROM works w JOIN sources s ON s.id = w.source_id WHERE s.key = ? LIMIT 1",
                [&e.source_key],
                |r| r.get(0),
            )
            .ok();
    }
    row
}

fn card(
    conn: &isnady_core::db::Connection,
    e: &catalog::Entry,
    lang: &str,
) -> Result<Value, String> {
    let st = catalog::status(conn, e).map_err(err)?;
    let mut desc = if e.builtin {
        about(&e.id, &e.description, lang)
    } else {
        e.description.clone()
    };
    if e.kind == "sahifa" {
        if let Some(known) = e.aliases.iter().find_map(|a| sahifa::find(a)) {
            desc = sahifa::text(&known.lead, lang);
        }
    }
    let name = if e.builtin {
        works::book_title(e.book.as_deref().unwrap_or(&e.id), &e.title, lang)
    } else {
        e.title.clone()
    };
    let key = if st.imported { work_key(conn, e) } else { None };
    // the Qur'an's languages: those downloaded, else those the reader reads
    let (have, wanted): (Vec<String>, Vec<String>) = if e.kind == "quran" {
        (
            if quran::available() {
                quran::editions().into_iter().map(|x| x.language).collect()
            } else {
                vec![]
            },
            quran::wanted_languages(),
        )
    } else {
        (vec![], vec![])
    };
    let quran_names: Vec<Value> = if e.kind == "quran" {
        e.languages
            .iter()
            .map(|l| json!(quran::default_of(l).unwrap_or("")))
            .collect()
    } else {
        vec![]
    };
    Ok(json!({
        "id": e.id, "repo": e.repo, "builtin": e.builtin, "kind": e.kind, "auth_kind": e.auth_kind, "road": if e.kind == "quran" { Value::Null } else { json!(catalog::road(e)) },
        "name": name, "title": e.title, "title_ar": e.title_ar, "license": e.license,
        "shareable": tier_for_license(e.license.as_deref()) != "C", "compiler": e.compiler, "size": e.size,
        "hadith": e.hadith, "origin_name": e.origin_name, "origin_url": e.origin_url, "ours": e.repo == sources::tables().default_id,
        "about": desc, "location": e.location, "book": e.book, "languages": e.languages,
        "language_labels": e.languages.iter().map(|c| catalog::language_label(c)).collect::<Vec<_>>(),
        "status": {"imported": st.imported, "languages": st.languages, "when": st.when, "count": st.count, "tier": st.tier},
        "work_key": key, "quran_have": have, "quran_wanted": wanted, "quran_names": quran_names,
    }))
}

fn why_of(u: &catalog::Update, lang: &str) -> Value {
    let name = works::book_title(
        u.entry.book.as_deref().unwrap_or(&u.entry.id),
        &u.entry.title,
        lang,
    );
    json!({"why": u.why, "name": name, "new": u.new.iter().map(|c| catalog::language_label(c)).collect::<Vec<_>>()})
}

/// Everything the page shows (`SourcesPage.refresh`).
#[tauri::command]
pub async fn src_page() -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(page)
        .await
        .map_err(err)?
}

fn page() -> Result<Value, String> {
    let lang = i18n::ui_language();
    let conn = store::connect_rw(None)?;
    let repo = repos::selected();
    let (how, how_error) = match repos::catalog(&repo, false) {
        Ok((_, from)) => (
            json!(match from {
                repos::From::Network => "network",
                repos::From::Saved => "saved",
                repos::From::Shipped if repo.url.is_none() => "shipped-own",
                repos::From::Shipped => "shipped",
            }),
            Value::Null,
        ),
        Err(e) => (Value::Null, json!(e.0)),
    };
    let (entries, entries_error) = match catalog::builtin_entries(None) {
        Ok(e) => (e, Value::Null),
        Err(e) => (vec![], json!(e.0)),
    };
    let updates: Vec<catalog::Update> = if entries.is_empty() {
        vec![]
    } else {
        catalog::updates(&conn, None).unwrap_or_default()
    };
    let mut cards = vec![];
    for e in &entries {
        let mut c = card(&conn, e, &lang)?;
        if let Some(u) = updates.iter().find(|u| u.entry.id == e.id) {
            c["update"] = why_of(u, &lang);
        }
        cards.push(c);
    }
    let mine: Vec<Value> = catalog::user_entries()
        .iter()
        .map(|e| card(&conn, e, &lang))
        .collect::<Result<_, _>>()?;
    let languages: Vec<Value> = catalog::language_overview(&conn)
        .unwrap_or_default()
        .into_iter()
        .map(|r| json!({"code": r.code, "label": r.label, "offered": r.offered, "imported": r.imported}))
        .collect();
    Ok(json!({
        "repo": {"id": repo.id, "name": repo.name, "description": repo.description, "homepage": repo.homepage,
                 "url": repo.url, "builtin": repo.builtin, "default": repo.default, "how": how, "how_error": how_error},
        "repos": repos::all_repositories().iter().map(|r| json!({"id": r.id, "name": r.name, "builtin": r.builtin})).collect::<Vec<_>>(),
        "default_id": sources::tables().default_id,
        "entries": cards, "entries_error": entries_error,
        "updates": updates.iter().map(|u| why_of(u, &lang)).collect::<Vec<_>>(),
        "languages": languages, "mine": mine,
        "formats": sources::importers().iter().map(|(id, title, _)| json!([id, title])).collect::<Vec<_>>(),
        "folder": {"path": paths::shown(&paths::data_dir()), "source": paths::data_dir_source(), "pending": store::pending()},
    }))
}

/// The repository chosen (`SourcesPage._use_repo`).
#[tauri::command]
pub fn src_use_repo(id: String) -> Result<(), String> {
    repos::select(&id).map(|_| ())
}

/// Refresh: the catalog read again; "network" when it was.
#[tauri::command]
pub async fn src_refresh_repo() -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let repo = repos::selected();
        repos::catalog(&repo, true)
            .map(|(_, from)| {
                if from == repos::From::Network {
                    "network"
                } else {
                    "other"
                }
                .to_string()
            })
            .map_err(|e| e.0)
    })
    .await
    .map_err(err)?
}

/// Add a repository (an address or a folder); it is the repository in use then.
#[tauri::command]
pub async fn src_add_repo(text: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let repo = repos::add_repository(text.trim(), None).map_err(|e| e.to_string())?;
        repos::select(&repo.id)?;
        Ok(repo.name)
    })
    .await
    .map_err(err)?
}

#[tauri::command]
pub fn src_remove_repo() -> Result<(), String> {
    let repo = repos::selected();
    repos::remove_repository(&repo.id);
    Ok(())
}

fn entry_of(id: &str, repo: &str) -> Option<catalog::Entry> {
    if repo.is_empty() {
        catalog::user_entries().into_iter().find(|e| e.id == id)
    } else {
        repos::find(repo)
            .and_then(|r| repos::entries(&r, false).ok())
            .and_then(|list| list.into_iter().find(|e| e.id == id))
    }
}

/// Remove data: everything imported from one source (`catalog.remove_entry`).
#[tauri::command]
pub async fn src_remove(id: String, repo: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let entry = entry_of(&id, &repo).ok_or_else(|| format!("No source '{id}'."))?;
        let conn = store::connect_rw(None)?;
        catalog::remove_entry(&conn, &entry).map_err(err)?;
        cache::clear();
        Ok(entry.title)
    })
    .await
    .map_err(err)?
}

/// Remove from every book (LANG2): how many editions.
#[tauri::command]
pub async fn src_remove_language(code: String) -> Result<usize, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let conn = store::connect_rw(None)?;
        let n = catalog::remove_languages(&conn, &[code])?;
        cache::clear();
        Ok(n)
    })
    .await
    .map_err(err)?
}

/// Delete from list: a user source.
#[tauri::command]
pub fn src_delete(id: String) -> bool {
    catalog::delete_user_entry(&id)
}

/// Add a source… (AddSourceDialog).
#[tauri::command]
pub fn src_add(
    title: String,
    format_id: String,
    location: String,
    license: Option<String>,
    auth_kind: String,
    key_name: Option<String>,
    book: Option<String>,
) -> Result<(), String> {
    catalog::add_user_entry(
        &title,
        &format_id,
        &location,
        license.as_deref().filter(|x| !x.trim().is_empty()),
        &auth_kind,
        key_name.as_deref().filter(|x| !x.trim().is_empty()),
        "header",
        book.as_deref().filter(|x| !x.trim().is_empty()),
    )
    .map(|_| ())
}

/// Use a ready database…: staged for the next start.
#[tauri::command]
pub async fn src_stage(path: String) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let info = store::stage(&path)?;
        Ok(json!({"books": info.books, "hadith": info.hadith, "schema": info.schema}))
    })
    .await
    .map_err(err)?
}

/// Change… / Use default: another data folder from the next start.
#[tauri::command]
pub fn src_relocate(target: Option<String>, mode: String) -> Result<String, String> {
    paths::relocate(target.as_deref(), &mode).map(|p| paths::shown(&p))
}

// ------------------------------------------------------------------ importing (sources_page.Worker)

struct Job {
    running: bool,
    text: String,
    result: Value,
}

static JOB: std::sync::Mutex<Job> = std::sync::Mutex::new(Job {
    running: false,
    text: String::new(),
    result: Value::Null,
});

fn job_say(m: &str) {
    if let Ok(mut j) = JOB.lock() {
        j.text = m.to_string();
    }
}

fn narrators(n: &sources::imports::Narrators) -> Value {
    match n {
        Some((identified, links)) => json!({"identified": identified, "links": links}),
        None => Value::Null,
    }
}

fn failed(f: &sources::imports::Failed) -> Value {
    json!(f.iter().map(|(a, b)| json!([a, b])).collect::<Vec<_>>())
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// One import of the page, as its Worker does it: "import" (one entry, its languages and credentials),
/// "import_all" (the languages ticked on each card), "update" (what is new or changed), "add_languages".
fn run_job(action: &str, args: &Value) -> Result<Value, String> {
    let conn = store::connect_rw(None)?;
    match action {
        "import_all" => {
            let mut each = std::collections::HashMap::new();
            if let Some(o) = args.get("languages").and_then(Value::as_object) {
                for (id, codes) in o {
                    let codes = strings(codes);
                    if !codes.is_empty() {
                        each.insert(id.clone(), codes);
                    }
                }
            }
            let (imported, f, n) =
                catalog::import_all(&conn, &catalog::Languages::Each(each), &job_say)?;
            Ok(
                json!({"action": action, "imported": imported.len(), "total": imported.len() + f.len(),
                      "narrators": narrators(&n), "failed": failed(&f)}),
            )
        }
        "update" => {
            let items = catalog::updates(&conn, None)?;
            let (imported, f, n) = catalog::update(&conn, &items, &job_say)?;
            Ok(
                json!({"action": action, "imported": imported.len(), "narrators": narrators(&n), "failed": failed(&f)}),
            )
        }
        // the Qur'an: these translations downloaded, those removed (AddTranslations); from the reader, they are also
        // the ones shown (QuranView._add_translations)
        "quran" => {
            let editions = strings(&args["editions"]);
            let removed = strings(&args["removed"]);
            if args["viewer"].as_bool().unwrap_or(false) {
                let mut chosen =
                    isnady_core::quran::shown(isnady_core::quran::saved_choice().as_deref());
                if !removed.is_empty() {
                    chosen.retain(|k| !removed.contains(k));
                }
                for k in &editions {
                    if !chosen.contains(k) {
                        chosen.push(k.clone());
                    }
                }
                let joined = chosen.join(",");
                isnady_core::config::state_set(
                    "quran.editions",
                    json!(if joined.is_empty() {
                        ",".into()
                    } else {
                        joined
                    }),
                );
            }
            for k in &removed {
                isnady_core::quran::remove(k);
            }
            let kept = if editions.is_empty() {
                vec![]
            } else {
                isnady_core::quran::fetch(&editions, &job_say)?
            };
            Ok(
                json!({"action": "import", "kind": "quran", "title": "The Holy Qur'an",
                      "texts": isnady_core::quran::editions().len(), "kept": kept}),
            )
        }
        // the Qur'an's card: a language ticked gets its translation, one unticked loses its translations (_import_quran)
        "quran_card" => {
            use isnady_core::quran;
            let ticked = strings(&args["ticked"]);
            let have: Vec<quran::Edition> = if quran::available() {
                quran::editions()
                    .into_iter()
                    .filter(|e| e.key != quran::ARABIC)
                    .collect()
            } else {
                vec![]
            };
            for e in &have {
                if !ticked.contains(&e.language) {
                    quran::remove(&e.key);
                }
            }
            let mut wanted: Vec<String> = have
                .iter()
                .filter(|e| ticked.contains(&e.language))
                .map(|e| e.key.clone())
                .collect();
            let langs: Vec<&str> = have.iter().map(|e| e.language.as_str()).collect();
            let mut sorted = ticked.clone();
            sorted.sort();
            for l in sorted {
                if !langs.contains(&l.as_str()) {
                    if let Some(k) = quran::default_of(&l) {
                        wanted.push(k.to_string());
                    }
                }
            }
            if wanted.is_empty() && !quran::available() {
                wanted = quran::default_editions();
            }
            if wanted.is_empty() {
                return Ok(json!({"action": "refresh"}));
            }
            quran::fetch(&wanted, &job_say)?;
            Ok(
                json!({"action": "import", "kind": "quran", "title": "The Holy Qur'an", "texts": quran::editions().len()}),
            )
        }
        "add_languages" => {
            let codes = strings(&args["codes"]);
            let (imported, f, _n) = catalog::add_languages(&conn, &codes, &job_say)?;
            Ok(
                json!({"action": action, "imported": imported.len(), "failed": failed(&f),
                      "names": codes.iter().map(|c| catalog::language_label(c)).collect::<Vec<_>>()}),
            )
        }
        _ => {
            let id = args["id"].as_str().unwrap_or("");
            let entry = entry_of(id, args["repo"].as_str().unwrap_or(""))
                .ok_or_else(|| format!("No source '{id}'."))?;
            let a = &args["auth"];
            let auth = sources::Auth {
                kind: entry.auth_kind.clone(),
                username: a["username"].as_str().map(str::to_string),
                password: a["password"].as_str().map(str::to_string),
                token: a["token"].as_str().map(str::to_string),
                key_name: entry.key_name.clone(),
                key_in: entry.key_in.clone(),
            };
            let languages = strings(&args["languages"]);
            let languages = (!languages.is_empty()).then_some(languages);
            let (report, n) =
                catalog::import_entry(&conn, &entry, languages.as_deref(), Some(&auth), &job_say)
                    .map_err(err)?;
            Ok(
                json!({"action": "import", "kind": entry.kind, "title": entry.title, "texts": report.texts,
                      "narrators": narrators(&n)}),
            )
        }
    }
}

/// Import, Update, Import all, Get updates, Add to every book: started in the background; the page asks how it goes.
#[tauri::command]
pub fn src_job(action: String, args: Value) -> Result<(), String> {
    {
        let mut j = JOB.lock().map_err(err)?;
        if j.running {
            return Err("An import is running already.".into());
        }
        j.running = true;
        j.text = String::new();
        j.result = Value::Null;
    }
    std::thread::spawn(move || {
        let started = std::time::Instant::now();
        let out = run_job(&action, &args);
        cache::clear();
        if let Err(e) = &out {
            isnady_core::log::error("import", &format!("{action}: {e}"));
        } else {
            isnady_core::log::info(
                "import",
                &format!("{action} done in {}", isnady_core::log::since(started)),
            );
        }
        if let Ok(mut j) = JOB.lock() {
            j.running = false;
            j.result = match out {
                Ok(v) => v,
                Err(e) => json!({"error": e}),
            };
        }
    });
    Ok(())
}

#[tauri::command]
pub fn src_job_status() -> Value {
    match JOB.lock() {
        Ok(j) => json!({"running": j.running, "text": j.text, "result": j.result}),
        Err(_) => json!({"running": false, "text": "", "result": null}),
    }
}

/// The translations of the Qur'an there are, isnady-data's first, then the source's (AddTranslations' list).
#[tauri::command]
pub async fn quran_offered() -> Value {
    let have: Vec<String> = isnady_core::quran::editions()
        .into_iter()
        .map(|e| e.key)
        .collect();
    match isnady_core::quran::catalog(&|_| {}, true) {
        Ok(list) => json!({"own": isnady_core::quran::OWN, "arabic": isnady_core::quran::ARABIC,
            "list": list.into_iter().map(|(k, v)| json!({"key": k, "language": v.language, "author": v.author,
                "source": v.source, "comments": v.comments, "origin": v.origin, "url": v.url,
                "have": have.contains(&k)})).collect::<Vec<_>>()}),
        Err(e) => json!({"error": e}),
    }
}
