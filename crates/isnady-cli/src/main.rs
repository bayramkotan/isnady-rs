//! iy-rs — isnady's command line in Rust. Every command prints what the Python `iy` prints for the same command, so
//! that tools/parity.py can compare the two word for word. A command or option not ported yet says so and points to
//! the Python `iy`; PARITY.md lists them.

use clap::{Parser, Subcommand};
use isnady_core::{db, isnad, narrators, paths, py, search, VERSION};
use std::collections::HashMap;
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
    /// Show the chain of one hadith
    Chain(ChainArgs),
}

#[derive(clap::Args)]
struct ChainArgs {
    /// Collection key, e.g. bukhari
    book: String,
    /// Hadith number, e.g. 1; none = the book's numbers, first to last
    number: Option<String>,
    /// Also print the chain as written
    #[arg(long)]
    raw: bool,
    /// (not ported yet: the hadith of one chapter — use iy chain BOOK --chapter CH)
    #[arg(long, value_name = "CH")]
    chapter: Option<String>,
    /// The chain in words in these languages, e.g. tr,en (default: the languages the texts are shown in)
    #[arg(long)]
    lang: Option<String>,
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
            println!("isnady-rs {VERSION} — iy-rs search WORDS, iy-rs chain BOOK NUMBER, iy-rs sources, iy-rs datadir, iy-rs --help");
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
        Some(Command::Chain(a)) => chain_cmd(a),
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
    pub type Result<T> = std::result::Result<T, rusqlite_error::Error>;
    pub mod rusqlite_error {
        pub use isnady_core::db::Error;
    }
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

const TAHWIL_NOTE: &str = " (roads joined by tahwil, a second road after a link — \"وعن\" / \"وحدثنا\" — or two narrators at one link)";

fn chain_cmd(a: ChainArgs) -> ExitCode {
    if a.chapter.is_some() {
        return not_yet(
            "The hadith of one chapter (--chapter)",
            "iy chain BOOK --chapter CH",
        );
    }
    let conn = match open() {
        Ok(c) => c,
        Err(code) => return code,
    };
    match chain_shown(&conn, &a) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(1)
        }
    }
}

fn chain_shown(conn: &db::Connection, a: &ChainArgs) -> rusqlite_conn::Result<ExitCode> {
    let number = a.number.clone().unwrap_or_default();
    if number.is_empty() {
        let Some(rng) = isnad::number_range(conn, &a.book)? else {
            eprintln!("No book '{}'. See: iy book", a.book);
            return Ok(ExitCode::from(1));
        };
        println!(
            "{}: hadith {} – {} ({} hadith). Example: iy chain {} {}",
            a.book,
            rng.first,
            rng.last,
            py::thousands(rng.count),
            a.book,
            isnad::random_number(conn, &a.book).unwrap_or_else(|| "None".into())
        );
        return Ok(ExitCode::SUCCESS);
    }
    if !isnad::isnads_ready(conn) || !narrators::links_ready(conn) {
        eprintln!(
            "The chains of the books imported last are not read yet. The Python isnady reads them once, for every \
             book: run iy chain {} {} (or open the Isnad Chains page), then iy-rs shows them.",
            a.book, number
        );
        return Ok(ExitCode::from(1));
    }
    let Some((hid, name)) = isnad::find_hadith(conn, &a.book, &number)? else {
        match isnad::number_range(conn, &a.book)? {
            Some(rng) => eprintln!(
                "No hadith {number} in '{}'. Its numbers run from {} to {}; the nearest is {}.",
                a.book,
                rng.first,
                rng.last,
                isnad::nearest_number(conn, &a.book, &number).unwrap_or_else(|| "None".into())
            ),
            None => eprintln!("No hadith {number} in '{}'. See: iy book", a.book),
        }
        return Ok(ExitCode::from(1));
    };
    let chains = isnad::chain(conn, hid)?;
    let loaded = narrators::loaded(conn);
    let borrowed = isnady_core::tables::get().isnad.borrowed.clone();
    let mut people: HashMap<i64, narrators::Person> = HashMap::new();
    println!("{name} {number}");
    if chains.is_empty() {
        println!("  no chain stored (no Arabic text for this hadith)");
    }
    let n = chains.len();
    for (k, ch) in chains.iter().enumerate() {
        if n > 1 {
            println!(
                "  Chain {} of {n}{}",
                k + 1,
                if k == 0 { TAHWIL_NOTE } else { "" }
            );
        }
        match ch.problem.as_deref() {
            Some(p) if p == borrowed => println!(
                "  taken from the hadith before it (the book says \"وعنه\" / \"وبهذا الإسناد\"); not counted in the statistics"
            ),
            Some(p) if !p.is_empty() => println!("  not split: {p}"),
            _ => {}
        }
        for (i, link) in ch.links.iter().enumerate() {
            if let Some(met) = &ch.meeting {
                if link.position == met.first_position {
                    if met.said {
                        println!(
                            "      ⤷ tahwil: the roads meet here — the text says so (كلهم عن …); attested in {} other hadith",
                            met.attested
                        );
                    } else {
                        println!(
                            "      ⤷ tahwil: the roads meet here — not said in the text: the narrator before is found narrating from this one in {} other hadith, in chains read whole",
                            met.attested
                        );
                    }
                }
            }
            let (arabic, meaning) = isnad::term_label(&link.transmission);
            let meaning = if meaning.starts_with('(') {
                meaning
            } else {
                format!("({meaning})")
            };
            println!(
                "  {:>2}. {arabic} {meaning}  {}",
                link.position, link.raw_name
            );
            let nxt = ch
                .links
                .get(i + 1)
                .map(|l| l.raw_name.as_str())
                .unwrap_or("");
            let note = isnad::link_note(&link.raw_name, nxt);
            if !note.is_empty() && link.person_id.is_none() {
                println!("      = {note}");
            } else if let Some(pid) = link.person_id {
                if let std::collections::hash_map::Entry::Vacant(e) = people.entry(pid) {
                    if let Some(p) = narrators::describe(conn, pid) {
                        e.insert(p);
                    }
                }
                let Some(who) = people.get(&pid) else {
                    continue;
                };
                let mut facts: Vec<String> = Vec::new();
                if let Some(t) = who.tabaqa.filter(|t| *t != 0) {
                    facts.push(format!("tabaqa {t}"));
                }
                if let Some(d) = who.death_year_ah.filter(|d| *d != 0) {
                    facts.push(format!("d. {d} AH"));
                }
                if let Some(v) = who.verdicts.first() {
                    let phrase = if py::len(&v.phrase) > 160 {
                        py::take(&v.phrase, 160) + " …"
                    } else {
                        v.phrase.clone()
                    };
                    let rank = match v.rank.filter(|r| *r != 0) {
                        Some(_) if v.rank_scheme.as_deref() == Some("shia") => {
                            format!(" ({})", v.rank_label)
                        }
                        Some(r) => format!(" (rank {r})"),
                        None => String::new(),
                    };
                    facts.push(format!("{}: {phrase}{rank}", v.critic_name));
                }
                let how = match link.match_confidence {
                    None => "",
                    Some(c) if c >= 0.8 => "",
                    Some(c) if c >= 0.6 => "  [by the narrators around him]",
                    Some(_) => "  [the famous one of this name]",
                };
                println!("      = {}{how}", who.display_name);
                println!("        {}", facts.join("; "));
            } else if let (Some(c), true) = (link.candidates, loaded) {
                if c != 0 {
                    println!(
                        "      = not identified ({c} possible narrator{})",
                        if c != 1 { "s" } else { "" }
                    );
                } else {
                    println!("      = not identified (no narrator of that name found)");
                }
            }
        }
        if !ch.links.is_empty() {
            let end = isnad::end_label(ch);
            if end.is_empty() {
                println!("      (chain stops here)");
            } else {
                println!("      -> {end}");
            }
        }
        if a.raw {
            if let Some(raw) = ch.raw.as_deref().filter(|r| !r.is_empty()) {
                println!("  raw: {raw}");
            }
        }
        let codes: Vec<String> = match &a.lang {
            Some(l) if !l.is_empty() => l
                .split(',')
                .map(str::trim)
                .filter(|c| !c.is_empty())
                .map(String::from)
                .collect(),
            _ => {
                let shown: Vec<String> = isnady_core::config::get_str("view.content_languages", "")
                    .split(',')
                    .filter(|p| !p.is_empty())
                    .map(String::from)
                    .collect();
                isnad::chain_codes(&shown)
            }
        };
        for l in &ch.links {
            if let Some(pid) = l.person_id {
                if let std::collections::hash_map::Entry::Vacant(e) = people.entry(pid) {
                    if let Some(p) = narrators::describe(conn, pid) {
                        e.insert(p);
                    }
                }
            }
        }
        for code in codes {
            let words = isnad::chain_in_words(ch, &code, &people);
            if !words.is_empty() {
                println!("  [{code}] {words}");
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}
