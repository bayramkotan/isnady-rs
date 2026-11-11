//! iy-rs export, import-data (EX1, A3): the same as File → Export… / Import…, as the Python isnady's
//! `cli.cmd_export` and `cmd_import_data`.

use clap::Args;
use std::io::{IsTerminal, Write};
use std::process::ExitCode;

use isnady_core::py::thousands;
use isnady_core::{formats, grades, search, transfer};

#[derive(Args)]
pub struct ExportArgs {
    #[arg(value_parser = ["hadith", "book", "search", "bookmarks", "narrators", "network", "settings", "profile", "backup", "statistics"])]
    what: Option<String>,
    /// hadith: bukhari:1 bukhari:5-10; book/network: bukhari, several (bukhari muslim) or all / sunni / shia;
    /// search and narrators: the words; profile: its name, or several
    target: Vec<String>,
    /// Several books: each in its own file, in the folder --output (isnady books and profiles always are)
    #[arg(long)]
    separate: bool,
    /// json, jsonl, csv, tsv, xlsx, md, html, txt, epub, docx, pdf, sqlite, isnady-book, graphml, gexf …
    /// (iy export --list)
    #[arg(long, short = 'f')]
    format: Option<String>,
    /// The file to write (default: a name in this folder)
    #[arg(long, short = 'o')]
    output: Option<String>,
    /// book: only this chapter (its number)
    #[arg(long)]
    chapter: Option<i64>,
    /// hadith/book/search: only these languages (repeatable), e.g. Arabic, Turkish
    #[arg(long)]
    language: Vec<String>,
    /// hadith/book/search: without the chains
    #[arg(long)]
    no_chains: bool,
    /// search: only hadith of this grade, as iy search --grade (repeatable)
    #[arg(long = "grade", value_name = "GRADE", value_parser = clap::builder::PossibleValuesParser::new(grades::filter_codes()))]
    grade: Vec<String>,
    /// search: every grade among those chosen
    #[arg(long)]
    every_grader: bool,
    /// search: al-Bukhari's and Muslim's not counted sahih
    #[arg(long)]
    no_sahihayn: bool,
    /// search: only in this chapter, as iy search --chapter (repeatable)
    #[arg(long = "in-chapter", value_name = "BOOK:CHAPTER")]
    in_chapter: Vec<String>,
    /// search: only in this book (repeatable)
    #[arg(long = "book", value_name = "KEY")]
    search_book: Vec<String>,
    /// narrators (default sunni) and network
    #[arg(long, value_parser = ["sunni", "shia", "all"])]
    tradition: Option<String>,
    /// What can be exported and imported, in which formats
    #[arg(long)]
    list: bool,
}

#[derive(Args)]
pub struct ImportDataArgs {
    #[arg(value_parser = ["book", "bookmarks", "settings", "profile", "backup"])]
    what: String,
    /// One file or several
    #[arg(required = true)]
    file: Vec<String>,
    /// When the file name does not say it
    #[arg(long)]
    format: Option<String>,
    /// bookmarks/settings: replace instead of adding
    #[arg(long)]
    replace: bool,
    /// bookmarks: put them in this folder
    #[arg(long)]
    folder: Option<String>,
    /// profile: the new profile's name
    #[arg(long)]
    name: Option<String>,
}

pub fn export_cmd(a: ExportArgs) -> ExitCode {
    let Some(kind) = a.what.clone().filter(|_| !a.list) else {
        for (kind, _label, export, import) in transfer::KINDS {
            println!("{kind:10} export: {}", export.join(", "));
            if !import.is_empty() {
                println!("{:10} import: {}", "", import.join(", "));
            }
        }
        return ExitCode::SUCCESS;
    };
    let spec = transfer::kind(&kind).expect("a kind clap let through");
    let fmt = a.format.clone().unwrap_or_else(|| spec.2[0].to_string());
    let mut target = a.output.clone().unwrap_or_else(|| {
        let tail = if a.target.is_empty() {
            String::new()
        } else {
            format!("_{}", a.target.join("_"))
        };
        format!("isnady_{kind}{tail}")
            .replace(':', "-")
            .replace('/', "_")
            + &transfer::suffix(&fmt)
    });
    let k = kind.as_str();
    let conn = if k == "settings" || k == "profile" {
        None
    } else {
        if k == "search" {
            let ready = isnady_core::sources::store::connect_rw(None)
                .map_err(|e| e.to_string())
                .and_then(|rw| {
                    search::ensure_index(&rw, &|m| eprintln!("{m}")).map_err(|e| e.to_string())
                });
            if let Err(e) = ready {
                eprintln!("{e}");
                return ExitCode::from(1);
            }
        }
        match crate::open() {
            Ok(c) => Some(c),
            Err(code) => return code,
        }
    };
    let road = a.tradition.clone().filter(|t| t != "all");
    let interactive = std::io::stderr().is_terminal();
    let started = std::time::Instant::now();
    let bar = move |done: usize, total: usize| -> bool {
        #[allow(clippy::manual_checked_ops)]
        if total > 0 {
            let gone = started.elapsed().as_secs();
            eprint!(
                "\r  {:3}%  {} / {} hadith  {} min {} s ",
                done * 100 / total,
                thousands(done as i64),
                thousands(total as i64),
                gone / 60,
                gone % 60
            );
            let _ = std::io::stderr().flush();
        }
        true
    };
    let mut filters = None;
    if k == "search" {
        let conn = conn.as_ref().expect("opened above");
        let Some(chapters) = crate::chapters_given(conn, &a.in_chapter) else {
            return ExitCode::from(1);
        };
        let mut books = a.search_book.clone();
        if books.is_empty() {
            books = chapters.iter().map(|c| c.0.clone()).collect();
            books.sort();
            books.dedup();
        }
        filters = Some(transfer::SearchFilters {
            mode: None,
            whole_words: false,
            collections: books,
            languages: a.language.clone(),
            grades: a.grade.clone(),
            every_grader: a.every_grader,
            sahihayn: !a.no_sahihayn,
            chapters: chapters.iter().map(|c| c.1.id).collect(),
        });
    }
    let first = a.target.first().map(String::as_str);
    let several = (["book", "network", "profile"].contains(&k) && a.target.len() > 1)
        || (["book", "network"].contains(&k) && matches!(first, Some("all" | "sunni" | "shia")));
    if several
        && k != "network"
        && (a.separate || fmt == "isnady-book" || k == "profile")
        && a.output.is_none()
    {
        // a folder
        let name = target.rsplit('/').next().unwrap_or("");
        if name.contains('.') {
            target = target
                .rsplit_once('.')
                .map(|x| x.0.to_string())
                .unwrap_or(target);
        }
    }
    let one = |kinds: &[&str]| kinds.contains(&k) && !a.target.is_empty();
    let request = transfer::Export {
        kind: kind.clone(),
        fmt: fmt.clone(),
        target: target.into(),
        book: if one(&["book", "network", "statistics"]) && !several {
            a.target[0].clone()
        } else {
            String::new()
        },
        books: if several && ["book", "network"].contains(&k) {
            a.target.clone()
        } else {
            vec![]
        },
        profiles: if k == "profile" && a.target.len() > 1 {
            a.target.clone()
        } else {
            vec![]
        },
        separate: a.separate,
        chapter: a.chapter,
        refs: if k == "hadith" {
            a.target.clone()
        } else {
            vec![]
        },
        query: if k == "search" || k == "narrators" {
            a.target.join(" ")
        } else {
            String::new()
        },
        languages: a.language.clone(),
        chains: !a.no_chains,
        tradition: if k != "narrators" || a.tradition.is_some() {
            road
        } else {
            Some("sunni".into())
        },
        profile: if one(&["profile"]) {
            Some(a.target[0].clone())
        } else {
            None
        },
        search: filters,
        tick: if interactive { Some(&bar) } else { None },
    };
    let say = |m: &str| {
        if !interactive {
            eprintln!("  {m}");
        }
    };
    match transfer::export(conn.as_ref(), &request, &say) {
        Ok(path) => {
            if interactive {
                eprintln!();
            }
            println!("Written ({}): {}", formats::label(&fmt), path.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{}{e}", if interactive { "\n" } else { "" });
            ExitCode::from(1)
        }
    }
}

pub fn import_data_cmd(a: ImportDataArgs) -> ExitCode {
    let conn = if a.what == "book" {
        match isnady_core::sources::store::connect_rw(None) {
            Ok(c) => Some(c),
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::from(1);
            }
        }
    } else {
        None
    };
    let request = transfer::Import {
        kind: a.what.clone(),
        fmt: a.format.clone().unwrap_or_default(),
        replace: a.replace,
        folder: a.folder.clone().unwrap_or_default(),
        name: a.name.clone().unwrap_or_default(),
    };
    let mut failed = 0;
    for file in &a.file {
        match transfer::import_file(conn.as_ref(), file, &request, &|m| eprintln!("  {m}")) {
            Ok(said) => println!("{said}"),
            Err(e) => {
                eprintln!("{file}: {e}");
                failed += 1;
            }
        }
    }
    ExitCode::from(if failed > 0 { 1 } else { 0 })
}
