//! iy-rs formats, repo, catalog, database, remove, import (RS16, A2): the data repositories and the sources, as the
//! Python isnady's `cli.cmd_formats`, `cmd_repo`, `cmd_catalog`, `cmd_database`, `cmd_remove`, `cmd_import`.

use clap::Args;
use std::io::Write;
use std::process::ExitCode;

use isnady_core::py::thousands;
use isnady_core::sources::importers::{ImportError, ImportReport};
use isnady_core::sources::imports::{self, ImportRequest, Narrators};
use isnady_core::sources::{catalog, repos, store, Auth};

fn rw() -> Result<isnady_core::db::Connection, ExitCode> {
    store::connect_rw(None).map_err(|e| {
        eprintln!("{e}");
        ExitCode::from(1)
    })
}

macro_rules! conn {
    () => {
        match rw() {
            Ok(c) => c,
            Err(code) => return code,
        }
    };
}

/// What an import says while it works: indented, at once (`print(f"  {m}")`).
fn say(m: &str) {
    println!("  {m}");
    let _ = std::io::stdout().flush();
}

fn narrators_line(narrators: &Narrators) {
    if let Some((identified, links)) = narrators {
        println!(
            "  narrators identified: {identified} of {links} links ({:.1}%)",
            100.0 * *identified as f64 / *links as f64
        );
    }
}

/// `_print_report`.
fn print_report(report: &ImportReport, narrators: &Narrators) {
    println!(
        "Done: source '{}' — {} edition(s), {} texts, {} new hadith, {} new grades",
        report.source_key,
        report.editions.len(),
        report.texts,
        report.new_hadiths,
        report.grades
    );
    if !report.skipped_editions.is_empty() {
        println!(
            "  skipped diacritics-free copies (use --include-plain to keep): {}",
            report.skipped_editions.join(", ")
        );
    }
    for w in &report.warnings {
        println!("  warning: {w}");
    }
    narrators_line(narrators);
}

fn import_failed(e: ImportError) -> ExitCode {
    match e {
        ImportError::Resource(m) => eprintln!("Import failed: {m}"),
        ImportError::Db(e) => eprintln!("{e}"),
    }
    ExitCode::from(1)
}

fn ask_line(prompt: &str) -> String {
    print!("{prompt}");
    let _ = std::io::stdout().flush();
    let mut line = String::new();
    let _ = std::io::stdin().read_line(&mut line);
    line.trim_end_matches(['\r', '\n']).to_string()
}

fn ask_secret(prompt: &str) -> String {
    rpassword::prompt_password(prompt).unwrap_or_default()
}

/// iy-rs formats.
pub fn formats_cmd() -> ExitCode {
    for (id, title, description) in isnady_core::sources::importers() {
        println!("{id:14} {title}\n{:14} {description}", "");
    }
    ExitCode::SUCCESS
}

#[derive(Args)]
pub struct RepoArgs {
    #[arg(default_value = "list", value_parser = ["list", "use", "show", "add", "remove"])]
    action: String,
    /// use/show/remove: a repository id; add: a GitHub or Hugging Face repository, a catalog URL or a folder
    target: Option<String>,
    /// add: the name shown for it
    #[arg(long)]
    name: Option<String>,
    /// show: read the catalog again from the repository
    #[arg(long)]
    refresh: bool,
}

/// iy-rs repo list|use|show|add|remove.
pub fn repo_cmd(a: RepoArgs) -> ExitCode {
    if a.action == "list" {
        let chosen = repos::selected().id;
        for r in repos::all_repositories() {
            let mark = if r.id == chosen { "*" } else { " " };
            let kind = if r.default {
                "default"
            } else if !r.builtin {
                "added by you"
            } else {
                "optional"
            };
            println!(" {mark} {:22} {:46} {kind}", r.id, r.name);
        }
        println!(
            "\n* the repository in use. Choose: iy repo use ID · its sources: iy catalog list"
        );
        return ExitCode::SUCCESS;
    }
    if a.action == "add" {
        let Some(target) = a.target.filter(|t| !t.is_empty()) else {
            eprintln!("Give the repository: iy repo add https://github.com/owner/name");
            return ExitCode::from(2);
        };
        return match repos::add_repository(&target, a.name.as_deref()) {
            Ok(r) => {
                println!("Added {}: {}. Use it: iy repo use {}", r.id, r.name, r.id);
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{e}");
                ExitCode::from(1)
            }
        };
    }
    let repo = match a.target.as_deref().filter(|t| !t.is_empty()) {
        Some(t) => repos::find(t),
        None => Some(repos::selected()),
    };
    let Some(repo) = repo else {
        eprintln!(
            "No repository '{}'. See: iy repo list",
            a.target.unwrap_or_default()
        );
        return ExitCode::from(1);
    };
    match a.action.as_str() {
        "use" => {
            if let Err(e) = repos::select(&repo.id) {
                eprintln!("{e}");
                return ExitCode::from(1);
            }
            println!("Using {}. Its sources: iy catalog list", repo.name);
            ExitCode::SUCCESS
        }
        "remove" => {
            if repo.builtin {
                eprintln!("{} ships with isnady and stays in the list.", repo.name);
                return ExitCode::from(1);
            }
            repos::remove_repository(&repo.id);
            println!(
                "Removed {} from the list (what you imported from it stays).",
                repo.name
            );
            ExitCode::SUCCESS
        }
        _ => {
            let (cat, from) = match repos::catalog(&repo, a.refresh) {
                Ok(x) => x,
                Err(e) => {
                    eprintln!("{e}");
                    return ExitCode::from(1);
                }
            };
            println!("{} ({})\n  {}", repo.name, repo.id, repo.description);
            if let Some(h) = &repo.homepage {
                println!("  {h}");
            }
            match &repo.url {
                Some(url) => println!(
                    "  catalog: {url} — read from {}",
                    match from {
                        repos::From::Network => "the repository now",
                        repos::From::Saved => "the copy saved in the last day",
                        repos::From::Shipped => "the copy shipped with isnady (not reachable)",
                    }
                ),
                None => println!(
                    "  catalog: shipped with isnady (this dataset publishes none of its own)"
                ),
            }
            for e in cat
                .get("entries")
                .and_then(|v| v.as_array())
                .into_iter()
                .flatten()
            {
                let g = |k: &str| e.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
                println!("  {:20} {:6} {}", g("id"), g("tradition"), g("title"));
            }
            ExitCode::SUCCESS
        }
    }
}

#[derive(Args)]
pub struct CatalogArgs {
    /// languages: one language for every book — iy catalog languages [add|remove] tr en (LANG2); updates: what is
    /// new or changed in the repository; update: import only that (UPD5)
    #[arg(value_parser = ["list", "import", "remove", "add", "delete", "languages", "updates", "update"])]
    action: String,
    /// source id (see list); for add: a title
    id: Option<String>,
    /// add: data format, see 'iy formats'
    format: Option<String>,
    /// add: file path or URL
    location: Option<String>,
    /// import: language code or name (repeatable; default ara, tur, eng)
    #[arg(long)]
    language: Vec<String>,
    /// import: every source of the repository (hadith books first, then the rijal works)
    #[arg(long)]
    all: bool,
    /// add: licence of the data
    #[arg(long)]
    license: Option<String>,
    /// add: book key for index files (e.g. bukhari)
    #[arg(long)]
    book: Option<String>,
    /// add: kind of authentication; credentials are asked at import and never stored
    #[arg(long, default_value = "none", value_parser = ["none", "basic", "bearer", "apikey"])]
    auth: String,
    /// add: header name for an API key
    #[arg(long)]
    api_key_header: Option<String>,
    /// add: query parameter name for an API key
    #[arg(long)]
    api_key_param: Option<String>,
    /// list / import: the sources of this repository instead of the chosen one
    #[arg(long)]
    repo: Option<String>,
}

fn kind_name(kind: &str) -> &str {
    match kind {
        "quran" => "The Qur'an",
        "hadith" => "Hadith collections",
        "sahifa" => "Written sources",
        "sira" => "Sira",
        "tabaqat" => "Tabaqat",
        "rijal" => "Rijal",
        k => k,
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f
            .to_uppercase()
            .chain(c.flat_map(|x| x.to_lowercase()))
            .collect(),
        None => String::new(),
    }
}

fn list(conn: &isnady_core::db::Connection, repo_arg: Option<&str>) -> ExitCode {
    let repo = match repo_arg.filter(|r| !r.is_empty()) {
        Some(r) => repos::find(r),
        None => Some(repos::selected()),
    };
    let Some(repo) = repo else {
        eprintln!(
            "No repository '{}'. See: iy repo list",
            repo_arg.unwrap_or_default()
        );
        return ExitCode::from(1);
    };
    for builtin in [true, false] {
        let (mut entries, title) = if builtin {
            match repos::entries(&repo, false) {
                Ok(e) => (e, format!("Sources of {}", repo.name)),
                Err(e) => {
                    eprintln!("{e}");
                    return ExitCode::from(1);
                }
            }
        } else {
            (
                catalog::user_entries(),
                "Your sources (User Resources)".to_string(),
            )
        };
        let mut heads: Vec<(String, String)> = vec![];
        if builtin {
            catalog::sort_for_list(&mut entries);
            let mut folders: Vec<(String, String, String)> = vec![];
            for e in &entries {
                let road = catalog::road(e);
                let folder = if road.is_empty() {
                    format!("{}/", e.kind)
                } else {
                    format!("{}/{road}/", e.kind)
                };
                if folders.iter().any(|(f, _, _)| *f == folder) {
                    continue;
                }
                let label = format!(
                    "{}{}",
                    kind_name(&e.kind),
                    if road.is_empty() {
                        String::new()
                    } else {
                        format!(" · {}", capitalize(road))
                    }
                );
                folders.push((
                    folder.clone(),
                    e.id.clone(),
                    format!("{label}  (isnady-data/{folder})"),
                ));
            }
            heads = folders.into_iter().map(|(_, id, h)| (id, h)).collect();
        }
        println!("\n{title}");
        if entries.is_empty() {
            println!("  none yet — add one: iy catalog add TITLE FORMAT FILE-OR-URL");
        }
        for e in &entries {
            if let Some((_, h)) = heads.iter().find(|(id, _)| *id == e.id) {
                println!("\n  — {h} —");
            }
            let st = match catalog::status(conn, e) {
                Ok(s) => s,
                Err(err) => {
                    eprintln!("{err}");
                    return ExitCode::from(1);
                }
            };
            let state = if st.imported {
                let langs = if st.languages.is_empty() {
                    String::new()
                } else {
                    format!(
                        " ({})",
                        st.languages
                            .iter()
                            .map(|l| catalog::shown_language(l))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                };
                format!("imported{langs}, {}", thousands(st.count))
            } else {
                "not imported".to_string()
            };
            let kind = if e.builtin || e.kind != "hadith" {
                e.kind.as_str()
            } else {
                ""
            };
            println!("  {:38} {kind:7} {:46} {state}", e.id, e.title);
        }
    }
    ExitCode::SUCCESS
}

/// iy-rs catalog list|import|remove|add|delete|languages|updates|update.
pub fn catalog_cmd(a: CatalogArgs) -> ExitCode {
    let action = a.action.as_str();
    if action == "updates" || action == "update" {
        let conn = conn!();
        let items = match catalog::updates(&conn, a.repo.as_deref()) {
            Ok(i) => i,
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::from(1);
            }
        };
        if items.is_empty() {
            println!("Up to date: no new book, no changed book, no new language of the languages you read.");
            return ExitCode::SUCCESS;
        }
        for i in &items {
            let why = match i.why.as_str() {
                "new" => "new in the repository".to_string(),
                "changed" => "changed in the repository".to_string(),
                _ => format!(
                    "new language(s): {}",
                    i.new
                        .iter()
                        .map(|c| catalog::language_label(c))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            };
            println!("  {:46} {why}", i.entry.id);
        }
        if action == "updates" {
            println!("{} to bring up to date: iy catalog update", items.len());
            return ExitCode::SUCCESS;
        }
        return match catalog::update(&conn, &items, &say) {
            Ok((imported, failed, _n)) => {
                println!("Brought up to date: {} of {}", imported.len(), items.len());
                for (label, why) in &failed {
                    eprintln!("  failed: {label}: {why}");
                }
                if failed.is_empty() {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::from(1)
                }
            }
            Err(e) => {
                eprintln!("{e}");
                ExitCode::from(1)
            }
        };
    }
    if action == "languages" {
        let conn = conn!();
        let what = a.id.clone().unwrap_or_default().to_lowercase();
        let mut named: Vec<String> = vec![];
        if let Some(f) = &a.format {
            named.push(f.clone());
            named.extend(a.location.clone());
        }
        named.extend(a.language.clone());
        let codes: Vec<String> = named
            .iter()
            .filter(|x| !x.is_empty())
            .map(|x| catalog::code_for(x))
            .collect();
        if what.is_empty() || what == "list" {
            match catalog::language_overview(&conn) {
                Ok(rows) => {
                    for r in rows {
                        println!(
                            "  {}  {:12} imported in {:>2} of the {} books that offer it",
                            r.code, r.label, r.imported, r.offered
                        );
                    }
                }
                Err(e) => {
                    eprintln!("{e}");
                    return ExitCode::from(1);
                }
            }
            println!("Add one to every book: iy catalog languages add tr en   ·   remove: iy catalog languages remove ur");
            return ExitCode::SUCCESS;
        }
        if codes.is_empty() {
            eprintln!("Name the languages: iy catalog languages add tr en");
            return ExitCode::from(2);
        }
        if what == "add" {
            return match catalog::add_languages(&conn, &codes, &say) {
                Ok((imported, failed, _n)) => {
                    let tail = if failed.is_empty() {
                        ".".to_string()
                    } else {
                        format!(
                            "; failed: {}",
                            failed
                                .iter()
                                .map(|(a, b)| format!("{a} ({b})"))
                                .collect::<Vec<_>>()
                                .join("; ")
                        )
                    };
                    println!("Added to {} book(s){tail}", imported.len());
                    if failed.is_empty() {
                        ExitCode::SUCCESS
                    } else {
                        ExitCode::from(1)
                    }
                }
                Err(e) => {
                    eprintln!("{e}");
                    ExitCode::from(1)
                }
            };
        }
        if what == "remove" {
            if codes.iter().any(|c| c == "ara") {
                eprintln!("Arabic stays: the chains and the narrators are read from it.");
            }
            return match catalog::remove_languages(&conn, &codes) {
                Ok(n) => {
                    println!("Removed {n} edition(s).");
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{e}");
                    ExitCode::from(1)
                }
            };
        }
        eprintln!("iy catalog languages [list | add CODES | remove CODES]");
        return ExitCode::from(2);
    }
    if action == "list" {
        let conn = conn!();
        return list(&conn, a.repo.as_deref());
    }
    if action == "add" {
        let (Some(title), Some(format), Some(location)) = (&a.id, &a.format, &a.location) else {
            eprintln!("Give a title, a format and a file or URL: iy catalog add \"My Bukhari\" fawazahmed0 ./bukhari.json");
            return ExitCode::from(2);
        };
        return match catalog::add_user_entry(
            title,
            format,
            location,
            a.license.as_deref(),
            &a.auth,
            a.api_key_header.as_deref().or(a.api_key_param.as_deref()),
            if a.api_key_param.is_some() {
                "query"
            } else {
                "header"
            },
            a.book.as_deref(),
        ) {
            Ok(e) => {
                println!(
                    "Added {}: {}. Import it with: iy catalog import {}",
                    e.id, e.title, e.id
                );
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{e}");
                ExitCode::from(2)
            }
        };
    }
    if action == "delete" {
        let id = a.id.clone().unwrap_or_default();
        let ok = catalog::delete_user_entry(&id);
        if ok {
            println!("Deleted.");
        } else {
            println!("No user source '{id}'. See: iy catalog list");
        }
        return if ok {
            ExitCode::SUCCESS
        } else {
            ExitCode::from(1)
        };
    }
    if action == "import" && a.all {
        let conn = conn!();
        let languages = catalog::Languages::Every(
            (!a.language.is_empty())
                .then(|| a.language.iter().map(|l| catalog::code_for(l)).collect()),
        );
        let (imported, failed, narrators) = match catalog::import_all(&conn, &languages, &say) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::from(1);
            }
        };
        println!(
            "Done: {} source(s) imported{}",
            imported.len(),
            if failed.is_empty() {
                String::new()
            } else {
                format!(", {} failed", failed.len())
            }
        );
        for (label, message) in &failed {
            println!("  failed: {label}: {message}");
        }
        narrators_line(&narrators);
        return if failed.is_empty() {
            ExitCode::SUCCESS
        } else {
            ExitCode::from(1)
        };
    }
    let wanted = match (&a.repo, &a.id) {
        (Some(r), Some(i)) if !r.is_empty() && !i.is_empty() => format!("{r}/{i}"),
        _ => a.id.clone().unwrap_or_default(),
    };
    let Some(entry) = catalog::find(&wanted) else {
        eprintln!(
            "No source '{}'. See: iy catalog list",
            a.id.clone().unwrap_or_else(|| "None".into())
        );
        return ExitCode::from(1);
    };
    let conn = conn!();
    if action == "remove" {
        return match catalog::remove_entry(&conn, &entry) {
            Ok(true) => {
                println!("Removed what {} imported.", entry.id);
                ExitCode::SUCCESS
            }
            Ok(false) => {
                println!("{} was not imported.", entry.id);
                ExitCode::from(1)
            }
            Err(e) => {
                eprintln!("{e}");
                ExitCode::from(1)
            }
        };
    }
    let auth = match entry.auth_kind.as_str() {
        "basic" => {
            let user = ask_line(&format!("Username for {}: ", entry.title));
            Auth {
                kind: "basic".into(),
                username: Some(user),
                password: Some(ask_secret("Password: ")),
                ..Default::default()
            }
        }
        k @ ("bearer" | "apikey") => Auth {
            kind: k.into(),
            token: Some(ask_secret(if k == "bearer" {
                "Token: "
            } else {
                "API key: "
            })),
            key_name: entry.key_name.clone(),
            key_in: entry.key_in.clone(),
            ..Default::default()
        },
        _ => Auth::default(),
    };
    let languages: Option<Vec<String>> =
        (!a.language.is_empty()).then(|| a.language.iter().map(|l| catalog::code_for(l)).collect());
    match catalog::import_entry(&conn, &entry, languages.as_deref(), Some(&auth), &say) {
        Ok((report, narrators)) => {
            print_report(&report, &narrators);
            ExitCode::SUCCESS
        }
        Err(e) => import_failed(e),
    }
}

#[derive(Args)]
pub struct DatabaseArgs {
    #[arg(default_value = "show", value_parser = ["show", "use"])]
    action: String,
    /// use: isnady.db or isnady-db-<date>.zip
    file: Option<String>,
}

/// iy-rs database [use FILE].
pub fn database_cmd(a: DatabaseArgs) -> ExitCode {
    if a.action == "use" {
        let Some(file) = a.file.filter(|f| !f.is_empty()) else {
            eprintln!("Give the file: iy database use ~/Downloads/isnady-db-2026-10-10.zip");
            return ExitCode::from(2);
        };
        let info = match store::stage(&file) {
            Ok(i) => i,
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::from(1);
            }
        };
        match store::apply_pending() {
            Err(_) => {
                println!(
                    "Ready: {} books, {} hadith. isnady is open and holds the database; it is used from the next start.",
                    info.books,
                    thousands(info.hadith)
                );
            }
            Ok(backup) => {
                println!(
                    "Using the ready database: {} books, {} hadith (schema {}){}.",
                    info.books,
                    thousands(info.hadith),
                    info.schema,
                    backup
                        .map(|b| format!("; your previous database is kept as {}", b.display()))
                        .unwrap_or_default()
                );
                println!("If the isnady window is open, close and start it again.");
            }
        }
        return ExitCode::SUCCESS;
    }
    println!(
        "database: {}{}",
        isnady_core::paths::db_path().display(),
        if store::pending() {
            "  (a ready database is waiting for the next start)"
        } else {
            ""
        }
    );
    ExitCode::SUCCESS
}

/// iy-rs remove KEY: a source and everything imported from it.
pub fn remove_cmd(key: &str) -> ExitCode {
    let conn = conn!();
    match store::remove_source(&conn, key) {
        Ok(true) => {
            println!("Removed source '{key}' and everything imported from it.");
            ExitCode::SUCCESS
        }
        Ok(false) => {
            eprintln!("No source with key '{key}'. See: iy sources");
            ExitCode::from(1)
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(1)
        }
    }
}

/// iy-rs reset books|all (`cli.cmd_reset`): the books out of the database, or the whole profile.
pub fn reset_data(what: &str) -> ExitCode {
    let conn = conn!();
    if what == "books" {
        return match store::reset_books(&conn) {
            Ok(n) => {
                println!("{n} sources deleted. Import them again with: iy catalog import --all");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{e}");
                ExitCode::from(1)
            }
        };
    }
    match store::reset_everything(&conn) {
        Ok(out) => {
            println!(
                "Everything deleted: {} sources, the settings{}.",
                out.books,
                if out.bookmarks { ", the bookmarks" } else { "" }
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(1)
        }
    }
}

#[derive(Args)]
pub struct ImportArgs {
    /// data format, see 'iy formats'
    format: String,
    /// local file path or http(s) URL
    location: String,
    /// display name of the source
    #[arg(long)]
    name: Option<String>,
    /// license of the data, e.g. Unlicense, CC-BY-4.0
    #[arg(long)]
    license: Option<String>,
    /// mark as built-in data instead of a User Resource
    #[arg(long)]
    builtin: bool,
    /// licence tier: A redistributable, B with conditions, C not (default: from --license, unknown = C)
    #[arg(long, value_parser = ["A", "B", "C"])]
    tier: Option<String>,
    /// edition name, e.g. tur-bukhari (repeatable)
    #[arg(long)]
    edition: Vec<String>,
    /// book key, e.g. bukhari (repeatable)
    #[arg(long)]
    book: Vec<String>,
    /// language name or code, e.g. Turkish or tur (repeatable)
    #[arg(long)]
    language: Vec<String>,
    /// import every edition in the index
    #[arg(long)]
    all: bool,
    /// edition name for a single file whose file name is not the edition name
    #[arg(long)]
    edition_key: Option<String>,
    /// also import diacritics-free Arabic copies (skipped by default)
    #[arg(long)]
    include_plain: bool,
    /// username for HTTP Basic authentication; the password is asked
    #[arg(long)]
    user: Option<String>,
    /// read the Basic password from this environment variable instead
    #[arg(long)]
    password_env: Option<String>,
    /// bearer token (visible in shell history — prefer --token-env)
    #[arg(long)]
    token: Option<String>,
    /// read the bearer token from this environment variable
    #[arg(long)]
    token_env: Option<String>,
    /// API key (visible in shell history — prefer --api-key-env)
    #[arg(long)]
    api_key: Option<String>,
    /// read the API key from this environment variable
    #[arg(long)]
    api_key_env: Option<String>,
    /// header name for the API key (default X-API-Key)
    #[arg(long)]
    api_key_header: Option<String>,
    /// send the API key as this query parameter instead of a header
    #[arg(long)]
    api_key_param: Option<String>,
}

fn secret_from_env(name: &Option<String>) -> Result<Option<String>, ExitCode> {
    let Some(name) = name.as_deref().filter(|n| !n.is_empty()) else {
        return Ok(None);
    };
    match std::env::var(name) {
        Ok(v) => Ok(Some(v)),
        Err(_) => {
            eprintln!("Environment variable {name} is not set.");
            Err(ExitCode::from(1))
        }
    }
}

/// `_auth_from_args`.
fn auth_from_args(a: &ImportArgs) -> Result<Auth, ExitCode> {
    let token = match a.token.clone().filter(|t| !t.is_empty()) {
        Some(t) => Some(t),
        None => secret_from_env(&a.token_env)?,
    };
    let api_key = match a.api_key.clone().filter(|t| !t.is_empty()) {
        Some(t) => Some(t),
        None => secret_from_env(&a.api_key_env)?,
    };
    let user = a.user.clone().filter(|u| !u.is_empty());
    let chosen = [user.is_some(), token.is_some(), api_key.is_some()];
    if chosen.iter().filter(|x| **x).count() > 1 {
        eprintln!("Use only one of: --user, --token/--token-env, --api-key/--api-key-env.");
        return Err(ExitCode::from(1));
    }
    if let Some(user) = user {
        let password = if a.password_env.is_some() {
            secret_from_env(&a.password_env)?
        } else {
            Some(ask_secret(&format!("Password for {user}: ")))
        };
        return Ok(Auth {
            kind: "basic".into(),
            username: Some(user),
            password,
            ..Default::default()
        });
    }
    if let Some(t) = token {
        return Ok(Auth {
            kind: "bearer".into(),
            token: Some(t),
            ..Default::default()
        });
    }
    if let Some(k) = api_key {
        if let Some(param) = a.api_key_param.clone() {
            return Ok(Auth {
                kind: "apikey".into(),
                token: Some(k),
                key_name: Some(param),
                key_in: "query".into(),
                ..Default::default()
            });
        }
        return Ok(Auth {
            kind: "apikey".into(),
            token: Some(k),
            key_name: Some(
                a.api_key_header
                    .clone()
                    .unwrap_or_else(|| "X-API-Key".into()),
            ),
            key_in: "header".into(),
            ..Default::default()
        });
    }
    Ok(Auth::default())
}

/// iy-rs import FORMAT LOCATION (`cmd_import`).
pub fn import_cmd(a: ImportArgs) -> ExitCode {
    if !isnady_core::sources::has_importer(&a.format) {
        let mut known: Vec<&str> = isnady_core::sources::importers()
            .iter()
            .map(|i| i.0.as_str())
            .collect();
        known.sort();
        eprintln!(
            "Unknown format '{}'. Known formats: {}",
            a.format,
            known.join(", ")
        );
        return ExitCode::from(2);
    }
    let auth = match auth_from_args(&a) {
        Ok(x) => x,
        Err(code) => return code,
    };
    let list = |v: &Vec<String>| {
        if v.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::json!(v)
        }
    };
    let mut options = serde_json::Map::new();
    options.insert("editions".into(), list(&a.edition));
    options.insert("books".into(), list(&a.book));
    options.insert("languages".into(), list(&a.language));
    options.insert("all".into(), serde_json::json!(a.all));
    options.insert("edition_key".into(), serde_json::json!(a.edition_key));
    options.insert("include_plain".into(), serde_json::json!(a.include_plain));
    let request = ImportRequest {
        format_id: a.format.clone(),
        location: a.location.clone(),
        name: a.name.clone(),
        license: a.license.clone(),
        tier: a.tier.clone(),
        origin: if a.builtin { "builtin" } else { "user" }.into(),
        auth,
        options,
    };
    let conn = conn!();
    match imports::run_import(&conn, &request, &say) {
        Ok((report, narrators)) => {
            print_report(&report, &narrators);
            ExitCode::SUCCESS
        }
        Err(e) => import_failed(e),
    }
}
