//! iy-rs — isnady's command line in Rust. Every command prints what the Python `iy` prints for the same command, so
//! that tools/parity.py can compare the two word for word. A command or option not ported yet says so and points to
//! the Python `iy`; PARITY.md lists them.

use clap::{Parser, Subcommand};
use isnady_core::{
    db, grades, huffaz, i18n, isnad, lives, narrators, paths, people, py, readmarks, sahaba,
    scholars, search, tables, works, VERSION,
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
    /// The chains of transmission read from the Arabic texts: the report of every book
    Isnads {
        /// (not ported yet: reading every chain again — use iy isnads --rebuild)
        #[arg(long)]
        rebuild: bool,
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
            println!("isnady-rs {VERSION} — iy-rs search WORDS, iy-rs chain BOOK NUMBER, iy-rs isnads, iy-rs book, iy-rs narrator NAME, iy-rs life NAME, iy-rs scholar, iy-rs group, iy-rs sources, iy-rs datadir, iy-rs --help");
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
        Some(Command::Isnads { rebuild }) => isnads_cmd(rebuild),
        Some(Command::Book(a)) => book_cmd(a),
        Some(Command::Narrator(a)) => narrator_cmd(a),
        Some(Command::Life(a)) => life_cmd(a),
        Some(Command::Scholar(a)) => scholar_cmd(a),
        Some(Command::Group(a)) => group_cmd(a),
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
            return not_yet(
                "Al-Dhahabi's entry in his own words",
                "iy scholar --huffaz NAME --source",
            );
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
                    not_yet(
                        &format!("Downloading {}", e.title),
                        &format!("iy life {text} --source (once; then iy-rs reads it too)"),
                    );
                    continue;
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

/// `iy isnads`: how far the chains of every book were read, and why the rest were kept whole; then how many of their
/// narrators are identified. Reading them is the Python isnady's (the parser stays there for now).
fn isnads_cmd(rebuild: bool) -> ExitCode {
    if rebuild {
        return not_yet("Reading every chain again", "iy isnads --rebuild");
    }
    let conn = match open() {
        Ok(c) => c,
        Err(code) => return code,
    };
    if !isnad::isnads_ready(&conn) {
        eprintln!(
            "The chains of the books imported last are not read yet. The Python isnady reads them once, for every \
             book: run iy isnads (or open the Isnad Chains page), then iy-rs shows them."
        );
        return ExitCode::from(1);
    }
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
