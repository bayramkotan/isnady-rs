//! The lives of the Companions, the Successors, the narrators and the scholars (BIO2) — the Python isnady's
//! `core/lives.py`, with what it needs of `core/source_entries.py` and `core/huffaz.py`.
//!
//! Two kinds of life, both in the interface language: a life WRITTEN for isnady (assets/lives, carried here in
//! data/lives.json by tools/gen_tables.py), and for every other narrator the life READ from the data — his name, his
//! generation, his death, his rank, his books, his teachers and students in the imported chains. Each road by its
//! own works (SH1): a Shia narrator's life is from the Imami rijal works, never the Sunni ones.

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::OnceLock;

use crate::narrators::{self, Langs};
use crate::{i18n, paths, people, py};

struct Data {
    lives: Vec<Value>,
    kinds: HashMap<String, String>,
    facts: HashMap<String, String>,
    sources: HashMap<String, Value>,
    /// work → life id → (volume, number)
    entries: HashMap<String, HashMap<String, (i64, i64)>>,
    works: HashMap<String, Value>,
    huffaz_work: String,
    /// Taqrib key → (number, generation)
    huffaz: HashMap<String, (i64, i64)>,
}

fn data() -> &'static Data {
    static DATA: OnceLock<Data> = OnceLock::new();
    DATA.get_or_init(|| {
        let v: Value =
            serde_json::from_str(include_str!("../data/lives.json")).expect("data/lives.json");
        let map = |k: &str| -> HashMap<String, String> {
            v[k].as_object()
                .map(|o| {
                    o.iter()
                        .map(|(a, b)| (a.clone(), b.as_str().unwrap_or("").to_string()))
                        .collect()
                })
                .unwrap_or_default()
        };
        let mut entries = HashMap::new();
        if let Some(o) = v["entries"].as_object() {
            for (work, lives) in o {
                let mut m = HashMap::new();
                if let Some(l) = lives.as_object() {
                    for (id, at) in l {
                        m.insert(
                            id.clone(),
                            (at[0].as_i64().unwrap_or(0), at[1].as_i64().unwrap_or(0)),
                        );
                    }
                }
                entries.insert(work.clone(), m);
            }
        }
        let huffaz = v["huffaz"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|h| {
                        Some((h[0].as_str()?.to_string(), (h[1].as_i64()?, h[2].as_i64()?)))
                    })
                    .collect()
            })
            .unwrap_or_default();
        Data {
            lives: v["lives"].as_array().cloned().unwrap_or_default(),
            kinds: map("kinds"),
            facts: map("facts"),
            sources: v["sources"]
                .as_object()
                .map(|o| o.iter().map(|(a, b)| (a.clone(), b.clone())).collect())
                .unwrap_or_default(),
            entries,
            works: v["works"]
                .as_object()
                .map(|o| o.iter().map(|(a, b)| (a.clone(), b.clone())).collect())
                .unwrap_or_default(),
            huffaz_work: v["huffaz_work"].as_str().unwrap_or("tadhkira").to_string(),
            huffaz,
        }
    })
}

/// Every life written for isnady, in its order.
pub fn written() -> &'static [Value] {
    &data().lives
}

/// A text in a language: its own, else English, else Turkish, else Arabic (`lives._in`).
fn in_lang(values: &Value, lang: &str) -> Value {
    match values {
        Value::Object(o) => {
            for code in [lang, "en", "tr", "ar"] {
                if let Some(v) = o.get(code) {
                    let empty = match v {
                        Value::String(s) => s.is_empty(),
                        Value::Array(a) => a.is_empty(),
                        Value::Null => true,
                        _ => false,
                    };
                    if !empty {
                        return v.clone();
                    }
                }
            }
            Value::String(String::new())
        }
        Value::Null => Value::String(String::new()),
        other => other.clone(),
    }
}

fn in_text(values: &Value, lang: &str) -> String {
    in_lang(values, lang).as_str().unwrap_or("").to_string()
}

/// A classical work's entry a life is summarised from, to be read in its author's words.
#[derive(Debug, Clone, Serialize)]
pub struct Entry {
    pub work: String,
    pub volume: i64,
    pub number: i64,
    pub title: String,
    #[serde(rename = "where")]
    pub place: String,
}

/// A link under "Read more": a book of isnady or a page on the web.
#[derive(Debug, Clone, Serialize)]
pub struct ReadMore {
    pub kind: String,
    pub book: Option<String>,
    pub url: Option<String>,
    pub text: String,
}

/// A life in one language (`lives.view` / `lives.read`).
#[derive(Debug, Clone, Serialize)]
pub struct Life {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub road: String,
    pub facts: Vec<(String, String)>,
    pub fact_keys: Vec<String>,
    pub paragraphs: Vec<String>,
    pub sources: Vec<String>,
    pub read: Vec<ReadMore>,
    pub entries: Vec<Entry>,
}

/// The title of a classical work in a language (`source_entries.title`).
pub fn work_title(work: &str, lang: &str) -> String {
    let names = &data().works.get(work).cloned().unwrap_or(Value::Null)["title"];
    names[lang]
        .as_str()
        .filter(|s| !s.is_empty())
        .or_else(|| names["en"].as_str())
        .unwrap_or(work)
        .to_string()
}

/// How big the download of a work is, in MB.
pub fn work_size(work: &str) -> i64 {
    data()
        .works
        .get(work)
        .and_then(|w| w["size_mb"].as_i64())
        .unwrap_or(0)
}

fn source_name(key: &str, lang: &str) -> String {
    match data().sources.get(key) {
        Some(v) => in_text(v, lang),
        None => key.to_string(),
    }
}

/// The written life of a narrator, by his stable key in the rijal work (`lives.of_person`).
pub fn of_person(conn: &Connection, person_id: i64) -> Option<&'static Value> {
    let key: String = conn
        .query_row(
            "SELECT key FROM persons WHERE id = ?",
            params![person_id],
            |r| r.get(0),
        )
        .optional()
        .ok()
        .flatten()?;
    written().iter().find(|l| {
        l["persons"]
            .as_array()
            .is_some_and(|p| p.iter().any(|k| k.as_str() == Some(key.as_str())))
    })
}

/// The narrator a written life is about, when his rijal work is imported here (`lives.person_of`).
pub fn person_of(conn: &Connection, life: &Value) -> Option<i64> {
    for key in life["persons"].as_array().into_iter().flatten() {
        let found: Option<i64> = conn
            .query_row(
                "SELECT id FROM persons WHERE key = ?",
                params![key.as_str().unwrap_or("")],
                |r| r.get(0),
            )
            .optional()
            .ok()
            .flatten();
        if found.is_some() {
            return found;
        }
    }
    None
}

/// The entry of al-Dhahabi's Tadhkirat al-Huffaz of a narrator, by his Taqrib key (`huffaz.of_key`): (number,
/// generation).
pub fn hafiz_of(key: &str) -> Option<(i64, i64)> {
    if key.is_empty() {
        return None;
    }
    data().huffaz.get(key).copied()
}

fn hafiz_entry(n: i64, tabaqa: i64, lang: &str) -> Entry {
    let work = data().huffaz_work.clone();
    Entry {
        title: work_title(&work, lang),
        place: i18n::t(
            lang,
            "generation {tabaqa}, no. {number}",
            &[("tabaqa", tabaqa.to_string()), ("number", n.to_string())],
        ),
        work,
        volume: 0,
        number: n,
    }
}

/// A written life in one language (`lives.view`).
pub fn view(life: &Value, lang: &str) -> Life {
    let d = data();
    let id = life["id"].as_str().unwrap_or("").to_string();
    let mut facts = Vec::new();
    let mut fact_keys = Vec::new();
    for f in life["facts"].as_array().into_iter().flatten() {
        let text = in_text(f, lang);
        if text.is_empty() {
            continue;
        }
        let k = f["k"].as_str().unwrap_or("");
        let label = d.facts.get(k).cloned().unwrap_or_else(|| k.to_string());
        facts.push((i18n::t(lang, &label, &[]), text));
        fact_keys.push(k.to_string());
    }
    let paragraphs = in_lang(&life["text"], lang)
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|p| p.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let sources = life["sources"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|s| s.as_str())
        .map(|s| source_name(s, lang))
        .collect();
    let read = life["read"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|r| ReadMore {
            kind: r["kind"].as_str().unwrap_or("url").to_string(),
            book: r["book"].as_str().map(String::from),
            url: r["url"].as_str().map(String::from),
            text: in_text(r, lang),
        })
        .collect();
    let mut entries = Vec::new();
    let mut works: Vec<&String> = d.entries.keys().collect();
    works.sort();
    for work in works {
        if let Some((volume, number)) = d.entries[work].get(&id) {
            if d.works.contains_key(work) {
                entries.push(Entry {
                    work: work.clone(),
                    volume: *volume,
                    number: *number,
                    title: work_title(work, lang),
                    place: i18n::t(
                        lang,
                        "vol. {volume}, no. {number}",
                        &[
                            ("volume", volume.to_string()),
                            ("number", number.to_string()),
                        ],
                    ),
                });
            }
        }
    }
    for key in life["persons"].as_array().into_iter().flatten() {
        if let Some((n, tabaqa)) = hafiz_of(key.as_str().unwrap_or("")) {
            entries.push(hafiz_entry(n, tabaqa, lang));
            break;
        }
    }
    let kind = life["kind"].as_str().unwrap_or("");
    Life {
        title: in_text(&life["name"], lang),
        kind: i18n::t(
            lang,
            d.kinds.get(kind).map(String::as_str).unwrap_or(""),
            &[],
        ),
        road: life["road"].as_str().unwrap_or("sunni").to_string(),
        id,
        facts,
        fact_keys,
        paragraphs,
        sources,
        read,
        entries,
    }
}

/// The Common Era year in which a hijri year mostly falls (`lives.ce`).
pub fn ce(year_ah: i64) -> i64 {
    let x = year_ah as f64 * 0.970229 + 621.5643;
    // Python's round(): half to even
    let r = x.round();
    if (x - x.trunc()).abs() == 0.5 && (r as i64) % 2 != 0 {
        (r - x.signum()) as i64
    } else {
        r as i64
    }
}

/// (reading, Arabic) of the name a narrator is known by (`narrators.reading`).
pub fn reading(conn: &Connection, person_id: i64, name_ar: &str, langs: Langs) -> (String, String) {
    let view = narrators::name_view(conn, person_id, name_ar, langs);
    let shown = narrators::display_name(name_ar);
    let reading = if view.reading.is_empty() {
        view.full_reading.clone()
    } else {
        view.reading.clone()
    };
    (
        reading,
        if view.arabic.is_empty() {
            shown
        } else {
            view.arabic
        },
    )
}

fn tr(lang: &str, text: &str, values: &[(&str, String)]) -> String {
    i18n::t(lang, text, values)
}

/// The life of any narrator as far as the data says it (`lives.read`).
pub fn read(conn: &Connection, person_id: i64, langs: Langs) -> Option<Life> {
    let d = data();
    let lang = langs.words;
    let who = people::describe(conn, person_id)?;
    let shia = who.tradition == "shia";
    let view = narrators::name_view(conn, person_id, &who.person.name_ar, langs);
    let display = narrators::display_name(&who.person.name_ar);
    let title = if !view.reading.is_empty() {
        view.reading.clone()
    } else if !view.full_reading.is_empty() {
        view.full_reading.clone()
    } else {
        display
    };
    let fact = |k: &str| tr(lang, d.facts.get(k).map(String::as_str).unwrap_or(k), &[]);
    let mut facts: Vec<(String, String)> = Vec::new();
    let mut paras: Vec<String> = Vec::new();
    if !view.full_reading.is_empty() && view.full_reading != title {
        facts.push((fact("name"), view.full_reading.clone()));
    }
    let year = who.person.death_year_ah.filter(|y| *y != 0);
    if let Some(y) = year {
        facts.push((
            fact("died"),
            tr(
                lang,
                "{year} AH (about {ce} CE)",
                &[("year", y.to_string()), ("ce", ce(y).to_string())],
            ),
        ));
    }
    let mut first: Vec<String> = Vec::new();
    if let Some(tb) = who.person.tabaqa.filter(|x| *x != 0) {
        first.push(tr(
            lang,
            "In Ibn Hajar's Taqrib he is in tabaqa {n}: {label}.",
            &[
                ("n", tb.to_string()),
                ("label", tr(lang, &who.tabaqa_label, &[])),
            ],
        ));
    }
    if let Some(y) = year {
        first.push(tr(
            lang,
            "He died in {year} AH, about {ce} CE.",
            &[("year", y.to_string()), ("ce", ce(y).to_string())],
        ));
    }
    for v in &who.verdicts {
        if v.rank_label.is_empty() {
            continue;
        }
        let label = tr(lang, &v.rank_label, &[]);
        let critic = tr(lang, &v.critic_name, &[]);
        if v.rank_scheme.as_deref() == Some("shia") {
            let mut s = tr(
                lang,
                "{critic} counts him as {label}.",
                &[("critic", critic), ("label", label)],
            );
            if !v.madhhab_label.is_empty() {
                s.push(' ');
                s.push_str(&tr(
                    lang,
                    "His creed, in the critic's words: {creed}.",
                    &[("creed", tr(lang, &v.madhhab_label, &[]))],
                ));
            }
            first.push(s);
        } else {
            first.push(tr(
                lang,
                "{critic} ranks him {rank} of 12: {label}.",
                &[
                    ("critic", critic),
                    ("rank", py::opt(v.rank)),
                    ("label", label),
                ],
            ));
        }
    }
    if !who.marks.is_empty() {
        let books: Vec<String> = who.marks.iter().map(|m| tr(lang, &m.1, &[])).collect();
        first.push(tr(
            lang,
            "His hadith are in: {books}.",
            &[("books", books.join("; "))],
        ));
    }
    if !first.is_empty() {
        paras.push(first.join(" "));
    }
    let rel = people::relations(conn, person_id, 5).ok()?;
    let (_rows, hadith) = people::hadiths_of(conn, person_id, 1, lang).ok()?;
    let mut second: Vec<String> = Vec::new();
    if who.in_chains != 0 {
        let template = if who.in_chains == 1 {
            "In the books imported here he is in {chains:,} chain, of {hadith:,} hadith."
        } else {
            "In the books imported here he is in {chains:,} chains, of {hadith:,} hadith."
        };
        second.push(tr(
            lang,
            template,
            &[
                ("chains:,", py::thousands(who.in_chains)),
                ("hadith:,", py::thousands(hadith)),
                ("n", who.in_chains.to_string()),
            ],
        ));
    }
    let names = |items: &[people::Related]| -> String {
        items
            .iter()
            .map(|r| {
                let (reading, _ar) = reading(conn, r.id, &r.name, langs);
                format!(
                    "{} ({})",
                    if reading.is_empty() {
                        &r.name
                    } else {
                        &reading
                    },
                    py::thousands(r.count)
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    };
    if !rel.teachers.is_empty() {
        second.push(tr(
            lang,
            "He narrates most often from {names}.",
            &[("names", names(&rel.teachers))],
        ));
    }
    if !rel.students.is_empty() {
        second.push(tr(
            lang,
            "Those who narrate most often from him: {names}.",
            &[("names", names(&rel.students))],
        ));
    }
    if !rel.compilers.is_empty() {
        let books: Vec<String> = rel
            .compilers
            .iter()
            .take(6)
            .map(|c| format!("{} ({})", c.0, py::thousands(c.2)))
            .collect();
        second.push(tr(
            lang,
            "He is a teacher of the compilers themselves in: {books}.",
            &[("books", books.join(", "))],
        ));
    }
    if !second.is_empty() {
        paras.push(second.join(" "));
    }
    if !rel.teachers.is_empty() {
        facts.push((
            fact("teachers"),
            names(&rel.teachers[..rel.teachers.len().min(3)]),
        ));
    }
    if !rel.students.is_empty() {
        facts.push((
            fact("students"),
            names(&rel.students[..rel.students.len().min(3)]),
        ));
    }
    // the rijal work his entry is from (al-Najashi, al-Tusi's Fihrist or his Rijal; the Taqrib)
    let work = who.key.split(':').next().unwrap_or("");
    let mut sources = vec![
        source_name(
            if shia && ["najashi", "tusi_fihrist", "tusi_rijal"].contains(&work) {
                work
            } else if shia {
                "najashi"
            } else {
                "taqrib"
            },
            lang,
        ),
        tr(lang, "the chains of the books imported here", &[]),
    ];
    let mut entries = Vec::new();
    if !shia {
        if let Some((n, tabaqa)) = hafiz_of(&who.key) {
            facts.push((
                fact("known"),
                tr(
                    lang,
                    "a hafiz of hadith: al-Dhahabi's Tadhkirat al-Huffaz, generation {tabaqa}",
                    &[("tabaqa", tabaqa.to_string())],
                ),
            ));
            sources.push(source_name("tadhkira", lang));
            entries.push(hafiz_entry(n, tabaqa, lang));
        }
    }
    let fact_keys = Vec::new();
    Some(Life {
        id: String::new(),
        title,
        kind: tr(
            lang,
            d.kinds
                .get("narrator")
                .map(String::as_str)
                .unwrap_or("Narrator"),
            &[],
        ),
        road: if shia { "shia" } else { "sunni" }.to_string(),
        facts,
        fact_keys,
        paragraphs: paras,
        sources,
        read: Vec::new(),
        entries,
    })
}

/// (the life of a narrator, written) — his written life when there is one, else the one read from the data.
pub fn life(conn: &Connection, person_id: i64, langs: Langs) -> (Option<Life>, bool) {
    if let Some(found) = of_person(conn, person_id) {
        return (Some(view(found, langs.words)), true);
    }
    (read(conn, person_id, langs), false)
}

/// Written lives whose name, in any language, holds the words typed (`lives.find`, for `iy life NAME`).
pub fn find(text: &str) -> Vec<&'static Value> {
    use crate::normalize::normalize;
    let fold = |s: &str| -> String {
        let n = normalize(s);
        let arabic: Vec<&str> = py::words(&n)
            .into_iter()
            .map(|w| if w == "ابن" { "بن" } else { w })
            .collect();
        let both = format!("{} {}", people::fold_latin(s), arabic.join(" "));
        py::words(&both).join(" ")
    };
    let query = fold(text);
    let words: Vec<String> = py::words(&query).into_iter().map(String::from).collect();
    let mut scored: Vec<(bool, usize, String, &'static Value)> = Vec::new();
    for item in written() {
        let mut names: Vec<String> = item["name"]
            .as_object()
            .map(|o| {
                o.values()
                    .map(|v| match v {
                        Value::String(s) => s.clone(),
                        other => other.to_string(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        let id = item["id"].as_str().unwrap_or("");
        names.push(id.replace('-', " "));
        let folded_names: Vec<String> = names.iter().map(|n| fold(n)).collect();
        let hay = folded_names.join(" ");
        if !words.is_empty() && words.iter().all(|w| hay.contains(w.as_str())) {
            let mut folded: Vec<String> = folded_names
                .iter()
                .filter(|f| words.iter().all(|w| f.contains(w.as_str())))
                .cloned()
                .collect();
            if folded.is_empty() {
                folded = vec![hay.clone()];
            }
            let exact = folded
                .iter()
                .any(|f| *f == query || f.ends_with(&format!(" {query}")));
            let shortest = folded.iter().map(|f| f.chars().count()).min().unwrap_or(0);
            scored.push((!exact, shortest, id.to_string(), item));
        }
    }
    scored.sort_by(|a, b| (a.0, a.1, &a.2).cmp(&(b.0, b.1, &b.2)));
    scored.into_iter().map(|x| x.3).collect()
}

// ------------------------------------------------------------------ the classical entries (source_entries)

fn entries_db(work: &str) -> std::path::PathBuf {
    paths::data_dir().join("sources").join(format!("{work}.db"))
}

/// The work is downloaded and split into its entries (`source_entries.available`).
pub fn available(work: &str) -> bool {
    let p = entries_db(work);
    if !p.exists() {
        return false;
    }
    Connection::open_with_flags(p, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .ok()
        .and_then(|c| {
            c.query_row("SELECT COUNT(*) FROM entries", [], |r| r.get::<_, i64>(0))
                .ok()
        })
        .is_some_and(|n| n > 0)
}

/// (heading, text) of one entry, or None: not downloaded, or no such entry (`source_entries.entry`).
pub fn entry(work: &str, volume: i64, number: i64) -> Option<(String, String)> {
    let p = entries_db(work);
    if !p.exists() {
        return None;
    }
    let c = Connection::open_with_flags(p, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).ok()?;
    c.query_row(
        "SELECT heading, text FROM entries WHERE volume = ? AND number = ?",
        params![volume, number],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .optional()
    .ok()
    .flatten()
}

/// Where each work is in the OpenITI corpus, when isnady-data cannot give it (`source_entries.WORKS[…]["url"]`).
fn corpus_url(work: &str) -> Option<&'static str> {
    match work {
        "siyar" => Some(
            "https://raw.githubusercontent.com/OpenITI/0750AH/master/data/0748Dhahabi/0748Dhahabi.SiyarAclamNubala/\
             0748Dhahabi.SiyarAclamNubala.Shamela0010906-ara1.mARkdown",
        ),
        "tadhkira" => Some(
            "https://raw.githubusercontent.com/OpenITI/0750AH/master/data/0748Dhahabi/0748Dhahabi.TadhkiratHuffaz/\
             0748Dhahabi.TadhkiratHuffaz.JK000532-ara1.mARkdown",
        ),
        _ => None,
    }
}

/// Where isnady-data keeps this work (its catalog's library), or None (`source_entries._own_location`).
fn own_location(work: &str) -> Option<String> {
    use crate::sources::repos;
    let mut chosen = vec![repos::selected()];
    if let Some(d) = repos::find(&crate::sources::tables().default_id) {
        chosen.push(d);
    }
    for repo in chosen {
        let Ok((cat, _)) = repos::catalog(&repo, false) else {
            continue;
        };
        for item in cat["library"].as_array().into_iter().flatten() {
            if item["entries"].as_str() == Some(work) {
                if let Some(loc) = item["location"].as_str().filter(|l| !l.is_empty()) {
                    return Some(crate::sources::absolute(
                        repo.url.as_deref().unwrap_or(""),
                        loc,
                    ));
                }
            }
        }
    }
    None
}

/// Download a work once and keep its entries (`source_entries.fetch`): from isnady-data first, from the OpenITI
/// corpus when isnady-data cannot give it; how many entries.
pub fn fetch(work: &str, say: &dyn Fn(&str)) -> Result<usize, String> {
    say(&format!(
        "Downloading {} (about {} MB)",
        work_title(work, "en"),
        work_size(work)
    ));
    let places: Vec<String> = own_location(work)
        .into_iter()
        .chain(corpus_url(work).map(String::from))
        .collect();
    let mut last = format!("no place to download {work} from");
    for place in places {
        match crate::sources::read_bytes(&place, Some(600)) {
            Ok(raw) => {
                say("Reading the entries");
                let text = String::from_utf8_lossy(&raw).to_string();
                return crate::sources::importers::openiti_work::store(work, &text, &place)
                    .map_err(|e| e.to_string());
            }
            Err(e) => last = e.to_string(),
        }
    }
    Err(last)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn written_lives() {
        let found = find("Abu Hurayra");
        assert_eq!(found[0]["id"], "abu-hurayra");
        let v = view(found[0], "tr");
        assert_eq!(v.title, "Ebû Hüreyre");
        assert!(!v.paragraphs.is_empty());
        assert_eq!(ce(57), 677);
    }
}
