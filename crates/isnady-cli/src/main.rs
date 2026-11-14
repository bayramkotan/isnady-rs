//! iy-rs — isnady's command line in Rust. Every command prints what the Python `iy` prints for the same command, so
//! that tools/parity.py can compare the two word for word. A command or option not ported yet says so and points to
//! the Python `iy`; PARITY.md lists them.

mod ai_cmd;
mod doctor_cmd;
mod export_cmd;
mod sources_cmd;
mod stats_cmd;
use clap::{Parser, Subcommand};
use isnady_core::{
    bookmarks, db, genealogy, grades, huffaz, i18n, isnad, learn, lives, narrators, paths, people,
    py, quran, readmarks, sahaba, sahifa, scholars, search, sira, tables, works, VERSION,
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
    /// Show or change the data folder (database, settings, your sources)
    Datadir {
        /// New folder; without it the current one is shown
        path: Option<String>,
        /// Use the folder as it is instead of copying your data there
        #[arg(long = "as-is")]
        as_is: bool,
        /// Go back to the platform's default folder
        #[arg(long)]
        default: bool,
    },
    /// Appearance and other settings (the same as Edit → Preferences)
    Config(ConfigArgs),
    /// Profiles: several people on one isnady, each with their own settings, place and bookmarks (the same as File →
    /// Profiles)
    Profile(ProfileArgs),
    /// What is imported: every source with its editions and texts
    Sources,
    /// List supported data formats
    Formats,
    /// Built-in sources and your own: list, import, updates, update, remove, add, delete
    Catalog(sources_cmd::CatalogArgs),
    /// A ready database: download isnady-data's, or use one from a file on this computer (isnady.db or the isnady-db
    /// zip of isnady-data)
    Database(sources_cmd::DatabaseArgs),
    /// Data repositories: list, use, show, add, remove (isnady's own is the default)
    Repo(sources_cmd::RepoArgs),
    /// Import a file or URL
    Import(sources_cmd::ImportArgs),
    /// Remove a source and everything imported from it
    Remove {
        /// source key, see 'iy sources'
        key: String,
    },
    /// Count what the database holds; corpus, narrators, chains, grades, books, graders for the statistics in depth
    Stats(stats_cmd::StatsArgs),
    /// Search the hadith
    Search(SearchArgs),
    /// Show the chain of one hadith
    Chain(ChainArgs),
    /// The chains of transmission read from the Arabic texts: the report of every book
    Isnads {
        /// read every chain again
        #[arg(long)]
        rebuild: bool,
        /// (for comparing with the Python isnady) every road of every hadith, one line each, to this file
        #[arg(long, hide = true)]
        dump: Option<String>,
        /// (with --dump) only the first so many hadith
        #[arg(long, hide = true)]
        limit: Option<usize>,
    },
    /// isnady's own inference about who a narrator is: off (the rijal works' words alone) unless turned on; every
    /// narrator it finds is marked as its guess
    Guess {
        /// turn it on or off (nothing: show which)
        #[arg(value_parser = ["on", "off"])]
        state: Option<String>,
    },
    /// Read a book in its own order: the list, a book's chapters, or a chapter
    Book(BookArgs),
    /// Look up a narrator: verdict, rank, tabaqa, death, books
    Narrator(NarratorArgs),
    /// The life of a Companion, a narrator or a scholar, in English, Turkish or Arabic
    Life(LifeArgs),
    /// The hadith scholars in the data: compilers, graders, critics, with measured statistics
    Scholar(ScholarArgs),
    /// The ten promised Paradise, the people of the cloak, the Mothers of the Believers; the Fourteen Infallibles on the
    /// Shia road
    Group(GroupArgs),
    /// The life of the Prophet as a timeline: events, dates, verses, Ibn Hisham's chapter and the hadith that name each
    Sira(SiraArgs),
    /// The cache that keeps the counts of books, narrators, scholars …: info, clear
    Cache {
        #[arg(default_value = "info", value_parser = ["info", "clear"])]
        action: String,
    },
    /// isnady's log: the last lines, errors only, a text, follow it, crash reports
    Log(LogArgs),
    /// Back to zero: delete all settings, all books, or everything
    Reset {
        #[arg(value_parser = ["settings", "books", "all"])]
        what: String,
        /// Do not ask
        #[arg(long)]
        yes: bool,
    },
    /// Put isnady on the desktop and in the applications menu, with its icon
    Shortcut {
        /// Only the applications menu
        #[arg(long = "no-desktop")]
        no_desktop: bool,
        /// Only the desktop
        #[arg(long = "no-menu")]
        no_menu: bool,
    },
    /// The Qur'an with its translations, and the hadith that name each verse (downloaded once on request)
    Quran(QuranArgs),
    /// The written sources: Hammam's sahifa with its parallels, the writings that live in later books, writing in the
    /// chains (Sunni and Shia apart)
    Sahifa(SahifaArgs),
    /// Genealogies: the prophets, Quraysh, a narrator's family, his line of learning
    Tree(TreeArgs),
    /// Bookmarks in folders: list, folders, add, remove, export, import
    Bookmark(BookmarkArgs),
    /// The glossary: a term of hadith and its sciences, or a search
    Term(TermArgs),
    /// History and method of hadith: how they were written down and gathered, the logic of verification, the numbers
    /// of the books imported (counted live)
    Learn(LearnArgs),
    /// Hadith, a book or chapter, search results, bookmarks, narrators, the isnad network, settings, a profile or a
    /// backup, in many formats (iy export --list)
    Export(export_cmd::ExportArgs),
    /// Bring back a book, bookmarks, settings, a profile or a backup from a file made by iy export or File → Export…
    #[command(name = "import-data")]
    ImportData(export_cmd::ImportDataArgs),
    /// Artificial-intelligence features: build or check the meaning index and takhrij
    Ai(ai_cmd::AiArgs),
    /// Other narrations of a hadith in the imported books (takhrij)
    Tahric(ai_cmd::TahricArgs),
    /// Every copy of isnady-rs, which one each command starts, and what is out of date
    Doctor(doctor_cmd::DoctorArgs),
    /// Update every copy of isnady-rs where it is installed (crates.io or its clone)
    Update(doctor_cmd::UpdateArgs),
}

#[derive(clap::Args)]
struct BookmarkArgs {
    #[arg(value_parser = ["list", "folders", "add", "remove", "export", "import"])]
    action: String,
    /// add: hadith | narrator | scholar | term | chapter | tree; list: only that kind; remove: the id;
    /// export/import: the file
    kind: Option<String>,
    /// add: bukhari:1, a narrator's key, a scholar's or term's id, bukhari:3, prophets:musa
    #[arg(name = "ref")]
    reference: Option<String>,
    /// A folder, Prayer or Prayer/Friday (made if missing)
    #[arg(long)]
    folder: Option<String>,
    /// The title to keep (default: isnady's)
    #[arg(long)]
    title: Option<String>,
    /// A note of your own
    #[arg(long)]
    note: Option<String>,
}

#[derive(clap::Args)]
struct TermArgs {
    /// e.g. sahih, mursal, tadlis, kunya
    name: Option<String>,
    /// Words to look for in names and definitions
    #[arg(long)]
    search: Option<String>,
    #[arg(long, default_value = "en", value_parser = ["en", "tr", "ar"])]
    lang: String,
}

#[derive(clap::Args)]
struct LearnArgs {
    /// tadwin, writing, method or numbers (none: the list)
    article: Option<String>,
    /// The language (default: the interface language)
    #[arg(long, value_parser = ["en", "tr", "ar"])]
    lang: Option<String>,
    /// Characters per line
    #[arg(long, default_value_t = 100)]
    width: usize,
}

#[derive(clap::Args)]
struct TreeArgs {
    #[arg(value_parser = ["prophets", "quraysh", "family", "learning"])]
    which: String,
    /// A narrator (family, learning) or someone to find in the tree
    name: Option<String>,
    /// Learning: steps of teachers and of students (default 2)
    #[arg(long, default_value_t = 2)]
    depth: usize,
    /// Learning: how many at each step (default 4)
    #[arg(long, default_value_t = 4)]
    width: usize,
    /// Prophets: the hadith that speak of the prophet found (e.g. iy tree prophets Musa --hadith)
    #[arg(long)]
    hadith: bool,
}

#[derive(clap::Args)]
struct SiraArgs {
    /// badr, uhud, hijra, revelation …; none = the timeline
    event: Option<String>,
    /// The language (default: the interface language)
    #[arg(long, value_parser = ["en", "tr", "ar"])]
    lang: Option<String>,
    /// Characters per line
    #[arg(long, default_value_t = 100)]
    width: usize,
}

#[derive(clap::Args)]
struct LogArgs {
    /// The last N lines (0: all; default 60)
    #[arg(long, short = 'n', default_value_t = 60)]
    lines: usize,
    /// Only lines of this level and above
    #[arg(long, value_parser = ["info", "warning", "error"])]
    level: Option<String>,
    /// Only lines holding this text
    #[arg(long)]
    grep: Option<String>,
    /// Keep showing new lines (Ctrl+C ends)
    #[arg(long, short = 'f')]
    follow: bool,
    /// The newest crash report instead
    #[arg(long)]
    crash: bool,
    /// List the log files
    #[arg(long)]
    files: bool,
    /// Print where the log is
    #[arg(long)]
    path: bool,
    /// Remove what was logged before this day
    #[arg(long, value_name = "YYYY-MM-DD")]
    before: Option<String>,
    /// Empty the log and remove the crash reports
    #[arg(long)]
    clear: bool,
}

#[derive(clap::Args)]
struct ConfigArgs {
    #[arg(value_parser = ["list", "get", "set", "reset"])]
    action: String,
    /// e.g. text.arabic.size, colors.dark.gilt, ui.family
    key: Option<String>,
    /// Value for set; an empty string restores the default
    value: Option<String>,
    /// List only keys starting with this, e.g. text.arabic
    #[arg(long, default_value = "")]
    prefix: String,
}

#[derive(clap::Args)]
struct ProfileArgs {
    #[arg(value_parser = ["list", "new", "use", "rename", "delete", "export", "import"])]
    action: String,
    /// The profile's name (import: the file)
    name: Option<String>,
    /// rename: the new name; export: the file or folder to write; import: a name for the new profile
    value: Option<String>,
    /// new: start from this profile's settings
    #[arg(long, value_name = "PROFILE")]
    copy: Option<String>,
}

#[derive(clap::Args)]
struct QuranArgs {
    /// none: the suras; 2 / 2:255 / 2:255-257: verses; search WORDS; download [EDITION …]; editions [--language L |
    /// --installed]; remove EDITION
    what: Vec<String>,
    /// The translations to show (keys from iy quran editions --installed); default: those downloaded
    #[arg(long)]
    edition: Vec<String>,
    /// Under each verse, the hadith that name it (Sunni and Shia books apart)
    #[arg(long)]
    hadith: bool,
    /// editions: only this language, e.g. Turkish
    #[arg(long)]
    language: Option<String>,
    /// editions: only those downloaded
    #[arg(long)]
    installed: bool,
    /// editions: only those isnady-data keeps (★)
    #[arg(long)]
    own: bool,
    /// The language of the sura names
    #[arg(long, value_parser = ["en", "tr", "ar"])]
    lang: Option<String>,
    /// search: how many verses to show
    #[arg(long, default_value_t = 30)]
    limit: usize,
    /// Characters per line
    #[arg(long, default_value_t = 100)]
    width: usize,
}

#[derive(clap::Args)]
struct SahifaArgs {
    /// hammam, sadiqa, ali, zakat, hazm, chains-sunni; kitab-ali, jamia, fatima, chains-shia; none = the list
    id: Option<String>,
    /// The list: one road, or both
    #[arg(long, default_value = "all", value_parser = ["sunni", "shia", "all"])]
    tradition: String,
    /// How many hadith to list
    #[arg(long, default_value_t = 40)]
    limit: usize,
    /// The language (default: the interface language)
    #[arg(long, value_parser = ["en", "tr", "ar"])]
    lang: Option<String>,
    /// Characters per line
    #[arg(long, default_value_t = 100)]
    width: usize,
}

#[derive(clap::Args)]
struct GroupArgs {
    /// ten, kisa, mothers, masumin; none = the list
    id: Option<String>,
    /// The list: one road, or both
    #[arg(long, default_value = "all", value_parser = ["sunni", "shia", "all"])]
    tradition: String,
    /// The language (default: the interface language)
    #[arg(long, value_parser = ["en", "tr", "ar"])]
    lang: Option<String>,
    /// Characters per line
    #[arg(long, default_value_t = 100)]
    width: usize,
}

#[derive(clap::Args)]
struct ScholarArgs {
    /// Part of a name, e.g. albani, bukhari; none = the list
    name: Vec<String>,
    /// The list: the Sunni scholars (default), the Shia ones, or all
    #[arg(long, default_value = "sunni", value_parser = ["sunni", "shia", "all"])]
    tradition: String,
    /// al-Dhahabi's Tadhkirat al-Huffaz: 1,176 huffaz in 21 generations, tied to the Taqrib; with a NAME, one hafiz
    #[arg(long)]
    huffaz: bool,
    /// With --huffaz: only generation N (1-21)
    #[arg(long, value_name = "N")]
    tabaqa: Option<i64>,
    /// (not ported yet: al-Dhahabi's entry in his own words — use iy scholar --huffaz NAME --source)
    #[arg(long)]
    source: bool,
}

#[derive(clap::Args)]
struct LifeArgs {
    /// Any part of the name, in Arabic or Latin letters
    #[arg(required = true)]
    name: Vec<String>,
    /// The language (default: the interface language)
    #[arg(long, value_parser = ["en", "tr", "ar"])]
    lang: Option<String>,
    /// Look only among the narrators of one tradition
    #[arg(long, value_parser = ["sunni", "shia"])]
    tradition: Option<String>,
    /// How many people to show
    #[arg(long, default_value_t = 1)]
    limit: usize,
    /// Also print his entry in the classical work the life is summarised from (al-Dhahabi's Siyar)
    #[arg(long)]
    source: bool,
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

fn main() -> ExitCode {
    let cli = Cli::parse();
    if cli.version {
        return version();
    }
    // no book yet (FR1): one command brings them all, as one click does in the window
    if !matches!(
        cli.command,
        Some(Command::Version)
            | Some(Command::Reset { .. })
            | Some(Command::Catalog(_))
            | Some(Command::Doctor(_))
            | Some(Command::Update(_))
            | Some(Command::Database(_))
            | None
    ) {
        let empty = db::connect(None)
            .ok()
            .map(|c| {
                c.query_row("SELECT 1 FROM hadiths LIMIT 1", [], |_| Ok(()))
                    .is_err()
            })
            .unwrap_or(true);
        if empty {
            eprintln!("isnady has no books yet. Import every hadith book and rijal work of isnady-data with one command: iy catalog import --all — or download the ready database: iy database download");
        }
        // an import cut short the last time (ROB1)
        if let Some((titles, started)) = isnady_core::sources::imports::interrupted() {
            eprintln!(
                "Note: an import did not finish ({}): {}. Import it again: iy catalog import ID (iy catalog list shows \
                 what is imported).",
                started.replace('T', " "),
                titles.iter().take(6).cloned().collect::<Vec<_>>().join(", ")
            );
        }
    }
    // every command in the log too (LOG1): what was asked, how it ended, how long it took — not `iy-rs log` itself
    let logged = !matches!(cli.command, Some(Command::Log(_)));
    let started = std::time::Instant::now();
    if logged {
        isnady_core::log::setup("command line");
        let args: Vec<String> = std::env::args().skip(1).collect();
        isnady_core::log::info("cli", &format!("▶ iy-rs {}", args.join(" ")));
    }
    let code = run(cli.command);
    if logged {
        let ok = code == ExitCode::SUCCESS;
        isnady_core::log::info(
            "cli",
            &format!(
                "{} exit {} in {:.1} s",
                if ok { "✔" } else { "✖" },
                if ok { "0" } else { "1" },
                started.elapsed().as_secs_f64()
            ),
        );
    }
    code
}

fn run(command: Option<Command>) -> ExitCode {
    match command {
        None => {
            println!("isnady-rs {VERSION} — iy-rs search WORDS, iy-rs chain BOOK NUMBER, iy-rs isnads, iy-rs book, iy-rs narrator NAME, iy-rs life NAME, iy-rs scholar, iy-rs group, iy-rs sira, iy-rs tree, iy-rs bookmark, iy-rs term, iy-rs learn, iy-rs sources, iy-rs datadir, iy-rs --help");
            ExitCode::SUCCESS
        }
        Some(Command::Version) => version(),
        Some(Command::Datadir {
            path,
            as_is,
            default,
        }) => datadir_cmd(path, as_is, default),
        Some(Command::Config(a)) => config_cmd(a),
        Some(Command::Cache { action }) => cache_cmd(&action),
        Some(Command::Log(a)) => log_cmd(a),
        Some(Command::Reset { what, yes }) => reset_cmd(&what, yes),
        Some(Command::Shortcut {
            no_desktop,
            no_menu,
        }) => shortcut_cmd(!no_desktop, !no_menu),
        Some(Command::Profile(a)) => profile_cmd(a),
        Some(Command::Sources) => sources(),
        Some(Command::Formats) => sources_cmd::formats_cmd(),
        Some(Command::Catalog(a)) => sources_cmd::catalog_cmd(a),
        Some(Command::Database(a)) => sources_cmd::database_cmd(a),
        Some(Command::Repo(a)) => sources_cmd::repo_cmd(a),
        Some(Command::Import(a)) => sources_cmd::import_cmd(a),
        Some(Command::Remove { key }) => sources_cmd::remove_cmd(&key),
        Some(Command::Stats(a)) => stats_cmd::stats_cmd(a),
        Some(Command::Search(a)) => search_cmd(a),
        Some(Command::Chain(a)) => chain_cmd(a),
        Some(Command::Isnads {
            rebuild,
            dump,
            limit,
        }) => isnads_cmd(rebuild, dump, limit),
        Some(Command::Guess { state }) => guess_cmd(state),
        Some(Command::Book(a)) => book_cmd(a),
        Some(Command::Narrator(a)) => narrator_cmd(a),
        Some(Command::Life(a)) => life_cmd(a),
        Some(Command::Scholar(a)) => scholar_cmd(a),
        Some(Command::Group(a)) => group_cmd(a),
        Some(Command::Sira(a)) => sira_cmd(a),
        Some(Command::Sahifa(a)) => sahifa_cmd(a),
        Some(Command::Quran(a)) => quran_cmd(a),
        Some(Command::Tree(a)) => tree_cmd(a),
        Some(Command::Bookmark(a)) => bookmark_cmd(a),
        Some(Command::Export(a)) => export_cmd::export_cmd(a),
        Some(Command::ImportData(a)) => export_cmd::import_data_cmd(a),
        Some(Command::Ai(a)) => ai_cmd::ai_cmd(a),
        Some(Command::Tahric(a)) => ai_cmd::tahric_cmd(a),
        Some(Command::Doctor(a)) => doctor_cmd::doctor_cmd(a),
        Some(Command::Update(a)) => doctor_cmd::update_cmd(a),
        Some(Command::Term(a)) => term_cmd(a),
        Some(Command::Learn(a)) => learn_cmd(a),
    }
}

/// iy-rs scholar --huffaz (HFZ1): al-Dhahabi's Tadhkirat al-Huffaz, generation by generation, tied to the Taqrib
/// (`cli._huffaz`).
fn huffaz_cmd(a: &ScholarArgs, conn: &rusqlite_conn::Connection) -> ExitCode {
    let lang = i18n::ui_language();
    let reading = |r: &huffaz::Hafiz| huffaz::reading(r, if lang == "tr" { "tr" } else { "en" });
    let died = |r: &huffaz::Hafiz| {
        let (y, whose) = huffaz::death_of(r);
        let Some(y) = y else { return String::new() };
        let note = if whose == "taqrib" {
            " (Taqrib)".to_string()
        } else {
            match r.taqrib_death {
                Some(o) if o != 0 && o != y => format!(" (Taqrib: {o})"),
                _ => String::new(),
            }
        };
        format!("d. {y}{note}")
    };
    let rows = huffaz::find(Some(conn), &a.name.join(" "), a.tabaqa);
    if a.name.is_empty() {
        println!("al-Dhahabi (d. 748 AH), Tadhkirat al-Huffaz: the masters of hadith, generation by generation");
        for g in huffaz::generations(Some(conn)) {
            if a.tabaqa.is_some_and(|t| t != g.tabaqa) {
                continue;
            }
            let stated = g
                .stated
                .map(|n| format!(", al-Dhahabi: {n}"))
                .unwrap_or_default();
            let span = match (g.median, g.from, g.to) {
                (Some(m), Some(f), Some(t)) if m != 0 => format!("d. {f}–{t} AH (median {m})"),
                _ => String::new(),
            };
            println!(
                "  generation {:2}: {:4} huffaz{stated}   {span}   in the Taqrib: {}",
                g.tabaqa, g.count, g.in_taqrib
            );
        }
        let centuries: Vec<String> = huffaz::by_century(Some(conn))
            .iter()
            .map(|(c, n)| format!("{c}: {n}"))
            .collect();
        println!("  died, by century AH: {}", centuries.join(", "));
        if a.tabaqa.is_none() {
            println!("\n(iy scholar --huffaz --tabaqa N lists one generation; iy scholar --huffaz NAME finds one man)");
            return ExitCode::SUCCESS;
        }
    }
    if rows.is_empty() {
        eprintln!("No hafiz of that name in the Tadhkira.");
        return ExitCode::from(1);
    }
    if !a.name.is_empty() && rows.len() == 1 {
        let r = &rows[0];
        println!("{}  {}", reading(r), r.entry.head);
        println!(
            "  Tadhkirat al-Huffaz no. {}, generation {}   {}",
            r.entry.n,
            r.entry.tabaqa,
            died(r)
        );
        if !r.entry.rumuz.is_empty() {
            let means: Vec<String> = huffaz::rumuz_text(&r.entry.rumuz)
                .iter()
                .map(|x| i18n::t(&lang, x, &[]))
                .collect();
            println!(
                "  narrates in: {}   [{}]",
                means.join("; "),
                r.entry.rumuz.join(" ")
            );
        }
        if let (Some(p), Some(name)) = (r.person, &r.taqrib_name) {
            println!(
                "  in the Taqrib: {} — in {} chain links here (iy narrator {p})",
                narrators::display_name(name),
                py::thousands(r.in_chains)
            );
        }
        if a.source {
            let work = huffaz::work();
            if !lives::available(work) {
                eprintln!("  … Downloading (≈{} MB, once)", lives::work_size(work));
                if let Err(e) = lives::fetch(work, &|m| eprintln!("  … {m}")) {
                    eprintln!("{e}");
                    return ExitCode::from(1);
                }
            }
            if let Some((heading, body)) = lives::entry(work, 0, r.entry.n) {
                println!("\n  {heading}\n");
                println!("{body}");
            }
            return ExitCode::SUCCESS;
        }
        println!("  (--source prints al-Dhahabi's entry in his own words)");
        return ExitCode::SUCCESS;
    }
    for r in &rows {
        let read: String = reading(r).chars().take(34).collect();
        println!(
            "  {:5}  g{:<2}  {:22} {:34}  {}",
            r.entry.n,
            r.entry.tabaqa,
            died(r),
            read,
            r.entry.name
        );
    }
    println!("{} huffaz", rows.len());
    ExitCode::SUCCESS
}

/// iy-rs group [ID] (AM1, EB1): the groups of the Companions and the Household, or one of them — its members and the
/// hadith that name it (`cli.cmd_group`).
fn group_cmd(a: GroupArgs) -> ExitCode {
    let lang = a.lang.clone().unwrap_or_else(i18n::ui_language);
    let road = (a.tradition != "all").then_some(a.tradition.as_str());
    let Some(id) = &a.id else {
        for g in sahaba::groups(road) {
            println!(
                "  {:10} {:6} {}",
                g.id,
                g.road,
                sahaba::text(&g.title, &lang)
            );
        }
        println!("\n(iy group ID shows one: its members and the hadith that name them)");
        return ExitCode::SUCCESS;
    };
    let Some(group) = sahaba::find(id) else {
        eprintln!("No group '{id}'. See: iy group --tradition all");
        return ExitCode::from(1);
    };
    let conn = match open() {
        Ok(c) => c,
        Err(code) => return code,
    };
    println!("{}   [{}]\n", sahaba::text(&group.title, &lang), group.road);
    println!(
        "{}\n",
        py::fill(&sahaba::text(&group.lead, &lang), a.width, "")
    );
    let shia = group.road == "shia";
    for m in sahaba::members(Some(&conn), group, &lang) {
        println!("  {}  {}", m.name, m.arabic);
        let mut facts: Vec<String> = vec![];
        if !m.died.is_empty() {
            facts.push(m.died.clone());
        }
        if shia {
            facts.push(format!(
                "{} chains end here in the Shia books",
                py::thousands(m.ends.unwrap_or(0))
            ));
        } else if m.person.is_some() {
            facts.push(format!(
                "in {} chain links, of {} hadith",
                py::thousands(m.links.unwrap_or(0)),
                py::thousands(m.hadith.unwrap_or(0))
            ));
        }
        if !facts.is_empty() {
            println!("     {}", facts.join(" · "));
        }
    }
    if shia {
        let shared = sahaba::shared_ends(&conn);
        if shared != 0 {
            println!(
                "\n  {} more chains end at an Imam not named or named by a shared kunya (Abu al-Hasan)",
                py::thousands(shared)
            );
        }
    }
    let found = sahaba::hadith(&conn, group);
    if !group.hadith.is_empty() {
        let refs: Vec<String> = found.iter().map(|(b, n, _)| format!("{b} {n}")).collect();
        let listed = if refs.is_empty() {
            "none imported".to_string()
        } else {
            refs.join(", ")
        };
        let first = found
            .first()
            .map(|(b, n, _)| format!("   (iy chain {b} {n})"))
            .unwrap_or_default();
        println!("\nThe hadith that name them: {listed}{first}");
    }
    ExitCode::SUCCESS
}

/// iy-rs sira [EVENT] (SIY1): the life of the Prophet ﷺ as a timeline, or one event with its place, verses, Ibn Hisham's
/// chapter and the hadith that name it (`cli.cmd_sira`).
fn sira_cmd(a: SiraArgs) -> ExitCode {
    let lang = a.lang.clone().unwrap_or_else(i18n::ui_language);
    let Some(id) = &a.event else {
        for (key, name) in sira::periods() {
            println!("\n{}", sira::text(name, &lang));
            for e in sira::events(Some(key)) {
                println!(
                    "  {:12} {:38} {}",
                    e.id,
                    sira::year_label(e, &lang),
                    sira::text(&e.title, &lang)
                );
            }
        }
        println!("\n(iy sira EVENT: its date, place, verses, Ibn Hisham's chapter and the hadith that name it)");
        return ExitCode::SUCCESS;
    };
    let Some(e) = sira::find(id) else {
        eprintln!("No event '{id}'. See: iy sira");
        return ExitCode::from(1);
    };
    println!(
        "{}   {}",
        sira::text(&e.title, &lang),
        e.title.get("ar").map(String::as_str).unwrap_or("")
    );
    println!(
        "{} · {} · {} · age {}\n",
        sira::year_label(e, &lang),
        sira::text(&e.when, &lang),
        sira::text(&e.place, &lang),
        sira::age(e)
    );
    println!("{}", py::fill(&sira::text(&e.text, &lang), a.width, ""));
    if let Some(d) = &e.differ {
        println!(
            "{}",
            py::fill(
                &format!("Reports differ: {}", sira::text(d, &lang)),
                a.width,
                ""
            )
        );
    }
    if !e.quran.is_empty() {
        let verses: Vec<String> = e
            .quran
            .iter()
            .map(|(s, x, y)| quran::label(*s, *x, *y, &lang))
            .collect();
        println!("\nVerses: {}", verses.join(", "));
    }
    let conn = match open() {
        Ok(c) => c,
        Err(code) => return code,
    };
    let chapter = sira::sira_chapter(&conn, e);
    let order = chapter
        .and_then(|_| works::find_work(&conn, sira::work(), &lang).ok().flatten())
        .and_then(|w| works::reading_order(&conn, w.id).ok())
        .unwrap_or_default();
    if !e.sira.is_empty() {
        let at = chapter.and_then(|c| order.iter().position(|x| *x == c));
        let tail = match at {
            Some(i) => format!("  (iy book sira {})", i + 1),
            None => "  (import it: iy catalog import sira)".to_string(),
        };
        println!("Ibn Hisham: {}{tail}", e.sira);
    }
    if !e.phrase.is_empty() {
        let n = match sira::hadith_counts(&conn) {
            Ok(c) => c.get(&e.id).copied().unwrap_or(0),
            Err(err) => {
                eprintln!("{err}");
                return ExitCode::from(1);
            }
        };
        println!(
            "Hadith that say \"{}\": about {} in the Sunni books (iy search \"{}\" --mode phrase)",
            e.phrase,
            py::thousands(n),
            e.phrase
        );
    }
    ExitCode::SUCCESS
}

/// iy-rs cache info|clear (`cli.cmd_cache`): the folder of the kept counts.
fn cache_cmd(action: &str) -> ExitCode {
    let folder = paths::data_dir().join("cache");
    let jsons = || -> Vec<std::path::PathBuf> {
        std::fs::read_dir(&folder)
            .map(|d| {
                d.flatten()
                    .map(|e| e.path())
                    .filter(|p| p.extension().map(|x| x == "json").unwrap_or(false))
                    .collect()
            })
            .unwrap_or_default()
    };
    if action == "clear" {
        isnady_core::log::info("cache", "⟲ kept files cleared");
        isnady_core::cache::clear();
        let n = jsons()
            .iter()
            .filter(|p| std::fs::remove_file(p).is_ok())
            .count();
        println!("Deleted {n} kept file(s); everything is counted again when next asked for.");
        return ExitCode::SUCCESS;
    }
    let files = jsons();
    let size: u64 = files
        .iter()
        .filter_map(|p| p.metadata().ok())
        .map(|m| m.len())
        .sum();
    println!("Cache folder: {}", paths::shown(&folder));
    println!(
        "{} file(s), {} KB — counted once per change of the database, read from here after.",
        files.len(),
        py::thousands(py::round_even(size as f64 / 1024.0) as i64)
    );
    ExitCode::SUCCESS
}

/// iy-rs log (`cli.cmd_log`, LOG1).
fn log_cmd(a: LogArgs) -> ExitCode {
    use isnady_core::log;
    if a.path {
        println!("{}", paths::shown(&log::path()));
        return ExitCode::SUCCESS;
    }
    if a.clear {
        println!(
            "{} log file(s) emptied or removed: {}",
            log::clear(),
            paths::shown(&log::folder())
        );
        return ExitCode::SUCCESS;
    }
    if let Some(day) = &a.before {
        println!("{} line(s) before {day} removed", log::delete_before(day));
        return ExitCode::SUCCESS;
    }
    if a.files {
        for f in log::files() {
            if let Ok(m) = f.metadata() {
                let when = m.modified().map(chrono_like).unwrap_or_default();
                println!(
                    "{when}  {:8.1} KB  {}",
                    m.len() as f64 / 1024.0,
                    paths::shown(&f)
                );
            }
        }
        return ExitCode::SUCCESS;
    }
    let mut target = log::path();
    if a.crash {
        match log::files().into_iter().find(|f| {
            f.file_name()
                .map(|n| n.to_string_lossy().starts_with("crash_"))
                .unwrap_or(false)
        }) {
            Some(f) => target = f,
            None => {
                println!("No crash reports.");
                return ExitCode::SUCCESS;
            }
        }
    }
    if !target.exists() {
        println!("No log yet: {}", paths::shown(&target));
        return ExitCode::SUCCESS;
    }
    println!(
        "{}",
        log::read(
            &target,
            a.lines,
            a.level.as_deref().unwrap_or(""),
            a.grep.as_deref().unwrap_or("")
        )
    );
    if a.follow {
        use std::io::{Read, Seek, Write};
        let mut size = target.metadata().map(|m| m.len()).unwrap_or(0);
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
            let now = target.metadata().map(|m| m.len()).unwrap_or(0);
            if now < size {
                size = 0;
            }
            if now > size {
                if let Ok(mut f) = std::fs::File::open(&target) {
                    let _ = f.seek(std::io::SeekFrom::Start(size));
                    let mut buf = Vec::new();
                    let _ = f.read_to_end(&mut buf);
                    let mut out = std::io::stdout();
                    let _ = out.write_all(String::from_utf8_lossy(&buf).as_bytes());
                    let _ = out.flush();
                }
                size = now;
            }
        }
    }
    ExitCode::SUCCESS
}

fn chrono_like(t: std::time::SystemTime) -> String {
    isnady_core::py::local_time(t, "%Y-%m-%d %H:%M")
}

/// iy-rs reset settings|books|all (`cli.cmd_reset`, RST1).
fn reset_cmd(what: &str, yes: bool) -> ExitCode {
    use std::io::IsTerminal;
    let shown = match what {
        "settings" => "every setting and remembered place",
        "books" => "every imported book and source",
        _ => "the books, the settings, the bookmarks and the downloaded data",
    };
    if !yes {
        if !std::io::stdin().is_terminal() {
            eprintln!("Add --yes to delete without being asked.");
            return ExitCode::from(2);
        }
        eprint!("Delete {shown}? This cannot be undone. Type yes: ");
        let mut answer = String::new();
        let _ = std::io::stdin().read_line(&mut answer);
        if !["yes", "evet", "y", "e"].contains(&answer.trim().to_lowercase().as_str()) {
            println!("Nothing deleted.");
            return ExitCode::from(1);
        }
    }
    if what != "settings" {
        return sources_cmd::reset_data(what);
    }
    match isnady_core::config::reset_settings() {
        Ok(n) => {
            isnady_core::log::warning("reset", &format!("⟲ every setting reset ({n} stored)"));
            println!("{n} settings deleted; isnady is as at its first start.");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(1)
        }
    }
}

/// iy-rs shortcut [--no-desktop] [--no-menu] (`cli.cmd_shortcut`): the window on the desktop and in the menu.
fn shortcut_cmd(desktop: bool, menu: bool) -> ExitCode {
    use isnady_core::shortcut;
    let (program, how) = shortcut::launch_command();
    match shortcut::create(desktop, menu) {
        Ok(made) => {
            for (what, path) in made {
                println!("  {what:8} {}", paths::shown(&path));
            }
            println!("It starts {how}: {}", paths::shown(&program));
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("The shortcut could not be created: {e}");
            ExitCode::from(1)
        }
    }
}

/// iy-rs datadir [PATH] [--as-is] [--default] (`cli.cmd_datadir`).
fn datadir_cmd(path: Option<String>, as_is: bool, default: bool) -> ExitCode {
    if path.is_none() && !default {
        println!(
            "{}  ({})",
            paths::shown(&paths::data_dir()),
            paths::data_dir_source()
        );
        println!("default: {}", paths::shown(&paths::default_data_dir()));
        return ExitCode::SUCCESS;
    }
    let wanted = if default { None } else { path.as_deref() };
    match paths::relocate(wanted, if as_is { "as-is" } else { "copy" }) {
        Ok(target) => {
            let how = if as_is {
                "used as it is"
            } else {
                "your data was copied there; the old folder is kept"
            };
            println!("isnady now uses {} ({how}).", paths::shown(&target));
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(2)
        }
    }
}

/// iy-rs config list|get|set|reset (`cli.cmd_config`).
fn config_cmd(a: ConfigArgs) -> ExitCode {
    use isnady_core::config;
    let done = (|| -> Result<(), String> {
        match a.action.as_str() {
            "list" => {
                for (key, value, default) in config::all_values() {
                    if !a.prefix.is_empty() && !key.starts_with(&a.prefix) {
                        continue;
                    }
                    let mark = if config::same_value(&value, &default) {
                        ""
                    } else {
                        "   (changed)"
                    };
                    let shown = config::py_str(&value);
                    let shown = if shown.is_empty() {
                        "(default)".to_string()
                    } else {
                        shown
                    };
                    println!("{key:32} {shown}{mark}");
                }
                println!("\nSaved in {}", paths::shown(&config::settings_path()));
            }
            "get" => {
                let key = a
                    .key
                    .as_deref()
                    .ok_or("Give the setting: iy config get text.arabic.size")?;
                config::validate(key, &"".into())?;
                println!("{}", config::py_str(&config::effective(key)));
            }
            "set" => {
                let (Some(key), Some(value)) = (a.key.as_deref(), a.value.as_deref()) else {
                    return Err(
                        "Give the setting and the value: iy config set text.arabic.size 22".into(),
                    );
                };
                config::set(key, value.into())?;
                println!("{key} = {}", config::py_str(&config::effective(key)));
            }
            _ => {
                let prefix = a.key.clone().unwrap_or_default();
                let removed = config::reset(&prefix)?;
                println!(
                    "Reset {removed} setting{} to the default{}",
                    if removed != 1 { "s" } else { "" },
                    if prefix.is_empty() {
                        String::new()
                    } else {
                        format!(" under {prefix}")
                    }
                );
            }
        }
        Ok(())
    })();
    match done {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(2)
        }
    }
}

/// iy-rs profile list|new|use|rename|delete|export|import (`cli.cmd_profile`, PRF1).
fn profile_cmd(a: ProfileArgs) -> ExitCode {
    use isnady_core::config;
    let which = |text: Option<&str>| -> Result<Option<String>, String> {
        match text.filter(|t| !t.is_empty()) {
            None => Ok(None),
            Some(t) => config::find_profile(t)
                .map(Some)
                .ok_or_else(|| format!("No profile {t}. See: iy profile list")),
        }
    };
    let done = (|| -> Result<(), String> {
        let name = a.name.as_deref();
        match a.action.as_str() {
            "list" => {
                for p in config::all_profiles() {
                    let shown = if p.name.is_empty() {
                        "Default"
                    } else {
                        &p.name
                    };
                    println!(
                        "{} {:24} {:20} {}",
                        if p.active { '*' } else { ' ' },
                        shown,
                        p.id,
                        paths::shown(&config::folder_of(&p.id))
                    );
                }
                println!("\n* the profile in use. Its settings, place and bookmarks are its own; the books are shared.");
            }
            "new" => {
                let from = match &a.copy {
                    Some(c) => which(Some(c))?,
                    None => None,
                };
                let pid = config::create_profile(name.unwrap_or(""), from.as_deref())?;
                let n = name.unwrap_or("None");
                println!("Made the profile {n} ({pid}). Use it: iy profile use \"{n}\"");
            }
            "use" => {
                let pid = which(name)?.unwrap_or_else(|| config::DEFAULT_PROFILE.to_string());
                config::switch_profile(&pid)?;
                let now = config::profile_name(&config::active_profile());
                println!("In use: {}", if now.is_empty() { "Default" } else { &now });
            }
            "rename" => {
                let Some(value) = a.value.as_deref().filter(|v| !v.is_empty()) else {
                    return Err("Give the new name: iy profile rename OLD NEW".into());
                };
                let pid = which(name)?.unwrap_or_else(|| config::DEFAULT_PROFILE.to_string());
                config::rename_profile(&pid, value)?;
                println!("Renamed to {value}");
            }
            "delete" => {
                let pid = which(name)?.unwrap_or_default();
                config::delete_profile(&pid)?;
                println!("Deleted {}", name.unwrap_or("None"));
            }
            "export" => {
                let pid = match name {
                    Some(_) => which(name)?.unwrap_or_default(),
                    None => config::active_profile(),
                };
                let written = config::export_profile(&pid, a.value.as_deref().unwrap_or("."))?;
                println!("Written: {}", paths::shown(&written));
            }
            _ => {
                let Some(file) = name.filter(|n| !n.is_empty()) else {
                    return Err("Give the file: iy profile import FILE.isnady-profile.zip".into());
                };
                let pid = config::import_profile(file, a.value.as_deref())?;
                let n = config::profile_name(&pid);
                println!("Imported as {n} ({pid}). Use it: iy profile use \"{n}\"");
            }
        }
        Ok(())
    })();
    match done {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(2)
        }
    }
}

/// iy-rs quran (Q1, Q2): the suras, a sura or verses with the translations chosen and the hadith that name each
/// verse, a search; editions downloaded, remove (`cli.cmd_quran`). Downloading comes with the data sources.
fn quran_cmd(a: QuranArgs) -> ExitCode {
    let lang = a.lang.clone().unwrap_or_else(i18n::ui_language);
    let words = &a.what;
    let first = words.first().map(String::as_str);
    let say = |m: &str| eprintln!("  … {m}");
    if first == Some("download") {
        let wanted: Vec<String> = if words.len() > 1 {
            words[1..].to_vec()
        } else {
            quran::default_editions()
        };
        return match quran::fetch(&wanted, &say) {
            Ok(kept) => {
                println!(
                    "Downloaded: {} — {}",
                    if kept.is_empty() {
                        "nothing new".to_string()
                    } else {
                        kept.join(", ")
                    },
                    quran::path().display()
                );
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{e}");
                ExitCode::from(1)
            }
        };
    }
    if first == Some("editions") {
        if a.installed || (words.len() == 1 && a.language.is_none()) {
            for e in quran::editions() {
                println!(
                    "  {:28} {:12} {:40} [{}]",
                    e.key, e.language, e.author, e.origin
                );
            }
            if !quran::available() {
                println!("The Qur'an is not downloaded: iy quran download");
            }
            return ExitCode::SUCCESS;
        }
        let everything = match quran::catalog(&say, !a.own) {
            Ok(x) => x,
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::from(1);
            }
        };
        let mut rows: Vec<&(String, quran::Offered)> = everything.iter().collect();
        rows.sort_by(|x, y| {
            (
                &x.1.language,
                x.1.origin != quran::OWN,
                x.1.author.to_lowercase(),
            )
                .cmp(&(
                    &y.1.language,
                    y.1.origin != quran::OWN,
                    y.1.author.to_lowercase(),
                ))
        });
        let mut shown = 0;
        for (key, v) in rows {
            if let Some(l) = &a.language {
                let l = l.to_lowercase();
                if l != v.language.to_lowercase() && l != key.chars().take(3).collect::<String>() {
                    continue;
                }
            }
            if key.ends_with("-la") {
                continue;
            }
            let star = if v.origin == quran::OWN { "★" } else { " " };
            println!(
                "  {star} {key:30} {:12} {}   {}",
                v.language,
                v.author,
                if v.source.is_empty() {
                    &v.origin
                } else {
                    &v.source
                }
            );
            shown += 1;
        }
        let own = everything
            .iter()
            .filter(|(_, v)| v.origin == quran::OWN)
            .count();
        if own > 0 {
            println!("{shown} shown · ★ {own} kept in isnady-data");
        } else {
            println!(
                "{shown} shown · isnady-data has none yet: every text is read from {}",
                quran::SOURCE
            );
        }
        return ExitCode::SUCCESS;
    }
    if first == Some("remove") {
        for key in &words[1..] {
            println!(
                "{key}: {}",
                if quran::remove(key) {
                    "removed"
                } else {
                    "not downloaded (the Arabic is always kept)"
                }
            );
        }
        return ExitCode::SUCCESS;
    }
    if !quran::available() {
        println!("The Qur'an is not downloaded yet (about 3 MB, once): iy quran download\n  more translations: iy quran editions --language Turkish, then iy quran download KEY …");
        return ExitCode::from(1);
    }
    let chosen = if a.edition.is_empty() {
        quran::shown(quran::saved_choice().as_deref())
    } else {
        a.edition.clone()
    };
    let eds = quran::editions();
    if first == Some("search") {
        let text = words[1..].join(" ");
        let found = quran::search(&text, &chosen, 200);
        for r in found.iter().take(a.limit) {
            println!(
                "  {:24} {}",
                quran::label(r.sura, r.ayah, r.ayah, &lang),
                py::take(&r.text, a.width)
            );
        }
        println!(
            "{} verses{}",
            found.len(),
            if found.len() >= 200 {
                " (the first 200)"
            } else {
                ""
            }
        );
        return ExitCode::SUCCESS;
    }
    let Some(first) = first else {
        for x in quran::suras() {
            let name = match lang.as_str() {
                "tr" if !x.tr.is_empty() => x.tr.clone(),
                "ar" if !x.arabic.is_empty() => x.arabic.clone(),
                _ => format!("{} ({})", x.name, x.english),
            };
            println!(
                "  {:3}. {:34} {:3} verses  {}",
                x.n,
                name,
                x.verses,
                if x.place == "Mecca" {
                    "Meccan"
                } else {
                    "Medinan"
                }
            );
        }
        println!(
            "\n(iy quran 2, iy quran 2:255, iy quran 33:28-34 --hadith, iy quran search WORDS)"
        );
        return ExitCode::SUCCESS;
    };
    let Some((sura, start, end)) = quran::parse_ref(first) else {
        eprintln!("Write a sura, or a verse: iy quran 2, iy quran 2:255, iy quran 2:255-257");
        return ExitCode::from(1);
    };
    let conn = if a.hadith {
        match open() {
            Ok(c) => Some(c),
            Err(code) => return code,
        }
    } else {
        None
    };
    let named = match conn.as_ref().map(quran::named) {
        Some(Ok(n)) => Some(n),
        Some(Err(e)) => {
            eprintln!("{e}");
            return ExitCode::from(1);
        }
        None => None,
    };
    println!(
        "{sura}. {}  {}\n",
        quran::sura_name(sura, &lang),
        quran::sura_name(sura, "ar")
    );
    for v in quran::verses(sura, &chosen, start, end) {
        println!(
            "  {sura}:{}  {}",
            v.ayah,
            v.texts.get(quran::ARABIC).map(String::as_str).unwrap_or("")
        );
        for key in &chosen {
            if let Some(t) = v.texts.get(key).filter(|t| !t.is_empty()) {
                let author = eds
                    .iter()
                    .find(|e| &e.key == key)
                    .map(|e| e.author.clone())
                    .unwrap_or_else(|| key.clone());
                println!("{}   [{author}]", py::fill(t, a.width, "      "));
            }
        }
        if let (Some(conn), Some(named)) = (&conn, &named) {
            let hids = quran::hadith_of(named, sura, v.ayah);
            if !hids.is_empty() {
                let sql = format!(
                    "SELECT c.key, h.number, COALESCE(c.tradition, 'sunni') FROM hadiths h JOIN collections c \
                     ON c.id = h.collection_id WHERE h.id IN ({})",
                    vec!["?"; hids.len()].join(",")
                );
                let rows: Vec<(String, String, String)> = conn
                    .prepare(&sql)
                    .and_then(|mut st| {
                        st.query_map(isnady_core::db::params_from_iter(hids.iter()), |r| {
                            Ok((r.get(0)?, isnady_core::db::number_text(r, 1)?, r.get(2)?))
                        })
                        .and_then(|rows| rows.collect())
                    })
                    .unwrap_or_default();
                for road in ["sunni", "shia"] {
                    let mine: Vec<String> = rows
                        .iter()
                        .filter(|(_, _, r)| r == road)
                        .map(|(k, n, _)| format!("{k} {n}"))
                        .collect();
                    if !mine.is_empty() {
                        let more = if mine.len() > 15 {
                            format!(" … +{}", mine.len() - 15)
                        } else {
                            String::new()
                        };
                        println!(
                            "      named in the {} books: {}{more}",
                            py::upper_first(road),
                            mine[..mine.len().min(15)].join(", ")
                        );
                    }
                }
            }
        }
        println!();
    }
    ExitCode::SUCCESS
}

/// iy-rs sahifa [ID] (SHF1): the written sources — Hammam's sahifa in the Musnad with its parallels, the writings
/// that live in later books, writing in the chains; the Sunni road and the Shia road apart (`cli.cmd_sahifa`).
fn sahifa_cmd(a: SahifaArgs) -> ExitCode {
    let lang = a.lang.clone().unwrap_or_else(i18n::ui_language);
    let conn = match open() {
        Ok(c) => c,
        Err(code) => return code,
    };
    match sahifa_shown(&conn, &a, &lang) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(1)
        }
    }
}

fn sahifa_shown(
    conn: &rusqlite_conn::Connection,
    a: &SahifaArgs,
    lang: &str,
) -> rusqlite_conn::Result<ExitCode> {
    let c = sahifa::counted(conn)?;
    let Some(id) = &a.id else {
        let counts = sahifa::summary(&c);
        for road in ["sunni", "shia"] {
            if a.tradition != "all" && a.tradition != road {
                continue;
            }
            println!("\n{} road", py::upper_first(road));
            for s in sahifa::sources(Some(road)) {
                let r = counts.iter().find(|r| r.id == s.id).expect("summary");
                let extra = if s.id == "hammam" {
                    format!(
                        ", {} with parallels ({} hadith)",
                        r.with_parallels, r.parallels
                    )
                } else {
                    String::new()
                };
                println!(
                    "  {:14} {:52} {:>5} hadith{extra}",
                    s.id,
                    sahifa::text(&s.title, lang),
                    py::thousands(r.count as i64)
                );
            }
        }
        println!("\n(iy sahifa ID: what it is, where it lives, its hadith)");
        return Ok(ExitCode::SUCCESS);
    };
    let Some(s) = sahifa::find(id) else {
        eprintln!("No written source '{id}'. See: iy sahifa");
        return Ok(ExitCode::from(1));
    };
    println!("{}   [{} road]\n", sahifa::text(&s.title, lang), s.road);
    println!("{}", py::fill(&sahifa::text(&s.lead, lang), a.width, ""));
    println!(
        "{}\n",
        py::fill(
            &format!("How isnady finds it: {}", sahifa::text(&s.how, lang)),
            a.width,
            ""
        )
    );
    let ids = sahifa::hadith(&c, &s.id);
    let books: Vec<String> = sahifa::by_book(conn, &ids)?
        .iter()
        .map(|(k, _, n)| format!("{k} {}", py::thousands(*n)))
        .collect();
    println!(
        "{} hadith in the books imported here: {}",
        py::thousands(ids.len() as i64),
        books.join(", ")
    );
    for (i, (hid, key, num)) in sahifa::refs(conn, &ids)?.iter().take(a.limit).enumerate() {
        let mut line = format!("  {:4}. {key} {num}", i + 1);
        if s.id == "hammam" {
            let par = sahifa::refs(conn, &sahifa::parallels(&c, *hid))?;
            line += "   ← ";
            if par.is_empty() {
                line += "no parallel through Hammam";
            } else {
                let p: Vec<String> = par.iter().map(|(_, k, n)| format!("{k} {n}")).collect();
                line += &p.join(", ");
            }
        }
        println!("{line}");
    }
    if ids.len() > a.limit {
        println!(
            "  … {} more (--limit)",
            py::thousands((ids.len() - a.limit) as i64)
        );
    }
    Ok(ExitCode::SUCCESS)
}

/// iy-rs tree WHICH [NAME] (GEN1): the prophets, Quraysh, a narrator's family, a narrator's line of learning
/// (`cli.cmd_tree`).
fn tree_cmd(a: TreeArgs) -> ExitCode {
    let ui = i18n::ui_language();
    let langs = narrators::Langs {
        reading: &ui,
        words: &ui,
    };
    if genealogy::is_written(&a.which) {
        let tree = genealogy::written(&a.which, "en").expect("written tree");
        if let Some(name) = &a.name {
            let hits = genealogy::find(&tree, name);
            if hits.is_empty() {
                eprintln!("No one named '{name}' in this tree.");
                return ExitCode::from(1);
            }
            for n in hits.iter().take(5) {
                let line: Vec<String> = genealogy::path_to(&tree, &n.id)
                    .iter()
                    .map(|x| x.name("en"))
                    .collect();
                println!("{}", line.join(" → "));
                if n.note.is_some() {
                    println!("  note: {}", n.text("note", "en"));
                }
                if n.disputed.is_some() {
                    println!("  disputed: {}", n.text("disputed", "en"));
                }
            }
            if a.hadith {
                let n = hits[0];
                if n.id == "muhammad" {
                    println!("\nHis life ﷺ: iy life muhammad");
                    return ExitCode::SUCCESS;
                }
                let conn = match open() {
                    Ok(c) => c,
                    Err(code) => return code,
                };
                let found = match genealogy::prophet_hadith(&conn, &n.id, a.width * 10) {
                    Ok(f) => f,
                    Err(e) => {
                        eprintln!("{e}");
                        return ExitCode::from(1);
                    }
                };
                println!(
                    "\nHadith that speak of {}: {}{}",
                    n.name("en"),
                    py::thousands(found.total as i64),
                    if found.total > 0 {
                        ""
                    } else {
                        " (none names him in words that can only mean him)"
                    }
                );
                for (road, items, count) in [
                    ("Sunni", &found.sunni, found.count_sunni),
                    ("Shia", &found.shia, found.count_shia),
                ] {
                    if !items.is_empty() {
                        let refs: Vec<String> = items
                            .iter()
                            .map(|i| format!("{} {}", i.book, i.number))
                            .collect();
                        println!(
                            "  {road} books ({}): {}",
                            py::thousands(count as i64),
                            refs.join(", ")
                        );
                    }
                }
            }
            return ExitCode::SUCCESS;
        }
        println!("{}", genealogy::as_text(&tree, "en", &ui));
        return ExitCode::SUCCESS;
    }
    let Some(name) = &a.name else {
        eprintln!("Give a narrator's name: iy tree family \"Ibn Umar\"");
        return ExitCode::from(2);
    };
    let conn = match open() {
        Ok(c) => c,
        Err(code) => return code,
    };
    let run = || -> rusqlite_conn::Result<ExitCode> {
        let (rows, _total) = narrators::find(&conn, name, None, langs)?;
        let Some(first) = rows.first() else {
            eprintln!("No narrator found for '{name}'.");
            return Ok(ExitCode::from(1));
        };
        let pid = first.id;
        let tree = if a.which == "family" {
            match genealogy::family(&conn, pid, langs)? {
                Some(t) => t,
                None => {
                    println!("No kin among the narrators: no narrator's name makes him a father or a son.");
                    return Ok(ExitCode::SUCCESS);
                }
            }
        } else {
            genealogy::learning(&conn, pid, a.depth, a.width, langs)?
        };
        println!("{}", genealogy::as_text(&tree, "en", &ui));
        if a.which == "family" {
            println!(
                "\n(read from the narrators' names: \"X b. Y b. Z\" is the son of \"Y b. Z\" when exactly one narrator \
                 of that name and of an earlier or the same generation is found)"
            );
        }
        Ok(ExitCode::SUCCESS)
    };
    run().unwrap_or_else(|e| {
        eprintln!("{e}");
        ExitCode::from(1)
    })
}

/// iy-rs bookmark ACTION (BM1): list, add, remove, folders, export, import — the same file as the Bookmarks page
/// (`cli.cmd_bookmark`).
fn bookmark_cmd(a: BookmarkArgs) -> ExitCode {
    let ui = i18n::ui_language();
    let langs = narrators::Langs {
        reading: &ui,
        words: &ui,
    };
    let mut store = bookmarks::Store::load();
    let save = |s: &bookmarks::Store| -> ExitCode {
        match s.save() {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("{e}");
                ExitCode::from(1)
            }
        }
    };
    match a.action.as_str() {
        "list" => {
            let folder = a.folder.as_deref().map(|f| store.folder_named(f));
            if let Some(None) = folder {
                eprintln!("No folder '{}'.", a.folder.as_deref().unwrap_or(""));
                return ExitCode::from(1);
            }
            let mut items: Vec<&bookmarks::Item> = match folder.flatten() {
                Some(f) => store.in_folder(Some(&f.id)),
                None => store.items.iter().collect(),
            };
            if let Some(k) = &a.kind {
                items.retain(|i| &i.kind == k);
            }
            let conn = if items.iter().any(|i| i.kind == "narrator") {
                db::connect(None).ok()
            } else {
                None
            };
            for i in &items {
                let place: Vec<String> = i.folders.iter().map(|f| store.path_of(f)).collect();
                let more = bookmarks::detail(conn.as_ref(), i, &ui);
                let mut line = format!(
                    "  {}  [{}] {}",
                    i.id,
                    bookmarks::kind_name(&i.kind),
                    i.title
                );
                if !more.is_empty() {
                    line += &format!("  · {more}");
                }
                if !place.is_empty() {
                    line += &format!("   — {}", place.join(", "));
                }
                if !i.note.is_empty() {
                    line += &format!("\n      ✎ {}", i.note);
                }
                println!("{line}");
            }
            println!("{} bookmark(s)", items.len());
            ExitCode::SUCCESS
        }
        "folders" => {
            fn show(store: &bookmarks::Store, parent: Option<&str>, depth: usize) {
                for f in store.children(parent) {
                    println!(
                        "  {}📁 {}  ({})",
                        "  ".repeat(depth),
                        f.name,
                        store.in_folder(Some(&f.id)).len()
                    );
                    show(store, Some(&f.id), depth + 1);
                }
            }
            show(&store, None, 0);
            ExitCode::SUCCESS
        }
        "add" => {
            let (Some(kind), Some(reference)) = (&a.kind, &a.reference) else {
                eprintln!(
                    "iy bookmark add KIND REF — kinds: hadith (bukhari:1), narrator (his key), scholar, term, chapter, \
                     tree (prophets:musa, quraysh:ali, family:<narrator's key>, learning:<narrator's key>)"
                );
                return ExitCode::from(2);
            };
            if !bookmarks::KINDS.contains(&kind.as_str()) {
                eprintln!(
                    "unknown kind '{kind}'; one of {}",
                    bookmarks::KINDS.join(", ")
                );
                return ExitCode::from(1);
            }
            let (mut made_ref, mut title, mut subtitle) = (
                reference.clone(),
                a.title.clone().unwrap_or_else(|| reference.clone()),
                String::new(),
            );
            if ["hadith", "narrator", "chapter", "tree"].contains(&kind.as_str()) {
                let conn = match open() {
                    Ok(c) => c,
                    Err(code) => return code,
                };
                let made = match kind.as_str() {
                    "hadith" => bookmarks::resolve_hadith(&conn, reference)
                        .and_then(|h| bookmarks::hadith_ref(&conn, h, &ui)),
                    "narrator" => bookmarks::resolve_narrator(&conn, reference)
                        .and_then(|p| bookmarks::narrator_ref(&conn, p, langs)),
                    "chapter" => bookmarks::resolve_chapter(&conn, reference)
                        .and_then(|c| bookmarks::chapter_ref(&conn, c, &ui)),
                    _ => bookmarks::resolve_tree(Some(&conn), reference).and_then(
                        |(which, target)| {
                            if genealogy::is_written(&which) {
                                bookmarks::tree_ref(Some(&conn), &which, Some(&target), None, langs)
                            } else {
                                bookmarks::tree_ref(
                                    Some(&conn),
                                    &which,
                                    None,
                                    target.parse().ok(),
                                    langs,
                                )
                            }
                        },
                    ),
                };
                let Some((r, t, sub)) = made else {
                    if kind == "tree" {
                        eprintln!("No place '{reference}' in the genealogies (e.g. prophets:musa, family:taqrib:6340).");
                    } else {
                        eprintln!("No {kind} '{reference}' in the database.");
                    }
                    return ExitCode::from(1);
                };
                (made_ref, title, subtitle) = (r, t, sub);
            }
            let folder = a
                .folder
                .as_deref()
                .and_then(|f| store.ensure_path(f))
                .map(|f| f.id);
            let title = a.title.clone().unwrap_or(title);
            let item = store.add(
                kind,
                &made_ref,
                &title,
                &subtitle,
                folder.as_deref(),
                a.note.as_deref().unwrap_or(""),
            );
            let code = save(&store);
            println!("Bookmarked: {}  ({})", item.title, item.id);
            code
        }
        "remove" => {
            let ok = store.remove(a.kind.as_deref().unwrap_or(""));
            save(&store);
            println!(
                "{}",
                if ok {
                    "Removed."
                } else {
                    "No bookmark with that id (iy bookmark list shows the ids)."
                }
            );
            if ok {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        "export" => {
            let file = a
                .kind
                .clone()
                .unwrap_or_else(|| "isnady_bookmarks.json".into());
            if let Err(e) = store.save_to(std::path::Path::new(&file)) {
                eprintln!("{e}");
                return ExitCode::from(1);
            }
            println!("Written: {file}");
            ExitCode::SUCCESS
        }
        _ => {
            let file = a.kind.clone().unwrap_or_default();
            let other = bookmarks::Store::load_from(std::path::Path::new(&file));
            let (items, folders) = store.merge(&other);
            let code = save(&store);
            println!("{items} bookmark(s) and {folders} folder(s) added.");
            code
        }
    }
}

/// iy-rs term [NAME] (S1): a term of the glossary, or a search (`cli.cmd_term`).
fn term_cmd(a: TermArgs) -> ExitCode {
    let lang = a.lang.as_str();
    if a.search.is_some() || a.name.is_none() {
        let found = learn::search(a.search.as_deref().unwrap_or(""));
        for t in &found {
            let short: String = learn::text(t, "short", lang).chars().take(70).collect();
            println!(
                "  {:16} {:30} {:18} {short}",
                learn::field(t, "id"),
                learn::name(t, lang),
                learn::field(t, "ar")
            );
        }
        println!("{} term(s)", found.len());
        return ExitCode::SUCCESS;
    }
    let name = a.name.as_deref().unwrap_or("");
    let Some(t) = learn::get(name).or_else(|| learn::search(name).into_iter().next()) else {
        eprintln!("No term '{name}'. Try: iy term --search {name}");
        return ExitCode::from(1);
    };
    let arabic = if !learn::field(t, "ar").is_empty() && lang != "ar" {
        format!("  {}", learn::field(t, "ar"))
    } else {
        String::new()
    };
    println!(
        "{}{arabic}   [{}]",
        learn::name(t, lang),
        learn::category(&learn::field(t, "cat"), lang)
    );
    println!("  {}", learn::text(t, "short", lang));
    let long = learn::text(t, "long", lang);
    if !long.is_empty() {
        println!("  {long}");
    }
    let related: Vec<String> = t["related"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|r| {
            learn::get(r.as_str().unwrap_or(""))
                .map(|x| learn::name(x, lang))
                .unwrap_or_default()
        })
        .collect();
    if !related.is_empty() {
        println!("  related: {}", related.join(", "));
    }
    if !learn::field(t, "source").is_empty() {
        println!("  source: {}", learn::field(t, "source"));
    }
    ExitCode::SUCCESS
}

/// iy-rs learn [ARTICLE] (LRN-H1): the articles of History and method, with their live numbers (`cli.cmd_learn`).
fn learn_cmd(a: LearnArgs) -> ExitCode {
    let lang = a.lang.clone().unwrap_or_else(|| {
        let ui = i18n::ui_language();
        if ["en", "tr", "ar"].contains(&ui.as_str()) {
            ui
        } else {
            "en".into()
        }
    });
    let Some(id) = &a.article else {
        for x in learn::articles() {
            println!(
                "  {:10} {}",
                x["id"].as_str().unwrap_or(""),
                learn::say_text(&x["title"], &lang)
            );
        }
        println!("Read one: iy learn tadwin   (writing, method, numbers; --lang tr|en|ar)");
        return ExitCode::SUCCESS;
    };
    let Some(art) =
        learn::article(id).or_else(|| learn::search_articles(id, &lang).into_iter().next())
    else {
        eprintln!("No article '{id}'. Try: iy learn");
        return ExitCode::from(1);
    };
    let para = |text: &str, indent: &str| println!("{}", py::fill(text, a.width, indent));
    println!("{}", learn::say_text(&art["title"], &lang).to_uppercase());
    println!();
    para(&learn::say_text(&art["lead"], &lang), "");
    let mut conn: Option<rusqlite_conn::Connection> = None;
    for sec in art["sections"].as_array().into_iter().flatten() {
        println!();
        println!("## {}", learn::say_text(&sec["heading"], &lang));
        for p in learn::say_list(&sec["text"], &lang) {
            println!();
            para(&p, "  ");
        }
        if let Some(kind) = sec["block"].as_str() {
            if conn.is_none() && kind != "timeline" {
                conn = match open() {
                    Ok(c) => Some(c),
                    Err(code) => return code,
                };
            }
            let rows = match learn::block(conn.as_ref(), kind, &lang) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("{e}");
                    return ExitCode::from(1);
                }
            };
            if !rows.is_empty() {
                println!();
                let wide = rows.iter().map(|r| r.0.chars().count()).max().unwrap_or(0);
                for (label, value, note) in &rows {
                    let mut line = if value.is_empty() {
                        format!("  [{label}]")
                    } else {
                        format!("    {label:<wide$}  {value}")
                    };
                    if !note.is_empty() && !value.is_empty() {
                        line += &format!("   ({note})");
                    }
                    println!("{line}");
                }
            }
        }
    }
    if !art["sources"].is_null() {
        println!();
        para(
            &format!(
                "{}: {}",
                learn::label("sources", &lang),
                learn::say_text(&art["sources"], &lang)
            ),
            "",
        );
    }
    ExitCode::SUCCESS
}

fn pct(x: f64) -> String {
    format!("{:.1}", 100.0 * x)
}

/// iy-rs scholar [NAME]: the hadith scholars in the data, or one of them with what the data measures of him
/// (`cli.cmd_scholar`).
fn scholar_cmd(a: ScholarArgs) -> ExitCode {
    let conn = match open() {
        Ok(c) => c,
        Err(code) => return code,
    };
    if a.huffaz || a.tabaqa.is_some_and(|t| t != 0) {
        return huffaz_cmd(&a, &conn);
    }
    let people = scholars::present(&conn);
    if a.name.is_empty() {
        for (s, roles) in &people {
            if a.tradition != "all" && scholars::tradition_of(s) != a.tradition {
                continue;
            }
            println!(
                "  {:13} {:38} {:6} {}",
                s.id,
                s.name,
                scholars::tradition_of(s),
                roles.join(", ")
            );
        }
        return ExitCode::SUCCESS;
    }
    let want = a.name.join(" ").to_lowercase();
    let Some((s, roles)) = people.iter().find(|(p, _)| {
        format!("{} {} {}", p.id, p.name, p.full)
            .to_lowercase()
            .contains(&want)
    }) else {
        eprintln!("No such scholar in the imported data. See: iy scholar");
        return ExitCode::from(1);
    };
    let lang = i18n::ui_language();
    let run = || -> rusqlite_conn::Result<()> {
        println!(
            "{}  ({})  {}  [{}]\n  roles: {}; works: {}",
            s.full,
            s.arabic,
            s.dates,
            scholars::tradition_of(s),
            roles.join(", "),
            s.works.join("; ")
        );
        for st in scholars::compiler_stats(&conn, s)? {
            println!(
                "\n  {}: {} hadith, {} chains, {} of {} narrators identified",
                st.book,
                py::thousands(st.hadith),
                py::thousands(st.chains),
                py::thousands(st.identified),
                py::thousands(st.links)
            );
            let teachers: Vec<String> = st
                .teachers
                .iter()
                .take(6)
                .map(|t| format!("{} ({})", t.name, t.count))
                .collect();
            println!("  teachers: {}", teachers.join("; "));
            if !st.ends.is_empty() {
                let ends: Vec<String> = st
                    .ends
                    .iter()
                    .take(6)
                    .map(|(_, label, n)| format!("{label} ({})", py::thousands(*n)))
                    .collect();
                println!("  his chains reach: {}", ends.join("; "));
            }
        }
        if let Some(g) = scholars::grader_stats(&conn, s)? {
            let dist: Vec<String> = g
                .distribution
                .iter()
                .map(|(k, v)| format!("{k} {}", py::thousands(*v as i64)))
                .collect();
            println!(
                "\n  grades: {} hadith — {}",
                py::thousands(g.graded as i64),
                dist.join(", ")
            );
            for (other, x) in &g.agreement {
                let k = x
                    .kappa
                    .map(|k| format!("{k:.2}"))
                    .unwrap_or_else(|| "-".into());
                let flag = if x.agree >= 0.95 {
                    "   (unusually high: possibly not independent in the source)"
                } else {
                    ""
                };
                println!(
                    "  vs {other}: agree {}% of {}, kappa {k}, he lower {}% / higher {}%{flag}",
                    pct(x.agree),
                    py::thousands(x.common as i64),
                    pct(x.lower),
                    pct(x.higher)
                );
            }
            if let (Some(st), Some(lo), Some(hi)) = (g.strictness, g.lower, g.higher) {
                println!(
                    "  strictness {st:+.2} by order: lower {}%, higher {}% of {} comparisons (negative = stricter)",
                    pct(lo),
                    pct(hi),
                    py::thousands(g.compared as i64)
                );
            }
        }
        if let Some(cr) = scholars::critic_stats(&conn, s)? {
            if cr.tradition == "shia" {
                let ranks: Vec<String> = cr
                    .ranks
                    .iter()
                    .map(|(_, label, n)| {
                        let label = i18n::t(&lang, label, &[]);
                        format!(
                            "{} {}",
                            label.split(" (").next().unwrap_or(""),
                            py::thousands(*n as i64)
                        )
                    })
                    .collect();
                println!(
                    "\n  verdicts on narrators: {} — in the Imami terms: {}, no judgment {}",
                    py::thousands(cr.verdicts as i64),
                    ranks.join(", "),
                    py::thousands(cr.unranked as i64)
                );
                if !cr.creeds.is_empty() {
                    let creeds: Vec<String> = cr
                        .creeds
                        .iter()
                        .map(|(label, n)| {
                            format!(
                                "{} {}",
                                i18n::t(&lang, label, &[]),
                                py::thousands(*n as i64)
                            )
                        })
                        .collect();
                    println!("  creed, where stated: {}", creeds.join(", "));
                }
            } else {
                let ranks: Vec<String> = cr
                    .ranks
                    .iter()
                    .map(|(r, _, n)| format!("{r}:{n}"))
                    .collect();
                println!(
                    "\n  verdicts on narrators: {} — {}",
                    py::thousands(cr.verdicts as i64),
                    ranks.join(", ")
                );
            }
        }
        Ok(())
    };
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(1)
        }
    }
}

/// iy-rs life NAME: the life of a Companion, a narrator or a scholar (BIO2), as `iy life` prints it.
fn life_cmd(a: LifeArgs) -> ExitCode {
    let ui = i18n::ui_language();
    let lang = a.lang.clone().unwrap_or_else(|| ui.clone());
    let langs = narrators::Langs {
        reading: &ui,
        words: &lang,
    };
    let text = a.name.join(" ");
    let mut found: Vec<(lives::Life, bool)> = lives::find(&text)
        .into_iter()
        .map(|w| (lives::view(w, &lang), true))
        .collect();
    if found.is_empty() {
        let conn = match open() {
            Ok(c) => c,
            Err(code) => return code,
        };
        let rows = match narrators::find(&conn, &text, a.tradition.as_deref(), langs) {
            Ok((rows, _)) => rows,
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::from(1);
            }
        };
        for r in rows.iter().take(a.limit) {
            if let (Some(info), written) = lives::life(&conn, r.id, langs) {
                found.push((info, written));
            }
        }
    }
    if found.is_empty() {
        eprintln!("No one named '{text}' found.");
        return ExitCode::from(1);
    }
    let width = 100;
    for (info, written) in found.iter().take(a.limit) {
        println!(
            "\n{}  —  {}{}",
            info.title,
            info.kind,
            if *written { "" } else { "  (from the data)" }
        );
        for (label, value) in &info.facts {
            println!(
                "{}",
                py::fill_with(value, width, &format!("  {label}: "), "    ")
            );
        }
        for paragraph in &info.paragraphs {
            println!();
            println!("{}", py::fill(paragraph, width, "  "));
        }
        if !info.sources.is_empty() {
            println!();
            println!(
                "{}",
                py::fill_with(
                    &info.sources.join("; "),
                    width,
                    &format!("  {}: ", i18n::t(&lang, "Sources", &[])),
                    "    "
                )
            );
        }
        for r in &info.read {
            match (&r.url, &r.book) {
                (Some(url), _) => println!("  → {}: {url}", r.text),
                (None, book) => println!(
                    "  → {} (iy book {})",
                    r.text,
                    book.as_deref().unwrap_or("None")
                ),
            }
        }
        for e in &info.entries {
            println!(
                "\n  {}: {}, {}",
                i18n::t(&lang, "In the source", &[]),
                e.title,
                e.place
            );
            if a.source {
                if !lives::available(&e.work) {
                    eprintln!(
                        "  … {} (≈{} MB, once)",
                        i18n::t(&lang, "Downloading", &[]),
                        lives::work_size(&e.work)
                    );
                    if let Err(err) = lives::fetch(&e.work, &|m| eprintln!("  … {m}")) {
                        eprintln!("{err}");
                        return ExitCode::from(1);
                    }
                }
                if let Some((heading, body)) = lives::entry(&e.work, e.volume, e.number) {
                    println!("  {heading}\n");
                    println!("{body}");
                }
            } else {
                println!("  (iy life ... --source prints it in the author's own words)");
            }
        }
    }
    ExitCode::SUCCESS
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

/// iy-rs search --in narrators|scholars: the same people search as the Search page (S3, `cli._search_people`).
fn search_people(
    conn: &db::Connection,
    a: &SearchArgs,
    text: &str,
) -> rusqlite_conn::Result<ExitCode> {
    if a.kind == "scholars" {
        let found = scholars::search(conn, text);
        println!(
            "{} scholar{} found",
            found.len(),
            if found.len() != 1 { "s" } else { "" }
        );
        for f in found.iter().skip(a.offset).take(a.limit) {
            let s = f.scholar;
            let roles = if f.roles.is_empty() {
                "nothing of his imported yet".to_string()
            } else {
                f.roles.join(", ")
            };
            let arabic = if s.known_ar.is_empty() {
                &s.arabic
            } else {
                &s.known_ar
            };
            println!(
                "\n{}  {arabic}  ({}){}",
                s.name,
                s.id,
                if f.close { "  ≈ close spelling" } else { "" }
            );
            if s.dates.is_empty() {
                println!("  {}", s.tr);
            } else {
                println!("  {} · {}", s.tr, s.dates);
            }
            println!("  roles here: {roles}");
            println!("  works: {}", s.works.join(", "));
        }
        return Ok(ExitCode::SUCCESS);
    }
    let reading = i18n::ui_language();
    let langs = narrators::Langs {
        reading: &reading,
        words: "en",
    };
    let road = a.tradition.as_deref();
    let (mut rows, total) = narrators::find(conn, text, road, langs)?;
    if a.offset + a.limit > rows.len() && total > rows.len() {
        let have: Vec<i64> = rows.iter().map(|r| r.id).collect();
        let wanted = a.offset + a.limit - rows.len();
        let more = narrators::find_more(conn, text, road, narrators::SEARCH_RERANK, wanted, langs)?;
        rows.extend(more.into_iter().filter(|r| !have.contains(&r.id)));
    }
    let more = if total > narrators::SEARCH_RERANK {
        format!(
            " (the closest {} are ranked first; --offset and --limit reach the others)",
            narrators::SEARCH_RERANK
        )
    } else {
        String::new()
    };
    println!(
        "{total} narrator{} found{more}",
        if total != 1 { "s" } else { "" }
    );
    let t = tables::get();
    for r in rows.iter().skip(a.offset).take(a.limit) {
        let view = &r.view;
        let known = r
            .famous
            .clone()
            .filter(|f| !f.is_empty())
            .unwrap_or_else(|| {
                if view.reading.is_empty() {
                    view.full_reading.clone()
                } else {
                    view.reading.clone()
                }
            });
        let arabic = if view.arabic.is_empty() {
            &r.name
        } else {
            &view.arabic
        };
        let mark = if r.weak {
            "  ≈ same consonants, other vowels"
        } else if r.close {
            "  ≈ close spelling"
        } else if r.unread {
            "  ≈ same consonants, reading unknown"
        } else {
            ""
        };
        let head = if known.is_empty() {
            String::new()
        } else {
            format!("{known}  ")
        };
        println!("\n{head}{arabic}  (id {}, {}){mark}", r.id, r.tradition);
        if !view.full_reading.is_empty() && view.full_reading != known {
            println!("  {}", view.full_reading);
        }
        let mut facts: Vec<String> = Vec::new();
        if r.tradition == "shia" {
            facts.push(match r.rank.filter(|x| *x != 0) {
                Some(k) => t
                    .rijal
                    .shia_ranks
                    .get(&k.to_string())
                    .cloned()
                    .unwrap_or_default(),
                None => "no judgment".into(),
            });
            if let Some(m) = r.madhhab.as_deref().filter(|m| !m.is_empty()) {
                facts.push(
                    t.narrators
                        .madhhabs
                        .get(m)
                        .cloned()
                        .unwrap_or_else(|| m.to_string()),
                );
            }
        } else if let Some(k) = r.rank.filter(|x| *x != 0) {
            facts.push(format!(
                "rank {k}: {}",
                t.rijal
                    .ranks
                    .get(&k.to_string())
                    .cloned()
                    .unwrap_or_default()
            ));
        }
        if let Some(tb) = r.tabaqa.filter(|x| *x != 0) {
            facts.push(format!("tabaqa {tb}"));
        }
        if let Some(d) = r.death.filter(|x| *x != 0) {
            facts.push(format!("d. {d} AH"));
        }
        facts.push(format!("{} in chains", r.in_chains));
        println!("  {}", facts.join(" · "));
    }
    Ok(ExitCode::SUCCESS)
}

/// iy search --mode meaning (YZ1a): the hadith closest in meaning, across languages (`cli._search_meaning`).
fn search_meaning(a: &SearchArgs, text: &str) -> ExitCode {
    let conn = match open() {
        Ok(c) => c,
        Err(code) => return code,
    };
    let mut q = search::SearchQuery::new(text);
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
    let st = isnady_core::semantic::status(&conn);
    let page = match isnady_core::semantic::search(&conn, &q, 0.10) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let stale = if st["stale"] == serde_json::json!(true) {
        "  (the meaning index is older than the data: iy ai build)"
    } else {
        ""
    };
    println!(
        "{} hadith by meaning in {} ms{stale}",
        page.total, page.elapsed_ms
    );
    if !page.unknown_words.is_empty() {
        println!("  no imported text uses: {}", page.unknown_words.join(", "));
    }
    for (r, score) in page.results.iter().zip(&page.scores) {
        println!(
            "\n{} #{}   meaning {:.2}",
            r.collection_name, r.number, score
        );
        for t in &r.texts {
            let body = t.text.split_whitespace().collect::<Vec<_>>().join(" ");
            let n = body.chars().count();
            let head: String = body.chars().take(a.width).collect();
            println!(
                "  [{}] {head}{}",
                t.language,
                if n > a.width { " ..." } else { "" }
            );
        }
    }
    ExitCode::SUCCESS
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
        let conn = match open() {
            Ok(c) => c,
            Err(code) => return code,
        };
        return match search_people(&conn, &a, &text) {
            Ok(code) => code,
            Err(e) => {
                eprintln!("{e}");
                ExitCode::from(1)
            }
        };
    }
    if a.mode == "meaning" {
        return search_meaning(&a, &text);
    }
    // every text indexed first, as iy search does (search.ensure_index)
    match isnady_core::sources::store::connect_rw(None) {
        Ok(rw) => {
            if let Err(e) = search::ensure_index(&rw, &|m| eprintln!("{m}")) {
                eprintln!("{e}");
                return ExitCode::from(1);
            }
        }
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(1);
        }
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

/// `iy isnads`: the chains read from the Arabic texts (every one again with --rebuild), how far the chains of every
/// book were read and why the rest were kept whole; then how many of their narrators are identified.
/// `iy guess [on|off]` (cmd_guess): isnady's own inference about who a narrator is — off unless turned on.
fn guess_cmd(state: Option<String>) -> ExitCode {
    let conn = match isnady_core::sources::store::connect_rw(None) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(1);
        }
    };
    let say = |m: &str| println!("{m}");
    let loaded = narrators::loaded(&conn);
    let done = match state.as_deref() {
        Some(s) => {
            if !loaded {
                println!("No rijal work imported: there are no narrators to identify.");
                return ExitCode::from(1);
            }
            isnady_core::reader::matcher::set_guesses(&conn, s == "on", &say).map(|_| ())
        }
        None if loaded => {
            isnady_core::reader::matcher::link_narrators(&conn, &say, false).map(|_| ())
        }
        None => Ok(()),
    };
    if let Err(e) = done {
        eprintln!("{e}");
        return ExitCode::from(1);
    }
    let on = isnady_core::reader::matcher::guesses_on(&conn);
    let (total, found, guessed): (i64, i64, i64) = conn
        .query_row(
            "SELECT COUNT(*), COALESCE(SUM(person_id IS NOT NULL), 0), \
             COALESCE(SUM(person_id IS NOT NULL AND match_confidence < 0.8), 0) FROM isnad_links",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap_or((0, 0, 0));
    if on {
        println!(
            "Narrator guesses (isnady's own inference): on — narrators are identified by the rijal works' own \
             words and by isnady's own rules, each of these marked as isnady's guess."
        );
    } else {
        println!(
            "Narrator guesses (isnady's own inference): off — narrators are identified by the rijal works' own \
             words alone (turn on: iy guess on)."
        );
    }
    if total > 0 {
        println!(
            "  {} of {} links identified ({:.1}%), {} of them isnady's guesses.",
            isnady_core::py::thousands(found),
            isnady_core::py::thousands(total),
            100.0 * found as f64 / total as f64,
            isnady_core::py::thousands(guessed)
        );
    }
    ExitCode::SUCCESS
}

fn isnads_cmd(rebuild: bool, dump: Option<String>, limit: Option<usize>) -> ExitCode {
    if let Some(path) = dump {
        let conn = match open() {
            Ok(c) => c,
            Err(code) => return code,
        };
        let mut f = match std::fs::File::create(&path) {
            Ok(f) => std::io::BufWriter::new(f),
            Err(e) => {
                eprintln!("{path}: {e}");
                return ExitCode::from(1);
            }
        };
        return match isnady_core::reader::store::dump(&conn, &mut f, limit) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("{e}");
                ExitCode::from(1)
            }
        };
    }
    {
        let conn = match isnady_core::sources::store::connect_rw(None) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::from(1);
            }
        };
        let say = |m: &str| println!("{m}");
        match isnady_core::reader::store::ensure_isnads(&conn, &say, rebuild) {
            Ok(stats) => {
                if stats.parsed > 0 || stats.raw_only > 0 {
                    println!("Read {} chains now.", stats.parsed + stats.raw_only);
                }
            }
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::from(1);
            }
        }
    }
    let conn = match open() {
        Ok(c) => c,
        Err(code) => return code,
    };
    let run = || -> rusqlite_conn::Result<()> {
        println!("Parser: {}", isnady_core::tables::get().parser_id);
        for b in isnad::book_stats(&conn)? {
            let shia = b.tradition == "shia";
            let end = if shia {
                "a Ma'sum (the Prophet or an Imam)"
            } else {
                "the Prophet"
            };
            println!(
                "  {}{}: {} chains, {} split into narrators ({:.1}%), {} reach {end}, {} links",
                works::book_title(&b.key, &b.stored_name, "en"),
                if shia { " (Shia)" } else { "" },
                b.chains,
                b.split,
                100.0 * b.split as f64 / b.chains as f64,
                b.marfu,
                b.links
            );
            for (problem, count) in &b.problems {
                println!("    {count:6}  not split: {problem}");
            }
        }
        if narrators::loaded(&conn) {
            // as cmd_isnads: the narrators identified after the report of the chains
            match isnady_core::sources::store::connect_rw(None) {
                Ok(rw) => {
                    if let Err(e) = isnady_core::reader::matcher::link_narrators(
                        &rw,
                        &|m| println!("{m}"),
                        false,
                    ) {
                        eprintln!("{e}");
                    }
                }
                Err(e) => eprintln!("{e}"),
            }
            if !narrators::links_ready(&conn) {
                eprintln!(
                    "The narrators of the chains are not identified yet. The Python isnady does it once: run iy \
                     isnads, then iy-rs shows them."
                );
                return Ok(());
            }
            for (book, total, found) in isnad::identified_by_book(&conn)? {
                println!(
                    "  {book}: {found} of {total} narrators in chains identified ({:.1}%)",
                    100.0 * found as f64 / total as f64
                );
            }
        }
        Ok(())
    };
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(1)
        }
    }
}

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
    if !isnad::isnads_ready(conn) {
        // the chains read once (core_isnad.ensure_isnads), on a connection that writes
        match isnady_core::sources::store::connect_rw(None) {
            Ok(rw) => {
                if let Err(e) = isnady_core::reader::store::ensure_isnads(&rw, &|_| {}, false) {
                    eprintln!("{e}");
                    return Ok(ExitCode::from(1));
                }
            }
            Err(e) => {
                eprintln!("{e}");
                return Ok(ExitCode::from(1));
            }
        }
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
    if narrators::loaded(conn) && !narrators::links_ready(conn) {
        if let Ok(rw) = isnady_core::sources::store::connect_rw(None) {
            if let Err(e) = isnady_core::reader::matcher::link_narrators(&rw, &|_| {}, false) {
                eprintln!("{e}");
            }
        }
    }
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
                    Some(c) if c >= 0.7 => "  [isnady's guess: its own rules]",
                    Some(c) if c >= 0.6 => "  [isnady's guess: the narrators around him]",
                    Some(_) => "  [isnady's guess: the famous one of this name]",
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
    // the tree of every hadith collection built, or built again when it changed (works.ensure_collection_works)
    if !works::ready(&conn) {
        let built = isnady_core::sources::store::connect_rw(None)
            .and_then(|rw| works::ensure_collection_works(&rw).map_err(|e| e.to_string()));
        if let Err(e) = built {
            eprintln!("{e}");
            return ExitCode::from(1);
        }
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
