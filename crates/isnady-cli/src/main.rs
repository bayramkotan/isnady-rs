//! iy-rs — isnady's command line in Rust. Every command prints what the Python `iy` prints for the same command, so
//! that tools/parity.py can compare the two word for word. A command or option not ported yet says so and points to
//! the Python `iy`; PARITY.md lists them.

use clap::{Parser, Subcommand};
use isnady_core::{db, paths, search, VERSION};
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "iy-rs",
    about = "isnady in Rust — the same data, the same rules as the Python isnady (iy)",
    disable_version_flag = true
)]
struct Cli {
    /// Show the version
    #[arg(short = 'V', long = "version")]
    version: bool,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Show the version
    Version,
    /// Where the data is kept
    Datadir {
        /// (not ported yet: moving the data folder — use iy datadir PATH)
        path: Option<String>,
    },
    /// What is imported: every source with its editions and texts
    Sources,
    /// Search the hadith
    Search(SearchArgs),
}

#[derive(clap::Args)]
struct SearchArgs {
    /// Words to search; Arabic diacritics and letter forms are ignored
    words: Vec<String>,
    /// all, any or phrase
    #[arg(long, default_value = "all")]
    mode: String,
    /// Match whole words only
    #[arg(long)]
    whole_words: bool,
    /// Collection key, e.g. bukhari (repeatable)
    #[arg(long = "book")]
    book: Vec<String>,
    /// Language name, e.g. Turkish (repeatable)
    #[arg(long = "language")]
    language: Vec<String>,
    #[arg(long, default_value_t = 10)]
    limit: usize,
    #[arg(long, default_value_t = 0)]
    offset: usize,
    /// Characters of text to show
    #[arg(long, default_value_t = 300)]
    width: usize,
    /// Also show texts that did not match
    #[arg(long)]
    all_texts: bool,
    /// (not ported yet: the grades filter — use iy search --grade)
    #[arg(long = "grade")]
    grade: Vec<String>,
    /// (not ported yet: the chapters filter — use iy search --chapter)
    #[arg(long = "chapter")]
    chapter: Vec<String>,
}

fn not_yet(what: &str, python: &str) -> ExitCode {
    eprintln!("{what} is not in isnady-rs {VERSION} yet; the Python isnady does it on the same data: {python}");
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    if cli.version {
        return version();
    }
    match cli.command {
        None => {
            println!("isnady-rs {VERSION} — iy-rs search WORDS, iy-rs sources, iy-rs datadir, iy-rs --help");
            ExitCode::SUCCESS
        }
        Some(Command::Version) => version(),
        Some(Command::Datadir { path }) => {
            if path.is_some() {
                return not_yet("Moving the data folder", "iy datadir PATH");
            }
            println!(
                "{}  ({})",
                paths::shown(&paths::data_dir()),
                paths::data_dir_source()
            );
            println!("default: {}", paths::shown(&paths::default_data_dir()));
            ExitCode::SUCCESS
        }
        Some(Command::Sources) => sources(),
        Some(Command::Search(a)) => search_cmd(a),
    }
}

fn version() -> ExitCode {
    println!("isnady {VERSION} (Rust)");
    ExitCode::SUCCESS
}

fn open() -> Result<rusqlite_conn::Connection, ExitCode> {
    db::connect(None).map_err(|e| {
        eprintln!("{e}");
        ExitCode::from(1)
    })
}

mod rusqlite_conn {
    pub use isnady_core::db::Connection;
}

fn sources() -> ExitCode {
    let conn = match open() {
        Ok(c) => c,
        Err(code) => return code,
    };
    let rows = match db::sources(&conn) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(1);
        }
    };
    if rows.is_empty() {
        println!("No sources yet. Add one with: iy import <format> <file-or-url>");
        return ExitCode::SUCCESS;
    }
    for r in rows {
        let label = if r.origin == "user" {
            "User Resource"
        } else {
            "Built-in"
        };
        println!("[{label}] {}  —  {}", r.key, r.name);
        let note = match r.tier.as_str() {
            "A" => "redistributable",
            "B" => "redistributable with conditions",
            _ => "not redistributable",
        };
        println!(
            "    format {}, auth {}, license {}, tier {} ({})",
            r.format,
            r.auth_type,
            r.license
                .as_deref()
                .filter(|l| !l.is_empty())
                .unwrap_or("not stated"),
            r.tier,
            note
        );
        println!(
            "    {} edition(s), {} texts, imported {}",
            r.editions,
            r.texts,
            r.imported_at.as_deref().unwrap_or("None")
        );
        println!("    last imported from {}", r.location);
    }
    ExitCode::SUCCESS
}

fn search_cmd(a: SearchArgs) -> ExitCode {
    if !a.grade.is_empty() {
        return not_yet("The grades filter (--grade)", "iy search --grade sahih ...");
    }
    if !a.chapter.is_empty() {
        return not_yet(
            "The chapters filter (--chapter)",
            "iy search --chapter BOOK:CHAPTER ...",
        );
    }
    if a.mode == "meaning" {
        return not_yet("Search by meaning", "iy search --mode meaning ...");
    }
    if a.words.is_empty() {
        eprintln!("Give the words to search: iy-rs search niyet");
        return ExitCode::from(2);
    }
    let text = a.words.join(" ");
    search::add_recent(&text, "hadith");
    let conn = match open() {
        Ok(c) => c,
        Err(code) => return code,
    };
    let mut q = search::SearchQuery::new(&text);
    q.mode = a.mode.clone();
    q.whole_words = a.whole_words;
    q.collections = a.book.clone();
    q.languages = a.language.clone();
    q.limit = a.limit;
    q.offset = a.offset;
    let page = match search::search(&conn, &q) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(1);
        }
    };
    println!(
        "{} hadith found in {} ms{}",
        page.total,
        page.elapsed_ms,
        if page.used_index {
            ""
        } else {
            " (no FTS5 index available; slower scan)"
        }
    );
    if !page.spellings.is_empty() {
        let text: Vec<String> = page
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
        println!("near spellings: {}", text.join("; "));
    }
    for r in &page.results {
        println!("\n{} #{}", r.collection_name, r.number);
        if !r.grades.is_empty() {
            let g: Vec<String> = r.grades.iter().map(|(g, v)| format!("{g}: {v}")).collect();
            println!("  grades: {}", g.join("; "));
        }
        for t in &r.texts {
            if !t.matched && !a.all_texts {
                continue;
            }
            let chars: Vec<char> = t.text.chars().collect();
            let mut body = String::new();
            let mut last = 0;
            for &(s, e) in &t.spans {
                body.extend(&chars[last..s]);
                body.push('[');
                body.extend(&chars[s..e]);
                body.push(']');
                last = e;
            }
            body.extend(&chars[last..]);
            let n = body.chars().count();
            if n > a.width {
                body = body.chars().take(a.width).collect::<String>() + " ...";
            }
            println!("  [{}] {}", t.language, body);
        }
    }
    ExitCode::SUCCESS
}
