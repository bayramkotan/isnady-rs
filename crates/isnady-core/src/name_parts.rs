//! The parts of an Arabic name (NM1) — the Python isnady's `core/name_parts.py`, word for word: ism (given name),
//! nasab (lineage: b. …), kunya (Abu …), laqab (by-name), nisbas (origin, tribe, trade), wala' (client of …), and
//! which of them he is KNOWN BY (shuhra) — al-A'mash is Sulayman b. Mihran, known by his laqab; Ibn 'Umar by his
//! father's name; Abu Hurayra by his kunya.

use regex::Regex;
use std::sync::OnceLock;

use crate::normalize::normalize;
use crate::{i18n, names, py, tables};

type Words = Vec<String>;

const KUNYA: [&str; 4] = ["أبو", "أبي", "أبا", "أم"];

/// (label, transliterated, Arabic, meaning) of each part (`PART_LABELS`), in English.
pub const PART_LABELS: [(&str, &str, &str, &str, &str); 7] = [
    ("ism", "Name", "ism", "الاسم", "the given name"),
    (
        "nasab",
        "Lineage",
        "nasab",
        "النسب",
        "son of … (b. = ibn, son of)",
    ),
    (
        "kunya",
        "Kunya",
        "kunya",
        "الكنية",
        "Abu … / Umm …: father or mother of …",
    ),
    ("laqab", "By-name", "laqab", "اللقب", "a nickname or title"),
    (
        "nisba",
        "Nisbas",
        "nisba",
        "النسبة",
        "where he was from, his tribe or his trade",
    ),
    (
        "wala",
        "Client of",
        "wala'",
        "الولاء",
        "a freedman or client of a tribe",
    ),
    ("also", "Also called", "", "", "other names the work gives"),
];

fn known_by(kind: &str) -> &'static str {
    match kind {
        "laqab" => "his by-name (laqab)",
        "nisba" => "his nisba",
        "ibn" => "his father's or an ancestor's name",
        "kunya" => "his kunya",
        "ism" => "his name",
        "given" => "this name",
        _ => "",
    }
}

#[derive(Debug, Clone, Default)]
pub struct Parts {
    pub ism: Option<Words>,
    pub nasab: Vec<Words>,
    pub kunya: Option<Words>,
    pub laqab: Vec<Words>,
    pub nisba: Vec<Words>,
    pub wala: Vec<Words>,
    pub also: Vec<Words>,
    /// (how he is known: "ism", "ibn", "kunya", "laqab", "nisba", "given"; the words)
    pub known: Option<(String, Words)>,
}

/// One row of the name laid out: the part, its (Arabic, reading) values, whether he is known by it.
pub type Row = (String, Vec<(String, String)>, bool);

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct View {
    pub arabic: String,
    pub reading: String,
    pub kind: String,
    pub how: String,
    pub full_arabic: String,
    pub full_reading: String,
    pub rows: Vec<Row>,
    pub spelled: bool,
}

fn words_of(s: &str) -> Words {
    py::words(s).into_iter().map(String::from).collect()
}

fn is(w: &str, set: &[&str]) -> bool {
    set.contains(&w)
}

/// How many words the name starting at i takes: عبد الله, عبيد الله, أبي بكر are one name.
fn take_name(words: &[String], i: usize) -> usize {
    if i + 1 < words.len() && (is(&words[i], &["عبد", "عبيد"]) || words[i + 1] == "الله")
    {
        return 2;
    }
    if i + 1 < words.len() && is(&words[i], &KUNYA) {
        return if i + 2 < words.len() && words[i + 1] == "عبد" {
            3
        } else {
            2
        };
    }
    1
}

fn slice(words: &[String], from: usize, to: usize) -> Words {
    words[from.min(words.len())..to.min(words.len())].to_vec()
}

fn ends_ya(w: &str) -> bool {
    w.ends_with('ي') || w.ends_with("ية")
}

/// The parts of a name (`name_parts.parse`): the rijal entry's own name, its kunya, the aliases the work gives, the
/// words of the entry ("يلقب …", "المعروف ب…"), and the name the caller knows him by (the scholars' register).
pub fn parse(
    name: &str,
    kunya: Option<&str>,
    aliases: &[String],
    text: Option<&str>,
    known_as: Option<&str>,
) -> Parts {
    let t = tables::get();
    let cleaned: String = name
        .chars()
        .map(|c| if "،,.:؛()[]".contains(c) { ' ' } else { c })
        .collect();
    let mut words = words_of(&cleaned);
    if let Some(cut) = words
        .iter()
        .position(|w| is(w, &["يلقب", "ويلقب", "الملقب", "لقبه", "ولقبه"]))
    {
        words.truncate(cut);
    }
    let mut out = Parts::default();
    let mut i = 0;
    let ubayy = words.first().is_some_and(|w| w == "أبي")
        && words.len() > 1
        && is(&words[1], &["بن", "ابن"]);
    if !words.is_empty() && is(&words[0], &KUNYA) && !ubayy {
        let n = take_name(&words, 0);
        out.kunya = Some(slice(&words, 0, n));
        i = n;
        if i + 1 < words.len() && !words[i].starts_with("ال") && is(&words[i + 1], &["بن", "بنت"])
        {
            let m = take_name(&words, i);
            out.ism = Some(slice(&words, i, i + m));
            i += m;
        }
    } else if ubayy {
        out.ism = Some(vec!["أبيّ".to_string()]);
        i = 1;
    } else if !words.is_empty() {
        let n = take_name(&words, 0);
        out.ism = Some(slice(&words, 0, n));
        i = n;
    }
    // a given name of several words, with no "b." after it: شاه ولي الله, زبير علي زئي
    while out.ism.as_ref().is_some_and(|v| !v.is_empty())
        && i < words.len()
        && !is(&words[i], &["بن", "بنت", "ابن", "مولى", "مولاهم"])
        && !words[i].starts_with("ال")
        && !is(&words[i], &KUNYA)
        && !(i + 1 < words.len() && words[i + 1] == "الدين")
    {
        let n = take_name(&words, i);
        let mut more = out.ism.take().unwrap_or_default();
        more.extend(slice(&words, i, i + n));
        out.ism = Some(more);
        i += n;
    }
    while i + 1 < words.len() && is(&words[i], &["بن", "بنت", "ابن"]) {
        let n = take_name(&words, i + 1);
        out.nasab.push(slice(&words, i, i + 1 + n));
        i += 1 + n;
    }
    while i < words.len() {
        let w = words[i].clone();
        if is(&w, &KUNYA) && i + 1 < words.len() {
            let n = take_name(&words, i);
            if out.kunya.is_none() {
                out.kunya = Some(slice(&words, i, i + n));
            }
            i += n;
            continue;
        }
        if w == "مولى" && i + 1 < words.len() {
            let n = take_name(&words, i + 1);
            out.wala.push(slice(&words, i, i + 1 + n));
            i += 1 + n;
            continue;
        }
        if w == "مولاهم" || w == "مولاه" {
            out.wala.push(vec![w.clone()]);
        } else if w.starts_with("ال") && w.chars().count() > 3 {
            if ends_ya(&w) {
                out.nisba.push(vec![w.clone()]);
            } else {
                out.laqab.push(vec![w.clone()]);
            }
        } else if w == "ثم" {
        } else if out.ism.as_ref().is_some_and(|v| !v.is_empty())
            && out.nisba.is_empty()
            && out.laqab.is_empty()
            && w.chars().count() > 2
            && i + 1 < words.len()
            && words[i + 1] == "الدين"
        {
            out.laqab.push(slice(&words, i, i + 2));
            i += 2;
            continue;
        }
        i += 1;
    }
    if out.kunya.as_ref().map_or(true, |v| v.is_empty()) {
        if let Some(k) = kunya.filter(|k| !k.is_empty()) {
            out.kunya = Some(words_of(k));
        }
    }
    static LAQAB: OnceLock<fancy_regex::Regex> = OnceLock::new();
    static KNOWN: OnceLock<fancy_regex::Regex> = OnceLock::new();
    let laqab_re = LAQAB.get_or_init(|| tables::pattern(&t.name_parts.laqab));
    let known_re = KNOWN.get_or_init(|| tables::pattern(&t.name_parts.known));
    let mut stated: Option<Words> = None;
    if let Some(text) = text.filter(|s| !s.is_empty()) {
        if let Some(m) = laqab_re
            .captures(text)
            .ok()
            .flatten()
            .and_then(|c| c.get(1))
        {
            let w = words_of(m.as_str());
            if !out.laqab.contains(&w) {
                out.laqab.push(w);
            }
        }
        if let Some(k) = known_re
            .captures(text)
            .ok()
            .flatten()
            .and_then(|c| c.get(1))
        {
            let s = words_of(&k.as_str().replace("بابن", "ابن"));
            if s.first().map(String::as_str) != Some("ابن") && !out.laqab.contains(&s) {
                out.laqab.push(s.clone());
            }
            stated = Some(s);
        }
    }
    let mut clean: Vec<Words> = Vec::new();
    for a in aliases {
        let aw = words_of(a);
        if !(1..=3).contains(&aw.len()) || aw.iter().any(|x| is(x, &["عن", "هو", "و"])) {
            continue;
        }
        if aw
            .iter()
            .any(|x| t.name_parts.not_names.contains(&normalize(x)))
        {
            continue;
        }
        clean.push(aw);
    }
    let mut known: Option<(String, Words)> = None;
    if let Some(known_as) = known_as.filter(|k| !k.is_empty()) {
        let kw = words_of(known_as);
        let ism = out.ism.clone().unwrap_or_default();
        known = Some(if !ism.is_empty() && kw == ism[..kw.len().min(ism.len())] {
            ("ism".into(), kw)
        } else if is(&kw[0], &["ابن", "بن"]) {
            ("ibn".into(), kw)
        } else if is(&kw[0], &KUNYA) {
            ("kunya".into(), kw)
        } else if kw.len() == 1 && kw[0].starts_with("ال") {
            (
                if kw[0].ends_with('ي') {
                    "nisba"
                } else {
                    "laqab"
                }
                .into(),
                kw,
            )
        } else {
            ("given".into(), kw)
        });
    }
    let mut ranked: Vec<(u8, (String, Words))> = Vec::new();
    for aw in &clean {
        if is(&aw[0], &["بن", "ابن"]) && aw.len() >= 2 {
            ranked.push((1, ("ibn".into(), aw.clone())));
        } else if aw.len() == 1 && aw[0].starts_with("ال") && words.contains(&aw[0]) {
            ranked.push(if aw[0].ends_with('ي') {
                (2, ("nisba".into(), aw.clone()))
            } else {
                (0, ("laqab".into(), aw.clone()))
            });
        }
    }
    if known.is_none() {
        if let Some(best) = ranked.iter().map(|r| r.0).min() {
            known = ranked.iter().find(|r| r.0 == best).map(|r| r.1.clone());
        }
    }
    if known.is_none()
        && words.first().is_some_and(|w| is(w, &KUNYA))
        && out.kunya.as_ref().is_some_and(|k| !k.is_empty())
        && out.ism.as_deref() != Some(&["أبيّ".to_string()][..])
    {
        known = Some(("kunya".into(), out.kunya.clone().unwrap_or_default()));
    }
    if known.is_none() {
        if let Some(s) = &stated {
            known = Some((
                if s.first().map(String::as_str) == Some("ابن") {
                    "ibn"
                } else {
                    "laqab"
                }
                .into(),
                s.clone(),
            ));
        }
    }
    if known.is_none() {
        if let Some(ism) = out.ism.as_ref().filter(|v| !v.is_empty()) {
            let mut w = ism.clone();
            if let Some(first) = out.nasab.first() {
                w.extend(first.iter().cloned());
            }
            known = Some(("ism".into(), w));
        }
    }
    if let Some((kind, w)) = &known {
        let list = match kind.as_str() {
            "laqab" => Some(&mut out.laqab),
            "nisba" => Some(&mut out.nisba),
            _ => None,
        };
        if let Some(list) = list {
            if !list.contains(w) {
                list.push(w.clone());
            }
        }
    }
    let shown: Vec<&Words> = out
        .laqab
        .iter()
        .chain(out.nisba.iter())
        .chain(out.wala.iter())
        .collect();
    let known_words: Words = known.as_ref().map(|k| k.1.clone()).unwrap_or_default();
    out.also = clean
        .iter()
        .filter(|aw| !shown.contains(aw) && **aw != known_words)
        .cloned()
        .collect();
    out.known = known;
    out
}

/// The reading of a part in a language, written as inside a name: "al-Kahili", "b. Mihran", not "Al-…", "B. …"
/// (`name_parts._r`).
fn reading_of(words: &[String], language: &str) -> String {
    static ART: OnceLock<Regex> = OnceLock::new();
    let art = ART.get_or_init(|| Regex::new(r"^(Al|El|Ez|Es|Et|Ed|En|Er|Eş|Ed)-|^B\. ").unwrap());
    let text = names::reading(&words.join(" "), language, 8).0;
    if art.is_match(&text) {
        py::lower_first(&text)
    } else {
        text
    }
}

fn spelled(words: &[String], language: &str) -> bool {
    !words.is_empty() && names::reading(&words.join(" "), language, 8).1
}

/// For display (`name_parts.present`): the known-as name (Arabic, reading, how), the full name, one row per part.
/// reading: the language names are read in (the interface's, `language.ui_language`); words: the language of the
/// interface's own words ("(place)", "their client").
pub fn present(parts: &Parts, reading: &str, words: &str) -> View {
    let t = tables::get();
    let (kind, kw) = parts
        .known
        .clone()
        .unwrap_or_else(|| ("ism".into(), Vec::new()));
    let mut arabic = kw.join(" ");
    let mut read;
    if kind == "ibn" {
        let rest_words = kw.get(1..).unwrap_or(&[]).to_vec();
        arabic = format!("ابن {}", rest_words.join(" "));
        let rest = reading_of(&rest_words, reading);
        read = if rest.is_empty() {
            String::new()
        } else {
            format!("Ibn {rest}")
        };
    } else {
        read = reading_of(&kw, reading);
    }
    let named = kw.iter().filter(|w| !is(w, &["بن", "ابن"])).count() as i64;
    if !read.is_empty() && (py::words(&read).len() as i64) < named - 1 {
        read = String::new();
    }
    let mut full: Words = parts.ism.clone().unwrap_or_default();
    for p in &parts.nasab {
        full.extend(p.iter().cloned());
    }
    let joined = |p: &Words| p.join(" ");
    let mut rows: Vec<Row> = Vec::new();
    if let Some(ism) = parts.ism.as_ref().filter(|v| !v.is_empty()) {
        rows.push((
            "ism".into(),
            vec![(joined(ism), reading_of(ism, reading))],
            kind == "ism",
        ));
    }
    if !parts.nasab.is_empty() {
        rows.push((
            "nasab".into(),
            parts
                .nasab
                .iter()
                .map(|p| (joined(p), reading_of(p, reading)))
                .collect(),
            kind == "ibn",
        ));
    }
    if let Some(k) = parts.kunya.as_ref().filter(|v| !v.is_empty()) {
        rows.push((
            "kunya".into(),
            vec![(joined(k), reading_of(k, reading))],
            kind == "kunya",
        ));
    }
    if !parts.laqab.is_empty() {
        rows.push((
            "laqab".into(),
            parts
                .laqab
                .iter()
                .map(|p| (joined(p), reading_of(p, reading)))
                .collect(),
            kind == "laqab",
        ));
    }
    if !parts.nisba.is_empty() {
        let place = i18n::t(words, "(place)", &[]);
        rows.push((
            "nisba".into(),
            parts
                .nisba
                .iter()
                .map(|p| {
                    let mut r = reading_of(p, reading);
                    if t.name_parts.places.contains(&normalize(&p[0])) {
                        r = format!("{r} {place}");
                    }
                    (joined(p), r)
                })
                .collect(),
            kind == "nisba",
        ));
    }
    if !parts.wala.is_empty() {
        rows.push((
            "wala".into(),
            parts
                .wala
                .iter()
                .map(|p| {
                    (
                        joined(p),
                        if p[0] == "مولاهم" {
                            i18n::t(words, "their client", &[])
                        } else {
                            reading_of(p, reading)
                        },
                    )
                })
                .collect(),
            false,
        ));
    }
    if !parts.also.is_empty() {
        rows.push((
            "also".into(),
            parts
                .also
                .iter()
                .map(|p| (joined(p), reading_of(p, reading)))
                .collect(),
            false,
        ));
    }
    View {
        arabic: if arabic.is_empty() {
            full.join(" ")
        } else {
            arabic
        },
        reading: read,
        how: i18n::t(words, known_by(&kind), &[]),
        kind,
        full_arabic: full.join(" "),
        full_reading: reading_of(&full, reading),
        rows,
        spelled: spelled(&kw, reading) || spelled(&full, reading),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_by() {
        let p = parse(
            "سليمان بن مهران الأسدي الكاهلي مولاهم الأعمش",
            None,
            &["الأعمش".into()],
            None,
            None,
        );
        assert_eq!(p.known.as_ref().unwrap().0, "laqab");
        assert_eq!(p.nasab, vec![vec!["بن".to_string(), "مهران".into()]]);
        let p = parse("أبو هريرة الدوسي", None, &[], None, None);
        assert_eq!(p.known.as_ref().unwrap().0, "kunya");
        let p = parse("أبي بن كعب بن قيس", None, &[], None, None);
        assert_eq!(p.ism, Some(vec!["أبيّ".to_string()]));
        let v = present(
            &parse(
                "عبد الله بن عمر بن الخطاب",
                None,
                &["ابن عمر".into()],
                None,
                None,
            ),
            "en",
            "en",
        );
        assert_eq!(v.arabic, "ابن عمر");
    }
}
