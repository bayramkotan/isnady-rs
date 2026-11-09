//! The chains written to the database (`ensure_isnads`, `_borrow_previous`, the stamp of core/isnad.py): the same
//! rows in the same order as the Python isnady writes them.

use rusqlite::{params, Connection, OptionalExtension};
use std::collections::{BTreeMap, HashMap};
use std::time::Instant;

use super::pyre;
use super::sunni::{cited_companion, complete_from_before, rescue_sunni};
use super::*;

pub fn parser_id() -> &'static str {
    &tables().parser_id
}

fn stamp(conn: &Connection) -> rusqlite::Result<String> {
    let row: [Option<i64>; 6] = conn.query_row(
        "SELECT (SELECT COUNT(*) FROM hadiths), (SELECT MAX(id) FROM hadiths), (SELECT COUNT(*) FROM texts), \
         (SELECT MAX(id) FROM texts), (SELECT COUNT(*) FROM isnads), (SELECT MAX(hadith_id) FROM isnads)",
        [],
        |r| Ok([r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?]),
    )?;
    let parts: Vec<String> = row
        .iter()
        .map(|v| v.map(|x| x.to_string()).unwrap_or_else(|| "None".into()))
        .collect();
    Ok(format!("{}|{}", parser_id(), parts.join("|")))
}

/// `isnads_ready`: every chain read by this parser, known without reading the texts.
pub fn isnads_ready(conn: &Connection) -> bool {
    let row: Option<String> = conn
        .query_row(
            "SELECT value FROM core_meta WHERE key = 'isnads_ready'",
            [],
            |r| r.get(0),
        )
        .optional()
        .ok()
        .flatten();
    matches!((row, stamp(conn)), (Some(a), Ok(b)) if a == b)
}

#[derive(Debug, Default)]
pub struct Stats {
    pub todo: usize,
    pub parsed: i64,
    pub reaches_prophet: i64,
    pub raw_only: i64,
    pub links: i64,
    pub problems: BTreeMap<String, i64>,
    pub borrowed: i64,
}

fn is_whole(c: &Parsed) -> bool {
    !c.links.is_empty()
        && c.problem.is_empty()
        && (c.reaches_prophet || !c.ends_with.is_empty() || is_companion(c.links.last().unwrap()))
}

/// The roads of one hadith as `_ensure_isnads` writes them (the book's ways of giving no chain included).
/// The second value: the whole chain the next hadith of the book may go on from (Sunni books).
pub fn chains_of(
    text: &str,
    tradition: &str,
    book_key: &str,
    before: Option<&Parsed>,
) -> (Vec<Parsed>, Option<Parsed>) {
    let shia = tradition == "shia";
    let mut chains = parse_all(text, Some(tradition));
    let mut whole = None;
    if !shia {
        complete_from_before(&mut chains, text, before);
        whole = chains.iter().find(|c| is_whole(c)).cloned();
    }
    let no_links = chains[0].links.is_empty();
    let pr = chains[0].problem.clone();
    if no_links
        && [
            problem("no_term"),
            "no narrator found before the text",
            problem("no_narrator"),
            problem("no_chain"),
        ]
        .contains(&pr.as_str())
    {
        let head_n = pyre::upto(&norm_name(text), 200).to_string();
        if tables().direct_books.contains(book_key) {
            chains[0] = Parsed {
                problem: problem("direct").into(),
                reaches_prophet: true,
                ends_with: "prophet".into(),
                ..Default::default()
            };
        } else if tables().citing_books.contains(book_key) {
            match cited_companion(text) {
                Some(named) => chains[0] = named,
                None => chains[0].problem = problem("no_chain").into(),
            }
        } else if !shia && p("_SAME_CHAIN_OLD_RE").is_match(&head_n) {
        } else if !shia && pre!(r"[\s\d.\-:]*(?:(?:قال|قالت)\s*:?\s*)?(?:و|ثم|ف)").is_match(&head_n) && !p("_FROM_HIM_RE").is_match(&head_n) {
            chains[0] = Parsed::with_problem(problem("continued"));
        } else if !shia && rescue_sunni(text).is_some() {
            chains = rescue_sunni(text).unwrap();
        } else if !shia
            && pre!(r"[\s\d.\-:]*قال (?:ابو عيسي|ابو داود|ابو عبد الرحمن|ابو عبد الله|محمد بن اسماعيل)(?!\w)").is_match(&head_n)
            && !pre!(r"(?<!\w)و?(?:بهذا الاسناد|بهذا الحديث|باسناده|بمثله|مثله|بنحوه|نحوه|بمعناه)(?!\w)").is_search(pyre::upto(&head_n, 80))
        {
            chains[0] = Parsed::with_problem(problem("note"));
        } else if !shia && (pre!(r"[\s\d.\-:]*(?:قال|قالت|قالا|حدثني|سمعت|عن)(?!\w)").is_match(&head_n) || tables().split_books.contains(book_key)) {
            chains[0] = Parsed::with_problem(problem("continued"));
        } else if !shia && !p("_FROM_HIM_RE").is_match(&head_n) {
            chains[0] = Parsed::with_problem(problem("no_chain"));
        }
    }
    if shia
        && chains[0].links.is_empty()
        && [
            "no narrator found before the text",
            "no chain: a name without a transmission after it",
            problem("no_term"),
        ]
        .contains(&chains[0].problem.as_str())
    {
        chains[0] = shia::shia_without_chain(text, &chains[0]);
    }
    if shia
        && chains[0].links.is_empty()
        && chains[0].problem == "refers to the chain of an earlier hadith (bi-isnadihi); not split"
    {
        let head_n = pyre::upto(&norm_name(text), 120).to_string();
        if let Some(man) = pre!(
            r"[\s\d.\-]*(?:و)?(?:روي|روى|قال)\s+([^\s،,]{2,20}(?:\s+بن\s+[^\s،,]{2,20})*)\s+باسناده"
        )
        .match_(&head_n)
        {
            chains[0] = Parsed {
                raw_isnad: man.g(1).into(),
                links: vec![Link::new(1, "qala", man.g(1), false)],
                problem: problem("no_chain").into(),
                ..Default::default()
            };
        } else if !pre!(r"[\s\d./\-]*(?:و)?(?:بهذا|باسناده|باسناد|بالاسناد|مثله)(?!\w)")
            .is_match(&head_n)
        {
            chains[0] = Parsed::with_problem(
                if pre!(r"[\s\d./\-]*(?:قلنا|فان قيل|قيل)(?!\w)").is_match(&head_n) {
                    problem("note")
                } else {
                    problem("no_chain")
                },
            );
        }
    }
    (chains, whole)
}

fn opt(s: &str) -> Option<&str> {
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// `ensure_isnads`: the Arabic text of every hadith with no derived chain yet, read.
pub fn ensure_isnads(
    conn: &Connection,
    say: &dyn Fn(&str),
    rebuild: bool,
) -> rusqlite::Result<Stats> {
    if !rebuild && isnads_ready(conn) {
        return Ok(Stats::default());
    }
    let stats = read_all(conn, say, rebuild)?;
    conn.execute(
        "INSERT OR REPLACE INTO core_meta (key, value) VALUES ('isnads_ready', ?)",
        [stamp(conn)?],
    )?;
    Ok(stats)
}

fn read_all(conn: &Connection, say: &dyn Fn(&str), rebuild: bool) -> rusqlite::Result<Stats> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS core_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
        [],
    )?;
    let row: Option<String> = conn
        .query_row(
            "SELECT value FROM core_meta WHERE key = 'isnad_parser'",
            [],
            |r| r.get(0),
        )
        .optional()?;
    let tx = conn.unchecked_transaction()?;
    let only = if rebuild {
        None
    } else {
        row.as_ref().and_then(|r| tables().partial_upgrades.get(r))
    };
    if let Some(only) = only {
        tx.execute(
            &format!(
                "DELETE FROM isnads WHERE derived_by IS NOT NULL AND hadith_id IN \
                 (SELECT hadith_id FROM isnads WHERE derived_by IS NOT NULL AND ({only}))"
            ),
            [],
        )?;
    } else if rebuild || row.as_deref().is_some_and(|r| r != parser_id()) {
        say("Isnad rules changed; parsing every chain again");
        tx.execute("DELETE FROM isnads WHERE derived_by IS NOT NULL", [])?;
    }
    tx.execute(
        "INSERT OR REPLACE INTO core_meta (key, value) VALUES ('isnad_parser', ?)",
        [parser_id()],
    )?;
    let todo: Vec<Todo> = {
        let mut stmt = tx.prepare(
            "SELECT h.id, t.text, e.source_id, c.tradition, c.key, h.collection_id FROM hadiths h
             JOIN collections c ON c.id = h.collection_id
             JOIN texts t ON t.hadith_id = h.id
             JOIN editions e ON e.id = t.edition_id
             WHERE e.language = 'Arabic'
               AND h.id NOT IN (SELECT hadith_id FROM isnads WHERE derived_by IS NOT NULL)
             GROUP BY h.id ORDER BY h.collection_id, h.number_sort, h.id",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
            ))
        })?;
        rows.collect::<Result<_, _>>()?
    };
    let mut stats = Stats {
        todo: todo.len(),
        ..Default::default()
    };
    if todo.is_empty() {
        tx.commit()?;
        return Ok(stats);
    }
    say(&format!("Reading isnads of {} hadith", todo.len()));
    let started = Instant::now();
    let mut no_links: Vec<i64> = vec![];
    let read = read_parallel(&todo);
    {
        let mut ins = tx.prepare(
            "INSERT INTO isnads (hadith_id, ordinal, raw_text, source_id, derived_by, reaches_prophet, problem, ends_with, raised)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )?;
        let mut link_ins = tx.prepare("INSERT INTO isnad_links (isnad_id, position, raw_name, transmission) VALUES (?, ?, ?, ?)")?;
        for ((hadith_id, _text, source_id, _tradition, _book_key, _collection_id), chains) in
            todo.iter().zip(read)
        {
            for (ordinal, c) in chains.iter().enumerate() {
                ins.execute(params![
                    hadith_id,
                    ordinal as i64 + 1,
                    opt(&c.raw_isnad),
                    source_id,
                    parser_id(),
                    c.reaches_prophet as i64,
                    opt(&c.problem),
                    opt(&c.ends_with),
                    c.raised as i64
                ])?;
                let isnad_id = tx.last_insert_rowid();
                for l in &c.links {
                    link_ins.execute(params![isnad_id, l.position as i64, l.raw_name, l.term])?;
                }
            }
            let c = &chains[0];
            if c.links.is_empty() {
                no_links.push(*hadith_id);
                stats.raw_only += 1;
                *stats.problems.entry(c.problem.clone()).or_default() += 1;
            } else {
                stats.parsed += 1;
                stats.links += c.links.len() as i64;
                stats.reaches_prophet += c.reaches_prophet as i64;
            }
        }
    }
    no_links.sort();
    stats.borrowed = borrow_previous(&tx, &no_links)?;
    tx.commit()?;
    crate::log::info(
        "chains",
        &format!(
            "✔ {} chains read by {} in {:.1} s: {} with links, {} reaching the Prophet, {} kept whole",
            crate::py::thousands(todo.len() as i64),
            parser_id(),
            started.elapsed().as_secs_f64(),
            crate::py::thousands(stats.parsed),
            crate::py::thousands(stats.reaches_prophet),
            crate::py::thousands(stats.raw_only)
        ),
    );
    Ok(stats)
}

type Todo = (
    i64,
    Option<String>,
    Option<i64>,
    Option<String>,
    String,
    i64,
);

/// The roads of every hadith to read, the books shared out among the processor's cores (a book is read in its
/// order, each hadith after the one before: "بهذا الإسناد" goes on from it); in the order of `todo`.
fn read_parallel(todo: &[Todo]) -> Vec<Vec<Parsed>> {
    let mut books: Vec<(i64, Vec<usize>)> = vec![];
    for (k, row) in todo.iter().enumerate() {
        match books.iter_mut().find(|b| b.0 == row.5) {
            Some(b) => b.1.push(k),
            None => books.push((row.5, vec![k])),
        }
    }
    books.sort_by_key(|b| std::cmp::Reverse(b.1.len()));
    let workers = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .clamp(1, 8);
    let next = std::sync::atomic::AtomicUsize::new(0);
    let out: std::sync::Mutex<Vec<Option<Vec<Parsed>>>> =
        std::sync::Mutex::new(vec![None; todo.len()]);
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| loop {
                let b = next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let Some((_, rows)) = books.get(b) else {
                    break;
                };
                let mut before: Option<Parsed> = None;
                let mut mine = vec![];
                for &k in rows {
                    let (_, text, _, tradition, book_key, _) = &todo[k];
                    let (chains, whole) = chains_of(
                        text.as_deref().unwrap_or(""),
                        tradition.as_deref().unwrap_or(""),
                        book_key,
                        before.as_ref(),
                    );
                    if whole.is_some() {
                        before = whole;
                    }
                    mine.push((k, chains));
                }
                let mut o = out.lock().unwrap();
                for (k, c) in mine {
                    o[k] = Some(c);
                }
            });
        }
    });
    out.into_inner()
        .unwrap()
        .into_iter()
        .map(|c| c.unwrap_or_default())
        .collect()
}

/// `_borrow_previous`: "وعنه", "وبهذا الإسناد": the chain of the hadith before, drawn for it (BORROWED).
fn borrow_previous(conn: &Connection, hadith_ids: &[i64]) -> rusqlite::Result<i64> {
    let mut done = 0;
    for hid in hadith_ids {
        let row: Option<(i64, Option<f64>, Option<String>)> = conn
            .query_row(
                "SELECT h.collection_id, h.number_sort, t.text FROM hadiths h JOIN texts t ON t.hadith_id = h.id \
                 JOIN editions e ON e.id = t.edition_id WHERE h.id = ? AND e.language = 'Arabic'",
                [hid],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        let Some((collection_id, number_sort, text)) = row else {
            continue;
        };
        let text = text.unwrap_or_default();
        let norm = pyre::upto(&norm_name(&text), 80).to_string();
        let whole = p("_SAME_CHAIN_OLD_RE").is_match(&norm);
        if !whole && !p("_FROM_HIM_RE").is_match(&norm) {
            continue;
        }
        let prev: Option<(i64, i64, Option<String>)> = conn
            .query_row(
                "SELECT i.id, i.reaches_prophet, i.ends_with FROM hadiths h JOIN isnads i ON i.hadith_id = h.id
                 WHERE h.collection_id = ? AND h.number_sort < ? AND i.ordinal = 1
                   AND EXISTS (SELECT 1 FROM isnad_links l WHERE l.isnad_id = i.id)
                 ORDER BY h.number_sort DESC LIMIT 1",
                params![collection_id, number_sort],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        let Some(mut prev) = prev else {
            continue;
        };
        let mut links: Vec<(i64, String, String)> = {
            let mut stmt = conn.prepare("SELECT position, raw_name, transmission FROM isnad_links WHERE isnad_id = ? ORDER BY position")?;
            let rows = stmt.query_map([prev.0], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
            rows.collect::<Result<_, _>>()?
        };
        if !whole {
            let last = links.pop();
            links = last.into_iter().collect();
        } else {
            let phrase = p("_SAME_CHAIN_START_RE").match_(&norm_name(&text));
            let rest = match &phrase {
                Some(ph) => pyre::from(&text, orig_len(&text, ph.end())).to_string(),
                None => String::new(),
            };
            let tradition: Option<String> = conn
                .query_row(
                    "SELECT tradition FROM collections WHERE id = ?",
                    [collection_id],
                    |r| r.get(0),
                )
                .optional()?
                .flatten();
            let shia = tradition.as_deref() == Some("shia");
            let mut own_read = None;
            if pre!(r"[\s،,:]*عن(?!\w)").is_match(&norm_name(&rest)) {
                let r = rest.trim_start_matches([' ', '،', ',', ':']);
                own_read = Some(if shia {
                    shia::parse_shia(&format!("فلان {r}"))
                } else {
                    sunni::parse_sunni(&format!("حدثنا فلان {r}"), 0)
                });
            }
            let own: Vec<Link> = own_read
                .as_ref()
                .map(|o| o.links.iter().skip(1).cloned().collect())
                .unwrap_or_default();
            if let Some(o) = &own_read {
                if !own.is_empty() && !o.ends_with.is_empty() {
                    prev = (prev.0, o.reaches_prophet as i64, Some(o.ends_with.clone()));
                }
            }
            if !own.is_empty() {
                let at = links.iter().position(|r| shared(&r.1, &own[0].raw_name));
                let mut v: Vec<(i64, String, String)> = match at {
                    Some(a) => links[..a].to_vec(),
                    None => vec![],
                };
                v.extend(own.iter().map(|l| (0, l.raw_name.clone(), l.term.clone())));
                links = v;
            }
        }
        let mine: Option<i64> = conn
            .query_row(
                "SELECT id FROM isnads WHERE hadith_id = ? AND ordinal = 1",
                [hid],
                |r| r.get(0),
            )
            .optional()?;
        let Some(mine) = mine else {
            continue;
        };
        let has: Option<i64> = conn
            .query_row(
                "SELECT 1 FROM isnad_links WHERE isnad_id = ?",
                [mine],
                |r| r.get(0),
            )
            .optional()?;
        if has.is_some() {
            continue;
        }
        conn.execute(
            "UPDATE isnads SET problem = ?, reaches_prophet = ?, ends_with = ? WHERE id = ?",
            params![problem("borrowed"), prev.1, prev.2, mine],
        )?;
        for (i, r) in links.iter().enumerate() {
            conn.execute(
                "INSERT INTO isnad_links (isnad_id, position, raw_name, transmission) VALUES (?, ?, ?, ?)",
                params![mine, i as i64 + 1, r.1, r.2],
            )?;
        }
        done += 1;
    }
    Ok(done)
}

/// One line per road, for comparing the reader with the Python isnady's (`iy-rs isnads --dump`).
pub fn dump(
    conn: &Connection,
    out: &mut dyn std::io::Write,
    limit: Option<usize>,
) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare(
        "SELECT h.id, t.text, c.tradition, c.key, h.collection_id FROM hadiths h
         JOIN collections c ON c.id = h.collection_id
         JOIN texts t ON t.hadith_id = h.id
         JOIN editions e ON e.id = t.edition_id
         WHERE e.language = 'Arabic'
         GROUP BY h.id ORDER BY h.collection_id, h.number_sort, h.id",
    )?;
    let rows: Vec<(i64, Option<String>, Option<String>, String, i64)> = stmt
        .query_map([], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })?
        .collect::<Result<_, _>>()?;
    let mut before: HashMap<i64, Parsed> = HashMap::new();
    for (k, (hid, text, tradition, key, cid)) in rows.into_iter().enumerate() {
        if limit.is_some_and(|l| k >= l) {
            break;
        }
        let text = text.unwrap_or_default();
        let tradition = tradition.unwrap_or_default();
        let (chains, whole) = chains_of(&text, &tradition, &key, before.get(&cid));
        if let Some(w) = whole {
            before.insert(cid, w);
        }
        for (i, c) in chains.iter().enumerate() {
            let links: Vec<String> = c
                .links
                .iter()
                .map(|l| format!("{}={}", l.term, l.raw_name))
                .collect();
            let _ = writeln!(
                out,
                "{hid}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                i + 1,
                c.reaches_prophet as i64,
                c.problem,
                c.ends_with,
                c.raised as i64,
                c.raw_isnad.replace(['\n', '\t'], " "),
                links.join(" | ").replace(['\n', '\t'], " ")
            );
        }
    }
    Ok(())
}
