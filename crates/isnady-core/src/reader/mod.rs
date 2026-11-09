//! The chain reader (RS17): the Python isnady's `core/isnad.py` parser (isnady-isnad-15), word for word — the chains
//! read from the Arabic texts with the same patterns (data/isnad_parser.json, written from the Python isnady), the
//! same roads, the same links, so that a database read by either program is the same row for row.

#[macro_use]
pub mod pyre;
pub mod matcher;
pub mod shia;
pub mod store;
pub mod sunni;

use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use crate::normalize::{normalize, normalize_with_map};
use pyre::{split, strip, Ps, Re};

#[derive(Deserialize)]
pub struct Tables {
    pub parser_id: String,
    pub patterns: HashMap<String, String>,
    pub terms: Vec<(String, String)>,
    pub shia_terms: Vec<String>,
    pub term_id: HashMap<String, String>,
    pub parallel_terms: Vec<String>,
    pub max_name_words: usize,
    pub same_person: Vec<Vec<String>>,
    pub waw_names: HashSet<String>,
    pub others: Vec<String>,
    pub kin_words: HashSet<String>,
    pub compiler_teachers: Vec<String>,
    pub description_words: Vec<String>,
    pub partial_upgrades: HashMap<String, String>,
    pub shia_lead: String,
    pub citing_books: HashSet<String>,
    pub direct_books: HashSet<String>,
    pub split_books: HashSet<String>,
    pub problems: HashMap<String, String>,
    pub name_words: HashSet<String>,
    pub masum_names: Vec<(String, String)>,
    pub always_masum: Vec<String>,
}

pub fn tables() -> &'static Tables {
    static T: OnceLock<Tables> = OnceLock::new();
    T.get_or_init(|| {
        serde_json::from_str(include_str!("../../data/isnad_parser.json"))
            .expect("data/isnad_parser.json is written by tools/gen_tables.py")
    })
}

/// One of the module's patterns, by its Python name ("_TERM_RE").
pub fn p(name: &str) -> &'static Re {
    static ALL: OnceLock<HashMap<String, Re>> = OnceLock::new();
    let all = ALL.get_or_init(|| {
        tables()
            .patterns
            .iter()
            .map(|(k, v)| (k.clone(), Re::new(v)))
            .collect()
    });
    all.get(name)
        .unwrap_or_else(|| panic!("no pattern {name} in isnad_parser.json"))
}

pub fn problem(key: &str) -> &'static str {
    tables().problems.get(key).map(|s| s.as_str()).unwrap_or("")
}

pub fn term_id(term: &str) -> &'static str {
    tables().term_id.get(term).map(|s| s.as_str()).unwrap_or("")
}

#[derive(Debug, Clone, PartialEq)]
pub struct Link {
    pub position: usize,
    pub term: String,
    pub raw_name: String,
    pub fresh: bool,
}

impl Link {
    pub fn new(position: usize, term: &str, raw_name: &str, fresh: bool) -> Link {
        Link {
            position,
            term: term.to_string(),
            raw_name: raw_name.to_string(),
            fresh,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Parsed {
    pub raw_isnad: String,
    pub links: Vec<Link>,
    pub reaches_prophet: bool,
    pub problem: String,
    pub ends_with: String,
    pub raised: bool,
    pub joined: i64,
    pub alternates: Vec<(usize, Link)>,
    pub parallel_at: i64,
    pub parallel_kind: String,
    pub others: Vec<Parsed>,
}

impl Default for Parsed {
    fn default() -> Self {
        Parsed {
            raw_isnad: String::new(),
            links: vec![],
            reaches_prophet: false,
            problem: String::new(),
            ends_with: String::new(),
            raised: false,
            joined: 0,
            alternates: vec![],
            parallel_at: -1,
            parallel_kind: String::new(),
            others: vec![],
        }
    }
}

impl Parsed {
    pub fn with_problem(p: &str) -> Parsed {
        Parsed {
            problem: p.to_string(),
            ..Default::default()
        }
    }
}

// ------------------------------------------------------------------ small helpers of the module

pub(crate) fn norm_name(s: &str) -> String {
    normalize(s)
}

/// `_norm_map`: the normal form for reading a Sunni chain, every position mapping back to the text.
pub(crate) fn norm_map(text: &str) -> (Ps, Vec<usize>) {
    let t = p("_TATWEEL_DASH_RE").sub(text, "-");
    let (norm, map) = normalize_with_map(&t);
    let norm = p("_NAME_COMMA_RE").sub_with(&norm, |m| {
        if m.has(1) && !m.g(1).is_empty() {
            format!("{} ", m.g(1))
        } else if m.has(2) && !m.g(2).is_empty() {
            format!("{} ", m.g(2))
        } else {
            " ".to_string()
        }
    });
    let norm = p("_BLESSING_DASHES_RE").sub_with(&norm, |m| format!(" {} ", m.g(1)));
    let norm = p("_BLANK_RE").sub_with(&norm, |m| " ".repeat(pyre::len(m.group0())));
    (Ps::new(&norm), map)
}

pub(crate) fn tahwil_before(norm: &Ps, end: usize) -> bool {
    p("_TAHWIL_RE")
        .finditer_in(norm, 0, norm.len())
        .iter()
        .any(|m| m.start() < end)
}

pub(crate) fn tahwil_at(norm: &Ps, mut pos: usize) -> bool {
    while pos < norm.len() && " \t،,".contains(norm.at(pos).unwrap()) {
        pos += 1;
    }
    pos < norm.len()
        && norm.at(pos) == Some('ح')
        && (pos == 0 || !pyre::isalnum(norm.at(pos - 1).unwrap()))
        && (pos + 1 >= norm.len() || !pyre::isalnum(norm.at(pos + 1).unwrap()))
}

pub(crate) fn clean_name(original: &str) -> String {
    pyre::strip_ws(strip(original, " \t\n\u{200f}\u{200e}،,:;.-\"'()")).to_string()
}

/// `_orig_len`: how many characters of the text make the first `norm_len` characters of its normal form.
pub(crate) fn orig_len(text: &str, norm_len: usize) -> usize {
    let (_norm, map) = normalize_with_map(text);
    if norm_len < map.len() {
        map[norm_len]
    } else {
        pyre::len(text)
    }
}

pub(crate) fn words(name: &str) -> usize {
    split(name)
        .iter()
        .filter(|w| !["بن", "ابن", "بنت"].contains(w))
        .count()
}

pub(crate) fn name_key(raw: &str) -> String {
    let name = norm_name(raw);
    let name = p("_JOINED_RE").sub(&name, " ");
    let name = name.replace('،', " ").replace(',', " ");
    split(&name).join(" ")
}

pub(crate) fn same_narrator(a: &str, b: &str) -> bool {
    let (a, b) = (name_key(a), name_key(b));
    if a.is_empty() || b.is_empty() {
        return false;
    }
    if a == b {
        return true;
    }
    let bare = pre!(r"(?<=\w\w)ا$");
    let (ab, bb) = (bare.sub(&a, ""), bare.sub(&b, ""));
    if ab == bb && !ab.contains(' ') {
        return true;
    }
    let plain = |x: &str| -> String {
        let x = pre!(r"^(?:ابي|ابا) ").sub(x, "ابو ");
        if !x.contains(' ') && x.starts_with("ال") && pyre::len(&x) > 4 {
            pyre::from(&x, 2).to_string()
        } else {
            x
        }
    };
    if plain(&a) == plain(&b) {
        return true;
    }
    let named = |x: &str, g: &str| -> bool {
        x == g
            || (x.starts_with(&format!("{g} "))
                && !pre!(r"(?:بن|بنت|ابن) ").is_match(pyre::from(x, pyre::len(g) + 1)))
    };
    for group in &tables().same_person {
        if group.iter().any(|g| named(&a, g)) && group.iter().any(|g| named(&b, g)) {
            return true;
        }
    }
    let (short, long) = if pyre::len(&b) < pyre::len(&a) {
        (&b, &a)
    } else {
        (&a, &b)
    };
    pyre::len(short) >= 4
        && long.starts_with(&format!("{short} "))
        && !format!("{short} ").contains(" بن ")
        && !long.starts_with(&format!("{short} بنت "))
}

/// `_strip_said`: a name without what follows it in the text.
pub(crate) fn strip_said(raw: &str) -> String {
    let name = p("_OF_PROPHET_RE").sub(&norm_name(raw), " ");
    let name = pre!(r"(?<!\w)(?:حدث\w*|اخبر\w*|يحدث\w*|انه|انها|قال|قالت|يقول|تقول|رضي الله.*)$")
        .sub(&name, " ");
    strip(&name, " ،,-").to_string()
}

pub(crate) fn is_companion_name(raw: &str) -> bool {
    crate::isnad::is_companion_name(raw)
}

pub(crate) fn is_companion(link: &Link) -> bool {
    is_companion_name(&strip_said(&link.raw_name))
}

/// `_shared`: one narrator named in two roads.
pub(crate) fn shared(a: &str, b: &str) -> bool {
    let (an, bn) = (name_key(&strip_said(a)), name_key(&strip_said(b)));
    let rel = p("_RELATIVE_RE");
    if an.is_empty() || bn.is_empty() || rel.is_match(&an) || rel.is_match(&bn) {
        return false;
    }
    if same_narrator(&an, &bn) {
        return true;
    }
    let (short, long) = if pyre::len(&bn) < pyre::len(&an) {
        (&bn, &an)
    } else {
        (&an, &bn)
    };
    if !short.contains(' ') && pyre::len(short) >= 3 && long.starts_with(&format!("{short} بن "))
    {
        return true;
    }
    let rest = format!("{} ", pyre::from(long, pyre::len(short)));
    if short.matches(" بن ").count() >= 2
        && long.starts_with(&format!("{short} "))
        && !rest.contains(" بن ")
    {
        return true;
    }
    // re.fullmatch(re.escape(short) + r" ال\w+", long)
    short.matches(" بن ").count() == 1
        && long
            .strip_prefix(short.as_str())
            .and_then(|r| r.strip_prefix(" ال"))
            .is_some_and(|r| !r.is_empty() && r.chars().all(|c| pyre::isalnum(c) || c == '_'))
}

pub(crate) fn name_words() -> &'static HashSet<String> {
    &tables().name_words
}

/// `_looks_named`.
pub(crate) fn looks_named(raw: &str, most: usize) -> bool {
    let n = norm_name(raw).replace('،', " ");
    let ws = split(&n);
    if ws.is_empty() || words(&ws.join(" ")) > most {
        return false;
    }
    let mut first = ws[0].to_string();
    if let Some(rest) = first.strip_prefix('و') {
        if name_words().contains(rest) {
            first = rest.to_string();
        }
    }
    if pre!(r"اب[ويا]\S{2,}").is_match(&first) && !["ابن", "ابي"].contains(&pyre::upto(&first, 3))
    {
        first = pyre::upto(&first, 3).to_string();
    }
    name_words().contains(&first)
        || tables().kin_words.contains(&first)
        || is_companion_name(&strip_said(raw))
}

/// `_name_prefix`.
pub(crate) fn name_prefix(norm: &Ps, start: usize, end: usize) -> Option<usize> {
    let seg = Ps::new(norm.sl(start, end));
    let spans = pre!(r"[^\s،,]+").finditer_in(&seg, 0, seg.len());
    let joiners = ["بن", "ابن", "بنت", "ابو", "ابي", "ابا", "ام", "عبد", "يعني"];
    let (mut k, mut nisbas) = (0usize, 0usize);
    while k < spans.len() {
        let w = spans[k].group0();
        if k == 0 || joiners.contains(&spans[k - 1].group0()) {
            k += 1;
            continue;
        }
        if ["بن", "ابن", "بنت", "ابو", "ابي", "ابا", "ام", "يعني"].contains(&w)
        {
            k += 1;
            continue;
        }
        if w.starts_with("ال")
            && nisbas < 2
            && pyre::len(w) > 3
            && !["الذي", "التي", "الذين"].contains(&w)
        {
            nisbas += 1;
            k += 1;
            continue;
        }
        break;
    }
    if k == 0 {
        return None;
    }
    if k >= spans.len() {
        return Some(start + spans.last().unwrap().end());
    }
    if joiners.contains(&spans[k - 1].group0()) {
        k -= 1;
    }
    if k > 0 {
        Some(start + spans[k - 1].end())
    } else {
        None
    }
}

/// `_companion_prefix`.
pub(crate) fn companion_prefix(norm: &Ps, start: usize, end: usize) -> Option<usize> {
    let seg = Ps::new(norm.sl(start, end.min(start + 80)));
    let spans = pre!(r"[^\s،,]+").finditer_in(&seg, 0, seg.len());
    for k in [7usize, 6, 5, 4, 3, 2, 1] {
        if spans.len() > k {
            let name = norm.sl(start, start + spans[k - 1].end());
            if is_companion_name(name) && !p("_SENTENCE_START_RE").is_match(name) {
                return Some(start + spans[k - 1].end());
            }
        }
    }
    None
}

// ------------------------------------------------------------------ the Ma'sumun (core/traditions.py)

/// `traditions.which_masum`.
pub(crate) fn which_masum(name: &str) -> String {
    let n = pre!(r"[()\[\]]").sub(&normalize(name), " ");
    let n = pre!(r"\s+").sub(&n, " ");
    let n = strip(&n, " ،,:");
    for (form, key) in &tables().masum_names {
        if n == form || n.starts_with(&format!("{form} ")) {
            return key.clone();
        }
    }
    String::new()
}

/// `traditions.masum_in`.
pub(crate) fn masum_in(text: &str) -> String {
    static FORMS: OnceLock<Vec<(Re, usize, String)>> = OnceLock::new();
    let forms = FORMS.get_or_init(|| {
        let mut v: Vec<(Re, usize, String)> = tables()
            .masum_names
            .iter()
            .map(|(f, k)| {
                (
                    Re::new(&format!(r"(?<!\w)(?:ل|ب|و|ف)?{f}(?!\w)")),
                    pyre::len(f),
                    k.clone(),
                )
            })
            .collect();
        v.sort_by_key(|x| std::cmp::Reverse(x.1));
        v
    });
    let n = pre!(r"[()\[\]،,:]").sub(&normalize(text), " ");
    let t = Ps::new(&n);
    let mut best: Option<(usize, usize, &String)> = None;
    for (rx, len, key) in forms {
        if let Some(m) = rx.search_at(&t, 0) {
            if best.map_or(true, |b| {
                m.start() < b.0 || (m.start() == b.0 && *len > b.1)
            }) {
                best = Some((m.start(), *len, key));
            }
        }
    }
    best.map(|b| b.2.clone()).unwrap_or_default()
}

// ------------------------------------------------------------------ the roads

/// `parse`: the first road of a text.
pub fn parse(text: &str, tradition: Option<&str>) -> Parsed {
    if tradition == Some("shia") {
        return shia::parse_shia(text);
    }
    sunni::parse_all_sunni(text, 0).remove(0)
}

/// `parse_all`: every chain at the head of the text.
pub fn parse_all(text: &str, tradition: Option<&str>) -> Vec<Parsed> {
    if tradition == Some("shia") {
        let mut first = shia::parse_shia(text);
        let others = std::mem::take(&mut first.others);
        let mut out = vec![first];
        out.extend(others);
        return out;
    }
    sunni::parse_all_sunni(text, 0)
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_pattern_compiles() {
        for name in super::tables().patterns.keys() {
            super::p(name);
        }
    }
}
