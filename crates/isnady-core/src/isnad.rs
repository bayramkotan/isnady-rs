//! The chains (isnads) the Python isnady read from the Arabic texts, and how they are shown — the Python isnady's
//! `isnady/core/isnad.py` (reading aids, the chain in words), `tahwil_meet.meeting`, and the parts of
//! `core/traditions.py` and `core/companions.py` a chain needs.
//!
//! Reading the chains from the texts (the parser, isnady-isnad-15) and identifying their narrators (the matcher,
//! isnady-narrators-17) stay with the Python isnady for now: `ready` says whether it has done so for the books
//! imported, and isnady-rs then shows exactly what it stored.

use regex::Regex;
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashMap;
use std::sync::OnceLock;

use crate::narrators::Person;
use crate::normalize::{normalize, normalize_with_map};
use crate::py;
use crate::{books, names, tables};

#[derive(Debug, Clone)]
pub struct Link {
    pub position: i64,
    pub raw_name: String,
    pub transmission: String,
    pub person_id: Option<i64>,
    pub match_confidence: Option<f64>,
    pub candidates: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct Meeting {
    pub first_position: i64,
    pub attested: i64,
    pub said: bool,
}

#[derive(Debug, Clone)]
pub struct Chain {
    pub id: i64,
    pub raw: Option<String>,
    pub reaches_prophet: bool,
    pub problem: Option<String>,
    pub ends_with: String,
    pub raised: bool,
    pub links: Vec<Link>,
    /// "sunni" or "shia": the road of the book
    pub tradition: String,
    pub meeting: Option<Meeting>,
}

fn re(p: &str) -> Regex {
    Regex::new(p).expect("pattern")
}

// ------------------------------------------------------------------ is the work done?

fn opt_i(conn: &Connection, sql: &str) -> rusqlite::Result<Option<i64>> {
    conn.query_row(sql, [], |r| r.get(0))
}

/// Every chain read by the Python isnady's parser of this version (`isnad.isnads_ready`).
pub fn isnads_ready(conn: &Connection) -> bool {
    let run = || -> rusqlite::Result<bool> {
        let row: Option<String> = conn
            .query_row(
                "SELECT value FROM core_meta WHERE key = 'isnads_ready'",
                [],
                |r| r.get(0),
            )
            .optional()?;
        let parts = [
            opt_i(conn, "SELECT COUNT(*) FROM hadiths")?,
            opt_i(conn, "SELECT MAX(id) FROM hadiths")?,
            opt_i(conn, "SELECT COUNT(*) FROM texts")?,
            opt_i(conn, "SELECT MAX(id) FROM texts")?,
            opt_i(conn, "SELECT COUNT(*) FROM isnads")?,
            opt_i(conn, "SELECT MAX(hadith_id) FROM isnads")?,
        ];
        let stamp = format!(
            "{}|{}",
            tables::get().parser_id,
            parts
                .iter()
                .map(|v| py::opt(*v))
                .collect::<Vec<_>>()
                .join("|")
        );
        Ok(row.as_deref() == Some(stamp.as_str()))
    };
    run().unwrap_or(false)
}

// ------------------------------------------------------------------ the stored chains

fn meeting(conn: &Connection, isnad_id: i64) -> Option<Meeting> {
    conn.query_row(
        "SELECT first_position, attested FROM isnad_meetings WHERE isnad_id = ?",
        params![isnad_id],
        |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)),
    )
    .optional()
    .ok()
    .flatten()
    .map(|(first_position, attested)| Meeting {
        first_position,
        attested: attested.abs(),
        said: attested < 0,
    })
}

/// The stored chains of a hadith with their links, for display (`isnad.chain`).
pub fn chain(conn: &Connection, hadith_id: i64) -> rusqlite::Result<Vec<Chain>> {
    let road: String = conn
        .query_row(
            "SELECT COALESCE(c.tradition, 'sunni') FROM hadiths h JOIN collections c ON c.id = h.collection_id \
             WHERE h.id = ?",
            params![hadith_id],
            |r| r.get(0),
        )
        .optional()?
        .unwrap_or_else(|| "sunni".into());
    let mut st = conn.prepare(
        "SELECT id, raw_text, reaches_prophet, problem, ends_with, raised FROM isnads \
         WHERE hadith_id = ? ORDER BY ordinal",
    )?;
    let heads = st
        .query_map(params![hadith_id], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, Option<i64>>(2)?.unwrap_or(0) != 0,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, Option<String>>(4)?.unwrap_or_default(),
                r.get::<_, Option<i64>>(5)?.unwrap_or(0) != 0,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut ls = conn.prepare(
        "SELECT position, raw_name, transmission, person_id, match_confidence, candidates \
         FROM isnad_links WHERE isnad_id = ? ORDER BY position",
    )?;
    let mut out = Vec::new();
    for (id, raw, reaches, problem, ends, raised) in heads {
        let links = ls
            .query_map(params![id], |r| {
                Ok(Link {
                    position: r.get(0)?,
                    raw_name: r.get(1)?,
                    transmission: r.get(2)?,
                    person_id: r.get(3)?,
                    match_confidence: r.get(4)?,
                    candidates: r.get(5)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        out.push(Chain {
            id,
            raw,
            reaches_prophet: reaches,
            problem,
            ends_with: if ends.is_empty() { String::new() } else { ends },
            raised,
            links,
            tradition: road.clone(),
            meeting: meeting(conn, id),
        });
    }
    Ok(out)
}

// ------------------------------------------------------------------ the two roads (core/traditions.py)

pub fn masum_label(key: &str) -> String {
    tables::get()
        .traditions
        .masum
        .get(key)
        .cloned()
        .unwrap_or_default()
}

/// 'group', 'previous', 'vague' or 'person' (`traditions.link_kind`).
pub fn link_kind(raw_name: &str) -> &'static str {
    let t = &tables::get().traditions;
    let folded = normalize(raw_name);
    let n = py::strip(&folded, " ،,");
    if t.group.iter().any(|g| g == n) {
        return "group";
    }
    if t.previous.iter().any(|g| g == n) {
        return "previous";
    }
    if t.vague.iter().any(|g| g == n)
        || ["بعض اصحاب", "رجل من", "بعض من", "من ذكره", "من حدثه"]
            .iter()
            .any(|p| n.starts_with(p))
    {
        return "vague";
    }
    "person"
}

/// The teachers al-Kulayni's "عدة من أصحابنا" stands for, read from the narrator it transmits from.
pub fn group_members(group: &str, next_name: &str) -> Vec<String> {
    let t = &tables::get().traditions;
    if normalize(group).trim() != t.group[0] {
        return Vec::new();
    }
    let nxt = normalize(next_name);
    for (head, members) in &t.idda {
        if nxt.starts_with(head.as_str()) {
            return members.clone();
        }
    }
    if nxt.starts_with("سهل") {
        return t.idda.get("سهل بن زياد").cloned().unwrap_or_default();
    }
    Vec::new()
}

// ------------------------------------------------------------------ reading aids

/// (Arabic, meaning) of a transmission term.
pub fn term_label(term: &str) -> (String, String) {
    match tables::get().isnad.terms.get(term) {
        Some((a, m)) => (a.clone(), m.clone()),
        None => (term.to_string(), term.to_string()),
    }
}

/// What the critics took a transmission term to mean, in English ("" for a term isnady does not know).
pub fn term_why(term: &str) -> String {
    tables::get()
        .isnad
        .term_why
        .get(term)
        .cloned()
        .unwrap_or_default()
}

/// What a link of a Shia chain is when it names no one person; "" for an ordinary name.
pub fn link_note(raw_name: &str, next_raw: &str) -> String {
    match link_kind(raw_name) {
        "group" => {
            let members = group_members(raw_name, next_raw);
            if members.is_empty() {
                "A group of al-Kulayni's teachers".into()
            } else {
                format!(
                    "A group of al-Kulayni's teachers: {} (as al-Kulayni explained it)",
                    members.join("، ")
                )
            }
        }
        "previous" => "\"From him\": the narrator who heads the previous chain in the book".into(),
        "vague" => "A narrator not named (mubham): the chain does not say who".into(),
        _ => String::new(),
    }
}

/// Whom a stored chain reaches, in words: "the Prophet ﷺ", "Imam Ja'far ibn Muhammad al-Sadiq" …
pub fn end_label(ch: &Chain) -> String {
    let key = if !ch.ends_with.is_empty() {
        ch.ends_with.clone()
    } else if ch.reaches_prophet {
        "prophet".into()
    } else {
        String::new()
    };
    if key.is_empty() {
        return String::new();
    }
    let mut label = if key == "imam" {
        "an Imam".to_string()
    } else {
        let m = masum_label(&key);
        if m.is_empty() {
            key.clone()
        } else {
            m
        }
    };
    if key == "prophet" {
        label = "the Prophet ﷺ".into();
    }
    if ch.raised {
        format!("raised (رفعه) to {label}")
    } else {
        label
    }
}

/// The name without clarifications ("- يعني ابن محمد -", "هو ابن اسلم") for compact display.
pub fn short_name(raw_name: &str) -> String {
    static RES: OnceLock<(Regex, Regex)> = OnceLock::new();
    let (a, b) = RES.get_or_init(|| {
        (
            re(r"\s*[،,]?\s*-\s*(?:وَ?يَعْنِي|و?يعني|وَ?هُوَ|و?هو)\s"),
            re(r"\s{2,}(?:هُوَ|هو)\s"),
        )
    });
    let name = a.split(raw_name).next().unwrap_or("");
    let name = b.split(name).next().unwrap_or("");
    py::strip(name, " ،,-").to_string()
}

/// A chain link's name in Latin letters as it is written in the chain — "حماد يعني ابن سلمة" → "Hammâd (yani İbn
/// Seleme)", "أبيه" → "babası". (text, spelled).
pub fn name_reading(raw_name: &str, language: &str) -> (String, bool) {
    static RES: OnceLock<(Regex, Regex, Regex)> = OnceLock::new();
    let (yani, huwa, ebu) = RES.get_or_init(|| {
        (
            re(r"\s*[،,]?\s*-?\s*(?:يَعْنِي|يعني)\s+"),
            re(r"\s*[،,]?\s*-\s*(?:وَ?هُوَ|و?هو)\s|\s{2,}(?:وَ?هُوَ|و?هو)\s"),
            re(r"\b(b\.|bint|İbn) Ebû "),
        )
    });
    let t = &tables::get().isnad;
    let tr = language == "tr";
    let lang = if tr { "tr" } else { "en" };
    let pick = |p: &(String, String)| if tr { p.0.clone() } else { p.1.clone() };
    let whole = py::strip(raw_name, " ،,-");
    let parts: Vec<&str> = yani.splitn(whole, 2).collect();
    let mut outs: Vec<String> = Vec::new();
    let mut spelled = false;
    for part in parts {
        let part = py::strip(huwa.split(part).next().unwrap_or(""), " ،,-");
        let key = normalize(part);
        if let Some(k) = t.kin.get(&key).or_else(|| t.kin_alone.get(&key)) {
            outs.push(pick(k));
            continue;
        }
        let words = py::words(part);
        let kin_word = if words.len() > 1 {
            normalize(words[0])
        } else {
            String::new()
        };
        if let Some(k) = t.kin.get(&kin_word) {
            let (text, guessed) = name_reading(&words[1..].join(" "), lang);
            let kin_text = pick(k);
            outs.push(if tr {
                format!("{kin_text} {text}")
            } else {
                format!("{kin_text}, {text}")
            });
            spelled = spelled || guessed;
            continue;
        }
        if !words.is_empty() && normalize(words[0]) == "ابن" {
            let mut rest = &words[1..];
            let mut head = if tr { "İbn" } else { "Ibn" }.to_string();
            if !rest.is_empty() && matches!(normalize(rest[0]).as_str(), "ابي" | "ابو" | "ابا")
            {
                head.push_str(if tr { " Ebî" } else { " Abi" });
                rest = &rest[1..];
            }
            let (text, guessed) = if rest.is_empty() {
                (String::new(), false)
            } else {
                names::reading(&rest.join(" "), lang, 8)
            };
            outs.push(format!("{head} {text}").trim().to_string());
            spelled = spelled || guessed;
            continue;
        }
        let (text, guessed) = names::reading(part, lang, 8);
        if !text.is_empty() {
            outs.push(text);
            spelled = spelled || guessed;
        }
    }
    if outs.is_empty() {
        return (String::new(), false);
    }
    let mut outs: Vec<String> = outs
        .into_iter()
        .map(|o| match o.strip_prefix("Abi ") {
            Some(rest) => format!("Abu {rest}"),
            None => o,
        })
        .collect();
    if tr {
        outs = outs
            .into_iter()
            .map(|o| ebu.replace_all(&o, "$1 Ebî ").into_owned())
            .collect();
    }
    outs[0] = py::upper_first(&outs[0]);
    if outs.len() == 2 {
        return (
            format!(
                "{} ({} {})",
                outs[0],
                if tr { "yani" } else { "that is," },
                outs[1]
            ),
            spelled,
        );
    }
    (outs.swap_remove(0), spelled)
}

// ------------------------------------------------------------------ the chain in words (ISN-TR1)

const TR_VOWELS: &str = "aeıioöuüâîû";

/// A Turkish name with its case ending by vowel harmony: Leys'ten, Ebû Hüreyre'den, Saîd'i, babasından.
pub fn tr_case(name: &str, case: &str) -> String {
    static ARTICLE: OnceLock<Regex> = OnceLock::new();
    let article = ARTICLE.get_or_init(|| re(r"^[ae][a-zş]{0,2}-"));
    let (head, tail) = match name.find(" (") {
        Some(i) => (&name[..i], Some(&name[i + 2..])),
        None => (name, None),
    };
    let word = head.trim_end();
    let letters: Vec<char> = word
        .chars()
        .flat_map(|c| c.to_lowercase())
        .filter(|c| py::isalpha(*c))
        .collect();
    if letters.is_empty() {
        return name.to_string();
    }
    let vowels: Vec<char> = letters
        .iter()
        .copied()
        .filter(|c| TR_VOWELS.contains(*c))
        .collect();
    let last = vowels.last().copied().unwrap_or('e');
    let back = "aıouâû".contains(last);
    let round = "ouöüû".contains(last);
    let ends_vowel = TR_VOWELS.contains(*letters.last().expect("letters"));
    let last_word = py::words(word)
        .into_iter()
        .rev()
        .find(|w| w.chars().any(py::isalpha))
        .unwrap_or(word);
    let common =
        last_word.chars().next().is_some_and(|c| c.is_lowercase()) && !article.is_match(last_word);
    let possessive = common
        && tables::get()
            .isnad
            .tr_possessive
            .iter()
            .any(|s| word.ends_with(s.as_str()));
    let four = if round {
        if back {
            "u"
        } else {
            "ü"
        }
    } else if back {
        "ı"
    } else {
        "i"
    };
    let suffix = match case {
        "abl" => {
            let lastc = *letters.last().expect("letters");
            let d = if possessive || !"fstkçşhp".contains(lastc) {
                "d"
            } else {
                "t"
            };
            format!(
                "{}{d}{}n",
                if possessive { "n" } else { "" },
                if back { "a" } else { "e" }
            )
        }
        "gen" => {
            if possessive || ends_vowel {
                format!("n{four}n")
            } else {
                format!("{four}n")
            }
        }
        "acc" => format!(
            "{}{four}",
            if possessive {
                "n"
            } else if ends_vowel {
                "y"
            } else {
                ""
            }
        ),
        _ => format!(
            "{}{}",
            if possessive {
                "n"
            } else if ends_vowel {
                "y"
            } else {
                ""
            },
            if back { "a" } else { "e" }
        ),
    };
    let mut out = format!(
        "{word}{}{suffix}",
        if common || word.ends_with('\'') {
            ""
        } else {
            "'"
        }
    );
    if let Some(tail) = tail {
        if !tail.is_empty() {
            out.push_str(" (");
            out.push_str(tail);
        }
    }
    out
}

/// A text of the chain's words in the language asked for, its {fields} filled (`isnad._said`).
fn said(text: &str, code: &str, values: &[(&str, String)]) -> String {
    let template = if code != "en" {
        tables::get()
            .isnad
            .tr
            .get(text)
            .filter(|_| code == "tr")
            .cloned()
            .unwrap_or_else(|| text.to_string())
    } else {
        text.to_string()
    };
    fill(&template, values)
        .or_else(|| {
            let names: Vec<(&str, String)> = values
                .iter()
                .filter(|(k, _)| *k == "name")
                .cloned()
                .collect();
            fill(text, &names)
        })
        .unwrap_or_else(|| text.to_string())
}

/// str.format with named fields: None when the template asks for one not given.
fn fill(template: &str, values: &[(&str, String)]) -> Option<String> {
    let mut out = String::new();
    let mut rest = template;
    while let Some(i) = rest.find('{') {
        out.push_str(&rest[..i]);
        let j = rest[i..].find('}')? + i;
        let key = &rest[i + 1..j];
        out.push_str(&values.iter().find(|(k, _)| *k == key)?.1);
        rest = &rest[j + 1..];
    }
    out.push_str(rest);
    Some(out)
}

fn known_as(person: Option<&Person>, code: &str) -> String {
    let Some(p) = person else {
        return String::new();
    };
    let arabic = if !p.known_arabic.is_empty() {
        p.known_arabic.clone()
    } else {
        py::words(&p.display_name)
            .into_iter()
            .take(3)
            .collect::<Vec<_>>()
            .join(" ")
    };
    let mut name = if arabic.is_empty() {
        String::new()
    } else {
        name_reading(&arabic, code).0
    };
    if code == "tr" {
        if let Some(rest) = name.strip_prefix("Ebî ") {
            name = format!("Ebû {rest}");
        }
    }
    name
}

pub(crate) fn is_companion_name(raw: &str) -> bool {
    static TRIM: OnceLock<fancy_regex::Regex> = OnceLock::new();
    let t = tables::get();
    let trim = TRIM.get_or_init(|| tables::pattern(&t.companions.trim));
    let folded = normalize(raw);
    let name = trim.replace_all(&folded, "").into_owned();
    let joined = py::words(&name).join(" ");
    t.companions.forms.contains(py::strip(&joined, " ،,-"))
}

fn is_companion_link(link: &Link, person: Option<&Person>) -> bool {
    if let Some(p) = person {
        let first = p
            .verdicts
            .iter()
            .filter(|v| v.rank_scheme.as_deref() != Some("shia"))
            .any(|v| v.rank == Some(1));
        return p.tabaqa == Some(1) || first;
    }
    is_companion_name(&short_name(&link.raw_name))
}

fn first_word_norm(raw_name: &str) -> String {
    py::words(&normalize(&short_name(raw_name)))
        .first()
        .map(|s| s.to_string())
        .unwrap_or_default()
}

/// Each link of the chain as a name in the language, without its case (`isnad.chain_link_names`).
pub fn chain_link_names(ch: &Chain, code: &str, people: &HashMap<i64, Person>) -> Vec<String> {
    let t = tables::get();
    let ti = &t.isnad;
    if !ti.languages.values().any(|c| c == code) || ch.links.is_empty() {
        return Vec::new();
    }
    static HONOUR: OnceLock<fancy_regex::Regex> = OnceLock::new();
    static ARTICLE: OnceLock<Regex> = OnceLock::new();
    static POSSESSOR: OnceLock<Regex> = OnceLock::new();
    let honour = HONOUR.get_or_init(|| tables::pattern(&ti.honour));
    let article = ARTICLE.get_or_init(|| re(r"^(?:A[lnrstdz]|E[lnrstdzş]|Aş)-"));
    let possessor = POSSESSOR.get_or_init(|| re(r"^(his|her) "));
    let tr = code == "tr";
    let mut names_out: Vec<String> = Vec::new();
    let mut bases: Vec<String> = Vec::new();
    let mut owners: Vec<String> = Vec::new();
    for (index, link) in ch.links.iter().enumerate() {
        let person = link.person_id.and_then(|id| people.get(&id));
        let mut raw = link.raw_name.clone();
        let (folded, wh) = normalize_with_map(&raw);
        if let Ok(Some(m)) = honour.find(&folded) {
            let start = py::char_index(&folded, m.start());
            if start > 0 {
                let cut = wh[start];
                raw = raw.chars().take(cut).collect();
            }
        }
        let mut words: Vec<&str> = py::words(&raw);
        if words.len() > 1
            && matches!(
                normalize(words[words.len() - 1]).as_str(),
                "انها" | "انه" | "انهما" | "انهم"
            )
        {
            words.pop();
        }
        let mut kin = String::new();
        if words.len() > 1 {
            if let Some(k) = ti.kin.get(&normalize(words[0])) {
                kin = if tr { k.0.clone() } else { k.1.clone() };
                words.remove(0);
            }
        }
        let mut raw = words.join(" ");
        let third_key = normalize(&raw);
        if let Some(third) = ti.kin_third.get(py::strip(&third_key, " ،,")) {
            raw = third.clone();
        }
        let (mut name, _spelled) = name_reading(&raw, code);
        let plain = name.clone();
        if !kin.is_empty() && !name.is_empty() {
            name = format!("{kin} {name}");
        }
        if name.is_empty() {
            name = short_name(&link.raw_name);
        }
        if tr {
            if let Some(rest) = name.strip_prefix("Ebî ") {
                name = format!("Ebû {rest}");
            }
        }
        let whose = if !link.raw_name.trim().is_empty() {
            let short_norm = normalize(&short_name(&link.raw_name));
            ti.whose.contains(&first_word_norm(&link.raw_name))
                || ti.kin_alone.contains_key(py::strip(&short_norm, " ،,"))
        } else {
            false
        };
        let known_any = known_as(person, code);
        let known = if whose {
            known_any.clone()
        } else {
            String::new()
        };
        if whose && index > 0 && !bases[index - 1].is_empty() {
            let owner = if !owners[index - 1].is_empty()
                && first_word_norm(&link.raw_name) == "جده"
                && matches!(
                    first_word_norm(&ch.links[index - 1].raw_name).as_str(),
                    "ابيه" | "ابوه"
                ) {
                owners[index - 1].clone()
            } else {
                bases[index - 1].clone()
            };
            owners.push(owner.clone());
            let relation = py::lower_first(if kin.is_empty() { &name } else { &kin });
            if tr {
                name = format!(
                    "{} {relation}{}",
                    tr_case(&owner, "gen"),
                    if !kin.is_empty() {
                        format!(" {plain}")
                    } else if !known.is_empty() {
                        format!(" {known}")
                    } else {
                        String::new()
                    }
                );
            } else {
                let relation = possessor.replace(&relation, "").into_owned();
                name = format!(
                    "{owner}'s {relation}{}",
                    if !kin.is_empty() {
                        format!(", {plain}")
                    } else if !known.is_empty() {
                        format!(", {known}")
                    } else {
                        String::new()
                    }
                );
            }
            bases.push(if !known.is_empty() {
                known.clone()
            } else if !kin.is_empty() {
                plain.clone()
            } else {
                name.clone()
            });
        } else {
            owners.push(String::new());
            if article.is_match(&name)
                || ti.kin.contains_key(&normalize(&short_name(&link.raw_name)))
            {
                name = py::lower_first(&name);
            }
            if whose && !known.is_empty() {
                name = if tr {
                    format!("{name} {known}")
                } else {
                    format!("{name}, {known}")
                };
            }
            bases.push(if !known_any.is_empty() {
                known_any.clone()
            } else {
                name.clone()
            });
        }
        if ch.tradition == "sunni" && is_companion_link(link, person) {
            name.push_str(if tr { " (r.a.)" } else { " (RA)" });
        }
        names_out.push(name);
    }
    names_out
}

/// One piece for each link and one for whom the chain reaches (`isnad.chain_word_parts`).
pub fn chain_word_parts(ch: &Chain, code: &str, people: &HashMap<i64, Person>) -> Vec<String> {
    let names = chain_link_names(ch, code, people);
    if names.is_empty() {
        return Vec::new();
    }
    let ti = &tables::get().isnad;
    let tr = code == "tr";
    let mut parts = Vec::new();
    for (link, name) in ch.links.iter().zip(names.iter()) {
        let term = if link.transmission.is_empty() {
            "bare"
        } else {
            link.transmission.as_str()
        };
        let template = ti
            .templates
            .get(term)
            .or_else(|| ti.templates.get("'an"))
            .cloned()
            .unwrap_or_default();
        let mut values = vec![("name", name.clone())];
        if tr {
            values.push(("name_abl", tr_case(name, "abl")));
            values.push(("name_acc", tr_case(name, "acc")));
            values.push(("name_dat", tr_case(name, "dat")));
            values.push(("name_gen", tr_case(name, "gen")));
        }
        parts.push(said(&template, code, &values));
    }
    let key = if !ch.ends_with.is_empty() {
        ch.ends_with.clone()
    } else if ch.reaches_prophet {
        "prophet".into()
    } else {
        String::new()
    };
    if !key.is_empty() && !ch.raised {
        let whom = if tr {
            ti.masum_tr
                .get(&key)
                .cloned()
                .unwrap_or_else(|| key.clone())
        } else if key == "prophet" {
            "the Prophet ﷺ".to_string()
        } else {
            let m = masum_label(&key);
            if m.is_empty() {
                key.clone()
            } else {
                m
            }
        };
        let mut values = vec![("name", whom.clone())];
        if tr {
            values.push(("name_abl", tr_case(&whom, "abl")));
        }
        parts.push(said(&ti.end, code, &values));
    } else if ch.raised {
        parts.push(said(&ti.raised, code, &[]));
    } else if ch.problem.as_deref().is_some_and(|p| !p.is_empty()) {
        parts.push(said(&ti.cut, code, &[]));
    }
    parts
}

/// The chain as the Arabic says it, in Turkish or English; "" for a language isnady cannot read names in.
pub fn chain_in_words(ch: &Chain, code: &str, people: &HashMap<i64, Person>) -> String {
    let parts = chain_word_parts(ch, code, people);
    if parts.is_empty() {
        return String::new();
    }
    let text = parts
        .iter()
        .filter(|p| !p.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    py::upper_first(&text) + "."
}

/// The languages, of those shown, the chain can be said in, as codes (`isnad.chain_codes`).
pub fn chain_codes(languages: &[String]) -> Vec<String> {
    let ti = &tables::get().isnad;
    let mut names: Vec<String> = if languages.is_empty() {
        let mut v: Vec<String> = ti.languages.keys().cloned().collect();
        v.sort();
        v
    } else {
        languages.to_vec()
    };
    let chosen = books::language_order();
    names.sort_by_key(|n| books::language_rank(n, &chosen));
    let mut out: Vec<String> = Vec::new();
    for n in names {
        if let Some(c) = ti.languages.get(&n) {
            if !out.contains(c) {
                out.push(c.clone());
            }
        }
    }
    out
}

// ------------------------------------------------------------------ the numbers of a book

pub struct NumberRange {
    pub first: String,
    pub last: String,
    pub count: i64,
}

/// The numbers of a book in its own order, or None (`isnad.number_range`).
pub fn number_range(conn: &Connection, book: &str) -> rusqlite::Result<Option<NumberRange>> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM hadiths h JOIN collections c ON c.id = h.collection_id WHERE c.key = ?",
        params![book],
        |r| r.get(0),
    )?;
    if count == 0 {
        return Ok(None);
    }
    let first: String = conn.query_row(
        "SELECT h.number FROM hadiths h JOIN collections c ON c.id = h.collection_id \
         WHERE c.key = ? ORDER BY h.number_sort, h.id LIMIT 1",
        params![book],
        |r| r.get(0),
    )?;
    let last: String = conn.query_row(
        "SELECT h.number FROM hadiths h JOIN collections c ON c.id = h.collection_id \
         WHERE c.key = ? ORDER BY h.number_sort DESC, h.id DESC LIMIT 1",
        params![book],
        |r| r.get(0),
    )?;
    Ok(Some(NumberRange { first, last, count }))
}

/// The place of a number written by the user in a book's order: "12a" after 12, "5:1740" as volume and hadith.
fn sort_key(number: &str) -> Option<f64> {
    let s = number.trim();
    if let Ok(v) = s.parse::<f64>() {
        return Some(v);
    }
    static NUM: OnceLock<Regex> = OnceLock::new();
    let m = NUM
        .get_or_init(|| re(r"^\s*(\d+)(?::(\d+))?"))
        .captures(number)?;
    let a: f64 = m[1].parse().ok()?;
    match m.get(2) {
        Some(b) => Some(a * 100000.0 + b.as_str().parse::<f64>().ok()?),
        None => Some(a),
    }
}

/// The book's number closest to one it does not have.
pub fn nearest_number(conn: &Connection, book: &str, number: &str) -> Option<String> {
    let key = sort_key(number)?;
    conn.query_row(
        "SELECT h.number FROM hadiths h JOIN collections c ON c.id = h.collection_id \
         WHERE c.key = ? ORDER BY ABS(h.number_sort - ?), h.number_sort LIMIT 1",
        params![book, key],
        |r| r.get(0),
    )
    .optional()
    .ok()
    .flatten()
}

/// A number of the book chosen at random — of a hadith whose chain was read, when the book has any.
pub fn random_number(conn: &Connection, book: &str) -> Option<String> {
    let with_chain: Option<String> = conn
        .query_row(
            "SELECT h.number FROM hadiths h JOIN collections c ON c.id = h.collection_id \
             WHERE c.key = ? AND EXISTS (SELECT 1 FROM isnads i JOIN isnad_links l ON l.isnad_id = i.id \
                                         WHERE i.hadith_id = h.id) \
             ORDER BY RANDOM() LIMIT 1",
            params![book],
            |r| r.get(0),
        )
        .optional()
        .ok()
        .flatten();
    with_chain.or_else(|| {
        conn.query_row(
            "SELECT h.number FROM hadiths h JOIN collections c ON c.id = h.collection_id \
             WHERE c.key = ? ORDER BY RANDOM() LIMIT 1",
            params![book],
            |r| r.get(0),
        )
        .optional()
        .ok()
        .flatten()
    })
}

/// The number before (delta < 0) or after (delta > 0) a number in the book's order; for a number the book does not
/// have, the nearest one on that side (`isnad.step_number`).
pub fn step_number(conn: &Connection, book: &str, number: &str, delta: i32) -> Option<String> {
    let rng = number_range(conn, book).ok().flatten()?;
    let number = number.trim();
    if number.is_empty() {
        return Some(if delta > 0 { rng.first } else { rng.last });
    }
    let here: Option<(f64, i64)> = conn
        .query_row(
            "SELECT h.number_sort, h.id FROM hadiths h JOIN collections c ON c.id = h.collection_id \
             WHERE c.key = ? AND h.number = ?",
            params![book, number],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .ok()
        .flatten();
    let (sort, id) = match here {
        Some(h) => h,
        None => match sort_key(number) {
            Some(k) => (k, if delta > 0 { -1 } else { 1i64 << 62 }),
            None => return Some(rng.first),
        },
    };
    let sql = if delta > 0 {
        "SELECT h.number FROM hadiths h JOIN collections c ON c.id = h.collection_id \
         WHERE c.key = ? AND (h.number_sort > ? OR (h.number_sort = ? AND h.id > ?)) \
         ORDER BY h.number_sort, h.id LIMIT 1"
    } else {
        "SELECT h.number FROM hadiths h JOIN collections c ON c.id = h.collection_id \
         WHERE c.key = ? AND (h.number_sort < ? OR (h.number_sort = ? AND h.id < ?)) \
         ORDER BY h.number_sort DESC, h.id DESC LIMIT 1"
    };
    let row: Option<String> = conn
        .query_row(sql, params![book, sort, sort, id], |r| r.get(0))
        .optional()
        .ok()
        .flatten();
    Some(row.unwrap_or_else(|| number.to_string()))
}

/// The hadith of a book by its number: (id, the book's name).
pub fn find_hadith(
    conn: &Connection,
    book: &str,
    number: &str,
) -> rusqlite::Result<Option<(i64, String)>> {
    conn.query_row(
        "SELECT h.id, c.name FROM hadiths h JOIN collections c ON c.id = h.collection_id \
         WHERE c.key = ? AND h.number = ?",
        params![book, number],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .optional()
}

/// The hadith of a book by its number as the book writes it (`isnad.hadith_id`).
pub fn hadith_id(conn: &Connection, book: &str, number: &str) -> Option<i64> {
    conn.query_row(
        "SELECT h.id FROM hadiths h JOIN collections c ON c.id = h.collection_id WHERE c.key = ? AND h.number = ?",
        params![book, number.trim()],
        |r| r.get(0),
    )
    .optional()
    .ok()
    .flatten()
}

/// The book's numbers that begin with what is typed, in the book's order (`isnad.numbers_starting`).
pub fn numbers_starting(conn: &Connection, book: &str, prefix: &str, limit: i64) -> Vec<String> {
    let prefix = prefix.trim();
    if prefix.is_empty() {
        return Vec::new();
    }
    let like = prefix
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
        + "%";
    let run = || -> rusqlite::Result<Vec<String>> {
        let mut st = conn.prepare(
            "SELECT h.number FROM hadiths h JOIN collections c ON c.id = h.collection_id \
             WHERE c.key = ? AND h.number LIKE ? ESCAPE '\\' ORDER BY h.number_sort, h.id LIMIT ?",
        )?;
        let rows = st.query_map(params![book, like, limit], |r| r.get(0))?;
        rows.collect()
    };
    run().unwrap_or_default()
}

/// (place, count): where a number stands among the book's hadith in its own order, or None for a number the book
/// does not have (`isnad.position`).
pub fn position(conn: &Connection, book: &str, number: &str) -> Option<(i64, i64)> {
    let (sort, id): (f64, i64) = conn
        .query_row(
            "SELECT h.number_sort, h.id FROM hadiths h JOIN collections c ON c.id = h.collection_id \
             WHERE c.key = ? AND h.number = ?",
            params![book, number.trim()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .ok()
        .flatten()?;
    let (before, count): (Option<i64>, i64) = conn
        .query_row(
            "SELECT SUM(CASE WHEN h.number_sort < ? OR (h.number_sort = ? AND h.id < ?) THEN 1 ELSE 0 END), COUNT(*) \
             FROM hadiths h JOIN collections c ON c.id = h.collection_id WHERE c.key = ?",
            params![sort, sort, id, book],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .ok()?;
    Some((before.unwrap_or(0) + 1, count))
}

/// A hadith's book, number and neighbours, for going through the book (`isnad.describe`).
#[derive(Debug, Clone)]
pub struct HadithInfo {
    pub id: i64,
    pub number: String,
    pub book: String,
    pub book_name: String,
    pub prev: Option<i64>,
    pub next: Option<i64>,
}

pub fn describe(conn: &Connection, hid: i64, lang: &str) -> Option<HadithInfo> {
    let (id, number, sort, cid, key, name): (i64, String, f64, i64, String, String) = conn
        .query_row(
            "SELECT h.id, h.number, h.number_sort, h.collection_id, c.key, c.name FROM hadiths h \
             JOIN collections c ON c.id = h.collection_id WHERE h.id = ?",
            params![hid],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                ))
            },
        )
        .optional()
        .ok()
        .flatten()?;
    let one = |sql: &str| -> Option<i64> {
        conn.query_row(sql, params![cid, sort, sort, id], |r| r.get(0))
            .optional()
            .ok()
            .flatten()
    };
    let prev = one(
        "SELECT id FROM hadiths WHERE collection_id = ? AND (number_sort < ? OR (number_sort = ? AND id < ?)) \
         ORDER BY number_sort DESC, id DESC LIMIT 1",
    );
    let next = one(
        "SELECT id FROM hadiths WHERE collection_id = ? AND (number_sort > ? OR (number_sort = ? AND id > ?)) \
         ORDER BY number_sort, id LIMIT 1",
    );
    Some(HadithInfo {
        id,
        number,
        book_name: crate::works::book_title(&key, &name, lang),
        book: key,
        prev,
        next,
    })
}

/// The Arabic text of a hadith, of its first Arabic edition (`isnad.arabic_text`).
pub fn arabic_text(conn: &Connection, hid: i64) -> Option<String> {
    conn.query_row(
        "SELECT t.text FROM texts t JOIN editions e ON e.id = t.edition_id \
         WHERE t.hadith_id = ? AND e.language = 'Arabic' ORDER BY e.id LIMIT 1",
        params![hid],
        |r| r.get(0),
    )
    .optional()
    .ok()
    .flatten()
}

/// (chain part, text part) of the Arabic text, by the stored raw chain (`isnad.split_text`).
pub fn split_text(text: &str, raw_isnad: Option<&str>) -> (String, String) {
    let Some(raw) = raw_isnad.filter(|r| !r.is_empty()) else {
        return (String::new(), text.to_string());
    };
    match text.find(raw) {
        Some(at) => {
            let end = at + raw.len();
            (text[..end].to_string(), text[end..].to_string())
        }
        None => (String::new(), text.to_string()),
    }
}

/// One book's chains as counted for the overview (`isnad.book_stats`).
#[derive(Debug, Clone)]
pub struct BookStats {
    pub key: String,
    /// the name stored with the book (the overview shows its title in the interface language)
    pub stored_name: String,
    pub chains: i64,
    pub split: i64,
    pub marfu: i64,
    pub tradition: String,
    pub links: i64,
    pub problems: Vec<(String, i64)>,
}

/// A value counted once while the database is unchanged, as the Python isnady keeps it (core.cache).
fn kept<T: Clone + Send + 'static>(
    store: &'static std::sync::Mutex<Option<(String, T)>>,
    conn: &Connection,
    count: impl FnOnce() -> rusqlite::Result<T>,
) -> rusqlite::Result<T> {
    let version: i64 = conn.query_row("PRAGMA data_version", [], |r| r.get(0))?;
    let stamp: (Option<i64>, Option<i64>, Option<i64>) = conn.query_row(
        "SELECT (SELECT MAX(id) FROM isnads), (SELECT COUNT(*) FROM isnads), (SELECT SUM(person_id) FROM isnad_links)",
        [],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    let key = format!("{:?}|{version}|{stamp:?}", conn.path());
    if let Ok(last) = store.lock() {
        if let Some((k, v)) = last.as_ref() {
            if *k == key {
                return Ok(v.clone());
            }
        }
    }
    let v = count()?;
    if let Ok(mut last) = store.lock() {
        *last = Some((key, v.clone()));
    }
    Ok(v)
}

/// Per book: chains read, split into narrators, reaching the Prophet (on road B: a Ma'sum), and why the rest were not
/// split, in the books' reading order.
pub fn book_stats(conn: &Connection) -> rusqlite::Result<Vec<BookStats>> {
    static KEPT: std::sync::Mutex<Option<(String, Vec<BookStats>)>> = std::sync::Mutex::new(None);
    kept(&KEPT, conn, || {
        let order = books::order_sql(conn, "c.key")?;
        let mut st = conn.prepare(&format!(
            "SELECT c.id, c.key, c.name, COUNT(*), SUM(i.problem IS NULL), \
                    SUM(i.problem IS NULL AND i.reaches_prophet = 1), COALESCE(c.tradition, 'sunni') \
             FROM isnads i JOIN hadiths h ON h.id = i.hadith_id JOIN collections c ON c.id = h.collection_id \
             WHERE i.derived_by IS NOT NULL AND (i.problem IS NULL OR i.ordinal = 1) \
             GROUP BY c.id ORDER BY {order}"
        ))?;
        type Row = (i64, String, String, i64, Option<i64>, Option<i64>, String);
        let rows: Vec<Row> = st
            .query_map([], |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                ))
            })?
            .collect::<rusqlite::Result<_>>()?;
        let mut problems = conn.prepare(
            "SELECT i.problem, COUNT(*) FROM isnads i JOIN hadiths h ON h.id = i.hadith_id \
             WHERE h.collection_id = ? AND i.problem IS NOT NULL AND i.ordinal = 1 GROUP BY i.problem ORDER BY 2 DESC",
        )?;
        let mut links = conn.prepare(
            "SELECT COUNT(*) FROM isnad_links l JOIN isnads i ON i.id = l.isnad_id JOIN hadiths h ON h.id = i.hadith_id \
             WHERE h.collection_id = ? AND i.problem IS NULL",
        )?;
        let mut out = Vec::new();
        for (cid, key, name, chains, split, marfu, tradition) in rows {
            let ps: Vec<(String, i64)> = problems
                .query_map(params![cid], |r| Ok((r.get(0)?, r.get(1)?)))?
                .collect::<rusqlite::Result<_>>()?;
            let n: i64 = links.query_row(params![cid], |r| r.get(0))?;
            out.push(BookStats {
                key,
                stored_name: name,
                chains,
                split: split.unwrap_or(0),
                marfu: marfu.unwrap_or(0),
                tradition,
                links: n,
                problems: ps,
            });
        }
        Ok(out)
    })
}

/// {book key: (links, identified, looked up)}
pub type Known = HashMap<String, (i64, i64, i64)>;

/// {book key: (links, identified, looked up)} of the whole chains (gui/isnad_page.py `_known_by_book`).
pub fn known_by_book(conn: &Connection) -> rusqlite::Result<Known> {
    static KEPT: std::sync::Mutex<Option<(String, Known)>> = std::sync::Mutex::new(None);
    kept(&KEPT, conn, || {
        let mut st = conn.prepare(
            "SELECT c.key, COUNT(*), SUM(l.person_id IS NOT NULL), SUM(l.candidates IS NOT NULL) FROM isnad_links l \
             JOIN isnads i ON i.id = l.isnad_id JOIN hadiths h ON h.id = i.hadith_id \
             JOIN collections c ON c.id = h.collection_id WHERE i.problem IS NULL GROUP BY c.key",
        )?;
        let rows = st.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                (
                    r.get::<_, i64>(1)?,
                    r.get::<_, Option<i64>>(2)?.unwrap_or(0),
                    r.get::<_, Option<i64>>(3)?.unwrap_or(0),
                ),
            ))
        })?;
        rows.collect()
    })
}

/// Per book, by the name stored with it: (name, links, identified) of the chains split into narrators, in the books'
/// reading order (what `iy isnads` says after the counts).
pub fn identified_by_book(conn: &Connection) -> rusqlite::Result<Vec<(String, i64, i64)>> {
    let order = books::order_sql(conn, "c.key")?;
    let mut st = conn.prepare(&format!(
        "SELECT c.name, COUNT(*), SUM(l.person_id IS NOT NULL) FROM isnad_links l \
         JOIN isnads i ON i.id = l.isnad_id JOIN hadiths h ON h.id = i.hadith_id \
         JOIN collections c ON c.id = h.collection_id WHERE i.problem IS NULL GROUP BY c.id ORDER BY {order}"
    ))?;
    let rows = st.query_map([], |r| {
        Ok((
            r.get(0)?,
            r.get(1)?,
            r.get::<_, Option<i64>>(2)?.unwrap_or(0),
        ))
    })?;
    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cases() {
        assert_eq!(tr_case("Leys", "abl"), "Leys'ten");
        assert_eq!(tr_case("Ebû Hüreyre", "abl"), "Ebû Hüreyre'den");
        assert_eq!(tr_case("babası", "abl"), "babasından");
        assert_eq!(
            tr_case("Hz. Peygamber (s.a.v.)", "abl"),
            "Hz. Peygamber'den (s.a.v.)"
        );
        assert_eq!(tr_case("Nâfi'", "abl"), "Nâfi'den");
    }

    #[test]
    fn short_names() {
        assert_eq!(short_name("حماد - يعني ابن سلمة -"), "حماد");
    }
}
