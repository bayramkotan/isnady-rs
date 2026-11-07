//! iy-rs — isnady's command line in Rust. Every command prints what the Python `iy` prints for the same command, so
//! that tools/parity.py can compare the two word for word. A command or option not ported yet says so and points to
//! the Python `iy`; PARITY.md lists them.

use clap::{Parser, Subcommand};
use isnady_core::{
    db, grades, isnad, narrators, paths, people, py, readmarks, search, works, VERSION,
};
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
    /// Read a book in its own order: the list, a book's chapters, or a chapter
    Book(BookArgs),
    /// Look up a narrator: verdict, rank, tabaqa, death, books
    Narrator(NarratorArgs),
}

#[derive(clap::Args)]
struct NarratorArgs {
    /// Part of the name, e.g. الزهري, 'سفيان بن عيينة', 'Ebu Bekir', 'Omer ibn Hattap'
    words: Vec<String>,
    #[arg(long, default_value_t = 5)]
    limit: usize,
    /// List the famous Companions by group (and road B's Ashab al-Ijma')
    #[arg(long)]
    famous: bool,
    /// Only this road's famous ones (with --famous: Sunni unless asked) or matches
    #[arg(long, value_parser = ["sunni", "shia", "all"])]
    tradition: Option<String>,
}

#[derive(clap::Args)]
struct BookArgs {
    /// e.g. bukhari, abudawud, taqrib; none = the list
    key: Option<String>,
    /// Chapter number in reading order (see: iy-rs book KEY)
    chapter: Option<i64>,
    /// Show only this language (repeatable)
    #[arg(long = "language")]
    language: Vec<String>,
    #[arg(long, default_value_t = 20)]
    limit: i64,
    #[arg(long, default_value_t = 160)]
    width: usize,
    /// The list: the Sunni books (default), the Shia ones, or all
    #[arg(long, default_value = "sunni", value_parser = ["sunni", "shia", "all"])]
    tradition: String,
    /// Go on where you left this book (its chapter and hadith)
    #[arg(long)]
    resume: bool,
    /// Put a marker of this name at the chapter shown
    #[arg(long, value_name = "NAME")]
    mark: Option<String>,
    /// Your markers and where you left each book
    #[arg(long)]
    marks: bool,
    /// Open a marker (by its name or the start of it)
    #[arg(long, value_name = "NAME")]
    go: Option<String>,
    /// Remove a marker (all: every marker)
    #[arg(long, value_name = "NAME")]
    unmark: Option<String>,
    /// Forget where you left a book (all: every book); markers are kept
    #[arg(long, value_name = "KEY")]
    forget: Option<String>,
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
    /// The hadith of one chapter (its number as the book gives it, e.g. 3 or 1:2, or the start of its title)
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
    /// The last 20 searches, the newest first (the same list as the app)
    #[arg(long)]
    recent: bool,
    /// Forget the recent searches
    #[arg(long)]
    clear_recent: bool,
    /// What to search: hadith text (default), narrators of both traditions by name, or scholars
    #[arg(long = "in", default_value = "hadith", value_parser = ["hadith", "narrators", "scholars"])]
    kind: String,
    /// With --in narrators: only the Sunni (Taqrib) or the Shia (al-Najashi) narrators
    #[arg(long, value_parser = ["sunni", "shia"])]
    tradition: Option<String>,
    /// Only hadith of this grade (repeatable: --grade sahih --grade hasan). Sunni books: sahih, hasan, daif,
    /// very-weak, fabricated, ungraded. Shia books, in their own scale: imami-sahih, imami-hasan, muwaththaq,
    /// imami-daif, imami-ungraded. A hadith passes when one of its graders gave a grade chosen
    #[arg(long = "grade", value_name = "GRADE", value_parser = clap::builder::PossibleValuesParser::new(grades::filter_codes()))]
    grade: Vec<String>,
    /// With --grade: only when every grade the hadith was given is among those chosen
    #[arg(long)]
    every_grader: bool,
    /// With --grade sahih: leave out the hadith of al-Bukhari and Muslim, which no grader grades (by default they
    /// count as sahih: by their compiler's condition and the community's acceptance of the two books)
    #[arg(long)]
    no_sahihayn: bool,
    /// Only in this chapter of a book and the chapters under it, by its number or the start of its title:
    /// --chapter bukhari:3, --chapter "bukhari:Knowledge" (repeatable)
    #[arg(long = "chapter", value_name = "BOOK:CHAPTER")]
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
            println!("isnady-rs {VERSION} — iy-rs search WORDS, iy-rs chain BOOK NUMBER, iy-rs book, iy-rs narrator NAME, iy-rs sources, iy-rs datadir, iy-rs --help");
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
        Some(Command::Book(a)) => book_cmd(a),
        Some(Command::Narrator(a)) => narrator_cmd(a),
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

/// --chapter BOOK:CHAPTER → (book, chapter), or None after saying which was not found (`cli._chapters`).
fn chapters_given(conn: &db::Connection, given: &[String]) -> Option<Vec<(String, works::Node)>> {
    let mut out = vec![];
    for item in given {
        let (book, label) = item.split_once(':').unwrap_or((item.as_str(), ""));
        let found = if label.trim().is_empty() {
            None
        } else {
            works::chapter_named(conn, book.trim(), label.trim(), "en")
                .ok()
                .flatten()
        };
        match found {
            Some(node) => out.push((book.trim().to_string(), node)),
            None => {
                eprintln!("No chapter '{label}' in '{book}'. See: iy book {book}");
                return None;
            }
        }
    }
    Some(out)
}

fn search_cmd(a: SearchArgs) -> ExitCode {
    if a.clear_recent {
        println!("Forgot {} recent searches.", search::clear_recent());
        return ExitCode::SUCCESS;
    }
    if a.recent || a.words.is_empty() {
        let rows = search::recent_items();
        if rows.is_empty() {
            if a.recent {
                println!("No recent searches.");
                return ExitCode::SUCCESS;
            }
            println!("Give the words to search: iy search niyet");
            return ExitCode::from(2);
        }
        for (i, (text, kind)) in rows.iter().enumerate() {
            let in_kind = if kind == "hadith" {
                String::new()
            } else {
                format!("   (in {kind})")
            };
            println!("{:>3}. {text}{in_kind}", i + 1);
        }
        return ExitCode::SUCCESS;
    }
    let text = a.words.join(" ");
    search::add_recent(&text, &a.kind);
    if a.kind != "hadith" {
        return not_yet(
            &format!("Searching the {}", a.kind),
            &format!("iy search --in {} ...", a.kind),
        );
    }
    if a.mode == "meaning" {
        return not_yet("Search by meaning", "iy search --mode meaning ...");
    }
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
    q.grades = a.grade.clone();
    q.every_grader = a.every_grader;
    q.sahihayn = !a.no_sahihayn;
    let Some(chapters) = chapters_given(&conn, &a.chapter) else {
        return ExitCode::from(1);
    };
    q.chapters = chapters.iter().map(|(_, n)| n.id).collect();
    if !chapters.is_empty() && q.collections.is_empty() {
        let mut keys: Vec<String> = chapters.iter().map(|(b, _)| b.clone()).collect();
        keys.sort();
        keys.dedup();
        q.collections = keys;
    }
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
    for (book, c) in &chapters {
        let label = c
            .label
            .as_deref()
            .filter(|l| !l.is_empty())
            .map(|l| format!("{l}. "))
            .unwrap_or_default();
        println!("in: {book} › {label}{}", c.title);
    }
    if !q.grades.is_empty() {
        println!(
            "grades: {} — {} found by the words in all",
            grades::describe_choice(&q.grades, q.every_grader, &|s| s.to_string()),
            page.before_grades
        );
    }
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
        } else if !q.grades.is_empty()
            && q.sahihayn
            && ["bukhari", "muslim"].contains(&r.collection_key.as_str())
        {
            println!(
                "  sahih: in the Sahihayn — by the compiler's condition and the community's acceptance, not a \
                 grader's grade"
            );
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
    let conn = match open() {
        Ok(c) => c,
        Err(code) => return code,
    };
    if let Some(ch) = a.chapter.clone().filter(|c| !c.is_empty()) {
        return match chain_chapter(&conn, &a.book, &ch) {
            Ok(code) => code,
            Err(e) => {
                eprintln!("{e}");
                ExitCode::from(1)
            }
        };
    }
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

/// iy chain BOOK --chapter CH: the hadith of one chapter with the start of their text and how far each chain was read.
fn chain_chapter(
    conn: &db::Connection,
    book: &str,
    label: &str,
) -> rusqlite_conn::Result<ExitCode> {
    let Some(node) = works::chapter_named(conn, book, label, "en")? else {
        eprintln!("No chapter '{label}' in '{book}'. See: iy book {book}");
        return Ok(ExitCode::from(1));
    };
    let wanted: Vec<String> = isnady_core::config::get_str("view.content_languages", "")
        .split(',')
        .filter(|p| !p.is_empty() && *p != "Arabic")
        .map(String::from)
        .collect();
    let rows = works::chapter_hadith(conn, node.id, wanted.first().map(String::as_str), 70)?;
    let head = format!(
        "{}. {}",
        node.label.clone().unwrap_or_else(|| "None".into()),
        node.title
    );
    if rows.is_empty() {
        println!(
            "{head}: no hadith directly under it — choose one of its chapters (iy book {book})"
        );
        return Ok(ExitCode::SUCCESS);
    }
    println!("{book} — {head}: {} hadith", rows.len());
    for r in &rows {
        let read = if r.links > 0 {
            format!("{:2} narrators", r.links)
        } else {
            "  not read  ".to_string()
        };
        println!("  {:>8}  {read}  {}", r.number, r.words);
    }
    println!("\n(iy chain {book} NUMBER shows one of them)");
    Ok(ExitCode::SUCCESS)
}

fn book_cmd(mut a: BookArgs) -> ExitCode {
    let conn = match open() {
        Ok(c) => c,
        Err(code) => return code,
    };
    if !works::ready(&conn) {
        eprintln!(
            "The books imported last are not laid out in chapters yet. The Python isnady does it once: run iy book \
             (or open the Books page), then iy-rs reads them."
        );
        return ExitCode::from(1);
    }
    match book_shown(&conn, &mut a) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(1)
        }
    }
}

fn book_shown(conn: &db::Connection, a: &mut BookArgs) -> rusqlite_conn::Result<ExitCode> {
    let titles: std::collections::HashMap<String, String> = works::list_works(conn, "en")?
        .into_iter()
        .map(|w| (w.key, w.title))
        .collect();
    let mut resume_mark: Option<readmarks::Place> = None;
    // the reader's markers: list them, put one, remove one
    if a.marks {
        let mine = readmarks::marks();
        println!(
            "{}",
            if mine.is_empty() {
                "No markers yet (iy book KEY CHAPTER --mark NAME)."
            } else {
                "Your markers:"
            }
        );
        for m in &mine {
            println!(
                "  {:30} {}",
                m.name,
                readmarks::describe(
                    m,
                    titles.get(&m.book).map(String::as_str).unwrap_or(""),
                    "en"
                )
            );
        }
        let left: Vec<readmarks::Place> = readmarks::recent(20)
            .into_iter()
            .filter(|p| titles.contains_key(&p.book))
            .collect();
        if !left.is_empty() {
            println!("\nWhere you left each book (iy book KEY --resume):");
            for p in &left {
                println!(
                    "  {:12} {}",
                    p.book,
                    readmarks::describe(p, &titles[&p.book], "en")
                );
            }
        }
        return Ok(ExitCode::SUCCESS);
    }
    if let Some(f) = a.forget.clone() {
        let n = readmarks::forget(if f == "all" { None } else { Some(&f) });
        if n > 0 {
            println!("Forgot where you left {n} book(s).");
        } else {
            println!("No place kept for '{f}'.");
        }
        return Ok(ExitCode::SUCCESS);
    }
    if a.unmark.as_deref() == Some("all") {
        let mine = readmarks::marks();
        for m in &mine {
            readmarks::remove(m.id.as_deref().unwrap_or(""));
        }
        println!("Removed {} marker(s).", mine.len());
        return Ok(ExitCode::SUCCESS);
    }
    if let Some(u) = a.unmark.clone() {
        let Some(m) = readmarks::find(&u) else {
            eprintln!("No marker '{u}'. See: iy book --marks");
            return Ok(ExitCode::from(1));
        };
        readmarks::remove(m.id.as_deref().unwrap_or(""));
        println!("Removed the marker: {}", m.name);
        return Ok(ExitCode::SUCCESS);
    }
    if let Some(g) = a.go.clone() {
        let Some(m) = readmarks::find(&g) else {
            eprintln!("No marker '{g}'. See: iy book --marks");
            return Ok(ExitCode::from(1));
        };
        a.key = Some(m.book.clone());
        resume_mark = Some(m);
    }
    let Some(key) = a.key.clone().filter(|k| !k.is_empty()) else {
        let road = if a.tradition == "all" {
            None
        } else {
            Some(a.tradition.as_str())
        };
        for w in works::list_works(conn, "en")? {
            if let Some(r) = road {
                if w.tradition != r && w.kind != "sira" && w.kind != "tabaqat" {
                    continue;
                }
            }
            println!(
                "  {:12} {:36} {:8} {:>6} items, {} chapters",
                works::short_name(&w.key),
                w.title,
                w.kind,
                py::thousands(w.leaves),
                w.chapters
            );
        }
        return Ok(ExitCode::SUCCESS);
    };
    let Some(work) = works::find_work(conn, &works::resolve(&key), "en")? else {
        eprintln!("No book '{key}'. See: iy book");
        return Ok(ExitCode::from(1));
    };
    let order = works::reading_order(conn, work.id)?;
    let mut focus: Option<i64> = None;
    let mut chapter = a.chapter;
    if a.resume || resume_mark.is_some() {
        let kept = resume_mark.clone().or_else(|| readmarks::last(&work.key));
        let Some(kept) = kept else {
            println!(
                "Nothing kept for {} yet: read a chapter (iy book {} N) and it is remembered.",
                work.title, work.key
            );
            return Ok(ExitCode::SUCCESS);
        };
        let (at, f) = readmarks::resolve(conn, &kept);
        focus = f;
        if let Some(at) = at {
            if let Some(i) = order.iter().position(|c| *c == at) {
                chapter = Some(i as i64 + 1);
                println!(
                    "Continuing at: {}",
                    readmarks::describe(&kept, &work.title, "en")
                );
            }
        }
    }
    let Some(chapter) = chapter else {
        for (i, cid) in order.iter().enumerate() {
            let trail: Vec<String> = works::path(conn, *cid, "en")
                .into_iter()
                .map(|p| p.title)
                .collect();
            println!(
                "  {:4}  {}  ({})",
                i + 1,
                trail.join(" › "),
                works::leaves(conn, *cid, 0, 0)?.1
            );
        }
        return Ok(ExitCode::SUCCESS);
    };
    if !(1..=order.len() as i64).contains(&chapter) {
        eprintln!("Chapter {chapter} does not exist: 1 to {}", order.len());
        return Ok(ExitCode::from(1));
    }
    let cid = order[(chapter - 1) as usize];
    let (items, total) = if let Some(f) = focus {
        let (every, _n) = works::leaves(conn, cid, 0, 1_000_000)?;
        let start = every
            .iter()
            .position(|it| it.hadith_id == Some(f))
            .unwrap_or(0);
        let total = every.len() as i64;
        let items: Vec<works::Leaf> = every
            .into_iter()
            .skip(start)
            .take(a.limit.max(0) as usize)
            .collect();
        (items, total)
    } else {
        works::leaves(conn, cid, 0, a.limit)?
    };
    let first_hadith = focus.or_else(|| items.iter().find_map(|it| it.hadith_id));
    let place = readmarks::place_of(conn, &work.key, Some(cid), first_hadith);
    readmarks::remember(
        &place.book,
        &place.chapter,
        &place.number,
        &place.title,
        place.place,
    );
    if let Some(name) = a.mark.clone() {
        let m = readmarks::add(
            &name,
            &place.book,
            &place.chapter,
            &place.number,
            &place.title,
            place.place,
        );
        println!(
            "Marker put: {} — {}",
            m.name,
            readmarks::describe(&m, &work.title, "en")
        );
    }
    let mut trail = vec![work.title.clone()];
    trail.extend(works::path(conn, cid, "en").into_iter().map(|p| p.title));
    println!("{}   ({total} items)", trail.join(" › "));
    let mut q = search::SearchQuery::new("");
    q.languages = a.language.clone();
    let cut = |body: &str| -> String {
        if py::len(body) > a.width {
            py::take(body, a.width) + " ..."
        } else {
            body.to_string()
        }
    };
    for it in &items {
        if it.kind == "hadith" {
            let Some(hid) = it.hadith_id else { continue };
            let r = search::load_result(conn, hid, &[], &q)?;
            println!("\n#{}", r.number);
            for t in &r.texts {
                let body = py::words(&t.text).join(" ");
                println!("  [{}] {}", t.language, cut(&body));
            }
        } else if work.kind == "sira" || work.kind == "tabaqat" {
            let label = it
                .label
                .clone()
                .filter(|l| !l.is_empty())
                .map(|l| format!("#{l}"))
                .unwrap_or_default();
            let title = it.title.clone().unwrap_or_default();
            let head = [label, title]
                .into_iter()
                .filter(|x| !x.is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            if head.is_empty() {
                println!("\n──");
            } else {
                println!("\n── {head}");
            }
            for para in it.text.clone().unwrap_or_default().split("\n\n") {
                println!("{}", py::fill(&py::words(para).join(" "), a.width, "  "));
            }
        } else {
            let source = it
                .text
                .clone()
                .filter(|t| !t.is_empty())
                .or_else(|| it.title.clone())
                .unwrap_or_default();
            let body = py::words(&source).join(" ");
            let head = it
                .label
                .clone()
                .filter(|l| !l.is_empty())
                .map(|l| format!("#{l}"))
                .unwrap_or_else(|| "→".into());
            println!("\n{head} {}", cut(&body));
        }
    }
    if total > items.len() as i64 && focus.is_none() {
        println!("\n… {} more (--limit)", total - items.len() as i64);
    }
    Ok(ExitCode::SUCCESS)
}

fn narrator_cmd(a: NarratorArgs) -> ExitCode {
    let conn = match open() {
        Ok(c) => c,
        Err(code) => return code,
    };
    match narrator_shown(&conn, &a) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(1)
        }
    }
}

/// iy narrator --famous: the famous Companions group by group, and road B's own famous narrators.
fn famous_list(conn: &db::Connection, tradition: Option<&str>) -> rusqlite_conn::Result<ExitCode> {
    for g in people::famous_groups(tradition) {
        let ids = people::famous_ids(conn, &g.keys);
        if ids.is_empty() {
            continue;
        }
        println!(
            "\n{}{}",
            g.title,
            if g.tradition == "shia" {
                "  (Shia, al-Najashi)"
            } else {
                ""
            }
        );
        for key in &g.keys {
            let Some(pid) = people::famous_ids(conn, std::slice::from_ref(key))
                .first()
                .copied()
            else {
                continue;
            };
            let (tab, death, n): (Option<i64>, Option<i64>, i64) = conn.query_row(
                "SELECT p.tabaqa, p.death_year_ah, (SELECT COUNT(*) FROM isnad_links l WHERE l.person_id = p.id) \
                 FROM persons p WHERE p.id = ?",
                [pid],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )?;
            let mut facts: Vec<String> = Vec::new();
            if let Some(t) = tab.filter(|t| *t != 0) {
                facts.push(format!("tabaqa {t}"));
            }
            if let Some(d) = death.filter(|d| *d != 0) {
                facts.push(format!("d. {d} AH"));
            }
            facts.push(format!("{n} in chains"));
            println!(
                "  {:<46} {}  (id {pid}) · {}",
                people::famous_title(key, "en"),
                people::famous_arabic(key),
                facts.join(" · ")
            );
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn narrator_shown(conn: &db::Connection, a: &NarratorArgs) -> rusqlite_conn::Result<ExitCode> {
    let road = match a.tradition.as_deref() {
        Some("all") => None,
        other => other,
    };
    if a.famous {
        return famous_list(
            conn,
            if a.tradition.as_deref() == Some("all") {
                None
            } else {
                Some(road.unwrap_or("sunni"))
            },
        );
    }
    if a.words.is_empty() {
        eprintln!("Name a narrator, e.g. iy narrator 'Ebu Bekir', or list the famous ones: iy narrator --famous");
        return Ok(ExitCode::from(2));
    }
    let text = a.words.join(" ");
    let mut found = people::search_persons(conn, &text, a.limit)?;
    let first = people::famous_ids(conn, &people::famous_named(&text, road));
    if !first.is_empty() {
        let mut merged: Vec<(i64, String)> = first.iter().map(|p| (*p, String::new())).collect();
        merged.extend(found.into_iter().filter(|f| !first.contains(&f.0)));
        merged.truncate(a.limit.max(first.len()));
        found = merged;
    }
    let titles = people::famous_titles(conn, "en");
    if found.is_empty() {
        println!("No narrator found. Import a rijal work first, for example: iy import taqrib <file-or-url>");
        return Ok(ExitCode::from(1));
    }
    let n_found = found.len();
    for (pid, _name) in &found {
        let Some(who) = people::describe(conn, *pid) else {
            continue;
        };
        let shown = &who.person.display_name;
        match titles.get(pid) {
            Some(t) => println!("\n{t}  —  {shown}"),
            None => println!("\n{shown}"),
        }
        if !who.other_names.is_empty() {
            let names: Vec<&str> = who.other_names.iter().take(6).map(String::as_str).collect();
            println!("  also: {}", names.join(", "));
        }
        if let Some(t) = who.person.tabaqa.filter(|t| *t != 0) {
            println!("  tabaqa {t}: {}", who.tabaqa_label);
        }
        let death = who.person.death_year_ah.filter(|d| *d != 0);
        let note = who.death_year_note.clone().filter(|n| !n.is_empty());
        if death.is_some() || note.is_some() {
            println!(
                "  death: {} AH{}",
                death.map(|d| d.to_string()).unwrap_or_else(|| "?".into()),
                note.map(|n| format!("  ({n})")).unwrap_or_default()
            );
        }
        for v in &who.verdicts {
            let rank = if v.rank_scheme.as_deref() == Some("shia") {
                if v.rank.is_some_and(|r| r != 0) {
                    format!(
                        "  {}{}",
                        v.rank_label,
                        if v.madhhab_label.is_empty() {
                            String::new()
                        } else {
                            format!(", {}", v.madhhab_label)
                        }
                    )
                } else {
                    "  no judgment in these words".to_string()
                }
            } else if let Some(r) = v.rank.filter(|r| *r != 0) {
                format!("  rank {r}: {}", v.rank_label)
            } else {
                String::new()
            };
            println!("  {}, {}: {}{rank}", v.critic_name, v.work, v.phrase);
        }
        if !who.marks.is_empty() {
            let m: Vec<String> = who
                .marks
                .iter()
                .map(|(m, meaning)| format!("{m} {meaning}"))
                .collect();
            println!("  books: {}", m.join(", "));
        }
        println!(
            "  identified in {} chain link{} of the imported collections",
            who.in_chains,
            if who.in_chains != 1 { "s" } else { "" }
        );
        if a.limit == 1 || n_found == 1 {
            let rel = people::relations(conn, *pid, 8)?;
            if !rel.teachers.is_empty() {
                let t: Vec<String> = rel
                    .teachers
                    .iter()
                    .map(|r| format!("{} ({})", r.name, r.count))
                    .collect();
                println!("  narrates from: {}", t.join("; "));
            }
            if !rel.students.is_empty() {
                let t: Vec<String> = rel
                    .students
                    .iter()
                    .map(|r| format!("{} ({})", r.name, r.count))
                    .collect();
                println!("  narrated to:   {}", t.join("; "));
            }
            if !rel.compilers.is_empty() {
                let c: Vec<String> = rel
                    .compilers
                    .iter()
                    .map(|(b, _k, n)| format!("{b} ({n})"))
                    .collect();
                println!(
                    "  compilers who narrate from him directly: {}",
                    c.join("; ")
                );
            }
            let (hadith, total) = people::hadiths_of(conn, *pid, 12, "en")?;
            if !hadith.is_empty() {
                let h: Vec<String> = hadith
                    .iter()
                    .map(|h| format!("{} {}", h.book, h.number))
                    .collect();
                println!(
                    "  in the chains of {total} hadith: {}{}",
                    h.join(", "),
                    if total > hadith.len() as i64 {
                        " …"
                    } else {
                        ""
                    }
                );
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}
