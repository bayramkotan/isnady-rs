//! Looking up a narrator — the Python isnady's `narrators.search_persons`, the whole `describe`, `relations` and
//! `hadiths_of` (`isnady/core/narrators.py`), and the famous ones (`isnady/core/famous.py`): the Companions everyone
//! looks for, found by any spelling people type ("Ebubekir", "Omer ibn Hattap", "Hz. Ali").

use regex::Regex;
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashMap;
use std::sync::OnceLock;
use unicode_normalization::char::is_combining_mark;
use unicode_normalization::UnicodeNormalization;

use crate::normalize::normalize;
use crate::{books, narrators, py, tables, works};

fn re(p: &str) -> Regex {
    Regex::new(p).expect("pattern")
}

pub(crate) fn arabic_letter(s: &str) -> bool {
    s.chars().any(|c| ('\u{0600}'..='\u{06FF}').contains(&c))
}

/// `re.sub(r"(.)\1+", r"\1", w)`: a letter written twice or more, once.
fn single(w: &str) -> String {
    let mut out = String::new();
    let mut last: Option<char> = None;
    for c in w.chars() {
        if last != Some(c) {
            out.push(c);
        }
        last = Some(c);
    }
    out
}

fn strip_marks_nfkd(w: &str) -> String {
    w.nfkd().filter(|c| !is_combining_mark(*c)).collect()
}

fn lower_tr(word: &str) -> String {
    word.replace('I', "ı").replace('İ', "i").to_lowercase()
}

// ------------------------------------------------------------------ skeletons (narrators.py)

fn ar_skeleton_letter(c: char) -> &'static str {
    match c {
        'ب' => "b",
        'ت' | 'ط' => "t",
        'ث' => "th",
        'ج' => "j",
        'ح' | 'ه' => "h",
        'خ' => "kh",
        'د' | 'ض' => "d",
        'ذ' => "dh",
        'ر' => "r",
        'ز' | 'ظ' => "z",
        'س' | 'ص' => "s",
        'ش' => "sh",
        'غ' => "gh",
        'ف' => "f",
        'ق' | 'ك' => "k",
        'ل' => "l",
        'م' => "m",
        'ن' => "n",
        _ => "",
    }
}

/// Consonants of an Arabic word (`narrators.arabic_skeleton`).
pub fn arabic_skeleton(word: &str) -> String {
    let mut original: String = word.trim().to_string();
    let mut w: Vec<char> = normalize(&original).chars().collect();
    if w.len() > 3 && w[0] == 'ا' && w[1] == 'ل' {
        w.drain(..2);
        if let Some(rest) = original.strip_prefix("ال") {
            original = rest.to_string();
        }
    }
    if original.ends_with('ة') && w.last() == Some(&'ه') {
        w.pop();
    }
    let mut out = String::new();
    for (i, ch) in w.iter().enumerate() {
        if i == 0 && (*ch == 'ي' || *ch == 'و') {
            out.push(if *ch == 'ي' { 'y' } else { 'w' });
        } else {
            out.push_str(ar_skeleton_letter(*ch));
        }
    }
    let nisba = original.ends_with('ي') && w.len() > 2;
    if nisba {
        out.push('y');
    }
    out
}

const DIGRAPHS: [(&str, &str); 5] = [
    ("th", "\x01"),
    ("kh", "\x02"),
    ("dh", "\x03"),
    ("sh", "\x04"),
    ("gh", "\x05"),
];

/// Consonants of a Latin spelling, comparable with `arabic_skeleton` (`narrators.latin_skeleton`).
pub fn latin_skeleton(word: &str) -> String {
    static ART: OnceLock<Regex> = OnceLock::new();
    static NOT_AZ: OnceLock<Regex> = OnceLock::new();
    static VOWELS: OnceLock<Regex> = OnceLock::new();
    let bare = lower_tr(py::strip(word.trim(), ".")).replace('ı', "i");
    if [
        "b", "bin", "ibn", "bn", "ben", "ibni", "ibnu", "ibnü", "bini",
    ]
    .contains(&bare.as_str())
    {
        return "bn".into();
    }
    if ["bt", "bint", "binti", "bintu"].contains(&bare.as_str()) {
        return "bnt".into();
    }
    let w = lower_tr(word)
        .replace('ş', "sh")
        .replace('ı', "i")
        .replace('c', "j")
        .replace('ç', "ch")
        .replace('v', "w");
    let w = strip_marks_nfkd(&w);
    let art = ART.get_or_init(|| {
        re(r"^(al|el|ad|an|ar|as|at|az|ash|adh|es|et|ez|ed|en|er|eş|ed|ad|ul|ül)-")
    });
    let w = art.replace(&w, "").into_owned();
    let w = NOT_AZ
        .get_or_init(|| re(r"[^a-z]"))
        .replace_all(&w, "")
        .into_owned();
    let w = single(&w);
    if w.is_empty() {
        return String::new();
    }
    let nisba = w.ends_with('i') || w.ends_with("iy");
    let first = w.chars().next().expect("letter");
    let initial = if first == 'y' || first == 'w' {
        first.to_string()
    } else {
        String::new()
    };
    let mut body = if initial.is_empty() {
        w.clone()
    } else {
        w[1..].to_string()
    };
    for (d, m) in DIGRAPHS {
        body = body.replace(d, m);
    }
    body = body.replace('q', "k");
    body = VOWELS
        .get_or_init(|| re(r"[aeiouyw]"))
        .replace_all(&body, "")
        .into_owned();
    for (d, m) in DIGRAPHS {
        body = body.replace(m, d);
    }
    let core = format!("{initial}{body}");
    if nisba && !core.is_empty() {
        core + "y"
    } else {
        core
    }
}

/// A name as normalised words, "ابن" as "بن", "ابا"/"ابي" as "ابو" (`narrators.tokens`).
pub fn tokens(name: &str) -> Vec<String> {
    normalize(name)
        .replace(['،', ','], " ")
        .replace("ابن ", "بن ")
        .split_whitespace()
        .map(|w| {
            if w == "ابا" || w == "ابي" {
                "ابو".to_string()
            } else {
                w.to_string()
            }
        })
        .collect()
}

/// "Abdullah", "Abdurrahman", عبدالله typed as one word, in two (`narrators.split_abd`).
pub fn split_abd(text: &str) -> String {
    static LAT: OnceLock<Regex> = OnceLock::new();
    static AR: OnceLock<fancy_regex::Regex> = OnceLock::new();
    static VOWEL: OnceLock<Regex> = OnceLock::new();
    let lat = LAT.get_or_init(|| re(r"\b([Aa]bd)([aeiouüı][a-zçğışöüâîû'-]{2,})"));
    let vowel = VOWEL.get_or_init(|| re(r"^[aeiouüı]"));
    let text = lat
        .replace_all(text, |c: &regex::Captures| {
            let mut rest = vowel.replace(&c[2], "").into_owned();
            if rest.starts_with('l') && !rest.starts_with("ll") {
                rest = rest[1..].to_string();
            }
            if rest.is_empty() {
                c[0].to_string()
            } else {
                format!("{} {rest}", &c[1])
            }
        })
        .into_owned();
    let ar = AR.get_or_init(|| fancy_regex::Regex::new(r"(?<![\w])(عبد)(ال\w+)").expect("pattern"));
    ar.replace_all(&text, "$1 $2").into_owned()
}

/// A Latin spelling reduced so that English and Turkish spellings compare closely (`narrators.fold_latin`).
pub fn fold_latin(text: &str) -> String {
    static ART: OnceLock<Regex> = OnceLock::new();
    static NOT_AZ: OnceLock<Regex> = OnceLock::new();
    let mut w = lower_tr(text);
    for (a, b) in [
        ("ş", "sh"),
        ("ı", "i"),
        ("ç", "ch"),
        ("c", "j"),
        ("v", "w"),
        ("ğ", ""),
        ("q", "k"),
    ] {
        w = w.replace(a, b);
    }
    let w = strip_marks_nfkd(&w);
    let art = ART.get_or_init(|| re(r"^(al|el|an|ar|as|at|az|ad|ash|adh|en|er|es|et|ez|ed)-"));
    let not_az = NOT_AZ.get_or_init(|| re(r"[^a-z]"));
    let mut words = Vec::new();
    for word in w.split(char::is_whitespace) {
        let word = art.replace(word, "").into_owned();
        let word = not_az.replace_all(&word, "").into_owned();
        if [
            "ibn", "bin", "bn", "b", "bint", "bt", "ibni", "ibnu", "bini", "binti",
        ]
        .contains(&word.as_str())
        {
            continue;
        }
        words.push(word.replace('e', "a").replace('o', "u").replace('y', "i"));
    }
    words
        .into_iter()
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn loose(word: &str) -> String {
    let mut w = word.to_string();
    if !w.is_empty() && !arabic_letter(&w) {
        for (d, s) in [
            ("kh", "h"),
            ("sh", "s"),
            ("th", "s"),
            ("dh", "z"),
            ("gh", "g"),
        ] {
            w = w.replace(d, s);
        }
        if w.chars().count() > 2 && w.ends_with('h') {
            w.pop();
        }
        if w.chars().count() > 2 && w.ends_with('y') {
            w.pop();
        }
    } else if w.starts_with("ال") && w.chars().count() > 3 {
        w = w.chars().skip(2).collect();
    }
    single(&w)
}

pub(crate) fn near(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    let (mut a, mut b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let (la, lb) = (a.len(), b.len());
    if la.abs_diff(lb) > 1 {
        return false;
    }
    if la == lb {
        let diff: Vec<usize> = (0..la).filter(|i| a[*i] != b[*i]).collect();
        return diff.len() == 1
            || (diff.len() == 2
                && diff[1] == diff[0] + 1
                && a[diff[0]] == b[diff[1]]
                && a[diff[1]] == b[diff[0]]);
    }
    if la > lb {
        std::mem::swap(&mut a, &mut b);
    }
    let mut i = 0;
    while i < a.len() && a[i] == b[i] {
        i += 1;
    }
    a[i..] == b[i + 1..]
}

fn starts(w: &str, q: &str) -> bool {
    w.starts_with(q)
}

pub(crate) fn exact(q: &str, word: &str) -> bool {
    let word = if word.starts_with("ال") && word.chars().count() > 3 && !q.starts_with("ال") {
        &word["ال".len()..]
    } else {
        word
    };
    if q.chars().count() < 3 {
        word == q
    } else {
        starts(word, q)
    }
}

pub(crate) fn same_loose(q: &str, word: &str) -> bool {
    let w = loose(word);
    w == q || (q.chars().count() >= 3 && w.starts_with(q))
}

pub(crate) fn one_off(q: &str, word: &str) -> bool {
    let w = loose(word);
    if w == q {
        return true;
    }
    if q.chars().count() < 4 || w.is_empty() || w.chars().next() != q.chars().next() {
        return false;
    }
    let qn = q.chars().count();
    near(q, &w) || (w.chars().count() > qn && near(q, &w.chars().take(qn).collect::<String>()))
}

pub(crate) fn ordered_score(
    wanted: &[String],
    name: &[String],
    whole: &[String],
    full_name: &[String],
    m: fn(&str, &str) -> bool,
) -> Option<i64> {
    let (mut i, mut score) = (0usize, 0i64);
    for q in wanted {
        while i < name.len() && !m(q, &name[i]) {
            i += 1;
        }
        if i == name.len() {
            return None;
        }
        score += i64::from(name[i] == *q);
        i += 1;
    }
    if !name.is_empty() && !wanted.is_empty() && name[0].starts_with(wanted[0].as_str()) {
        score += 2;
    }
    if full_name == whole {
        score += 100;
    }
    Some(score)
}

type IndexRow = (i64, Vec<String>, bool);

fn index(conn: &Connection, latin: bool) -> rusqlite::Result<Vec<IndexRow>> {
    let mut st = conn.prepare("SELECT person_id, name, kind FROM person_names")?;
    let rows: Vec<(i64, String, Option<String>)> = st
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<Result<_, _>>()?;
    let mut out = Vec::with_capacity(rows.len());
    for (pid, name, kind) in rows {
        let own = kind.as_deref() == Some("full");
        if latin {
            let words: Vec<&str> = py::words(&name);
            out.push((pid, words.iter().map(|w| arabic_skeleton(w)).collect(), own));
            if words.len() > 2
                && ["ابو", "ام", "ابي", "ابا"].contains(&normalize(words[0]).as_str())
            {
                let n = if normalize(words[1]) == "عبد" {
                    3
                } else {
                    2
                };
                out.push((
                    pid,
                    words[..n].iter().map(|w| arabic_skeleton(w)).collect(),
                    own,
                ));
            }
        } else {
            let words = tokens(&name);
            if words.len() > 2 && (words[0] == "ابو" || words[0] == "ام") {
                let n = if words[1] == "عبد" { 3 } else { 2 };
                out.push((pid, words.clone(), own));
                out.push((pid, words[..n].to_vec(), own));
            } else {
                out.push((pid, words, own));
            }
        }
    }
    Ok(out)
}

const FUZZY_BELOW: usize = 10;

/// Persons whose names contain the words typed, in that order (`narrators.search_persons`): (id, Arabic name).
pub fn search_persons(
    conn: &Connection,
    text: &str,
    limit: usize,
) -> rusqlite::Result<Vec<(i64, String)>> {
    search_persons_close(conn, text, limit, None)
}

/// search_persons, the ids found by a close spelling (not the letters typed) added to `close` (S4).
pub fn search_persons_close(
    conn: &Connection,
    text: &str,
    limit: usize,
    mut close: Option<&mut std::collections::HashSet<i64>>,
) -> rusqlite::Result<Vec<(i64, String)>> {
    if text.trim().is_empty() {
        return Ok(Vec::new());
    }
    let latin = !arabic_letter(text);
    let text = split_abd(text);
    let (whole, wanted): (Vec<String>, Vec<String>) = if latin {
        let whole: Vec<String> = py::words(&text)
            .into_iter()
            .map(latin_skeleton)
            .filter(|s| !s.is_empty())
            .collect();
        let wanted = whole
            .iter()
            .filter(|w| *w != "bn" && *w != "bnt")
            .cloned()
            .collect();
        (whole, wanted)
    } else {
        let whole = tokens(&text);
        let wanted = whole.iter().filter(|w| *w != "بن").cloned().collect();
        (whole, wanted)
    };
    if wanted.is_empty() {
        return Ok(Vec::new());
    }
    let idx = index(conn, latin)?;
    let ibn: &[&str] = if latin { &["bn", "bnt"] } else { &["بن"] };
    let core = |words: &[String]| -> Vec<String> {
        words
            .iter()
            .filter(|w| !ibn.contains(&w.as_str()))
            .cloned()
            .collect()
    };
    let mut order: Vec<i64> = Vec::new();
    let mut best: HashMap<i64, i64> = HashMap::new();
    let put =
        |best: &mut HashMap<i64, i64>, order: &mut Vec<i64>, pid: i64, score: i64, floor: i64| {
            let e = best.entry(pid).or_insert_with(|| {
                order.push(pid);
                floor
            });
            if score > *e {
                *e = score;
            }
        };
    for (pid, words, own) in &idx {
        if let Some(s) = ordered_score(&wanted, &core(words), &whole, words, exact) {
            put(&mut best, &mut order, *pid, s + 10 * i64::from(*own), -1);
        }
    }
    let loose_wanted: Vec<String> = wanted.iter().map(|w| loose(w)).collect();
    for (m, offset) in [
        (same_loose as fn(&str, &str) -> bool, -1000i64),
        (one_off, -2000),
    ] {
        if best.len() >= FUZZY_BELOW {
            break;
        }
        for (pid, words, own) in &idx {
            if best.get(pid).is_some_and(|b| *b > offset + 500) {
                continue;
            }
            if let Some(s) = ordered_score(&loose_wanted, &core(words), &whole, words, m) {
                put(
                    &mut best,
                    &mut order,
                    *pid,
                    s + 10 * i64::from(*own) + offset,
                    -1_000_000,
                );
                if let Some(c) = close.as_deref_mut() {
                    c.insert(*pid);
                }
            }
        }
    }
    let uses = link_counts(conn)?;
    order.sort_by_key(|pid| (-best[pid], -uses.get(pid).copied().unwrap_or(0)));
    order.truncate(limit);
    order
        .into_iter()
        .map(|pid| {
            let name: String = conn.query_row(
                "SELECT name_ar FROM persons WHERE id = ?",
                params![pid],
                |r| r.get(0),
            )?;
            Ok((pid, name))
        })
        .collect()
}

pub(crate) fn link_counts(conn: &Connection) -> rusqlite::Result<HashMap<i64, i64>> {
    static USES: OnceLock<HashMap<i64, i64>> = OnceLock::new();
    if let Some(u) = USES.get() {
        return Ok(u.clone());
    }
    let mut st = conn.prepare(
        "SELECT person_id, COUNT(*) FROM isnad_links WHERE person_id IS NOT NULL GROUP BY person_id",
    )?;
    let m: HashMap<i64, i64> = st
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    let _ = USES.set(m.clone());
    Ok(m)
}

// ------------------------------------------------------------------ the whole description

#[derive(Debug, Clone)]
pub struct FullVerdict {
    pub critic_name: String,
    pub work: String,
    pub phrase: String,
    pub rank_scheme: Option<String>,
    pub rank: Option<i64>,
    pub rank_label: String,
    pub madhhab_label: String,
}

#[derive(Debug, Clone)]
pub struct Described {
    pub person: narrators::Person,
    pub key: String,
    pub tradition: String,
    pub source: Option<String>,
    pub death_year_note: Option<String>,
    pub tabaqa_label: String,
    pub other_names: Vec<String>,
    pub marks: Vec<(String, String)>,
    pub verdicts: Vec<FullVerdict>,
    pub in_chains: i64,
}

/// Everything the rijal works say about one person (`narrators.describe`), labels in English.
pub fn describe(conn: &Connection, person_id: i64) -> Option<Described> {
    let t = tables::get();
    let person = narrators::describe(conn, person_id)?;
    let (key, tradition, note, source): (String, Option<String>, Option<String>, Option<String>) = conn
        .query_row(
            "SELECT p.key, p.tradition, p.death_year_note, s.name FROM persons p LEFT JOIN sources s ON s.id = p.source_id
             WHERE p.id = ?",
            params![person_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()
        .ok()
        .flatten()?;
    let mut verdicts = Vec::new();
    if let Ok(mut st) = conn.prepare(
        "SELECT critic_name, work, phrase, rank_scheme, rank, tradition, madhhab FROM verdicts WHERE person_id = ? ORDER BY id",
    ) {
        if let Ok(rows) = st.query_map(params![person_id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, Option<i64>>(4)?,
                r.get::<_, Option<String>>(6)?,
            ))
        }) {
            for (critic_name, work, phrase, rank_scheme, rank, madhhab) in rows.flatten() {
                let shia = rank_scheme.as_deref() == Some("shia");
                let rank_label = match rank.filter(|r| *r != 0) {
                    Some(r) if shia => t.rijal.shia_ranks.get(&r.to_string()).cloned().unwrap_or_default(),
                    Some(r) => t.rijal.ranks.get(&r.to_string()).cloned().unwrap_or_default(),
                    None => String::new(),
                };
                let madhhab_label = if shia {
                    t.narrators
                        .madhhabs
                        .get(madhhab.as_deref().unwrap_or(""))
                        .cloned()
                        .unwrap_or_default()
                } else {
                    String::new()
                };
                verdicts.push(FullVerdict {
                    critic_name,
                    work: work.unwrap_or_else(|| "None".into()),
                    phrase,
                    rank_scheme,
                    rank,
                    rank_label,
                    madhhab_label,
                });
            }
        }
    }
    let marks: Vec<(String, String)> = conn
        .prepare("SELECT mark FROM person_marks WHERE person_id = ?")
        .and_then(|mut st| {
            st.query_map(params![person_id], |r| r.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()
        })
        .unwrap_or_default()
        .into_iter()
        .map(|m| {
            let meaning = t
                .narrators
                .book_marks
                .get(&m)
                .cloned()
                .unwrap_or_else(|| m.clone());
            (m, meaning)
        })
        .collect();
    let other_names: Vec<String> = conn
        .prepare("SELECT name FROM person_names WHERE person_id = ? AND kind != 'full' ORDER BY kind, name")
        .and_then(|mut st| {
            st.query_map(params![person_id], |r| r.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()
        })
        .unwrap_or_default();
    let in_chains: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM isnad_links WHERE person_id = ?",
            params![person_id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let tabaqa_label = person
        .tabaqa
        .and_then(|n| t.rijal.tabaqat.get(&n.to_string()).cloned())
        .unwrap_or_default();
    Some(Described {
        person,
        key,
        tradition: tradition.unwrap_or_default(),
        source,
        death_year_note: note,
        tabaqa_label,
        other_names,
        marks,
        verdicts,
        in_chains,
    })
}

pub struct Related {
    pub id: i64,
    pub name: String,
    pub count: i64,
}

pub struct Relations {
    pub teachers: Vec<Related>,
    pub students: Vec<Related>,
    /// (book name, key, hadith)
    pub compilers: Vec<(String, String, i64)>,
}

/// Teachers and students as the imported chains show them (`narrators.relations`).
pub fn relations(conn: &Connection, person_id: i64, limit: i64) -> rusqlite::Result<Relations> {
    let side = |offset: i64| -> rusqlite::Result<Vec<Related>> {
        let mut st = conn.prepare(
            "SELECT o.person_id, p.name_ar, COUNT(*) AS n FROM isnad_links l
               JOIN isnad_links o ON o.isnad_id = l.isnad_id AND o.position = l.position + ?
               JOIN persons p ON p.id = o.person_id
               WHERE l.person_id = ? AND o.person_id IS NOT NULL
               GROUP BY o.person_id ORDER BY n DESC LIMIT ?",
        )?;
        let rows = st
            .query_map(params![offset, person_id, limit], |r| {
                Ok(Related {
                    id: r.get(0)?,
                    name: narrators::display_name(&r.get::<_, String>(1)?),
                    count: r.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    };
    let mut st = conn.prepare(
        "SELECT c.name, c.key, COUNT(DISTINCT i.hadith_id) FROM isnad_links l JOIN isnads i ON i.id = l.isnad_id
           JOIN hadiths h ON h.id = i.hadith_id JOIN collections c ON c.id = h.collection_id
           WHERE l.person_id = ? AND l.position = 1 GROUP BY c.id ORDER BY 3 DESC",
    )?;
    let compilers = st
        .query_map(params![person_id], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Relations {
        teachers: side(1)?,
        students: side(-1)?,
        compilers,
    })
}

pub struct HadithRef {
    pub hadith_id: i64,
    pub key: String,
    pub book: String,
    pub number: String,
}

/// The hadith whose chains include this narrator: (rows, total) (`narrators.hadiths_of`).
pub fn hadiths_of(
    conn: &Connection,
    person_id: i64,
    limit: i64,
    lang: &str,
) -> rusqlite::Result<(Vec<HadithRef>, i64)> {
    let total: i64 = conn.query_row(
        "SELECT COUNT(DISTINCT i.hadith_id) FROM isnad_links l JOIN isnads i ON i.id = l.isnad_id WHERE l.person_id = ?",
        params![person_id],
        |r| r.get(0),
    )?;
    let order = books::order_sql(conn, "c.key")?;
    let mut st = conn.prepare(&format!(
        "SELECT DISTINCT i.hadith_id, c.name, h.number, h.number_sort, c.key FROM isnad_links l
           JOIN isnads i ON i.id = l.isnad_id JOIN hadiths h ON h.id = i.hadith_id
           JOIN collections c ON c.id = h.collection_id
           WHERE l.person_id = ? ORDER BY {order}, h.number_sort LIMIT ?"
    ))?;
    let rows = st
        .query_map(params![person_id, limit], |r| {
            let name: String = r.get(1)?;
            let key: String = r.get(4)?;
            Ok(HadithRef {
                hadith_id: r.get(0)?,
                book: works::book_title(&key, &name, lang),
                key,
                number: r.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok((rows, total))
}

// ------------------------------------------------------------------ the famous ones (famous.py)

/// A name as one string of letters, as `famous._compact` folds it.
fn compact(text: &str) -> String {
    let f = &tables::get().narrators.famous;
    static HON: OnceLock<fancy_regex::Regex> = OnceLock::new();
    static HON_AR: OnceLock<Regex> = OnceLock::new();
    static IBN_I: OnceLock<Regex> = OnceLock::new();
    static ABD: OnceLock<Regex> = OnceLock::new();
    if arabic_letter(text) {
        let honour_ar = HON_AR.get_or_init(|| re(&f.honour_ar));
        let t = honour_ar.replace_all(&normalize(text), " ").into_owned();
        return t
            .replace("ابن", "بن")
            .split_whitespace()
            .map(|w| {
                if w.chars().count() > 4 {
                    w.strip_prefix("ال").unwrap_or(w).to_string()
                } else {
                    w.to_string()
                }
            })
            .collect();
    }
    let honour =
        HON.get_or_init(|| fancy_regex::Regex::new(&format!("(?i){}", f.honour)).expect("honour"));
    let t = honour.replace_all(&split_abd(text), " ").into_owned();
    let t = IBN_I
        .get_or_init(|| re(r"(?i)\b(ibn|bin)-i\b"))
        .replace_all(&t, "$1")
        .into_owned()
        .replace('-', " ")
        .replace('’', "'");
    let mut words: Vec<String> = Vec::new();
    for word in t.split_whitespace() {
        let bare = lower_tr(py::strip(word, ".")).replace('ı', "i");
        if f.ibn.contains(&bare) {
            words.push("b".into());
            continue;
        }
        let folded = fold_latin(word);
        if [
            "al", "el", "es", "as", "ad", "ed", "an", "en", "ar", "er", "at", "et", "az", "ez",
            "ash", "ul", "ud", "us", "ur", "un",
        ]
        .contains(&folded.as_str())
        {
            continue;
        }
        words.push(folded.replace(' ', ""));
    }
    let mut w = words.concat();
    for (a, b) in [
        ("kh", "h"),
        ("sh", "s"),
        ("th", "s"),
        ("dh", "z"),
        ("gh", "g"),
        ("p", "b"),
        ("ou", "u"),
        ("aa", "a"),
        ("ee", "i"),
        ("oo", "u"),
    ] {
        w = w.replace(a, b);
    }
    let w = single(&w);
    let mut w = ABD
        .get_or_init(|| re(r"abd[aiu]?lah?"))
        .replace_all(&w, "abdla")
        .into_owned();
    if w.chars().count() > 3 && w.ends_with('h') {
        w.pop();
    }
    w
}

fn skeleton(compact: &str) -> String {
    if arabic_letter(compact) {
        compact.to_string()
    } else {
        compact
            .chars()
            .filter(|c| !"aeiouyw'".contains(*c))
            .collect()
    }
}

/// The keys of the famous ones the words typed name, the closest first (`famous.named`).
pub fn famous_named(text: &str, tradition: Option<&str>) -> Vec<String> {
    let f = &tables::get().narrators.famous;
    let wanted = compact(text);
    if wanted.chars().count() < 3 || f.too_short.contains(&wanted) {
        return Vec::new();
    }
    let keys: Vec<&[String; 4]> = f
        .keys
        .iter()
        .filter(|k| tradition.map_or(true, |t| k[1] == t))
        .collect();
    let found = |pairs: Vec<&String>| -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for k in pairs {
            if !out.contains(k) {
                out.push(k.clone());
            }
        }
        out
    };
    let mut hits: Vec<&String> = keys
        .iter()
        .filter(|k| k[2] == wanted)
        .map(|k| &k[0])
        .collect();
    hits.extend(
        keys.iter()
            .filter(|k| k[2].starts_with(wanted.as_str()))
            .map(|k| &k[0]),
    );
    if !hits.is_empty() {
        return found(hits);
    }
    if wanted.chars().count() >= 4 {
        let needle = format!("b{wanted}");
        let sons: Vec<&String> = keys
            .iter()
            .filter(|k| k[2].contains(&needle))
            .map(|k| &k[0])
            .collect();
        if !sons.is_empty() {
            return found(sons);
        }
    }
    if arabic_letter(&wanted) {
        return Vec::new();
    }
    let skel = skeleton(&wanted);
    if skel.is_empty() {
        return Vec::new();
    }
    let first = wanted.chars().next();
    found(
        keys.iter()
            .filter(|k| k[3] == skel && k[2].chars().next() == first)
            .map(|k| &k[0])
            .collect(),
    )
}

/// The persons' row ids for the keys, in the same order.
pub fn famous_ids(conn: &Connection, keys: &[String]) -> Vec<i64> {
    let mut found: HashMap<String, i64> = HashMap::new();
    for k in keys {
        if let Ok(Some(id)) = conn
            .query_row("SELECT id FROM persons WHERE key = ?", params![k], |r| {
                r.get::<_, i64>(0)
            })
            .optional()
        {
            found.insert(k.clone(), id);
        }
    }
    keys.iter().filter_map(|k| found.get(k).copied()).collect()
}

fn person_of(key: &str) -> Option<&'static [String; 6]> {
    tables::get()
        .narrators
        .famous
        .people
        .iter()
        .find(|p| p[0] == key)
}

/// The name to show for a famous one: "Abu Bakr al-Siddiq (RA)", "Hz. Ebû Bekir es-Sıddîk (r.a.)".
pub fn famous_title(key: &str, language: &str) -> String {
    let Some(p) = person_of(key) else {
        return key.to_string();
    };
    let mut name = match language {
        "tr" => p[1].clone(),
        "ar" => p[3].clone(),
        _ => p[2].clone(),
    };
    let kind = p[4].as_str();
    let extra = if kind.is_empty() {
        ""
    } else if language == "ar" {
        match kind {
            "m" => "رضي الله عنه",
            "f" => "رضي الله عنها",
            _ => "رضي الله عنهما",
        }
    } else if language == "tr" {
        "(r.a.)"
    } else {
        "(RA)"
    };
    if language == "tr" && !kind.is_empty() {
        name = format!("Hz. {name}");
    }
    format!("{name} {extra}").trim().to_string()
}

/// Row id → the name to show, for every famous one in the database.
pub fn famous_titles(conn: &Connection, language: &str) -> HashMap<i64, String> {
    let keys: Vec<String> = tables::get()
        .narrators
        .famous
        .people
        .iter()
        .map(|p| p[0].clone())
        .collect();
    keys.iter()
        .zip(famous_ids_all(conn, &keys))
        .filter_map(|(k, id)| id.map(|i| (i, famous_title(k, language))))
        .collect()
}

fn famous_ids_all(conn: &Connection, keys: &[String]) -> Vec<Option<i64>> {
    keys.iter()
        .map(|k| {
            conn.query_row("SELECT id FROM persons WHERE key = ?", params![k], |r| {
                r.get::<_, i64>(0)
            })
            .optional()
            .ok()
            .flatten()
        })
        .collect()
}

pub struct Group {
    pub id: String,
    pub title: String,
    pub tradition: String,
    pub keys: Vec<String>,
}

/// The groups of a tradition (both when None), Sunni first.
pub fn famous_groups(tradition: Option<&str>) -> Vec<Group> {
    tables::get()
        .narrators
        .famous
        .groups
        .iter()
        .filter(|g| tradition.map_or(true, |t| g.2 == t))
        .map(|g| Group {
            id: g.0.clone(),
            title: g.1.clone(),
            tradition: g.2.clone(),
            keys: g.3.clone(),
        })
        .collect()
}

/// The Arabic name of a famous one, as the list shows it.
pub fn famous_arabic(key: &str) -> String {
    person_of(key).map(|p| p[3].clone()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spellings() {
        assert_eq!(latin_skeleton("Hurayra"), "hrr");
        assert_eq!(arabic_skeleton("الزهري"), "zhry");
        assert_eq!(split_abd("Abdurrahman"), "Abd rrahman");
        assert_eq!(fold_latin("Âişe"), "aisha");
        assert!(famous_named("Ebubekir", None).contains(&"taqrib:3467".to_string()));
        assert!(famous_named("Omer ibn Hattap", None).contains(&"taqrib:4888".to_string()));
    }
}
