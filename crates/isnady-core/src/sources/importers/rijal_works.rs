//! The rijal works in OpenITI mARkdown: Ibn Hajar's Taqrib al-Tahdhib (`openiti_taqrib.py`, format `taqrib`) and
//! the works read by profile, al-Najashi's Rijal first (`openiti_rijal.py`, format `najashi`). Each numbered entry
//! becomes a person with the critic's words and their reading; the book itself is readable in Books.

use rusqlite::{params, Connection};
use std::collections::{HashMap, HashSet};

use super::{ImportError, ImportReport, SourceInfo};
use crate::normalize::normalize;
use crate::pre;
use crate::reader::pyre::{self, Ps};
use crate::rijal;
use crate::sources::{now_iso, read_bytes_with, tier_for_license, ResourceError};

const CRITIC: &str = "Ibn Hajar al-'Asqalani";
const WORK: &str = "Taqrib al-Tahdhib";
const SCHEME: &str = "ibn-hajar-taqrib";

fn read_text(source: &SourceInfo, timeout: Option<u64>) -> Result<String, ResourceError> {
    let bytes = read_bytes_with(&source.location, &source.auth, timeout)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| ResourceError(format!("Not UTF-8 text: {}", source.location)))?;
    Ok(text.strip_prefix('\u{feff}').unwrap_or(&text).to_string())
}

/// `pattern.split(s, maxsplit=1)` (no groups).
fn split1(re: &pyre::Re, s: &str) -> Vec<String> {
    match re.search(s) {
        Some(m) => vec![
            pyre::upto(s, m.start()).to_string(),
            pyre::from(s, m.end()).to_string(),
        ],
        None => vec![s.to_string()],
    }
}

fn clean(text: &str) -> String {
    let t = pre!(r"PageV\d+P\d+|\bms\d+\b|\|").sub(text, " ");
    pyre::strip_ws(&pre!(r"\s+").sub(&t, " ")).to_string()
}

fn fix_ta_marbuta(text: &str) -> String {
    let words = [
        "الأولى",
        "الثانية",
        "الثالثة",
        "الرابعة",
        "الخامسة",
        "السادسة",
        "السابعة",
        "الثامنة",
        "التاسعة",
        "العاشرة",
        "الحادية",
        "عشرة",
    ];
    static RE: std::sync::OnceLock<(pyre::Re, HashMap<String, String>)> =
        std::sync::OnceLock::new();
    let (re, map) = RE.get_or_init(|| {
        let map: HashMap<String, String> = words
            .iter()
            .map(|w| (w.replace('ة', "ه"), w.to_string()))
            .collect();
        // the order of the alternatives as the Python dict keeps them
        let alts: Vec<String> = words
            .iter()
            .map(|w| regex::escape(&w.replace('ة', "ه")))
            .collect();
        (
            pyre::Re::new(&format!(r"(?<!\w)({})(?!\w)", alts.join("|"))),
            map,
        )
    });
    re.sub_with(text, |m| map[m.g(1)].clone())
}

/// One Taqrib entry, its parts (`parse_entry`).
#[derive(Debug, Clone)]
pub struct Entry {
    pub number: i64,
    pub name: String,
    pub verdict: String,
    pub rank: Option<i64>,
    pub tabaqa: Option<i64>,
    pub death_year: Option<i64>,
    pub death_note: Option<String>,
    pub kunya: Option<String>,
    pub kunyas: Vec<String>,
    pub marks: Vec<String>,
    pub text: String,
}

const TABAQA: &str = r"(?<!\w)من (?:(كبار|صغار|أوساط) )?(الحادية عشرة|الثانية عشرة|الأولى|الثانية|الثالثة|الرابعة|الخامسة|السادسة|السابعة|الثامنة|التاسعة|العاشرة)(?!\w)";

pub fn parse_entry(number: i64, text: &str) -> Entry {
    let text = fix_ta_marbuta(text);
    let mut tokens: Vec<&str> = pyre::split(&text);
    let mut marks = vec![];
    while tokens.last().is_some_and(|t| rijal::is_book_mark(t)) {
        marks.insert(0, tokens.pop().unwrap_or_default().to_string());
    }
    let body = tokens.join(" ");
    let b = Ps::new(&body);
    let (mut death_phrase, mut death_at) = (None, b.len());
    let death = pre!(r"(?<!\w)(?:مات|قتل|استشهد|توفي)(?: سنة| في سنة)? (.+)$").search(&body);
    if let Some(d) = &death {
        death_phrase = Some(d.group0().to_string());
        death_at = d.start();
    }
    let tabaqa_re = pre!(TABAQA);
    let (mut tabaqa, mut tabaqa_at) = (None, None);
    let before = tabaqa_re.finditer_in(&b, 0, death_at);
    let after = if death.is_some() {
        tabaqa_re.finditer_in(&b, death_at, b.len())
    } else {
        vec![]
    };
    let found = before.last().cloned().or_else(|| after.first().cloned()).or_else(|| {
        pre!(r"(?<!\w)الطبقة ()(الحادية عشرة|الثانية عشرة|الأولى|الثانية|الثالثة|الرابعة|الخامسة|السادسة|السابعة|الثامنة|التاسعة|العاشرة)(?!\w)").search(&body)
    });
    if let Some(f) = &found {
        tabaqa = rijal::tabaqa_word(f.g(2));
        tabaqa_at = (f.start() < death_at).then_some(f.start());
    }
    let head_end = tabaqa_at.unwrap_or(death_at);
    let head = pyre::strip_ws(b.sl(0, head_end)).to_string();
    let verdict_at = pre!(r"(?<!\w)(أم المؤمنين|الصحابي|الصحابية|صحابي|صحابية|له صحبة|لها صحبة|له رؤية|لها رؤية|ثقة|صدوقة|صدوقا|صدوق|مقبولة|مقبول|مستورة|مستور|مجهولة|مجهول|ضعيفة|ضعيفا|ضعيف|متروكة|متروكا|متروك|لا يعرف|كذاب|متهم|لين الحديث|لا بأس به|ليس به بأس|واه|ساقط|أمير المؤمنين)(?!\w)").search(&head);
    let (mut name, mut verdict) = match verdict_at {
        Some(v) if v.start() > 0 => (
            pyre::strip(pyre::upto(&head, v.start()), " ،,").to_string(),
            pyre::strip(pyre::from(&head, v.start()), " ،,").to_string(),
        ),
        _ => (pyre::strip(&head, " ،,").to_string(), String::new()),
    };
    if verdict.is_empty() {
        let life = pre!(
            r"\s(?:ولد|وهو|كان|أسلم|اسلم|شهد|استصغر|قدم|نزل|سكن|روى|هاجر|أحد|من السابقين)(?!\w)"
        )
        .search(&name);
        if let Some(l) = life.filter(|l| l.start() > 0) {
            let n = pyre::strip(pyre::upto(&name, l.start()), " ،,").to_string();
            verdict = pyre::strip(pyre::from(&name, l.start()), " ،,").to_string();
            name = n;
        }
    }
    let rank = if verdict.is_empty() {
        None
    } else {
        rijal::rank_of(&verdict)
    };
    if tabaqa.is_none() && rank == Some(1) {
        tabaqa = Some(1);
    }
    if tabaqa.is_none() && pre!(r"(?<!\w)مخضرم(?!\w)").search(&body).is_some() {
        tabaqa = Some(2);
    }
    let year = death_phrase
        .as_deref()
        .and_then(|d| rijal::year_of_death(d, tabaqa));
    let mut kunyas: Vec<String> = vec![];
    for k in pre!(
        r"(?<!بن )(?<!بنت )(?<!\w)(أبو|أم) (عبد \S+|\S+)|(?:يكنى|تكنى|كنيته) (أبا|أم) (عبد \S+|\S+)"
    )
    .finditer(&body)
    {
        let (first, second) = if !k.g(1).is_empty() {
            (k.g(1).to_string(), k.g(2).to_string())
        } else {
            (
                (if k.g(3) == "أبا" { "أبو" } else { "أم" }).to_string(),
                k.g(4).to_string(),
            )
        };
        let second = pyre::strip(&second, " ،,");
        let full = format!("{first} {second}");
        if !second.is_empty() && !kunyas.contains(&full) {
            kunyas.push(full);
        }
    }
    Entry {
        number,
        name,
        verdict,
        rank,
        tabaqa,
        death_year: year,
        death_note: death_phrase,
        kunya: kunyas.first().cloned(),
        kunyas,
        marks,
        text: body,
    }
}

/// An item of a book's outline (`read_outline`).
#[derive(Debug, Clone)]
pub enum Outline {
    Head(usize, String),
    Entry(i64),
    Xref(String),
}

pub fn read_outline(text: &str) -> Vec<Outline> {
    let mut out = vec![];
    for line in text.split('\n') {
        let Some(m) = pre!(r"^### (\|{1,3}|\${1,3}|\$DIC_NIS\$)\s*(.*)$").match_(line) else {
            continue;
        };
        let mark = m.g(1);
        let rest = pyre::strip_ws(m.g(2));
        if mark.starts_with('|') {
            let title = pyre::strip(rest, " ()");
            if !title.is_empty() {
                out.push(Outline::Head(pyre::len(mark), title.to_string()));
            }
        } else if mark == "$" || mark == "$$" {
            if let Some(n) = pre!(r"\d+").search(rest) {
                if let Ok(n) = n.group0().parse() {
                    out.push(Outline::Entry(n));
                }
            }
        } else if mark == "$$$" && !rest.is_empty() {
            out.push(Outline::Xref(rest.to_string()));
        }
    }
    out
}

/// A work's leaf: its number, name and text.
pub struct Leaf<'a> {
    pub number: i64,
    pub name: &'a str,
    pub text: &'a str,
    pub person: Option<i64>,
}

/// The rijal work as a readable work: its headings as chapters, each entry with its text and its narrator, each
/// cross-reference as it stands (`openiti_taqrib.write_work`).
#[allow(clippy::too_many_arguments)]
pub fn write_work(
    conn: &Connection,
    outline: &[Outline],
    leaves: &[Leaf],
    source_id: i64,
    key: &str,
    title: &str,
    title_ar: &str,
    author: &str,
) -> rusqlite::Result<i64> {
    conn.execute("DELETE FROM works WHERE key = ?", [key])?;
    conn.execute(
        "INSERT INTO works (key, kind, title, title_ar, author, source_id, stamp) VALUES (?, 'rijal', ?, ?, ?, ?, ?)",
        params![key, title, title_ar, author, source_id, format!("{key}|{}", leaves.len())],
    )?;
    let wid = conn.last_insert_rowid();
    // the entries of each number, in the order the numbers first come
    let mut order: Vec<i64> = vec![];
    let mut texts: HashMap<i64, std::collections::VecDeque<usize>> = HashMap::new();
    for (i, l) in leaves.iter().enumerate() {
        if !texts.contains_key(&l.number) {
            order.push(l.number);
        }
        texts.entry(l.number).or_default().push_back(i);
    }
    let mut ordinals: HashMap<Option<i64>, i64> = HashMap::new();
    let mut add = |parent: Option<i64>,
                   kind: &str,
                   label: Option<String>,
                   title: Option<&str>,
                   person: Option<i64>,
                   text: Option<&str>|
     -> rusqlite::Result<i64> {
        let o = ordinals.entry(parent).or_insert(0);
        *o += 1;
        conn.execute(
            "INSERT INTO work_nodes (work_id, parent_id, ordinal, kind, label, title, person_id, text) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            params![wid, parent, *o, kind, label, title, person, text],
        )?;
        Ok(conn.last_insert_rowid())
    };
    let mut stack: Vec<(usize, i64)> = vec![];
    let mut current: Option<i64> = None;
    for item in outline {
        match item {
            Outline::Head(level, heading) => {
                while stack.last().is_some_and(|(l, _)| l >= level) {
                    stack.pop();
                }
                let parent = stack.last().map(|(_, id)| *id);
                let id = add(
                    parent,
                    if *level == 1 { "kitab" } else { "bab" },
                    None,
                    Some(heading),
                    None,
                    None,
                )?;
                stack.push((*level, id));
                current = Some(id);
            }
            Outline::Entry(n) => {
                if let Some(i) = texts.get_mut(n).and_then(|q| q.pop_front()) {
                    let l = &leaves[i];
                    if current.is_none() {
                        current = Some(add(None, "kitab", None, Some("Introduction"), None, None)?);
                    }
                    add(
                        current,
                        "entry",
                        Some(l.number.to_string()),
                        Some(l.name),
                        l.person,
                        Some(l.text),
                    )?;
                }
            }
            Outline::Xref(t) => {
                if current.is_some() {
                    add(current, "reference", None, None, None, Some(t))?;
                }
            }
        }
    }
    for n in &order {
        let rest: Vec<usize> = texts
            .get_mut(n)
            .map(|q| q.drain(..).collect())
            .unwrap_or_default();
        for i in rest {
            let l = &leaves[i];
            if current.is_none() {
                current = Some(add(None, "kitab", None, Some("Entries"), None, None)?);
            }
            add(
                current,
                "entry",
                Some(l.number.to_string()),
                Some(l.name),
                l.person,
                Some(l.text),
            )?;
        }
    }
    Ok(wid)
}

/// The entries and the cross-references of the Taqrib (`read_entries`).
pub fn read_entries(text: &str) -> Result<(Vec<Entry>, Vec<String>), ResourceError> {
    let lines: Vec<&str> = text.split('\n').collect();
    let Some(start) = lines.iter().position(|l| l.starts_with("#META#Header#End")) else {
        return Err(ResourceError(
            "Not an OpenITI mARkdown file (no #META#Header#End#).".into(),
        ));
    };
    let mut entries = vec![];
    let mut xrefs = vec![];
    // (is entry, number, parts)
    let mut current: Option<(bool, i64, Vec<String>)> = None;
    let close = |c: &Option<(bool, i64, Vec<String>)>,
                 entries: &mut Vec<Entry>,
                 xrefs: &mut Vec<String>| {
        if let Some((is_entry, number, parts)) = c {
            let t = clean(&parts.join(" "));
            if *is_entry {
                entries.push(parse_entry(*number, &t));
            } else if !t.is_empty() {
                xrefs.push(t);
            }
        }
    };
    for line in &lines[start + 1..] {
        let m = pre!(r"^### \${1,2} (\d+)\s*(.*)$").match_(line);
        let x = if m.is_none() {
            pre!(r"^### \$\$\$\s*(.*)$").match_(line)
        } else {
            None
        };
        if let Some(m) = m {
            close(&current, &mut entries, &mut xrefs);
            current = Some((true, m.g(1).parse().unwrap_or(0), vec![m.g(2).to_string()]));
        } else if let Some(x) = x {
            close(&current, &mut entries, &mut xrefs);
            current = Some((false, 0, vec![x.g(1).to_string()]));
        } else if line.starts_with("~~") && current.is_some() {
            if let Some(c) = current.as_mut() {
                c.2.push(line[2..].to_string());
            }
        } else if line.starts_with('#') {
            close(&current, &mut entries, &mut xrefs);
            current = None;
        }
    }
    close(&current, &mut entries, &mut xrefs);
    Ok((entries, xrefs))
}

fn tokens(name: &str) -> Vec<String> {
    let n = normalize(name).replace("ابن ", "بن ");
    pyre::split(&n)
        .into_iter()
        .map(|w| {
            if w == "ابا" || w == "ابي" {
                "ابو".to_string()
            } else {
                w.to_string()
            }
        })
        .collect()
}

const COMMON: [&str; 6] = ["بن", "ابو", "ام", "بنت", "عبد", "الله"];
const EPITHETS: [&str; 8] = [
    "الحبر",
    "الفقيه",
    "مشهور",
    "المشهور",
    "الصحابي",
    "الحافظ",
    "الامام",
    "الإمام",
];
const MAX_ALIAS_TARGETS: usize = 8;

/// A cross-reference → (alias as written, the name tokens of every person it points to) (`_read_xref`).
fn read_xref(text: &str) -> (String, Vec<Vec<String>>) {
    let parts = split1(pre!(r"\s(?:هو|هي|اسمه|اسمها|صوابه|صوابها)\s"), text);
    let (alias, target, listing) = if parts.len() == 2 {
        (
            pyre::strip(&parts[0], " ،,").to_string(),
            parts[1].clone(),
            false,
        )
    } else {
        let words = pyre::split(text);
        if words.len() < 3 || !words[0].starts_with("ال") {
            return (String::new(), vec![]);
        }
        (words[0].to_string(), words[1..].join(" "), true)
    };
    let spaced = format!(" {}", pyre::strip_ws(&target));
    let target = match pre!(r"\s(?:وقيل|ويقال|أو|تقدم|تقدموا|يأتي|سيأتي|مشهور)(?!\w)")
        .search(&spaced)
    {
        Some(m) => pyre::upto(&spaced, m.start()).to_string(),
        None => spaced.clone(),
    };
    let pieces: Vec<String> = if listing {
        pre!(r"\s(?:و(?=\S))").split(&format!(" {}", pyre::strip_ws(&target)))
    } else {
        vec![target]
    };
    let alias = pyre::split(&alias)
        .into_iter()
        .filter(|w| !EPITHETS.contains(w))
        .collect::<Vec<_>>()
        .join(" ");
    let alias_tokens = tokens(&alias);
    let mut targets = vec![];
    for piece in pieces {
        let mut toks: Vec<String> = tokens(&piece)
            .into_iter()
            .filter(|t| !rijal::is_book_mark(t) && !EPITHETS.contains(&t.as_str()))
            .collect();
        while toks
            .first()
            .is_some_and(|t| ["ابنه", "اخوه", "ابوه", "اخته", "ابنته"].contains(&t.as_str()))
        {
            toks.remove(0);
        }
        if toks.is_empty() {
            continue;
        }
        if toks[0] == "بن" {
            let mut t: Vec<String> = alias_tokens.iter().take(1).cloned().collect();
            t.extend(toks);
            toks = t;
        } else if alias_tokens.first().map(String::as_str) == Some("بن")
            && !toks.iter().any(|t| t == "بن")
        {
            toks.extend(alias_tokens.iter().take(2).cloned());
        }
        toks.truncate(6);
        targets.push(toks);
    }
    (alias, targets)
}

fn is_subsequence(short: &[String], long: &[String]) -> bool {
    let mut it = long.iter();
    short.iter().all(|w| it.any(|x| x == w))
}

fn source_row(
    conn: &Connection,
    key: &str,
    name: &str,
    format_id: &str,
    source: &SourceInfo,
) -> rusqlite::Result<i64> {
    let tier = source
        .tier
        .clone()
        .unwrap_or_else(|| tier_for_license(source.license.as_deref()));
    conn.execute(
        "INSERT INTO sources (key, name, format, location, license, tier, origin, auth_type, imported_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(key) DO UPDATE SET name = excluded.name, location = excluded.location,
             license = COALESCE(excluded.license, sources.license), tier = excluded.tier,
             origin = excluded.origin, auth_type = excluded.auth_type, imported_at = excluded.imported_at",
        params![key, name, format_id, source.location, source.license, tier, source.origin, source.auth.describe(), now_iso()],
    )?;
    conn.query_row("SELECT id FROM sources WHERE key = ?", [key], |r| r.get(0))
}

fn person_key(prefix: &str, number: i64, used: &mut HashSet<String>) -> String {
    let mut key = format!("{prefix}:{number}");
    let mut suffix = 'b';
    while used.contains(&key) {
        key = format!("{prefix}:{number}{suffix}");
        suffix = char::from_u32(suffix as u32 + 1).unwrap_or('z');
    }
    used.insert(key.clone());
    key
}

/// Ibn Hajar's Taqrib al-Tahdhib (`TaqribImporter.run`).
pub fn taqrib(
    conn: &Connection,
    source: &SourceInfo,
    say: &dyn Fn(&str),
) -> Result<ImportReport, ImportError> {
    say(&format!("Reading {}", source.location));
    let text = read_text(source, None)?;
    let (entries, xrefs) = read_entries(&text)?;
    if entries.len() < 100 {
        return Err(ResourceError(format!(
            "Only {} entries found; is this the Taqrib al-Tahdhib?",
            entries.len()
        ))
        .into());
    }
    let lang = crate::i18n::ui_language();
    say(&crate::i18n::t(
        &lang,
        "{entries} narrators and {xrefs} cross-references",
        &[
            ("entries", entries.len().to_string()),
            ("xrefs", xrefs.len().to_string()),
        ],
    ));
    let name = source
        .name
        .clone()
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| "Ibn Hajar, Taqrib al-Tahdhib (OpenITI)".into());
    let key = "openiti-taqrib";
    let mut report = ImportReport {
        source_key: key.into(),
        ..Default::default()
    };
    let tx = conn.unchecked_transaction()?;
    // a summary, said once: not a warning (iy printed it again as one)
    let summary;
    let source_id = source_row(&tx, key, &name, "taqrib", source)?;
    tx.execute("DELETE FROM persons WHERE source_id = ?", [source_id])?;
    let mut used = HashSet::new();
    let mut token_index: Vec<(i64, Vec<String>)> = vec![];
    let mut token_of: HashMap<i64, HashSet<String>> = HashMap::new();
    let mut person_of: Vec<i64> = vec![];
    {
        let mut person = tx.prepare(
            "INSERT INTO persons (key, name_ar, kunya, death_year_ah, death_year_note, generation, tabaqa, \
             tradition, source_id) VALUES (?, ?, ?, ?, ?, ?, ?, 'sunni', ?)",
        )?;
        let mut names = tx.prepare(
            "INSERT INTO person_names (person_id, name, kind, source_id) VALUES (?, ?, ?, ?)",
        )?;
        let mut role = tx.prepare(
            "INSERT OR IGNORE INTO person_roles (person_id, role) VALUES (?, 'narrator')",
        )?;
        let mut mark = tx.prepare(
            "INSERT OR IGNORE INTO person_marks (person_id, mark, source_id) VALUES (?, ?, ?)",
        )?;
        let mut verdict = tx.prepare(
            "INSERT INTO verdicts (person_id, critic_name, work, phrase, rank_scheme, rank, tradition, source_id) \
             VALUES (?, ?, ?, ?, ?, ?, 'sunni', ?)",
        )?;
        for e in &entries {
            let pk = person_key("taqrib", e.number, &mut used);
            person.execute(params![
                pk,
                e.name,
                e.kunya,
                e.death_year,
                e.death_note,
                rijal::generation(e.tabaqa),
                e.tabaqa,
                source_id
            ])?;
            let pid = tx.last_insert_rowid();
            person_of.push(pid);
            names.execute(params![pid, e.name, "full", source_id])?;
            for k in &e.kunyas {
                if !e.name.starts_with(k.as_str()) {
                    names.execute(params![pid, k, "kunya", source_id])?;
                }
            }
            role.execute([pid])?;
            for m in &e.marks {
                mark.execute(params![pid, m, source_id])?;
            }
            if !e.verdict.is_empty() {
                verdict.execute(params![
                    pid, CRITIC, WORK, e.verdict, SCHEME, e.rank, source_id
                ])?;
            }
            let toks = tokens(&e.name);
            token_of.insert(pid, toks.iter().cloned().collect());
            token_index.push((pid, toks));
        }
        let mut linked = 0;
        for x in &xrefs {
            let (alias, targets) = read_xref(x);
            if alias.is_empty() || targets.is_empty() {
                continue;
            }
            let distinctive: Vec<String> = tokens(&alias)
                .into_iter()
                .filter(|w| !COMMON.contains(&w.as_str()))
                .collect();
            let mut persons: Vec<i64> = vec![];
            for target in &targets {
                let mut hits: Vec<i64> = token_index
                    .iter()
                    .filter(|(_, toks)| {
                        toks.first() == target.first() && is_subsequence(target, toks)
                    })
                    .map(|(p, _)| *p)
                    .collect();
                if hits.len() > MAX_ALIAS_TARGETS {
                    let begins: Vec<i64> = token_index
                        .iter()
                        .filter(|(_, toks)| {
                            toks.len() >= target.len() && toks[..target.len()] == target[..]
                        })
                        .map(|(p, _)| *p)
                        .collect();
                    if !begins.is_empty() {
                        hits = begins;
                    }
                }
                if hits.len() > 1 && !distinctive.is_empty() {
                    let every: Vec<i64> = hits
                        .iter()
                        .copied()
                        .filter(|p| distinctive.iter().all(|w| token_of[p].contains(w)))
                        .collect();
                    let some: Vec<i64> = hits
                        .iter()
                        .copied()
                        .filter(|p| distinctive.iter().any(|w| token_of[p].contains(w)))
                        .collect();
                    hits = if every.is_empty() { some } else { every };
                }
                if (1..=MAX_ALIAS_TARGETS).contains(&hits.len()) {
                    for h in hits {
                        if !persons.contains(&h) {
                            persons.push(h);
                        }
                    }
                }
            }
            for pid in &persons {
                names.execute(params![pid, alias, "variant", source_id])?;
            }
            linked += !persons.is_empty() as usize;
        }
        let leaves: Vec<Leaf> = entries
            .iter()
            .zip(&person_of)
            .map(|(e, p)| Leaf {
                number: e.number,
                name: &e.name,
                text: &e.text,
                person: Some(*p),
            })
            .collect();
        write_work(
            &tx,
            &read_outline(&text),
            &leaves,
            source_id,
            "taqrib",
            "Taqrib al-Tahdhib",
            "تقريب التهذيب",
            "ibnhajar",
        )?;
        let n = entries.len();
        let ranked = entries.iter().filter(|e| e.rank.is_some()).count();
        summary = format!(
            "{n} narrators; rank read for {ranked} ({:.1}%), tabaqa for {}, death year for {}; {linked} of {} \
             cross-references linked",
            100.0 * ranked as f64 / n as f64,
            entries.iter().filter(|e| e.tabaqa.is_some()).count(),
            entries.iter().filter(|e| e.death_year.is_some()).count(),
            xrefs.len()
        );
    }
    tx.commit()?;
    report.editions.push(WORK.into());
    report.texts = entries.len() as i64;
    say(&summary);
    Ok(report)
}

// ------------------------------------------------------------------ the works read by profile (openiti_rijal.py)
struct Profile {
    title: &'static str,
    title_ar: &'static str,
    author: &'static str,
    critic: &'static str,
    tradition: &'static str,
    scheme: &'static str,
    source_key: &'static str,
    minimum: usize,
    name: &'static str,
}

fn profile(key: &str) -> Option<Profile> {
    match key {
        "najashi" => Some(Profile {
            title: "Rijal al-Najashi",
            title_ar: "رجال النجاشي",
            author: "najashi",
            critic: "al-Najashi",
            tradition: "shia",
            scheme: "shia",
            source_key: "openiti-najashi",
            minimum: 1000,
            name: "al-Najashi, Rijal (OpenITI)",
        }),
        // 0.7.6: al-Tusi's two books of men (`PROFILES` of openiti_rijal.py)
        "tusi_rijal" => Some(Profile {
            title: "Rijal al-Tusi",
            title_ar: "رجال الطوسي",
            author: "tusi",
            critic: "al-Tusi",
            tradition: "shia",
            scheme: "shia",
            source_key: "openiti-tusi-rijal",
            minimum: 5000,
            name: "al-Tusi, Rijal (OpenITI)",
        }),
        "tusi_fihrist" => Some(Profile {
            title: "al-Fihrist — al-Tusi",
            title_ar: "الفهرست للطوسي",
            author: "tusi",
            critic: "al-Tusi",
            tradition: "shia",
            scheme: "shia",
            source_key: "openiti-tusi-fihrist",
            minimum: 800,
            name: "al-Tusi, al-Fihrist (OpenITI)",
        }),
        _ => None,
    }
}

fn clean_najashi(text: &str) -> String {
    let t = pre!(r"PageV\d+P\d+|ms\d+|~~|^#\s*|\s#\s").sub(text, " ");
    pyre::strip_ws(&pre!(r"\s+").sub(&t, " ")).to_string()
}

/// The narrator's name from the start of his entry (`entry_name`).
pub fn entry_name(text: &str) -> String {
    // the editions' footnote numbers and marks inside a name are no part of it
    let text = pre!(r"(?<!\S)(?:\d+|#)(?=[\s،,.:؛]|$)").sub(text, " ");
    let mut head = split1(pre!(r"[،,.:؛]|\s-\s"), pyre::lstrip(&text, " -")).swap_remove(0);
    let spaced = format!(" {head}");
    if let Some(m) = pre!(r"\s(?:كوفي|بصري|مدني|قمي|بغدادي|عربي|ثقة|روى|يروي|له كتاب|له كتب|له نسخة|يكنى|كان|ذكره|وكان|من أصحاب|من اصحاب|ضعيف|عن|أصله|اصله|يعرف|المعروف|يقال|نزل|نزيل|سكن|أسند عنه|اسند عنه|روى عنه|روى عنهما|مات|قتل|شهد)(?=[\s،,.]|$)").search(&spaced) {
        if m.start() > 0 {
            head = pyre::upto(&spaced, m.start()).to_string();
        }
    }
    let mut words: Vec<&str> = pyre::split(&head).into_iter().take(16).collect();
    while words
        .last()
        .is_some_and(|w| ["بن", "بنت", "أبو", "أبي", "ابن", "و"].contains(w))
    {
        words.pop();
    }
    pyre::strip_ws(&words.join(" ")).to_string()
}

struct Plain {
    number: i64,
    name: String,
    text: String,
}

fn read_plain(text: &str) -> Vec<Plain> {
    let mut out: Vec<(i64, String, Vec<String>)> = vec![];
    let mut current: Option<(i64, String, Vec<String>)> = None;
    for line in text.split('\n') {
        // the Fihrist's "### $ [12] 3 - name" (its entry number in brackets), then "### $ 12 - name", "### $ 580 ms018 - …"
        let found = pre!(r"^### \$\$? \[(\d+)\]\s*(?:\d+)?\s*(?:ms\d+\s*)?(?:-\s*)?(.*)$")
            .match_(line)
            .or_else(|| {
                pre!(r"^### \$\$? (?:\[\])?\s*(\d+)\s*(?:ms\d+\s*)?-\s*(.*)$").match_(line)
            });
        if let Some(m) = found {
            if let Some(c) = current.take() {
                out.push(c);
            }
            let name = pyre::strip(&clean_najashi(m.g(2)), " ،,.-").to_string();
            current = Some((m.g(1).parse().unwrap_or(0), name, vec![m.g(2).to_string()]));
        } else if line.starts_with("### ") {
            if let Some(c) = current.take() {
                out.push(c);
            }
        } else if let Some(c) = current.as_mut() {
            c.2.push(line.to_string());
        }
    }
    if let Some(c) = current {
        out.push(c);
    }
    out.into_iter()
        .map(|(number, name, lines)| {
            let text = clean_najashi(&lines.join(" "));
            let n = entry_name(&text);
            Plain {
                number,
                name: if n.is_empty() { name } else { n },
                text,
            }
        })
        .collect()
}

fn clip(text: &str, limit: usize) -> String {
    if pyre::len(text) <= limit {
        return text.to_string();
    }
    let cut = pyre::upto(text, limit);
    let head = match cut.rfind(' ') {
        Some(i) => &cut[..i],
        None => cut,
    };
    format!("{head} …")
}

/// A rijal work read by its profile (`RijalImporter.run`).
pub fn by_profile(
    profile_key: &str,
    conn: &Connection,
    source: &SourceInfo,
    say: &dyn Fn(&str),
) -> Result<ImportReport, ImportError> {
    let Some(prof) = profile(profile_key) else {
        return Err(ImportError::Resource(format!(
            "Unknown format '{profile_key}'"
        )));
    };
    say(&format!("Reading {}", source.location));
    let text = read_text(source, None)?;
    let entries = read_plain(&text);
    if entries.len() < prof.minimum {
        return Err(ResourceError(format!(
            "Only {} entries found; is this {}?",
            entries.len(),
            prof.title
        ))
        .into());
    }
    say(&format!("{} entries", entries.len()));
    let key = prof.source_key;
    let mut report = ImportReport {
        source_key: key.into(),
        ..Default::default()
    };
    let mut counts: HashMap<Option<i64>, usize> = HashMap::new();
    let tx = conn.unchecked_transaction()?;
    let name = source
        .name
        .clone()
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| prof.name.into());
    let source_id = source_row(&tx, key, &name, profile_key, source)?;
    tx.execute("DELETE FROM persons WHERE source_id = ?", [source_id])?;
    let mut used = HashSet::new();
    let mut person_of = vec![];
    for e in &entries {
        let pk = person_key(profile_key, e.number, &mut used);
        tx.execute(
            "INSERT INTO persons (key, name_ar, tradition, source_id) VALUES (?, ?, ?, ?)",
            params![pk, e.name, prof.tradition, source_id],
        )?;
        let pid = tx.last_insert_rowid();
        person_of.push(pid);
        tx.execute(
            "INSERT INTO person_names (person_id, name, kind, source_id) VALUES (?, ?, 'full', ?)",
            params![pid, e.name, source_id],
        )?;
        tx.execute(
            "INSERT OR IGNORE INTO person_roles (person_id, role) VALUES (?, 'narrator')",
            [pid],
        )?;
        let words = rijal::opening(&e.text);
        let (rank, madhhab) = rijal::assess(&words);
        *counts.entry(rank).or_insert(0) += 1;
        tx.execute(
            "INSERT INTO verdicts (person_id, critic_name, work, phrase, rank_scheme, rank, tradition, madhhab, source_id) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![pid, prof.critic, prof.title, clip(&words, 420), prof.scheme, rank, prof.tradition, madhhab, source_id],
        )?;
    }
    let leaves: Vec<Leaf> = entries
        .iter()
        .zip(&person_of)
        .map(|(e, p)| Leaf {
            number: e.number,
            name: &e.name,
            text: &e.text,
            person: Some(*p),
        })
        .collect();
    write_work(
        &tx,
        &read_outline(&text),
        &leaves,
        source_id,
        profile_key,
        prof.title,
        prof.title_ar,
        prof.author,
    )?;
    tx.commit()?;
    let n = entries.len();
    let judged = n - counts.get(&None).copied().unwrap_or(0);
    report.editions.push(prof.title.into());
    report.texts = n as i64;
    // a summary, said once: not a warning (iy printed it again as one)
    let summary = format!(
        "{n} narrators; a judgment read for {judged} ({:.1}%): {}",
        100.0 * judged as f64 / n as f64,
        rijal::SHIA_RANKS
            .iter()
            .map(|(k, label)| format!("{label} {}", counts.get(&Some(*k)).copied().unwrap_or(0)))
            .collect::<Vec<_>>()
            .join(", ")
    );
    say(&summary);
    Ok(report)
}
