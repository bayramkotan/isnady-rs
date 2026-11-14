//! Who is who in a chain (`link_narrators` of core/narrators.py, isnady-narrators-19, with core/tahwil_meet.py): the
//! names read from the chains linked to the narrators of the rijal works by the same evidence, in the same order —
//! the name, the book's mark, the generations, the relatives, the teacher and the student, the client, the bare
//! name, the famous one — so that both programs identify the same narrator at every link.

use rusqlite::{params, Connection, OptionalExtension};
use serde::Deserialize;
use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::rc::Rc;
use std::sync::OnceLock;

use super::pyre::{self, split, strip, Re};
use super::*;
use crate::normalize::normalize;

#[derive(Deserialize)]
pub struct Matcher {
    pub id: String,
    pub death_rule: String,
    pub teacher_min_tabaqa: i64,
    pub patterns: HashMap<String, String>,
    pub relative: HashMap<String, String>,
    pub relative_words: Vec<String>,
    pub kin_first_person: Vec<String>,
    pub edge: (i64, f64, f64),
    pub one_side: (i64, f64),
    pub famous_confidence: f64,
    pub guess_confidence: f64,
    pub trust: f64,
    pub gaps: (i64, i64, i64),
    pub mubhamat: Vec<(i64, i64)>,
    pub muhmal: HashMap<String, Vec<(String, Vec<String>, Vec<String>, Vec<String>)>>,
    pub known_as: HashMap<String, Vec<(String, Vec<String>)>>,
    pub famous: HashMap<String, Vec<(String, String)>>,
    pub authors: HashMap<String, String>,
    pub collection_marks: HashMap<String, Vec<String>>,
    pub group_forms: Vec<String>,
    pub previous_forms: Vec<String>,
    pub vague_forms: Vec<String>,
    pub compiler_teachers: HashMap<String, HashMap<String, String>>,
    pub min_attested: usize,
    pub shia_tiers: HashMap<String, i64>,
    pub spelling_start: HashSet<String>,
    pub spelling: HashSet<String>,
    pub father: Father,
}

/// The father read from the son's nasab and the son with his father (0.7.7).
#[derive(Deserialize)]
pub struct Father {
    pub not_his_kunya: HashSet<String>,
    pub city_nisbas: HashSet<String>,
    pub gap: i64,
    pub pair: (i64, usize),
    pub rasm: (String, String),
}

#[derive(Deserialize)]
struct Holder {
    matcher: Matcher,
}

pub fn m() -> &'static Matcher {
    static T: OnceLock<Matcher> = OnceLock::new();
    T.get_or_init(|| {
        let h: Holder = serde_json::from_str(include_str!("../../data/isnad_parser.json"))
            .expect("isnad_parser.json");
        h.matcher
    })
}

fn mp(name: &str) -> &'static Re {
    static ALL: OnceLock<HashMap<String, Re>> = OnceLock::new();
    ALL.get_or_init(|| {
        m().patterns
            .iter()
            .map(|(k, v)| (k.clone(), Re::new(v)))
            .collect()
    })
    .get(name)
    .unwrap()
}

type Set = BTreeSet<i64>;
/// (person, confidence, candidates)
pub type Res = (Option<i64>, Option<f64>, i64);

/// `tokens`.
pub fn tokens(name: &str) -> Vec<String> {
    let w = normalize(name)
        .replace('،', " ")
        .replace(',', " ")
        .replace("ابن ", "بن ");
    split(&w)
        .into_iter()
        .map(|x| {
            if x == "ابا" || x == "ابي" {
                "ابو".to_string()
            } else {
                x.to_string()
            }
        })
        .collect()
}

/// `without_spelling`: a rijal entry's name without Ibn Hajar's words on how it is spelt ("سماك بكسر اوله وتخفيف
/// الميم بن حرب" is "سماك بن حرب").
pub fn without_spelling(toks: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut inside = false;
    for w in toks {
        if m().spelling_start.contains(w) {
            inside = true;
            continue;
        }
        if inside && m().spelling.contains(w) {
            continue;
        }
        inside = false;
        out.push(w.clone());
    }
    out
}

fn chars(w: &str) -> usize {
    w.chars().count()
}

fn is_subsequence(short: &[String], long: &[String]) -> bool {
    let mut it = long.iter();
    short.iter().all(|w| it.any(|x| x == w))
}

pub struct Index {
    pub tradition: String,
    pub tabaqa: HashMap<i64, Option<i64>>,
    pub marks: HashMap<i64, HashSet<String>>,
    pub full: HashMap<i64, Vec<String>>,
    pub muhmal: HashMap<Vec<String>, Vec<(i64, HashSet<Vec<String>>, HashSet<Vec<String>>)>>,
    pub names: HashMap<String, Vec<(i64, Vec<String>, bool)>>,
    pub exact: HashMap<String, Set>,
    pub death: HashMap<i64, i64>,
    pub famous: HashMap<String, i64>,
    pub authors: HashMap<String, i64>,
    /// the Shia rijal work a person is from, in the order a name is looked up in them (al-Najashi 0)
    pub tier: HashMap<i64, i64>,
    /// never a candidate: the unnamed of the Taqrib, an entry repeated in al-Tusi's Rijal
    hidden: HashSet<i64>,
    /// `full` in the persons' order, for the names looked up inside the entries (`_elsewhere`)
    order: Vec<i64>,
    found: RefCell<HashMap<Vec<String>, Rc<Set>>>,
    /// every narrator by the kunya his entry gives him (`_kunya_index`), made once
    kunyas: RefCell<Option<Rc<HashMap<String, Set>>>>,
    /// the father of each narrator asked for (`_father_of`)
    fathers: RefCell<HashMap<i64, Option<i64>>>,
    /// 0.7.8: isnady's own inference asked for (Tools → Narrator guesses): the lists, the names it knows them by,
    /// a name looked up inside the entries, the famous one — the links it decides marked
    pub guess: bool,
    /// names whose candidates came from `elsewhere`
    guessed_keys: RefCell<HashSet<Vec<String>>>,
    /// names isnady's list gives a narrator the rijal work does not
    known_as_added: HashMap<String, Set>,
}

impl Index {
    pub fn new(conn: &Connection, tradition: &str, guess: bool) -> rusqlite::Result<Index> {
        let mut ix = Index {
            tradition: tradition.into(),
            tabaqa: HashMap::new(),
            marks: HashMap::new(),
            full: HashMap::new(),
            muhmal: HashMap::new(),
            names: HashMap::new(),
            exact: HashMap::new(),
            death: HashMap::new(),
            famous: HashMap::new(),
            authors: HashMap::new(),
            tier: HashMap::new(),
            hidden: HashSet::new(),
            order: Vec::new(),
            found: RefCell::new(HashMap::new()),
            kunyas: RefCell::new(None),
            fathers: RefCell::new(HashMap::new()),
            guess,
            guessed_keys: RefCell::new(HashSet::new()),
            known_as_added: HashMap::new(),
        };
        let mut unnamed: HashSet<i64> = HashSet::new();
        let mut repeated: HashSet<i64> = HashSet::new();
        let mut seen_in: HashSet<(String, Vec<String>)> = HashSet::new();
        let mut stmt = conn.prepare(
            "SELECT id, tabaqa, name_ar, key, death_year_ah FROM persons WHERE COALESCE(tradition, 'sunni') = ? \
             ORDER BY id",
        )?;
        let rows = stmt.query_map([tradition], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, Option<i64>>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, Option<i64>>(4)?,
            ))
        })?;
        let mut keys: HashMap<String, i64> = HashMap::new();
        for row in rows {
            let (pid, mut tabaqa, name, key, died) = row?;
            if let Some(d) = died.filter(|d| *d != 0) {
                ix.death.insert(pid, d);
            }
            let name = name.unwrap_or_default();
            let prefix = key
                .as_deref()
                .unwrap_or("")
                .split(':')
                .next()
                .unwrap_or("")
                .to_string();
            let tier = m().shia_tiers.get(&prefix).copied();
            ix.tier.insert(pid, tier.unwrap_or(0));
            if tier.is_some() && !seen_in.insert((prefix, tokens(&name))) {
                repeated.insert(pid);
            }
            if tabaqa.is_none() && tradition == "sunni" && !name.is_empty() {
                let plain = normalize(&name);
                if mp("_COMPANION").is_search(&plain) && !mp("_UNSURE").is_search(&plain) {
                    tabaqa = Some(1);
                }
            }
            ix.tabaqa.insert(pid, tabaqa);
            ix.full.insert(pid, without_spelling(&tokens(&name)));
            ix.order.push(pid);
            if let Some(k) = &key {
                if let Some(num) = k.strip_prefix("taqrib:") {
                    if !num.is_empty() && num.chars().all(|c| c.is_ascii_digit()) {
                        let v: i64 = num.parse().unwrap_or(-1);
                        if m().mubhamat.iter().any(|(lo, hi)| *lo <= v && v <= *hi) {
                            unnamed.insert(pid);
                        }
                    }
                }
                keys.insert(k.clone(), pid);
            }
        }
        let lists: &[(String, Vec<String>, Vec<String>, Vec<String>)] = if guess {
            &m().muhmal[if tradition == "sunni" {
                "sunni"
            } else {
                "shia"
            }]
        } else {
            &[]
        };
        for (key, names, teachers, students) in lists {
            if let Some(pid) = keys.get(key) {
                for name in names {
                    ix.muhmal.entry(tokens(name)).or_default().push((
                        *pid,
                        teachers.iter().map(|x| tokens(x)).collect(),
                        students.iter().map(|x| tokens(x)).collect(),
                    ));
                }
            }
        }
        let mut stmt = conn.prepare("SELECT person_id, mark FROM person_marks")?;
        for row in stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))? {
            let (pid, mark) = row?;
            ix.marks.entry(pid).or_default().insert(mark);
        }
        ix.hidden = unnamed.union(&repeated).copied().collect();
        let mut stmt = conn.prepare(
            "SELECT n.person_id, n.name, n.kind FROM person_names n JOIN persons p ON p.id = n.person_id \
             WHERE COALESCE(p.tradition, 'sunni') = ?",
        )?;
        for row in stmt.query_map([tradition], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
            ))
        })? {
            let (pid, name, kind) = row?;
            if unnamed.contains(&pid) || repeated.contains(&pid) {
                continue;
            }
            let toks = without_spelling(&tokens(&name));
            let full_kind = kind.as_deref() == Some("full");
            if !full_kind && toks.iter().skip(1).any(|t| t == "عن") {
                continue;
            }
            if !toks.is_empty() {
                ix.names
                    .entry(toks[0].clone())
                    .or_default()
                    .push((pid, toks.clone(), !full_kind));
                ix.exact.entry(toks.join(" ")).or_default().insert(pid);
                if tradition == "shia" && full_kind {
                    if let Some(i) = toks.iter().skip(1).position(|t| t == "بن").map(|i| i + 1) {
                        ix.names.entry("بن".into()).or_default().push((
                            pid,
                            toks[i..].to_vec(),
                            true,
                        ));
                    }
                }
            }
        }
        let road = if tradition == "sunni" {
            "sunni"
        } else {
            "shia"
        };
        if guess {
            for (written, owners) in &m().known_as[road] {
                for key in owners {
                    if let Some(pid) = keys.get(key) {
                        if ix.exact.entry(written.clone()).or_default().insert(*pid) {
                            ix.known_as_added
                                .entry(written.clone())
                                .or_default()
                                .insert(*pid);
                        }
                    }
                }
            }
            ix.famous = m().famous[road]
                .iter()
                .filter_map(|(w, k)| keys.get(k).map(|p| (w.clone(), *p)))
                .collect();
        }
        if tradition == "sunni" {
            ix.authors = m()
                .authors
                .iter()
                .filter_map(|(b, k)| keys.get(k).map(|p| (b.clone(), *p)))
                .collect();
        }
        Ok(ix)
    }

    pub fn t(&self, p: i64) -> Option<i64> {
        self.tabaqa.get(&p).copied().flatten()
    }

    pub fn candidates(&self, toks: &[String]) -> Rc<Set> {
        if toks.is_empty() {
            return Rc::new(Set::new());
        }
        if let Some(hit) = self.found.borrow().get(toks) {
            return hit.clone();
        }
        let mut found: Set = self.exact.get(&toks.join(" ")).cloned().unwrap_or_default();
        for variant in article_variants(toks) {
            found.extend(self.rule_matches(&variant));
        }
        if found.is_empty() && toks.iter().any(|w| chars(w) > 3 && w.ends_with('ا')) {
            // "سمعت أنسا": the name after a verb, with the alif of the accusative
            let bare: Vec<String> = toks
                .iter()
                .map(|w| {
                    if chars(w) > 3 && w.ends_with('ا') {
                        w[..w.len() - 'ا'.len_utf8()].to_string()
                    } else {
                        w.clone()
                    }
                })
                .collect();
            found = self.exact.get(&bare.join(" ")).cloned().unwrap_or_default();
            for variant in article_variants(&bare) {
                found.extend(self.rule_matches(&variant));
            }
        }
        if found.is_empty() && !toks.iter().any(|t| t == "بن") && toks.len() <= 4 && self.guess {
            found = self.elsewhere(toks);
            if !found.is_empty() {
                self.guessed_keys.borrow_mut().insert(toks.to_vec());
            }
        }
        if found.len() > 1 {
            // the earliest rijal work that has him (al-Najashi, then al-Tusi's Fihrist, then his Rijal)
            let tier = |p: &i64| self.tier.get(p).copied().unwrap_or(0);
            let first = found.iter().map(tier).min().unwrap_or(0);
            found.retain(|p| tier(p) == first);
        }
        let hit = Rc::new(found);
        self.found.borrow_mut().insert(toks.to_vec(), hit.clone());
        hit
    }

    /// `_elsewhere`: a name without "بن" that begins no entry — a kunya, a nisba, an ism with his nisba — found
    /// inside the entries' names.
    fn elsewhere(&self, toks: &[String]) -> Set {
        let mut found = Set::new();
        // (lead, rest): the kunya's two words as written; [] for a lone nisba; None for an ism and his nisba
        let (lead, rest): (Option<&[String]>, &[String]) = if toks[0] == "ابو" || toks[0] == "ام"
        {
            if toks.len() < 2 {
                return found;
            }
            (Some(&toks[..2]), &toks[2..])
        } else if toks.len() == 1 {
            if !(toks[0].starts_with("ال") && chars(&toks[0]) > 4) {
                return found;
            }
            (Some(&toks[..0]), toks)
        } else {
            (None, &toks[1..])
        };
        for pid in &self.order {
            if self.hidden.contains(pid) {
                continue;
            }
            let name = &self.full[pid];
            let hit = match lead {
                None => {
                    name.first() == toks.first()
                        && is_subsequence(rest, name.get(1..).unwrap_or(&[]))
                }
                Some(l) if !l.is_empty() => (0..name.len().saturating_sub(1)).any(|i| {
                    name[i] == l[0] && name[i + 1] == l[1] && is_subsequence(rest, &name[i + 2..])
                }),
                Some(_) => is_subsequence(rest, name.get(1..).unwrap_or(&[])),
            };
            if hit {
                found.insert(*pid);
            }
        }
        found
    }

    fn rule_matches(&self, toks: &[String]) -> Set {
        let mut found: Set = self.exact.get(&toks.join(" ")).cloned().unwrap_or_default();
        let head: &[String] = match toks.iter().position(|t| t == "بن") {
            Some(i) => &toks[..(i + 2).min(toks.len())],
            None => toks,
        };
        if let Some(list) = self.names.get(&toks[0]) {
            for (pid, name, _alias) in list {
                if name.len() >= head.len()
                    && name[..head.len()] == *head
                    && is_subsequence(&toks[head.len()..], &name[head.len()..])
                {
                    found.insert(*pid);
                }
            }
        }
        found
    }
}

fn article_variants(toks: &[String]) -> Vec<Vec<String>> {
    let mut out = vec![toks.to_vec()];
    if toks.len() > 3 && toks[0].starts_with("ال") && toks[1..].iter().any(|t| t == "بن") {
        let mut v = toks[1..].to_vec();
        v.push(toks[0].clone());
        out.push(v);
    }
    if toks.len() > 4
        && (toks[0] == "ابو" || toks[0] == "ام")
        && toks[2..].iter().any(|t| t == "بن")
    {
        let mut v = toks[2..].to_vec();
        v.extend_from_slice(&toks[..2]);
        out.push(v);
    }
    let first = &toks[0];
    if first.starts_with("ال") && pyre::len(first) > 4 {
        let mut v = vec![pyre::from(first, 2).to_string()];
        v.extend_from_slice(&toks[1..]);
        out.push(v);
    } else if !first.starts_with("ال")
        && !["ابو", "ام", "بن", "عبد"].contains(&first.as_str())
        && pyre::len(first) > 2
    {
        let mut v = vec![format!("ال{first}")];
        v.extend_from_slice(&toks[1..]);
        out.push(v);
    }
    out
}

const CANNOT: &str = "relative that cannot be read";

/// `name_tokens`.
pub fn name_tokens(
    raw_name: &str,
    previous: Option<&[String]>,
    after_father: bool,
) -> (Vec<String>, &'static str) {
    let text = mp("_HONORIFIC").sub(&mp("_OF_PROPHET").sub(&normalize(raw_name), ""), "");
    let text = strip(&text, " ،,-").to_string();
    let text = strip(&mp("_SAYS").sub(&text, ""), " ،,-").to_string();
    let clar = mp("_CLARIFY").search(&text);
    let base = match &clar {
        Some(c) => strip(pyre::upto(&text, c.start()), " ،,-").to_string(),
        None => text.clone(),
    };
    let mut toks: Vec<String> = if base == "ابي" || base == "ابوي" {
        vec!["ابيه".into()]
    } else if base == "جدي" {
        vec!["جده".into()]
    } else {
        tokens(&base)
    };
    let mut how = "name";
    if let Some(c) = &clar {
        let extra = tokens(c.g(1));
        toks = if extra.first().map(|s| s.as_str()) == Some("بن") {
            toks.into_iter().chain(extra).collect()
        } else if extra.len() >= 2 {
            extra
        } else {
            toks.into_iter().chain(extra).collect()
        };
        how = "name + clarification";
    }
    let relative = &m().relative;
    if toks == ["ابي"] {
        toks = vec!["ابيه".into()];
    } else if toks == ["جدي"] {
        toks = vec!["جده".into()];
    } else if toks.len() == 1 && m().kin_first_person.contains(&toks[0]) {
        return (vec![], CANNOT);
    }
    if !toks.is_empty() && relative.contains_key(&toks[0]) && toks.len() >= 3 {
        return (toks[1..].to_vec(), "relative named in the chain");
    }
    if toks == ["جده"] && after_father {
        toks = vec!["ابيه".into()];
    }
    if let Some(prev) = previous.filter(|p| !p.is_empty()) {
        if let Some(relation) = toks.first().and_then(|t| relative.get(t)) {
            let bins: Vec<usize> = prev
                .iter()
                .enumerate()
                .filter(|(_, t)| *t == "بن")
                .map(|(i, _)| i)
                .collect();
            if relation == "father" && !bins.is_empty() {
                toks = prev[bins[0] + 1..].to_vec();
                how = "father of the previous narrator";
                if toks.len() == 1 || toks.first().map(|s| s.as_str()) == Some("ابو") {
                    return (vec![], CANNOT);
                }
            } else if relation == "grandfather" && bins.len() >= 2 {
                toks = prev[bins[1] + 1..].to_vec();
                how = "grandfather of the previous narrator";
            } else {
                return (vec![], CANNOT);
            }
        }
    }
    (toks, how)
}

fn is_prophet(raw: &str) -> bool {
    let n = normalize(raw);
    let name = strip(&n, " ،,");
    p("_PROPHET_NAME_RE").fullmatch(name) || p("_PROPHET_RE").fullmatch(name)
}

fn link_kind(raw: &str) -> &'static str {
    let n = normalize(raw);
    let n = strip(&n, " ،,");
    let mm = m();
    if mm.group_forms.iter().any(|f| f == n) {
        return "group";
    }
    if mm.previous_forms.iter().any(|f| f == n) {
        return "previous";
    }
    if mm.vague_forms.iter().any(|f| f == n)
        || ["بعض اصحاب", "رجل من", "بعض من", "من ذكره", "من حدثه"]
            .iter()
            .any(|s| n.starts_with(s))
    {
        return "vague";
    }
    "person"
}

fn teacher_name(collection: &str, raw: &str) -> String {
    let n = normalize(raw);
    let mut name = strip(&n, " ،,").to_string();
    // "أبي ره", "أبي رضي الله عنه": the honorific after the name (al-Saduq's books)
    for tail in [" رضي الله عنه", " رحمه الله", " ره", " رض"] {
        if let Some(head) = name.strip_suffix(tail) {
            name = head.trim_end().to_string();
            break;
        }
    }
    m().compiler_teachers
        .get(collection)
        .and_then(|t| t.get(&name))
        .cloned()
        .unwrap_or_default()
}

/// `match_chain`: (results, candidates left, names written) for one chain.
pub fn match_chain(
    index: &Index,
    links: &[(i64, String)],
    collection: &str,
    reaches_prophet: bool,
) -> (Vec<Res>, Vec<Set>, Vec<String>) {
    let allowed: Option<HashSet<&String>> = m()
        .collection_marks
        .get(collection)
        .map(|v| v.iter().collect());
    let mut cands: Vec<Set> = vec![];
    let mut written: Vec<Vec<String>> = vec![];
    let mut previous: Option<Vec<String>> = None;
    let mut after_father = false;
    let shia = index.tradition == "shia";
    for (position, raw) in links {
        if (shia && link_kind(raw) != "person") || (!shia && is_prophet(raw)) {
            cands.push(Set::new());
            written.push(vec![]);
            previous = None;
            after_father = false;
            continue;
        }
        let (mut toks, how) = name_tokens(raw, previous.as_deref(), after_father);
        after_father = how == "father of the previous narrator";
        if shia && *position == 1 {
            let fuller = teacher_name(collection, raw);
            if !fuller.is_empty() {
                toks = tokens(&fuller);
            }
        }
        let mut found: Set = (*index.candidates(&toks)).clone();
        if let Some(allowed) = &allowed {
            if !found.is_empty() {
                found.retain(|p| {
                    index
                        .marks
                        .get(p)
                        .is_some_and(|ms| ms.iter().any(|x| allowed.contains(x)))
                });
            }
        }
        previous = Some(if found.len() == 1 {
            index
                .full
                .get(found.iter().next().unwrap())
                .cloned()
                .unwrap_or_default()
        } else {
            toks.clone()
        });
        cands.push(found);
        written.push(toks);
    }
    let mut first_counts: Vec<i64> = cands.iter().map(|c| c.len() as i64).collect();
    // decided by isnady's own inference (Index.guess)
    let mut guessed = vec![false; cands.len()];
    let n = cands.len();
    let author = index.authors.get(collection);
    if n > 0 && author.is_some_and(|a| cands[0].contains(a)) {
        cands[0] = [*author.unwrap()].into_iter().collect();
    } else if n > 0 && !cands[0].is_empty() && !(n == 1 && reaches_prophet) {
        let teachers: Set = cands[0]
            .iter()
            .copied()
            .filter(|p| index.t(*p).is_none_or_ge(m().teacher_min_tabaqa))
            .collect();
        if !teachers.is_empty() {
            cands[0] = teachers;
        }
    }
    if reaches_prophet && n > 0 && !cands[n - 1].is_empty() {
        let comps: Set = cands[n - 1]
            .iter()
            .copied()
            .filter(|p| matches!(index.t(*p), Some(1) | None))
            .collect();
        if !comps.is_empty() {
            cands[n - 1] = comps;
        }
    }
    let lowest = |g: &Set| -> Option<i64> {
        let v: Vec<Option<i64>> = g.iter().map(|q| index.t(*q)).collect();
        if v.contains(&None) {
            None
        } else {
            v.into_iter().flatten().min()
        }
    };
    let highest = |g: &Set| -> Option<i64> {
        let v: Vec<Option<i64>> = g.iter().map(|q| index.t(*q)).collect();
        if v.contains(&None) {
            None
        } else {
            v.into_iter().flatten().max()
        }
    };
    let mut changed = true;
    while changed {
        changed = false;
        for i in 0..n {
            let mut keep = cands[i].clone();
            let later = (i + 1..n).map(|j| &cands[j]).find(|c| !c.is_empty());
            let earlier = (0..i).rev().map(|j| &cands[j]).find(|c| !c.is_empty());
            if let Some(low) = later.and_then(lowest) {
                keep.retain(|p| index.t(*p).is_none_or_ge(low));
            }
            if let Some(high) = earlier.and_then(highest) {
                keep.retain(|p| index.t(*p).map_or(true, |t| high >= t));
            }
            if !keep.is_empty() && keep != cands[i] {
                cands[i] = keep;
                changed = true;
            }
        }
    }
    // evidence 6: the client of the next narrator
    for i in 0..n.saturating_sub(1) {
        if cands[i].len() > 1 && !written[i + 1].is_empty() {
            let after = &written[i + 1];
            let mut forms: BTreeSet<String> = BTreeSet::new();
            forms.insert(after.join(" "));
            if let Some(k) = after.iter().skip(1).position(|t| t == "بن").map(|k| k + 1) {
                forms.insert(after[k..].join(" "));
            }
            let marks: Vec<String> = forms
                .iter()
                .map(|f| format!(" مولي {f} "))
                .chain(forms.iter().map(|f| format!(" مولي {f}")))
                .collect();
            let hits: Set = cands[i]
                .iter()
                .copied()
                .filter(|p| {
                    let full = format!(
                        " {} ",
                        index.full.get(p).map(|v| v.join(" ")).unwrap_or_default()
                    );
                    marks.iter().any(|mk| full.contains(mk.as_str()))
                })
                .collect();
            if hits.len() == 1 {
                cands[i] = hits;
                first_counts[i] = first_counts[i].max(2);
            }
        }
    }
    // evidence 7: a bare name read by its neighbours
    for i in 0..n {
        if cands[i].len() > 1 {
            if let Some(list) = index.muhmal.get(&written[i]) {
                let after: Vec<String> = if i + 1 < n {
                    if !written[i + 1].is_empty() {
                        written[i + 1].clone()
                    } else {
                        tokens(&mp("_HONORIFIC").sub(&normalize(&links[i + 1].1), ""))
                    }
                } else {
                    vec![]
                };
                let before: Vec<String> = if i > 0 {
                    written[i - 1].clone()
                } else {
                    vec![]
                };
                let hits: Set = list
                    .iter()
                    .filter(|(pid, teachers, students)| {
                        cands[i].contains(pid)
                            && (teachers.contains(&after)
                                || students.iter().any(|x| {
                                    before.len() >= x.len().min(before.len())
                                        && before[..x.len().min(before.len())] == x[..]
                                }))
                    })
                    .map(|x| x.0)
                    .collect();
                if hits.len() == 1 {
                    cands[i] = hits;
                    first_counts[i] = first_counts[i].max(2);
                    guessed[i] = true; // isnady's list, not the rijal work's words
                }
            }
        }
    }
    let mut names = vec![];
    for ((_, raw), w) in links.iter().zip(written.iter()) {
        let bare = mp("_HONORIFIC").sub(&normalize(raw), "");
        let bare = strip(&bare, " ،,-");
        let plain = if bare == "ابي" || bare == "ابوي" {
            vec!["ابيه".to_string()]
        } else {
            tokens(bare)
        };
        names.push(
            if plain.first().is_some_and(|x| m().relative.contains_key(x)) {
                plain[0].clone()
            } else {
                w.join(" ")
            },
        );
    }
    let out: Vec<Res> = cands
        .iter()
        .enumerate()
        .map(|(i, c)| {
            if c.len() == 1 {
                let who = *c.iter().next().unwrap();
                let guessed_here = guessed[i]
                    || index.guessed_keys.borrow().contains(&written[i])
                    || index
                        .known_as_added
                        .get(&written[i].join(" "))
                        .is_some_and(|s| s.contains(&who));
                (
                    Some(who),
                    Some(if guessed_here {
                        m().guess_confidence
                    } else if first_counts[i] == 1 {
                        1.0
                    } else {
                        0.8
                    }),
                    first_counts[i],
                )
            } else {
                (None, None, c.len() as i64)
            }
        })
        .collect();
    (out, cands, names)
}

trait NoneOrGe {
    fn is_none_or_ge(self, v: i64) -> bool;
}
impl NoneOrGe for Option<i64> {
    /// `t(p) is None or t(p) >= v`
    fn is_none_or_ge(self, v: i64) -> bool {
        self.map_or(true, |t| t >= v)
    }
}

pub struct Chain {
    pub results: Vec<Res>,
    pub cands: Vec<Set>,
    pub names: Vec<String>,
}

fn conf_ge(c: Option<f64>, v: f64) -> bool {
    c.is_some_and(|x| x != 0.0 && x >= v)
}

fn edges_of(chains: &[Chain], trust: f64) -> HashMap<(i64, i64), i64> {
    let mut edges = HashMap::new();
    for ch in chains {
        let r = &ch.results;
        for i in 0..r.len().saturating_sub(1) {
            if let (Some(a), Some(b)) = (r[i].0, r[i + 1].0) {
                if conf_ge(r[i].1, trust) && conf_ge(r[i + 1].1, trust) {
                    *edges.entry((a, b)).or_insert(0) += 1;
                }
            }
        }
    }
    edges
}

/// Evidence 5: the teacher and the student.
pub fn by_neighbours(chains: &mut [Chain]) -> i64 {
    let (edge_min, edge_lead, edge_conf) = m().edge;
    let mut found = 0;
    for _ in 0..3 {
        let edges = edges_of(chains, m().trust);
        if edges.is_empty() {
            break;
        }
        let mut changed = 0;
        for ch in chains.iter_mut() {
            for i in 0..ch.results.len() {
                let (pid, _c, count) = ch.results[i];
                if pid.is_some() || ch.cands[i].len() < 2 {
                    continue;
                }
                let student = if i > 0 { ch.results[i - 1].0 } else { None };
                let teacher = if i + 1 < ch.results.len() {
                    ch.results[i + 1].0
                } else {
                    None
                };
                let last = i + 1 == ch.results.len();
                let Some(student) = student else {
                    continue;
                };
                if teacher.is_none() && !last {
                    continue;
                }
                let mut rows: Vec<(i64, i64)> = ch.cands[i]
                    .iter()
                    .map(|p| {
                        let below = edges.get(&(student, *p)).copied().unwrap_or(0);
                        let above = teacher
                            .map(|t| edges.get(&(*p, t)).copied().unwrap_or(0))
                            .unwrap_or(0);
                        (if last { below } else { below.min(above) }, *p)
                    })
                    .collect();
                rows.sort();
                let (best, who) = rows[rows.len() - 1];
                let second = rows[rows.len() - 2].0;
                let floor = if last { edge_min + 1 } else { edge_min };
                if best >= floor && best as f64 >= edge_lead * second as f64 && best > second {
                    ch.results[i] = (Some(who), Some(edge_conf), count);
                    changed += 1;
                }
            }
        }
        found += changed;
        if changed == 0 {
            break;
        }
    }
    found
}

pub fn by_one_side(chains: &mut [Chain]) -> i64 {
    let (one_min, one_lead) = m().one_side;
    let edge_conf = m().edge.2;
    let fc = m().famous_confidence;
    let mut edges = HashMap::new();
    for ch in chains.iter() {
        let r = &ch.results;
        for i in 0..r.len().saturating_sub(1) {
            if let (Some(a), Some(b)) = (r[i].0, r[i + 1].0) {
                if r[i].1.unwrap_or(0.0) >= m().trust && r[i + 1].1.unwrap_or(0.0) >= m().trust {
                    *edges.entry((a, b)).or_insert(0i64) += 1;
                }
            }
        }
    }
    let mut found = 0;
    for ch in chains.iter_mut() {
        for i in 0..ch.results.len() {
            let (pid, _c, count) = ch.results[i];
            if pid.is_some() || ch.cands[i].len() < 2 {
                continue;
            }
            let mut student = if i > 0 { ch.results[i - 1].0 } else { None };
            let mut teacher = if i + 1 < ch.results.len() {
                ch.results[i + 1].0
            } else {
                None
            };
            if student.is_some() && teacher.is_some() {
                let weak_s = ch.results[i - 1].1.unwrap_or(0.0) <= fc;
                let weak_t = ch.results[i + 1].1.unwrap_or(0.0) <= fc;
                if weak_s != weak_t {
                    if weak_s {
                        student = None;
                    } else {
                        teacher = None;
                    }
                }
            }
            if student.is_none() && teacher.is_none() {
                continue;
            }
            if let (Some(s), Some(t)) = (student, teacher) {
                // both known, the two-sided rule could not decide (0.7.7): the teacher alone, and the student must
                // not have been seen with another of them more
                let mut rows: Vec<(i64, i64)> = ch.cands[i]
                    .iter()
                    .map(|p| (edges.get(&(*p, t)).copied().unwrap_or(0), *p))
                    .collect();
                rows.sort();
                let (best, who) = rows[rows.len() - 1];
                let second = rows[rows.len() - 2].0;
                let seen = |p: &i64| edges.get(&(s, *p)).copied().unwrap_or(0);
                let most = ch.cands[i].iter().map(seen).max().unwrap_or(0);
                if best >= one_min
                    && best as f64 >= one_lead * second.max(1) as f64
                    && seen(&who) >= most
                {
                    ch.results[i] = (Some(who), Some(edge_conf), count);
                    found += 1;
                }
                continue;
            }
            let mut rows: Vec<(i64, i64)> = ch.cands[i]
                .iter()
                .map(|p| {
                    let c = match student {
                        Some(s) => edges.get(&(s, *p)).copied().unwrap_or(0),
                        None => edges.get(&(*p, teacher.unwrap())).copied().unwrap_or(0),
                    };
                    (c, *p)
                })
                .collect();
            rows.sort();
            let (best, who) = rows[rows.len() - 1];
            let second = rows[rows.len() - 2].0;
            if best >= one_min && best as f64 >= one_lead * second.max(1) as f64 {
                ch.results[i] = (Some(who), Some(edge_conf), count);
                found += 1;
            }
        }
    }
    found
}

pub fn by_famous(chains: &mut [Chain], famous: &HashMap<String, i64>) -> i64 {
    let fc = m().famous_confidence;
    let mut found = 0;
    for ch in chains.iter_mut() {
        for i in 0..ch.results.len() {
            let (pid, _c, count) = ch.results[i];
            if pid.is_none() && i < ch.names.len() {
                if let Some(who) = famous.get(&ch.names[i]) {
                    if ch.cands[i].contains(who) {
                        ch.results[i] = (Some(*who), Some(fc), count);
                        found += 1;
                    }
                }
            }
        }
    }
    found
}

/// `_kunya_index`: every narrator by the kunya his entry gives him anywhere in his name.
fn kunya_index(index: &Index) -> Rc<HashMap<String, Set>> {
    if let Some(k) = index.kunyas.borrow().as_ref() {
        return k.clone();
    }
    let not_his = &m().father.not_his_kunya;
    let mut found: HashMap<String, Set> = HashMap::new();
    for (pid, toks) in &index.full {
        if index.hidden.contains(pid) {
            continue;
        }
        for k in 0..toks.len().saturating_sub(1) {
            if toks[k] == "ابو" && (k == 0 || !not_his.contains(&toks[k - 1])) {
                found
                    .entry(compound(toks, k + 1).join(" "))
                    .or_default()
                    .insert(*pid);
            }
        }
    }
    let found = Rc::new(found);
    *index.kunyas.borrow_mut() = Some(found.clone());
    found
}

/// `_compound`: the name that starts at toks[k] — two words for "عبد الله", "عبيد الله", one otherwise.
fn compound(toks: &[String], k: usize) -> Vec<String> {
    if k + 1 < toks.len() && (toks[k] == "عبد" || toks[k] == "عبيد") {
        toks[k..k + 2].to_vec()
    } else {
        toks.get(k).cloned().into_iter().collect()
    }
}

/// `_lineage`: the names of a nasab one after the other.
fn lineage(toks: &[String]) -> Vec<Vec<String>> {
    let mut out = Vec::new();
    let mut k = 0;
    while k < toks.len() {
        let name = if (toks[k] == "ابو" || toks[k] == "ابي") && k + 1 < toks.len() {
            let mut v = vec!["ابو".to_string()];
            v.extend(compound(toks, k + 1));
            v
        } else {
            compound(toks, k)
        };
        k += name.len();
        out.push(name);
        if k >= toks.len() || toks[k] != "بن" {
            break;
        }
        k += 1;
    }
    out
}

/// `_patrons`: whose freedman an entry calls him.
fn patrons(toks: &[String]) -> HashSet<Vec<String>> {
    let mut out = HashSet::new();
    for k in 0..toks.len().saturating_sub(1) {
        if toks[k] == "مولي" {
            let n = if ["عبد", "عبيد", "ابو", "ابن", "بني", "ال", "ام"]
                .contains(&toks[k + 1].as_str())
            {
                2
            } else {
                1
            };
            out.insert(toks[k + 1..(k + 1 + n).min(toks.len())].to_vec());
        }
    }
    out
}

/// The letters without their dots (`_RASM`).
fn rasm(words: &[String]) -> Vec<String> {
    let (from, to) = &m().father.rasm;
    let map: HashMap<char, char> = from.chars().zip(to.chars()).collect();
    words
        .iter()
        .map(|w| w.chars().map(|c| *map.get(&c).unwrap_or(&c)).collect())
        .collect()
}

/// `_father_by_nasab`: the father of a narrator, read from the son's own nasab in the rijal work and found as an
/// entry of his own, weighed by what else of the son's nasab it carries (0.7.7).
fn father_by_nasab(index: &Index, son: i64) -> Option<i64> {
    let fa = &m().father;
    let son_toks = index.full.get(&son).cloned().unwrap_or_default();
    let i = son_toks.iter().position(|t| t == "بن")?;
    let mut seg: Vec<String> = son_toks[i + 1..].to_vec();
    if let Some(k) = seg
        .iter()
        .position(|t| ["او", "ويقال", "وقيل", "قيل", "يقال"].contains(&t.as_str()))
    {
        seg.truncate(k);
    }
    let mut kunya: Option<Vec<String>> = None;
    if (seg.first().map(|s| s.as_str()) == Some("ابي")
        || seg.first().map(|s| s.as_str()) == Some("ابو"))
        && seg.len() >= 2
    {
        let k = compound(&seg, 1);
        seg = seg[1 + k.len()..].to_vec();
        kunya = Some(k);
    }
    let mut ism: Option<Vec<String>> = None;
    if let Some(first) = seg.first() {
        if !["بن", "ابن", "مولي", "ابو", "ابي"].contains(&first.as_str())
            && (!first.starts_with("ال")
                || (first != "الله"
                    && first != "الرحمن"
                    && !first.ends_with('ي')
                    && !first.ends_with("يه")
                    && index
                        .names
                        .get(first)
                        .is_some_and(|v| v.iter().any(|(_, _, alias)| !alias))))
        {
            ism = Some(compound(&seg, 0));
        }
    }
    let above: Vec<Vec<String>> = if ism.is_some() {
        lineage(&seg).into_iter().skip(1).collect()
    } else if seg.first().map(|s| s.as_str()) == Some("بن") {
        lineage(&seg[1..])
    } else {
        vec![]
    };
    let grand = above.first().cloned();
    let mut told: Option<String> = None;
    for k in 0..son_toks.len().saturating_sub(2) {
        if son_toks[k] == "اسم" && son_toks[k + 1] == "ابيه" {
            told = Some(son_toks[k + 2].clone());
            break;
        }
    }
    let skip_words = [
        "وكيل",
        "من",
        "نزيل",
        "يروي",
        "اسمه",
        "قيل",
        "وقيل",
        "يقال",
        "ويقال",
        "هو",
        "او",
    ];
    let mut nisbas: HashSet<String> = son_toks
        .iter()
        .enumerate()
        .filter(|(k, t)| {
            t.starts_with("ال")
                && chars(t) > 3
                && *t != "الله"
                && *t != "الرحمن"
                && (*k == 0
                    || !(fa.not_his_kunya.contains(&son_toks[k - 1])
                        || skip_words.contains(&son_toks[k - 1].as_str())))
        })
        .map(|(_, t)| t.clone())
        .collect();
    for w in ism.iter().flatten().chain(kunya.iter().flatten()) {
        nisbas.remove(w);
    }
    let son_patrons = patrons(&son_toks);
    if ism.is_none() && kunya.is_none() {
        return None;
    }
    let low = index.t(son);
    let son_ism: Vec<String> =
        if ["عبد", "عبيد", "ابو"].contains(&son_toks[0].as_str()) && son_toks.len() > 1 {
            son_toks[..2].to_vec()
        } else {
            son_toks[..1].to_vec()
        };
    let son_art: Vec<String> = {
        let mut v = vec![match son_ism[0].strip_prefix("ال") {
            Some(rest) => rest.to_string(),
            None => format!("ال{}", son_ism[0]),
        }];
        v.extend_from_slice(&son_ism[1..]);
        v
    };
    let mut pool: BTreeSet<i64> = BTreeSet::new();
    let mut bearers: Vec<i64> = Vec::new();
    if let Some(ism) = &ism {
        for (pid, toks, alias) in index.names.get(&ism[0]).into_iter().flatten() {
            if !alias
                && !index.hidden.contains(pid)
                && toks.len() >= ism.len()
                && toks[..ism.len()] == ism[..]
            {
                bearers.push(*pid);
            }
        }
        pool.extend(bearers.iter().copied());
    }
    if let Some(k) = &kunya {
        if let Some(found) = kunya_index(index).get(&k.join(" ")) {
            pool.extend(found.iter().copied());
        }
    }
    // (weight, tier, person, a thing of his own)
    let mut scored: Vec<(f64, i64, i64, bool)> = Vec::new();
    for pid in pool {
        if pid == son || index.hidden.contains(&pid) {
            continue;
        }
        let toks = index.full.get(&pid).cloned().unwrap_or_default();
        let has_kunya = kunya.as_ref().is_some_and(|k| {
            (0..toks.len().saturating_sub(1)).any(|j| {
                toks[j] == "ابو"
                    && compound(&toks, j + 1) == *k
                    && (j == 0 || !fa.not_his_kunya.contains(&toks[j - 1]))
            })
        });
        let first_is = ism
            .as_ref()
            .is_some_and(|w| toks.len() >= w.len() && toks[..w.len()] == w[..]);
        let has_ism = first_is
            || ism
                .as_ref()
                .is_some_and(|w| w.len() == 1 && toks.iter().take(3).any(|t| *t == w[0]));
        if !(has_ism || has_kunya) {
            continue;
        }
        let mut more = 0.0f64;
        let mut own = false;
        if let (Some(_), true) = (&grand, first_is) {
            let theirs: Vec<Vec<String>> = lineage(&toks).into_iter().skip(1).collect();
            let mut same = 0usize;
            for (mine, his) in above.iter().zip(theirs.iter()) {
                if rasm(mine) != rasm(his) {
                    break;
                }
                same += 1;
            }
            if same < above.len().min(theirs.len()) {
                continue;
            }
            if same > 0 {
                more += 1.0 + same as f64;
                own = true;
            }
        } else if let Some(g) = &grand {
            let head = &toks[..toks.len().min(8)];
            let g_rasm = rasm(g);
            if (0..head.len().saturating_sub(1)).any(|j| {
                head[j] == "بن" && rasm(&head[j + 1..(j + 1 + g.len()).min(head.len())]) == g_rasm
            }) {
                more += 2.0;
                own = true;
            }
        }
        if has_kunya {
            more += 1.0;
            own = true;
            if first_is {
                more += 1.0;
            }
        }
        if let Some(told) = &told {
            if toks.contains(told) {
                more += 1.0;
            }
        }
        let theirs: HashSet<&String> = toks.iter().collect();
        for n in nisbas.iter().filter(|n| theirs.contains(n)) {
            more += if fa.city_nisbas.contains(n) { 0.5 } else { 1.0 };
        }
        if !son_patrons.is_disjoint(&patrons(&toks)) {
            more += 1.0;
            own = true;
        }
        let n = son_ism.len();
        for j in 0..toks.len().saturating_sub(n) {
            if (toks[j] == "والد" || toks[j] == "ابو")
                && (toks[j + 1..j + 1 + n] == son_ism[..] || toks[j + 1..j + 1 + n] == son_art[..])
            {
                if toks[j] == "والد" {
                    more += 3.0;
                    own = true;
                } else {
                    more += 1.0;
                }
                break;
            }
        }
        if more == 0.0 {
            continue;
        }
        if let (Some(low), Some(t)) = (low, index.t(pid)) {
            if !(low - fa.gap <= t && t <= low) {
                continue;
            }
        }
        scored.push((more, *index.tier.get(&pid).unwrap_or(&0), pid, own));
    }
    if scored.is_empty() {
        return None;
    }
    // the heaviest first; of two as heavy, the earlier Shia book's; then the later person (Python's reverse sort)
    scored.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap()
            .then(a.1.cmp(&b.1))
            .then(b.2.cmp(&a.2))
    });
    let (best, tier, who, own) = scored[0];
    let second = scored[1..]
        .iter()
        .filter(|x| x.1 == tier)
        .map(|x| x.0)
        .fold(0.0f64, f64::max);
    let alone = bearers.len() == 1 && bearers[0] == who;
    if best > second && (best >= 2.0 || (second == 0.0 && (own || alone))) {
        Some(who)
    } else {
        None
    }
}

/// `_father_of`: the one father a narrator's entry gives him, as by_relatives reads it.
fn father_of(index: &Index, son: i64) -> Option<i64> {
    if let Some(f) = index.fathers.borrow().get(&son) {
        return *f;
    }
    let mut found: Set = Set::new();
    let (toks, how) = name_tokens("ابيه", index.full.get(&son).map(|v| v.as_slice()), false);
    if !toks.is_empty() && how == "father of the previous narrator" {
        found = (*index.candidates(&toks)).clone();
        if found.is_empty() && toks.iter().any(|t| t == "بن") {
            let last = toks.len() - 1 - toks.iter().rev().position(|t| t == "بن").unwrap();
            let cut = if last + 1 < toks.len()
                && ["عبد", "عبيد", "ابو"].contains(&toks[last + 1].as_str())
            {
                last + 3
            } else {
                last + 2
            };
            found = (*index.candidates(&toks[..cut.min(toks.len())])).clone();
        }
        let low = index.t(son);
        found.retain(|p| {
            *p != son && (low.is_none() || index.t(*p).is_none() || index.t(*p) <= low)
        });
    }
    if found.len() != 1 {
        found = father_by_nasab(index, son).into_iter().collect();
    }
    let f = found.iter().next().copied();
    index.fathers.borrow_mut().insert(son, f);
    f
}

/// `by_father_pair` (0.7.7): a son open between several namesakes, and his father with him.
pub fn by_father_pair(chains: &mut [Chain], index: &Index) -> i64 {
    let (_, edge_lead, edge_conf) = m().edge;
    let (pair_min, pair_few) = m().father.pair;
    let mut edges = HashMap::new();
    for ch in chains.iter() {
        let r = &ch.results;
        for i in 0..r.len().saturating_sub(1) {
            if let (Some(a), Some(b)) = (r[i].0, r[i + 1].0) {
                if r[i].1.unwrap_or(0.0) >= m().trust && r[i + 1].1.unwrap_or(0.0) >= m().trust {
                    *edges.entry((a, b)).or_insert(0i64) += 1;
                }
            }
        }
    }
    let mut found = 0;
    for ch in chains.iter_mut() {
        let n = ch.results.len();
        for i in 0..n.saturating_sub(1) {
            if ch.results[i].0.is_some()
                || ch.cands[i].len() < 2
                || ch.results[i + 1].0.is_some()
                || i + 1 >= ch.names.len()
                || !(ch.names[i + 1] == "ابيه" || ch.names[i + 1] == "ابوه")
            {
                continue;
            }
            let student = if i > 0 { ch.results[i - 1].0 } else { None };
            let teacher = if i + 2 < n { ch.results[i + 2].0 } else { None };
            let mut rows: Vec<(i64, i64, i64)> = Vec::new();
            for c in &ch.cands[i] {
                let Some(f) = father_of(index, *c) else {
                    continue;
                };
                if let (Some(tt), Some(tf)) = (teacher.and_then(|t| index.t(t)), index.t(f)) {
                    if tt > tf {
                        continue;
                    }
                }
                let score = student.map_or(0, |s| edges.get(&(s, *c)).copied().unwrap_or(0))
                    + teacher.map_or(0, |t| edges.get(&(f, t)).copied().unwrap_or(0));
                rows.push((score, *c, f));
            }
            if rows.is_empty() {
                continue;
            }
            rows.sort();
            let (best, c, f) = rows[rows.len() - 1];
            let second = if rows.len() > 1 {
                rows[rows.len() - 2].0
            } else {
                0
            };
            let alone = rows.len() == 1
                && ch.cands[i].len() <= pair_few
                && (student.is_some() || teacher.is_some());
            if alone
                || (best >= pair_min && best as f64 >= edge_lead * second as f64 && best > second)
            {
                ch.results[i] = (Some(c), Some(edge_conf), ch.results[i].2);
                let cnt = if ch.results[i + 1].2 == 0 {
                    1
                } else {
                    ch.results[i + 1].2
                };
                ch.results[i + 1] = (Some(f), Some(edge_conf), cnt);
                ch.cands[i] = [c].into_iter().collect();
                ch.cands[i + 1] = [f].into_iter().collect();
                found += 2;
            }
        }
    }
    found
}

pub fn by_relatives(chains: &mut [Chain], index: &Index) -> i64 {
    let mut found = 0;
    for ch in chains.iter_mut() {
        for i in 0..ch.results.len() {
            let (pid, _c, count) = ch.results[i];
            if pid.is_some()
                || i == 0
                || i >= ch.names.len()
                || !m().relative_words.contains(&ch.names[i])
            {
                continue;
            }
            let Some(student) = ch.results[i - 1].0 else {
                continue;
            };
            let (toks, how) = name_tokens(
                &ch.names[i],
                index.full.get(&student).map(|v| v.as_slice()),
                false,
            );
            let unreadable = toks.is_empty() || how == CANNOT;
            let mut here: Set = if unreadable {
                Set::new()
            } else {
                (*index.candidates(&toks)).clone()
            };
            if here.is_empty() && !unreadable && toks.iter().any(|t| t == "بن") {
                let last = toks.len() - 1 - toks.iter().rev().position(|t| t == "بن").unwrap();
                let cut = if last + 1 < toks.len()
                    && ["عبد", "عبيد", "ابو"].contains(&toks[last + 1].as_str())
                {
                    last + 3
                } else {
                    last + 2
                };
                here = (*index.candidates(&toks[..cut.min(toks.len())])).clone();
            }
            if let Some(low) = index.t(student) {
                here.retain(|p| index.t(*p).map_or(true, |t| t <= low));
            }
            let teacher = if i + 1 < ch.results.len() {
                ch.results[i + 1].0
            } else {
                None
            };
            if let Some(tt) = teacher.and_then(|t| index.t(t)) {
                here.retain(|p| index.t(*p).is_none_or_ge(tt));
            }
            let mut weighed = false;
            if here.len() != 1 && (ch.names[i] == "ابيه" || ch.names[i] == "ابوه") && index.guess
            {
                if let Some(father) = father_by_nasab(index, student) {
                    let ok = match (teacher.and_then(|t| index.t(t)), index.t(father)) {
                        (Some(tt), Some(tf)) => tf >= tt,
                        _ => true,
                    };
                    if ok {
                        here = [father].into_iter().collect();
                        weighed = true;
                    }
                }
            }
            if here.len() == 1 {
                let who = *here.iter().next().unwrap();
                // his name, read from the son's — a guess when the son himself was one, or the father was weighed
                let sure = !weighed && ch.results[i - 1].1.unwrap_or(0.0) >= 0.8;
                let conf = if sure { 0.8 } else { m().guess_confidence };
                ch.results[i] = (Some(who), Some(conf), if count == 0 { 1 } else { count });
                ch.cands[i] = here;
                found += 1;
            }
        }
    }
    found
}

pub fn drop_impossible(results: &mut [Res], index: &Index) -> i64 {
    let (gap_tabaqa, gap_older, gap_death) = m().gaps;
    let mut dropped = 0;
    for _ in 0..results.len() {
        let mut bad = None;
        for i in 0..results.len().saturating_sub(1) {
            let (Some(s), Some(t)) = (results[i].0, results[i + 1].0) else {
                continue;
            };
            let (ts, tt) = (index.t(s), index.t(t));
            let (ds, dt) = (index.death.get(&s).copied(), index.death.get(&t).copied());
            let older = matches!((ts, tt), (Some(a), Some(b)) if a - b >= gap_older);
            let later = matches!((ts, tt), (Some(a), Some(b)) if b - a >= gap_tabaqa);
            let dead = matches!((ds, dt), (Some(a), Some(b)) if b - a >= gap_death);
            if s == t || older || later || dead {
                let (cs, ct) = (results[i].1.unwrap_or(0.0), results[i + 1].1.unwrap_or(0.0));
                bad = Some(if cs < ct || (cs == ct && i == 0 && s != t && !older) {
                    i
                } else {
                    i + 1
                });
                break;
            }
        }
        let Some(b) = bad else {
            break;
        };
        results[b] = (None, None, results[b].2);
        dropped += 1;
    }
    dropped
}

fn persons_now(conn: &Connection) -> rusqlite::Result<String> {
    let (n, mx): (i64, i64) = conn.query_row(
        "SELECT COUNT(*), COALESCE(MAX(id), 0) FROM persons",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    Ok(format!("({n}, {mx})"))
}

#[derive(Debug, Default)]
pub struct Linked {
    pub chains: usize,
    pub links: usize,
    pub identified: usize,
    pub note: String,
}

/// `link_narrators`: identify the narrators of every chain read from the texts.
/// `guesses_on`: whether isnady's own inference identifies narrators in this database (Tools → Narrator guesses).
pub fn guesses_on(conn: &Connection) -> bool {
    conn.query_row(
        "SELECT value FROM core_meta WHERE key = 'narrator_guess'",
        [],
        |r| r.get::<_, String>(0),
    )
    .map(|v| v == "on")
    .unwrap_or(false)
}

/// `set_guesses`: isnady's own inference on or off, every chain's narrators identified again.
pub fn set_guesses(conn: &Connection, on: bool, say: &dyn Fn(&str)) -> Result<Linked, String> {
    let e = |x: rusqlite::Error| x.to_string();
    conn.execute(
        "CREATE TABLE IF NOT EXISTS core_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
        [],
    )
    .map_err(e)?;
    conn.execute(
        "INSERT OR REPLACE INTO core_meta (key, value) VALUES ('narrator_guess', ?)",
        [if on { "on" } else { "off" }],
    )
    .map_err(e)?;
    link_narrators(conn, say, true)
}

pub fn link_narrators(
    conn: &Connection,
    say: &dyn Fn(&str),
    rebuild: bool,
) -> Result<Linked, String> {
    let e = |x: rusqlite::Error| x.to_string();
    let persons: i64 = conn
        .query_row("SELECT COUNT(*) FROM persons", [], |r| r.get(0))
        .map_err(e)?;
    if persons == 0 {
        return Ok(Linked {
            note: "no rijal work imported".into(),
            ..Default::default()
        });
    }
    // a database read before a better rule for the years of death (rijal.fix_death_years)
    crate::rijal::fix_death_years(conn).map_err(e)?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS core_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
        [],
    )
    .map_err(e)?;
    let guess = guesses_on(conn);
    let stamp = format!(
        "{}|{}|{}{}",
        m().id,
        persons_now(conn).map_err(e)?,
        tables().parser_id,
        if guess { "|guess" } else { "" }
    );
    let row: Option<String> = conn
        .query_row(
            "SELECT value FROM core_meta WHERE key = 'narrator_matcher'",
            [],
            |r| r.get(0),
        )
        .optional()
        .map_err(e)?;
    let pending: i64 = conn
        .query_row(
            "SELECT EXISTS (SELECT 1 FROM isnad_links WHERE candidates IS NULL)",
            [],
            |r| r.get(0),
        )
        .map_err(e)?;
    if !rebuild && row.as_deref() == Some(stamp.as_str()) && pending == 0 {
        return Ok(Linked {
            note: "up to date".into(),
            ..Default::default()
        });
    }
    say("Identifying narrators");
    let tx = conn.unchecked_transaction().map_err(e)?;
    tahwil_undo(&tx).map_err(e)?;
    let mut only_new = !rebuild && row.as_deref() == Some(stamp.as_str());
    if only_new {
        let all: i64 = tx
            .query_row(
                "SELECT COUNT(DISTINCT isnad_id) FROM isnad_links",
                [],
                |r| r.get(0),
            )
            .map_err(e)?;
        let new: i64 = tx
            .query_row(
                "SELECT COUNT(DISTINCT isnad_id) FROM isnad_links WHERE candidates IS NULL",
                [],
                |r| r.get(0),
            )
            .map_err(e)?;
        only_new = new * 50 < all;
    }
    let sql = format!(
        "SELECT i.id, i.reaches_prophet AND COALESCE(i.ends_with, 'prophet') = 'prophet', c.key, \
         COALESCE(c.tradition, 'sunni') FROM isnads i \
         JOIN hadiths h ON h.id = i.hadith_id JOIN collections c ON c.id = h.collection_id \
         WHERE EXISTS (SELECT 1 FROM isnad_links y WHERE y.isnad_id = i.id){}",
        if only_new {
            " AND EXISTS (SELECT 1 FROM isnad_links x WHERE x.isnad_id = i.id AND x.candidates IS NULL)"
        } else {
            ""
        }
    );
    let chains: Vec<(i64, i64, String, String)> = {
        let mut stmt = tx.prepare(&sql).map_err(e)?;
        let rows = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
            .map_err(e)?;
        rows.collect::<Result<_, _>>().map_err(e)?
    };
    let mut links_of: HashMap<i64, Vec<(i64, String)>> = HashMap::new();
    {
        let mut stmt = tx
            .prepare(
                "SELECT isnad_id, position, raw_name FROM isnad_links ORDER BY isnad_id, position",
            )
            .map_err(e)?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, Option<String>>(2)?,
                ))
            })
            .map_err(e)?;
        for row in rows {
            let (i, p, n) = row.map_err(e)?;
            links_of
                .entry(i)
                .or_default()
                .push((p, n.unwrap_or_default()));
        }
    }
    let mut indexes: HashMap<String, Index> = HashMap::new();
    let mut done: Vec<(String, Vec<Chain>)> = vec![];
    let mut order: Vec<(i64, String, usize)> = vec![];
    for (isnad_id, reaches, collection, tradition) in &chains {
        let road = if tradition == "shia" { "shia" } else { "sunni" }.to_string();
        if !indexes.contains_key(&road) {
            indexes.insert(road.clone(), Index::new(&tx, &road, guess).map_err(e)?);
        }
        let empty = vec![];
        let links = links_of.get(isnad_id).unwrap_or(&empty);
        let (results, cands, names) =
            match_chain(&indexes[&road], links, collection, *reaches != 0);
        let slot = match done.iter().position(|(r, _)| *r == road) {
            Some(k) => k,
            None => {
                done.push((road.clone(), vec![]));
                done.len() - 1
            }
        };
        done[slot].1.push(Chain {
            results,
            cands,
            names,
        });
        order.push((*isnad_id, road, done[slot].1.len() - 1));
    }
    for (road, pairs) in done.iter_mut() {
        let index = &indexes[road.as_str()];
        if !guess {
            // the rijal works' words alone: a father read from a son identified by them, nothing guessed
            by_relatives(pairs, index);
            continue;
        }
        by_neighbours(pairs);
        if !index.famous.is_empty() && by_famous(pairs, &index.famous) > 0 {
            by_neighbours(pairs);
        }
        if by_relatives(pairs, index) > 0 {
            by_neighbours(pairs);
        }
        if by_one_side(pairs) > 0 {
            by_neighbours(pairs);
            if by_relatives(pairs, index) > 0 {
                by_neighbours(pairs);
            }
        }
        if by_father_pair(pairs, index) > 0 {
            by_neighbours(pairs);
            if by_relatives(pairs, index) > 0 {
                by_neighbours(pairs);
            }
        }
    }
    let (mut total, mut identified) = (0, 0);
    {
        let mut up = tx
            .prepare("UPDATE isnad_links SET person_id = ?, match_confidence = ?, candidates = ? WHERE isnad_id = ? AND position = ?")
            .map_err(e)?;
        for (isnad_id, road, k) in &order {
            let slot = done.iter().position(|(r, _)| r == road).unwrap();
            let results = &mut done[slot].1[*k].results;
            drop_impossible(results, &indexes[road.as_str()]);
            let empty = vec![];
            let links = links_of.get(isnad_id).unwrap_or(&empty);
            for ((position, _), (pid, conf, count)) in links.iter().zip(results.iter()) {
                up.execute(params![pid, conf, count, isnad_id, position])
                    .map_err(e)?;
                total += 1;
                identified += pid.is_some() as usize;
            }
        }
    }
    let met = tahwil_join(&tx).map_err(e)?;
    tx.execute(
        "INSERT OR REPLACE INTO core_meta (key, value) VALUES ('narrator_matcher', ?)",
        [&stamp],
    )
    .map_err(e)?;
    tx.commit().map_err(e)?;
    crate::log::info(
        "narrators",
        &format!(
            "✔ narrators identified by {}: {} of {} links in {} chains; {} tahwil roads joined where the narrators attest the meeting (of {})",
            m().id,
            crate::py::thousands(identified as i64),
            crate::py::thousands(total as i64),
            crate::py::thousands(chains.len() as i64),
            crate::py::thousands(met.1),
            crate::py::thousands(met.0)
        ),
    );
    Ok(Linked {
        chains: chains.len(),
        links: total,
        identified,
        note: String::new(),
    })
}

// ------------------------------------------------------------------ core/tahwil_meet.py

fn meet_table(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS isnad_meetings (
            isnad_id INTEGER PRIMARY KEY, hadith_id INTEGER NOT NULL, ordinal INTEGER NOT NULL,
            first_position INTEGER NOT NULL, first_name TEXT NOT NULL, student_id INTEGER,
            teacher_id INTEGER, attested INTEGER NOT NULL, reaches_before INTEGER NOT NULL,
            ends_before TEXT)",
    )
}

fn tahwil_undo(conn: &Connection) -> rusqlite::Result<i64> {
    meet_table(conn)?;
    let rows: Vec<(i64, i64, i64, i64, String, i64, Option<String>)> = {
        let mut stmt = conn.prepare(
            "SELECT isnad_id, hadith_id, ordinal, first_position, first_name, reaches_before, ends_before FROM isnad_meetings",
        )?;
        let r = stmt.query_map([], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
            ))
        })?;
        r.collect::<Result<_, _>>()?
    };
    let mut undone = 0;
    for (isnad_id, hid, ordinal, first, name, reaches, ends) in rows {
        let row: Option<(i64, i64)> = conn
            .query_row(
                "SELECT hadith_id, ordinal FROM isnads WHERE id = ?",
                [isnad_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let link: Option<Option<String>> = conn
            .query_row(
                "SELECT raw_name FROM isnad_links WHERE isnad_id = ? AND position = ?",
                [isnad_id, first],
                |r| r.get(0),
            )
            .optional()?;
        if row != Some((hid, ordinal))
            || link.is_none()
            || link.unwrap().as_deref() != Some(name.as_str())
        {
            continue;
        }
        conn.execute(
            "DELETE FROM isnad_links WHERE isnad_id = ? AND position >= ?",
            [isnad_id, first],
        )?;
        conn.execute(
            "UPDATE isnads SET reaches_prophet = ?, ends_with = ? WHERE id = ?",
            params![reaches, ends, isnad_id],
        )?;
        undone += 1;
    }
    conn.execute("DELETE FROM isnad_meetings", [])?;
    Ok(undone)
}

type LinkRow = (
    i64,
    Option<String>,
    Option<String>,
    Option<i64>,
    Option<f64>,
    Option<i64>,
);

/// (looked, joined)
fn tahwil_join(conn: &Connection) -> rusqlite::Result<(i64, i64)> {
    meet_table(conn)?;
    let mut pairs: HashMap<(i64, i64), HashSet<i64>> = HashMap::new();
    {
        let mut stmt = conn.prepare(
            "SELECT i.hadith_id, l.isnad_id, l.position, l.person_id FROM isnad_links l JOIN isnads i ON i.id = l.isnad_id \
             WHERE i.problem IS NULL ORDER BY l.isnad_id, l.position",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, Option<i64>>(3)?,
            ))
        })?;
        let mut last: (Option<i64>, Option<i64>) = (None, None);
        for row in rows {
            let (hid, isnad_id, pid) = row?;
            if last.0 == Some(isnad_id) {
                if let (Some(a), Some(b)) = (last.1, pid) {
                    if a != b {
                        pairs.entry((a, b)).or_default().insert(hid);
                    }
                }
            }
            last = (Some(isnad_id), pid);
        }
    }
    let read_map = |sql: &str| -> rusqlite::Result<HashMap<i64, i64>> {
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        rows.collect()
    };
    let tabaqa = read_map("SELECT id, tabaqa FROM persons WHERE tabaqa IS NOT NULL")?;
    let died = read_map("SELECT id, death_year_ah FROM persons WHERE death_year_ah IS NOT NULL")?;
    let mut by_hadith: Vec<(i64, Vec<(i64, i64, i64, Option<String>)>)> = vec![];
    let mut borrowed: HashMap<i64, Vec<i64>> = HashMap::new();
    {
        let mut stmt = conn.prepare(
            "SELECT id, hadith_id, ordinal, reaches_prophet, ends_with, problem FROM isnads WHERE derived_by IS NOT NULL ORDER BY hadith_id, ordinal",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, i64>(3)?,
                r.get::<_, Option<String>>(4)?,
                r.get::<_, Option<String>>(5)?,
            ))
        })?;
        for row in rows {
            let (id, hid, ordinal, reaches, ends, prob) = row?;
            match prob {
                None => {
                    if by_hadith.last().map(|x| x.0) != Some(hid) {
                        by_hadith.push((hid, vec![]));
                    }
                    by_hadith
                        .last_mut()
                        .unwrap()
                        .1
                        .push((id, ordinal, reaches, ends));
                }
                Some(p) if p == problem("borrowed") && ordinal > 1 => {
                    borrowed.entry(hid).or_default().push(id)
                }
                _ => {}
            }
        }
    }
    let truthy = |x: Option<i64>| x.is_some_and(|v| v != 0);
    let fits = |student: i64, teacher: i64, seen: usize| -> bool {
        let back = pairs.get(&(teacher, student)).map(|s| s.len()).unwrap_or(0);
        if back >= seen {
            return false;
        }
        let (tt, ts) = (tabaqa.get(&teacher).copied(), tabaqa.get(&student).copied());
        if truthy(tt) && truthy(ts) && tt.unwrap() > ts.unwrap() {
            return false;
        }
        let (dt, ds) = (died.get(&teacher).copied(), died.get(&student).copied());
        if truthy(dt) && truthy(ds) {
            let d = ds.unwrap() - dt.unwrap();
            if !(-40..=100).contains(&d) {
                return false;
            }
        }
        true
    };
    let (mut joined, mut looked) = (0i64, 0i64);
    let named_re = pre!(
        r"(?<!\w)(?:كلهم|جميعا|كل هولاء|كل هؤلاء|كلاهما|ثلاثتهم|اربعتهم)\s*،?\s*(?:عن|قالوا حدثنا|قالا حدثنا|حدثنا|اخبرنا)\s+([^،,.:]{2,40})"
    );
    for (hid, roads) in &by_hadith {
        if roads.len() < 2 {
            continue;
        }
        let mut links: HashMap<i64, Vec<LinkRow>> = HashMap::new();
        for r in roads {
            let mut stmt = conn.prepare_cached(
                "SELECT position, raw_name, transmission, person_id, match_confidence, candidates FROM isnad_links WHERE isnad_id = ? ORDER BY position",
            )?;
            let rows = stmt.query_map([r.0], |x| {
                Ok((
                    x.get(0)?,
                    x.get(1)?,
                    x.get(2)?,
                    x.get(3)?,
                    x.get(4)?,
                    x.get(5)?,
                ))
            })?;
            links.insert(r.0, rows.collect::<Result<_, _>>()?);
        }
        let mut firsts: HashSet<Option<String>> = HashSet::new();
        for b in borrowed.get(hid).into_iter().flatten() {
            let f: Option<Option<String>> = conn
                .query_row(
                    "SELECT raw_name FROM isnad_links WHERE isnad_id = ? AND position = 1",
                    [b],
                    |r| r.get(0),
                )
                .optional()?;
            firsts.insert(f.flatten());
        }
        let mut norm: Option<String> = None;
        for (isnad_id, ordinal, reaches, ends) in roads {
            let mine = links[isnad_id].clone();
            if *reaches != 0 || ends.as_deref().is_some_and(|x| !x.is_empty()) || mine.is_empty() {
                continue;
            }
            let last = mine.last().unwrap().clone();
            let last_name = last.1.clone().unwrap_or_default();
            let Some(last_pid) = last.3 else {
                continue;
            };
            if is_companion(&Link::new(1, "", &last_name, false)) {
                continue;
            }
            if firsts.contains(&mine[0].1) {
                continue;
            }
            looked += 1;
            if norm.is_none() {
                let row: Option<Option<String>> = conn
                    .query_row(
                        "SELECT t.text FROM texts t JOIN editions e ON e.id = t.edition_id WHERE t.hadith_id = ? AND e.language = 'Arabic'",
                        [hid],
                        |r| r.get(0),
                    )
                    .optional()?;
                norm = Some(
                    pyre::upto(&normalize(&row.flatten().unwrap_or_default()), 3000).to_string(),
                );
            }
            let nm = norm.as_deref().unwrap();
            let np = pyre::Ps::new(nm);
            let at: i64 = if last_name.is_empty() {
                -1
            } else {
                np.find(pyre::upto(&normalize(&last_name), 20), 0)
                    .map(|x| x as i64)
                    .unwrap_or(-1)
            };
            let named: Vec<String> = named_re
                .finditer(nm)
                .into_iter()
                .filter(|mm| mm.start() as i64 > at && at >= 0)
                .map(|mm| mm.g(1).to_string())
                .collect();
            let mut best: Option<(bool, usize, i64, usize, i64, i64, Option<String>)> = None;
            for (other_id, _o, o_reaches, o_ends) in roads {
                let theirs = &links[other_id];
                if other_id == isnad_id
                    || !(*o_reaches != 0 || o_ends.as_deref().is_some_and(|x| !x.is_empty()))
                {
                    continue;
                }
                for j in 1..theirs.len() {
                    let Some(teacher) = theirs[j].3 else {
                        continue;
                    };
                    if teacher == last_pid {
                        continue;
                    }
                    let seen = pairs
                        .get(&(last_pid, teacher))
                        .map(|s| s.len() - s.contains(hid) as usize)
                        .unwrap_or(0);
                    let tn = theirs[j].1.clone().unwrap_or_default();
                    let said = named.iter().any(|n| shared(&tn, n));
                    if seen < if said { 1 } else { m().min_attested }
                        || !fits(last_pid, teacher, seen)
                    {
                        continue;
                    }
                    if best.as_ref().map_or(true, |b| (said, seen) > (b.0, b.1)) {
                        best = Some((
                            said,
                            seen,
                            *other_id,
                            j,
                            teacher,
                            *o_reaches,
                            o_ends.clone(),
                        ));
                    }
                }
            }
            let Some((said, seen, other_id, j, teacher, o_reaches, o_ends)) = best else {
                continue;
            };
            let tail: Vec<LinkRow> = links[&other_id][j..].to_vec();
            let first = mine.len() as i64 + 1;
            for (k, r) in tail.iter().enumerate() {
                conn.execute(
                    "INSERT INTO isnad_links (isnad_id, position, raw_name, transmission, person_id, match_confidence, candidates) VALUES (?, ?, ?, ?, ?, ?, ?)",
                    params![isnad_id, first + k as i64, r.1, r.2, r.3, r.4, r.5],
                )?;
            }
            conn.execute(
                "UPDATE isnads SET reaches_prophet = ?, ends_with = ? WHERE id = ?",
                params![o_reaches, o_ends, isnad_id],
            )?;
            conn.execute(
                "INSERT OR REPLACE INTO isnad_meetings (isnad_id, hadith_id, ordinal, first_position, first_name, student_id, teacher_id, attested, reaches_before, ends_before) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![isnad_id, hid, ordinal, first, tail[0].1, last_pid, teacher, if said { -(seen as i64) } else { seen as i64 }, reaches, ends],
            )?;
            let mut grown = mine.clone();
            grown.extend(
                tail.iter()
                    .enumerate()
                    .map(|(k, r)| (first + k as i64, r.1.clone(), r.2.clone(), r.3, r.4, r.5)),
            );
            links.insert(*isnad_id, grown);
            joined += 1;
        }
    }
    Ok((looked, joined))
}

#[allow(dead_code)]
fn unused(_: &Set) -> Option<()> {
    let _ = Rc::new(());
    None
}
