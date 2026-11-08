//! Learn (G7) and the glossary (S1): the Python isnady's `core/learn.py` — the terms of hadith and its sciences with
//! their Arabic, a one-sentence definition and a longer one — and `core/learn_history.py` (LRN-H1): the articles of
//! History and method, with blocks of numbers counted live from this database (each tradition on its own).
//!
//! The texts are read from the Python isnady (data/learn.json, by tools/gen_tables.py).

use std::collections::HashSet;
use std::sync::OnceLock;

use fancy_regex::Regex;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::cache;
use crate::normalize::normalize;

#[derive(Deserialize)]
struct Data {
    languages: Vec<(String, String)>,
    group_term: Vec<(i32, String)>,
    terms: Value,
    articles: Value,
    writing_forms: Vec<(String, String)>,
    writing_in_text: Vec<(String, String)>,
    akhbarana_books: Vec<String>,
    hammam_key: String,
    mutawatir_forms: String,
}

fn data() -> &'static Data {
    static DATA: OnceLock<Data> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../data/learn.json")).expect("data/learn.json")
    })
}

/// The glossary's languages: (code, name).
pub fn languages() -> &'static [(String, String)] {
    &data().languages
}

pub fn categories() -> &'static Map<String, Value> {
    data().terms["categories"].as_object().expect("categories")
}

pub fn terms() -> &'static [Value] {
    data().terms["terms"].as_array().expect("terms")
}

pub fn get(term_id: &str) -> Option<&'static Value> {
    terms().iter().find(|t| t["id"].as_str() == Some(term_id))
}

fn s(v: &Value) -> &str {
    v.as_str().unwrap_or("")
}

/// A plain field of a term ("id", "ar", "cat", "source"), "" when missing.
pub fn field(term: &Value, key: &str) -> String {
    s(&term[key]).to_string()
}

pub fn name(term: &Value, lang: &str) -> String {
    let own = s(&term[lang]);
    if own.is_empty() { s(&term["en"]) } else { own }.to_string()
}

/// A category's name in a language, English where it has none.
pub fn category(key: &str, lang: &str) -> String {
    let names = &categories()[key];
    let own = s(&names[lang]);
    if own.is_empty() { s(&names["en"]) } else { own }.to_string()
}

/// A term's "short" or "long" text in a language, English where it has none yet.
pub fn text(term: &Value, part: &str, lang: &str) -> String {
    let own = s(&term[part][lang]);
    if own.is_empty() {
        s(&term[part]["en"])
    } else {
        own
    }
    .to_string()
}

fn joined(v: &Value) -> String {
    v.as_object()
        .map(|o| o.values().map(s).collect::<Vec<_>>().join(" "))
        .unwrap_or_default()
}

/// The terms whose name (English, Turkish, Arabic) or definition holds the words — names first.
pub fn search(query: &str) -> Vec<&'static Value> {
    let words: Vec<String> = normalize(query)
        .split_whitespace()
        .map(String::from)
        .collect();
    if words.is_empty() {
        return terms().iter().collect();
    }
    let mut first = vec![];
    let mut then = vec![];
    for t in terms() {
        let names = normalize(
            &[
                s(&t["en"]),
                s(&t["tr"]),
                s(&t["ar"]),
                &s(&t["id"]).replace('_', " "),
            ]
            .join(" "),
        );
        let body = normalize(&format!("{} {}", joined(&t["short"]), joined(&t["long"])));
        if words.iter().all(|w| names.contains(w.as_str())) {
            first.push(t);
        } else if words
            .iter()
            .all(|w| names.contains(w.as_str()) || body.contains(w.as_str()))
        {
            then.push(t);
        }
    }
    first.extend(then);
    first
}

/// "name (Arabic): the short text" of a term — the hover text.
pub fn short(term_id: &str, lang: &str) -> String {
    get(term_id)
        .map(|t| {
            format!(
                "{} ({}): {}",
                name(t, lang),
                s(&t["ar"]),
                text(t, "short", lang)
            )
        })
        .unwrap_or_default()
}

/// The hover text of a grade group (sahih, hasan, da'if, very weak, fabricated).
pub fn for_group(group: i32, lang: &str) -> String {
    data()
        .group_term
        .iter()
        .find(|(g, _)| *g == group)
        .map(|(_, id)| short(id, lang))
        .unwrap_or_default()
}

// ====================================================================== History and method (learn_history)

pub fn articles() -> &'static [Value] {
    data().articles["articles"].as_array().expect("articles")
}

pub fn article(id: &str) -> Option<&'static Value> {
    articles().iter().find(|a| a["id"].as_str() == Some(id))
}

/// A text of the articles in a language (a dict of languages, or a list), English when the language has none.
pub fn say(value: &Value, lang: &str) -> Value {
    match value {
        Value::Object(o) => {
            let own = o.get(lang).filter(|v| !is_empty(v));
            own.or_else(|| o.get("en").filter(|v| !is_empty(v)))
                .cloned()
                .unwrap_or(Value::String(String::new()))
        }
        Value::Null => Value::String(String::new()),
        other => other.clone(),
    }
}

fn is_empty(v: &Value) -> bool {
    match v {
        Value::Null => true,
        Value::String(s) => s.is_empty(),
        Value::Array(a) => a.is_empty(),
        _ => false,
    }
}

/// `say` of a plain text.
pub fn say_text(value: &Value, lang: &str) -> String {
    match say(value, lang) {
        Value::String(s) => s,
        _ => String::new(),
    }
}

/// `say` of a list of paragraphs.
pub fn say_list(value: &Value, lang: &str) -> Vec<String> {
    match say(value, lang) {
        Value::Array(a) => a.iter().map(|x| s(x).to_string()).collect(),
        Value::String(s) if !s.is_empty() => vec![s],
        _ => vec![],
    }
}

pub fn label(key: &str, lang: &str) -> String {
    let labels = &data().articles["labels"];
    match labels.get(key) {
        Some(v) => say_text(v, lang),
        None => key.to_string(),
    }
}

/// The articles whose title or text has the words.
pub fn search_articles(query: &str, lang: &str) -> Vec<&'static Value> {
    let words: Vec<String> = normalize(query)
        .split_whitespace()
        .map(String::from)
        .collect();
    if words.is_empty() {
        return articles().iter().collect();
    }
    articles()
        .iter()
        .filter(|a| {
            let mut parts = vec![say_text(&a["title"], lang), say_text(&a["lead"], lang)];
            for sec in a["sections"].as_array().into_iter().flatten() {
                parts.push(format!(
                    "{} {}",
                    say_text(&sec["heading"], lang),
                    say_list(&sec["text"], lang).join(" ")
                ));
            }
            let hay = normalize(&parts.join(" "));
            words.iter().all(|w| hay.contains(w.as_str()))
        })
        .collect()
}

// ---------------------------------------------------------------------- the live numbers

const ROADS: [&str; 2] = ["sunni", "shia"];

/// (hadith id, book key, number, tradition, Arabic text normalised) of every hadith, in the books' order.
fn each_text(
    conn: &Connection,
    mut f: impl FnMut(i64, &str, &str, &str, &str),
) -> rusqlite::Result<()> {
    let mut st = conn.prepare(
        "SELECT h.id, c.key, h.number, COALESCE(c.tradition, 'sunni'), n.norm FROM text_norm n \
         JOIN texts t ON t.id = n.text_id JOIN editions e ON e.id = t.edition_id \
         JOIN hadiths h ON h.id = t.hadith_id JOIN collections c ON c.id = h.collection_id \
         WHERE e.language = 'Arabic' ORDER BY h.collection_id, h.number_sort",
    )?;
    let mut rows = st.query([])?;
    while let Some(r) = rows.next()? {
        let number: Option<String> = r.get(2)?;
        let norm: Option<String> = r.get(4)?;
        f(
            r.get(0)?,
            &r.get::<_, String>(1)?,
            number.as_deref().unwrap_or(""),
            &r.get::<_, String>(3)?,
            norm.as_deref().unwrap_or(""),
        );
    }
    Ok(())
}

/// One wording counted: how many hadith, per book (in the order met), the first examples.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Found {
    pub n: i64,
    pub books: Vec<(String, i64)>,
    pub examples: Vec<(String, String)>,
}

fn bump(list: &mut Vec<(String, i64)>, key: &str, by: i64) {
    match list.iter_mut().find(|(k, _)| k == key) {
        Some(x) => x.1 += by,
        None => list.push((key.to_string(), by)),
    }
}

fn note(slot: &mut Vec<(String, Found)>, key: &str, book: &str, number: &str) {
    if !slot.iter().any(|(k, _)| k == key) {
        slot.push((key.to_string(), Found::default()));
    }
    let row = &mut slot.iter_mut().find(|(k, _)| k == key).expect("row").1;
    row.n += 1;
    bump(&mut row.books, book, 1);
    if row.examples.len() < 4 {
        row.examples.push((book.to_string(), number.to_string()));
    }
}

/// The traces of writing of one tradition (`learn_history.writing`).
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Writing {
    pub texts: i64,
    pub forms: Vec<(String, Found)>,
    pub in_text: Vec<(String, Found)>,
    pub akhbarana: Vec<(String, i64)>,
    pub hammam: Vec<(String, i64)>,
}

/// A wording to look for: `(?<!\w)(A|B|…)(?!\w)` — words written out, not in the middle of another word. The
/// pattern holds only plain words, so it is looked for as such (the same matches as Python's `re.search`, fast).
pub struct Wording {
    alternatives: Vec<String>,
    whole_end: bool,
    other: Option<Regex>,
}

fn word_char(c: Option<char>) -> bool {
    c.is_some_and(|c| c.is_alphanumeric() || c == '_')
}

impl Wording {
    pub fn new(pattern: &str) -> Wording {
        let (body, whole_end) = match pattern.strip_suffix(r"(?!\w)") {
            Some(b) => (b, true),
            None => (pattern, false),
        };
        let plain = body.strip_prefix(r"(?<!\w)").map(|b| {
            b.strip_prefix('(')
                .and_then(|b| b.strip_suffix(')'))
                .unwrap_or(b)
        });
        match plain {
            Some(p)
                if !p.contains([
                    '(', ')', '[', ']', '\\', '?', '*', '+', '.', '^', '$', '{', '}',
                ]) =>
            {
                Wording {
                    alternatives: p.split('|').map(String::from).collect(),
                    whole_end,
                    other: None,
                }
            }
            _ => Wording {
                alternatives: vec![],
                whole_end,
                other: Some(Regex::new(pattern).expect("pattern")),
            },
        }
    }

    pub fn is_match(&self, text: &str) -> bool {
        if let Some(rx) = &self.other {
            return rx.is_match(text).unwrap_or(false);
        }
        self.alternatives.iter().any(|alt| {
            text.match_indices(alt.as_str()).any(|(at, _)| {
                !word_char(text[..at].chars().next_back())
                    && (!self.whole_end || !word_char(text[at + alt.len()..].chars().next()))
            })
        })
    }
}

fn compiled(list: &[(String, String)]) -> Vec<(String, Wording)> {
    list.iter()
        .map(|(k, p)| (k.clone(), Wording::new(p)))
        .collect()
}

/// The traces of writing in the chains and in the texts, per tradition — counted once per change of the database.
pub fn writing(conn: &Connection) -> rusqlite::Result<Vec<(String, Writing)>> {
    cache::kept(conn, "learn-writing", || {
        let forms = compiled(&data().writing_forms);
        let in_text = compiled(&data().writing_in_text);
        let mut out: Vec<(String, Writing)> = ROADS
            .iter()
            .map(|r| (r.to_string(), Writing::default()))
            .collect();
        let mut seen = HashSet::new();
        each_text(conn, |hid, key, number, tradition, norm| {
            if !seen.insert(hid) {
                return;
            }
            let w = &mut out[usize::from(tradition == "shia")].1;
            w.texts += 1;
            for (k, rx) in &forms {
                if rx.is_match(norm) {
                    note(&mut w.forms, k, key, number);
                }
            }
            for (k, rx) in &in_text {
                if rx.is_match(norm) {
                    note(&mut w.in_text, k, key, number);
                }
            }
        })?;
        let books = &data().akhbarana_books;
        let marks = vec!["?"; books.len()].join(",");
        let mut st = conn.prepare(&format!(
            "SELECT c.key, COUNT(*) FROM isnad_links l JOIN isnads i ON i.id = l.isnad_id \
             JOIN hadiths h ON h.id = i.hadith_id JOIN collections c ON c.id = h.collection_id \
             WHERE c.key IN ({marks}) AND l.transmission IN ('akhbarana', 'akhbarani') GROUP BY c.id ORDER BY c.id"
        ))?;
        for r in st.query_map(rusqlite::params_from_iter(books.iter()), |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        })? {
            out[0].1.akhbarana.push(r?);
        }
        let mut st = conn.prepare(
            "SELECT c.key, COUNT(DISTINCT i.hadith_id) FROM isnad_links l JOIN persons p ON p.id = l.person_id \
             JOIN isnads i ON i.id = l.isnad_id JOIN hadiths h ON h.id = i.hadith_id \
             JOIN collections c ON c.id = h.collection_id WHERE p.key = ? GROUP BY c.id ORDER BY 2 DESC",
        )?;
        for r in st.query_map([&data().hammam_key], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        })? {
            out[0].1.hammam.push(r?);
        }
        Ok(out)
    })
}

/// The size of what was handed down, of one tradition (`learn_history.corpus`).
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Corpus {
    pub books: i64,
    pub hadith: i64,
    pub chains: i64,
    pub links: i64,
    pub identified: i64,
    pub narrators: i64,
    pub median_length: i64,
    pub chains_to_end: usize,
    pub generations: Vec<(String, i64)>,
    pub top_companions: Vec<(i64, String, i64)>,
    pub grades: Vec<(String, i64)>,
    pub graded_hadith: i64,
}

pub fn corpus(conn: &Connection) -> rusqlite::Result<Vec<(String, Corpus)>> {
    cache::kept(conn, "learn-corpus", || {
        let famous_ids: HashSet<i64> = crate::people::famous_titles(conn, "en")
            .into_keys()
            .collect();
        let mut out = vec![];
        let w = "COALESCE(c.tradition, 'sunni') = ?";
        for road in ROADS {
            let mut d = Corpus::default();
            (d.books, d.hadith) = conn.query_row(
                &format!("SELECT COUNT(DISTINCT c.id), COUNT(h.id) FROM hadiths h JOIN collections c ON c.id = h.collection_id WHERE {w}"),
                [road],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            let known: Option<i64>;
            (d.chains, d.links, known, d.narrators) = conn.query_row(
                &format!(
                    "SELECT COUNT(DISTINCT i.id), COUNT(l.isnad_id), SUM(l.person_id IS NOT NULL), COUNT(DISTINCT l.person_id) \
                     FROM isnads i JOIN isnad_links l ON l.isnad_id = i.id JOIN hadiths h ON h.id = i.hadith_id \
                     JOIN collections c ON c.id = h.collection_id WHERE {w}"
                ),
                [road],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )?;
            d.identified = known.unwrap_or(0);
            let mut st = conn.prepare(&format!(
                "SELECT COUNT(*) FROM isnad_links l JOIN isnads i ON i.id = l.isnad_id JOIN hadiths h ON h.id = i.hadith_id \
                 JOIN collections c ON c.id = h.collection_id \
                 WHERE {w} AND i.problem IS NULL AND i.reaches_prophet = 1 GROUP BY i.id"
            ))?;
            let mut lengths: Vec<i64> = st
                .query_map([road], |r| r.get(0))?
                .collect::<rusqlite::Result<_>>()?;
            lengths.sort();
            d.median_length = lengths.get(lengths.len() / 2).copied().unwrap_or(0);
            d.chains_to_end = lengths.len();
            let mut st = conn.prepare(&format!(
                "SELECT COALESCE(p.generation, '?'), COUNT(DISTINCT p.id) FROM isnad_links l \
                 JOIN persons p ON p.id = l.person_id JOIN isnads i ON i.id = l.isnad_id \
                 JOIN hadiths h ON h.id = i.hadith_id JOIN collections c ON c.id = h.collection_id WHERE {w} GROUP BY 1"
            ))?;
            for r in st.query_map([road], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
            })? {
                d.generations.push(r?);
            }
            if road == "sunni" {
                let mut st = conn.prepare(&format!(
                    "SELECT p.id, p.name_ar, COUNT(DISTINCT i.hadith_id), p.generation FROM isnads i \
                     JOIN isnad_links l ON l.isnad_id = i.id \
                     JOIN persons p ON p.id = l.person_id JOIN hadiths h ON h.id = i.hadith_id \
                     JOIN collections c ON c.id = h.collection_id \
                     WHERE {w} AND i.reaches_prophet = 1 AND i.problem IS NULL \
                       AND COALESCE(p.generation, 'sahabi') = 'sahabi' \
                       AND l.position = (SELECT MAX(x.position) FROM isnad_links x WHERE x.isnad_id = i.id) \
                     GROUP BY p.id ORDER BY 3 DESC LIMIT 30"
                ))?;
                for r in st.query_map([road], |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, i64>(2)?,
                        r.get::<_, Option<String>>(3)?,
                    ))
                })? {
                    let (pid, name, n, gen) = r?;
                    if (gen.as_deref() == Some("sahabi") || famous_ids.contains(&pid))
                        && d.top_companions.len() < 8
                    {
                        d.top_companions.push((pid, name, n));
                    }
                }
            }
            let mut st = conn.prepare(&format!(
                "SELECT g.hadith_id, c.tradition, g.grade FROM grades g JOIN hadiths h ON h.id = g.hadith_id \
                 JOIN collections c ON c.id = h.collection_id WHERE {w}"
            ))?;
            for r in st.query_map([road], |r| {
                Ok((
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, Option<String>>(2)?,
                ))
            })? {
                let (tradition, grade) = r?;
                if let Some(code) =
                    crate::grades::code_of(grade.as_deref().unwrap_or(""), tradition.as_deref())
                {
                    bump(&mut d.grades, code, 1);
                }
            }
            d.graded_hadith = conn.query_row(
                &format!(
                    "SELECT COUNT(DISTINCT g.hadith_id) FROM grades g JOIN hadiths h ON h.id = g.hadith_id \
                     JOIN collections c ON c.id = h.collection_id WHERE {w}"
                ),
                [road],
                |r| r.get(0),
            )?;
            out.push((road.to_string(), d));
        }
        Ok(out)
    })
}

/// "Whoever lies about me deliberately …": in how many hadith, books and chains the Sunni books carry it.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Mutawatir {
    pub hadith: i64,
    pub books: Vec<(String, i64)>,
    pub chains: i64,
}

pub fn mutawatir(conn: &Connection) -> rusqlite::Result<Mutawatir> {
    cache::kept(conn, "learn-mutawatir", || {
        let rx = Wording::new(&data().mutawatir_forms);
        let mut ids: Vec<i64> = vec![];
        let mut seen = HashSet::new();
        let mut books = vec![];
        each_text(conn, |hid, key, _number, tradition, norm| {
            if tradition != "shia" && rx.is_match(norm) && seen.insert(hid) {
                ids.push(hid);
                bump(&mut books, key, 1);
            }
        })?;
        let list = if ids.is_empty() {
            "NULL".to_string()
        } else {
            ids.iter().map(i64::to_string).collect::<Vec<_>>().join(",")
        };
        let chains = conn.query_row(
            &format!("SELECT COUNT(*) FROM isnads WHERE hadith_id IN ({list})"),
            [],
            |r| r.get(0),
        )?;
        Ok(Mutawatir {
            hadith: ids.len() as i64,
            books,
            chains,
        })
    })
}

fn book(key: &str, lang: &str) -> String {
    crate::works::book_title(key, key, lang)
}

fn examples(row: &Found, lang: &str) -> String {
    let list: Vec<String> = row
        .examples
        .iter()
        .take(3)
        .map(|(k, n)| format!("{} {n}", book(k, lang)))
        .collect();
    format!("{} {}", label("e.g.", lang), list.join("; "))
}

fn thousands(n: i64) -> String {
    crate::py::thousands(n)
}

/// A block of live numbers as rows (label, value, note) in a language; [] when nothing is imported
/// (`learn_history.block`).
pub fn block(
    conn: Option<&Connection>,
    kind: &str,
    lang: &str,
) -> rusqlite::Result<Vec<(String, String, String)>> {
    let l = |k: &str| label(k, lang);
    let mut rows: Vec<(String, String, String)> = vec![];
    if kind == "timeline" {
        for e in data().articles["timeline"].as_array().into_iter().flatten() {
            let ah = s(&e["ah"]);
            let mut when = match lang {
                "en" => format!("{ah} AH"),
                "tr" => format!("h. {ah}"),
                _ => format!("{ah} هـ"),
            };
            if !s(&e["ce"]).is_empty() {
                when += &format!(" / {}", s(&e["ce"]));
            }
            rows.push((when, say_text(&e["what"], lang), String::new()));
        }
        return Ok(rows);
    }
    let Some(conn) = conn else { return Ok(rows) };
    if kind == "corpus" {
        for (road, d) in corpus(conn)? {
            if d.hadith == 0 {
                continue;
            }
            rows.push((l(&road), String::new(), String::new()));
            let share = if d.links > 0 {
                format!("{:.1}%", 100.0 * d.identified as f64 / d.links as f64)
            } else {
                "—".to_string()
            };
            rows.push((l("books"), thousands(d.books), String::new()));
            rows.push((l("hadith"), thousands(d.hadith), String::new()));
            rows.push((l("chains"), thousands(d.chains), l("chains_note")));
            rows.push((l("links"), thousands(d.links), l("links_note")));
            rows.push((
                l("identified"),
                format!("{} ({share})", thousands(d.identified)),
                String::new(),
            ));
            rows.push((l("narrators"), thousands(d.narrators), l("narrators_note")));
            if d.median_length != 0 {
                rows.push((
                    l("median_length"),
                    d.median_length.to_string(),
                    l("median_note"),
                ));
            }
            for gen in [
                "sahabi",
                "tabi'i",
                "contemporary of junior tabi'in",
                "tabi' al-tabi'in",
                "later",
            ] {
                if let Some((_, n)) = d.generations.iter().find(|(g, n)| g == gen && *n != 0) {
                    rows.push((l(&format!("gen:{gen}")), thousands(*n), String::new()));
                }
            }
            if !d.top_companions.is_empty() {
                let shown = crate::people::famous_titles(conn, lang);
                for (pid, name, n) in &d.top_companions {
                    let who = shown.get(pid).cloned().unwrap_or_else(|| {
                        name.split_whitespace()
                            .take(4)
                            .collect::<Vec<_>>()
                            .join(" ")
                    });
                    rows.push((
                        l("top_companion"),
                        who,
                        format!("{} {}", thousands(*n), l("hadith_unit")),
                    ));
                }
            }
            if d.graded_hadith != 0 {
                rows.push((
                    l("graded_hadith"),
                    thousands(d.graded_hadith),
                    l("graded_note"),
                ));
                let mut grades = d.grades.clone();
                grades.sort_by_key(|g| -g.1);
                for (code, n) in grades {
                    rows.push((l(&format!("grade:{code}")), thousands(n), l("grade_unit")));
                }
            }
        }
        return Ok(rows);
    }
    if kind == "writing" {
        for (road, d) in writing(conn)? {
            if d.texts == 0 {
                continue;
            }
            rows.push((
                l(&road),
                format!("{} {}", thousands(d.texts), l("hadith_unit")),
                String::new(),
            ));
            for (key, _) in &data().writing_forms {
                if let Some((_, r)) = d.forms.iter().find(|(k, _)| k == key) {
                    rows.push((l(&format!("w:{key}")), thousands(r.n), examples(r, lang)));
                }
            }
            for (key, n) in &d.akhbarana {
                rows.push((
                    format!("{} — {}", l("w:akhbarana"), book(key, lang)),
                    thousands(*n),
                    l("w:akhbarana_note"),
                ));
            }
            if !d.hammam.is_empty() {
                let total: i64 = d.hammam.iter().map(|x| x.1).sum();
                let list: Vec<String> = d
                    .hammam
                    .iter()
                    .map(|(k, v)| format!("{} {v}", book(k, lang)))
                    .collect();
                rows.push((l("w:hammam"), thousands(total), list.join(", ")));
            }
            for (key, _) in &data().writing_in_text {
                if let Some((_, r)) = d.in_text.iter().find(|(k, _)| k == key) {
                    rows.push((l(&format!("t:{key}")), thousands(r.n), examples(r, lang)));
                }
            }
        }
        return Ok(rows);
    }
    if kind == "mutawatir" {
        let d = mutawatir(conn)?;
        if d.hadith != 0 {
            let list: Vec<String> = d
                .books
                .iter()
                .map(|(k, v)| format!("{} {v}", book(k, lang)))
                .collect();
            rows.push((l("m:hadith"), thousands(d.hadith), String::new()));
            rows.push((
                l("m:books"),
                thousands(d.books.len() as i64),
                list.join(", "),
            ));
            rows.push((l("m:chains"), thousands(d.chains), l("m:note")));
        }
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_glossary() {
        assert_eq!(terms().len(), 81);
        let t = get("sahih").expect("sahih");
        assert_eq!(name(t, "tr"), "sahîh");
        assert!(!search("mursal").is_empty());
        assert_eq!(articles().len(), 4);
        assert!(!block(None, "timeline", "en").expect("timeline").is_empty());
        let w = Wording::new(r"(?<!\w)(كتب الي|كتب الينا)(?!\w)");
        assert!(w.is_match("ثم كتب الي فلان"));
        assert!(!w.is_match("ثم كتب اليه فلان"));
        assert!(w.other.is_none());
    }
}
