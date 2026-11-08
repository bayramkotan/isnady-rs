//! The huffaz (HFZ1): al-Dhahabi's Tadhkirat al-Huffaz (d. 748 AH) — the masters of hadith memorisation and
//! criticism, 1,176 of them in 21 generations (tabaqat), from Abu Bakr al-Siddiq to al-Dhahabi's own teachers: the
//! Python isnady's `core/huffaz.py`.
//!
//! What isnady keeps is an index read from the OpenITI text, never the text itself (data/huffaz.json, written by
//! tools/gen_tables.py from the Python isnady's assets/scholars/huffaz.json): al-Dhahabi's number of the entry, his
//! generation, the name he heads the entry with and its first twelve words, the sigla before the name, the year of
//! death as he writes it in words, and the man's entry in Ibn Hajar's Taqrib when the same man is there.

use std::collections::HashMap;
use std::sync::OnceLock;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::narrators::display_name;
use crate::normalize::normalize;
use crate::people::fold_latin;
use crate::{cache, names};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Entry {
    pub n: i64,
    pub tabaqa: i64,
    pub name: String,
    pub rumuz: Vec<String>,
    pub death: Option<i64>,
    pub taqrib: Option<String>,
    pub head: String,
}

/// An entry with the man's Taqrib entry joined (`huffaz.people`).
#[derive(Serialize, Clone, Debug)]
pub struct Hafiz {
    #[serde(flatten)]
    pub entry: Entry,
    pub person: Option<i64>,
    pub taqrib_name: Option<String>,
    pub taqrib_death: Option<i64>,
    pub in_chains: i64,
}

#[derive(Deserialize)]
struct Data {
    work: String,
    rumuz: Vec<(String, String)>,
    stated: Vec<(i64, i64)>,
    huffaz: Vec<Entry>,
}

fn data() -> &'static Data {
    static DATA: OnceLock<Data> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../data/huffaz.json")).expect("data/huffaz.json")
    })
}

/// The work's key among the classical texts that can be downloaded (`huffaz.WORK`).
pub fn work() -> &'static str {
    &data().work
}

/// Every entry of the index, in al-Dhahabi's order.
pub fn index() -> &'static [Entry] {
    &data().huffaz
}

/// al-Mizzi's sigla as al-Dhahabi uses them, in their order: (sign, what it means in English).
pub fn rumuz() -> &'static [(String, String)] {
    &data().rumuz
}

/// What each sign means (in English: the caller translates).
pub fn rumuz_text(signs: &[String]) -> Vec<String> {
    signs
        .iter()
        .map(|r| {
            rumuz()
                .iter()
                .find(|(k, _)| k == r)
                .map(|(_, v)| v.clone())
                .unwrap_or_else(|| r.clone())
        })
        .collect()
}

/// How many al-Dhahabi says a generation holds, where his heading says it.
pub fn stated(tabaqa: i64) -> Option<i64> {
    data()
        .stated
        .iter()
        .find(|(t, _)| *t == tabaqa)
        .map(|(_, n)| *n)
}

type Joined = HashMap<String, (i64, String, Option<i64>, i64)>;

fn joined(conn: &Connection) -> rusqlite::Result<Joined> {
    cache::kept(conn, "huffaz-people", || {
        let keys: Vec<&String> = index().iter().filter_map(|h| h.taqrib.as_ref()).collect();
        let mut found: Joined = HashMap::new();
        if keys.is_empty() {
            return Ok(found);
        }
        let marks = vec!["?"; keys.len()].join(",");
        let mut st = conn.prepare(&format!(
            "SELECT id, key, name_ar, death_year_ah FROM persons WHERE key IN ({marks})"
        ))?;
        let rows = st.query_map(rusqlite::params_from_iter(keys.iter()), |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<i64>>(3)?,
            ))
        })?;
        for row in rows {
            let (pid, key, name, death) = row?;
            found.insert(key, (pid, name, death, 0));
        }
        if !found.is_empty() {
            let ids: Vec<String> = found.values().map(|v| v.0.to_string()).collect();
            let mut counts: HashMap<i64, i64> = HashMap::new();
            let mut st = conn.prepare(&format!(
                "SELECT person_id, COUNT(*) FROM isnad_links WHERE person_id IN ({}) GROUP BY person_id",
                ids.join(",")
            ))?;
            for r in st.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)))? {
                let (pid, n) = r?;
                counts.insert(pid, n);
            }
            for v in found.values_mut() {
                v.3 = counts.get(&v.0).copied().unwrap_or(0);
            }
        }
        Ok(found)
    })
}

/// The index with each man's Taqrib entry joined. Without a database the index alone.
pub fn people(conn: Option<&Connection>) -> Vec<Hafiz> {
    let joined = conn.and_then(|c| joined(c).ok()).unwrap_or_default();
    index()
        .iter()
        .map(|e| {
            let j = e.taqrib.as_ref().and_then(|k| joined.get(k));
            Hafiz {
                entry: e.clone(),
                person: j.map(|j| j.0),
                taqrib_name: j.map(|j| j.1.clone()),
                taqrib_death: j.and_then(|j| j.2),
                in_chains: j.map(|j| j.3).unwrap_or(0),
            }
        })
        .collect()
}

/// The name in Latin letters, only where every word of it can be read: al-Dhahabi's short name, else the Taqrib's
/// name of the same man; "" when neither reads whole (then the Arabic is shown).
pub fn reading(r: &Hafiz, lang: &str) -> String {
    if lang == "ar" {
        return String::new();
    }
    let mut name = r.entry.name.as_str();
    for title in ["أمير المؤمنين", "أم المؤمنين"] {
        if let Some(rest) = name.strip_prefix(title) {
            if rest.starts_with(char::is_whitespace) {
                name = rest.trim_start();
                break;
            }
        }
    }
    let words: Vec<&str> = name.split_whitespace().collect();
    if let Some(whole) = names::latin(name, lang, 8).filter(|w| !w.is_empty()) {
        if words.len() == 1
            || Some(whole.clone()) != names::latin(&words[..words.len() - 1].join(" "), lang, 8)
        {
            return whole;
        }
    }
    if let Some(t) = &r.taqrib_name {
        let read = names::latin(&display_name(t), lang, 8).unwrap_or_default();
        let mut full: Vec<&str> = read.split_whitespace().collect();
        while full
            .last()
            .is_some_and(|w| ["Abd", "b.", "bint"].contains(w))
        {
            full.pop();
        }
        return full.join(" ");
    }
    String::new()
}

/// The entry of a narrator, by his Taqrib key, when al-Dhahabi counts him among the huffaz.
pub fn of_key(key: Option<&str>) -> Option<&'static Entry> {
    let key = key?;
    index().iter().find(|h| h.taqrib.as_deref() == Some(key))
}

pub fn entry(n: i64) -> Option<&'static Entry> {
    index().iter().find(|h| h.n == n)
}

/// (year, whose): al-Dhahabi's year when he gives one in words, else the Taqrib's for the same man.
pub fn death_of(h: &Hafiz) -> (Option<i64>, &'static str) {
    if let Some(y) = h.entry.death.filter(|y| *y != 0) {
        return (Some(y), "dhahabi");
    }
    if let Some(y) = h.taqrib_death.filter(|y| *y != 0) {
        return (Some(y), "taqrib");
    }
    (None, "")
}

/// One tabaqa as `huffaz.generations` counts it.
#[derive(Serialize, Clone, Debug)]
pub struct Generation {
    pub tabaqa: i64,
    pub count: usize,
    pub dated: usize,
    pub median: Option<i64>,
    pub from: Option<i64>,
    pub to: Option<i64>,
    pub in_taqrib: usize,
    pub stated: Option<i64>,
}

/// Each tabaqa: how many entries, how many with a year of death, the median and the middle half of those years, how
/// many are found in the Taqrib, al-Dhahabi's own count.
pub fn generations(conn: Option<&Connection>) -> Vec<Generation> {
    let rows = people(conn);
    let mut tabaqat: Vec<i64> = rows.iter().map(|r| r.entry.tabaqa).collect();
    tabaqat.sort();
    tabaqat.dedup();
    tabaqat
        .into_iter()
        .map(|t| {
            let rs: Vec<&Hafiz> = rows.iter().filter(|r| r.entry.tabaqa == t).collect();
            let mut years: Vec<i64> = rs.iter().filter_map(|r| death_of(r).0).collect();
            years.sort();
            let q = |f: f64| {
                (!years.is_empty())
                    .then(|| years[(years.len() - 1).min((f * years.len() as f64) as usize)])
            };
            Generation {
                tabaqa: t,
                count: rs.len(),
                dated: years.len(),
                median: q(0.5),
                from: q(0.25),
                to: q(0.75),
                in_taqrib: rs
                    .iter()
                    .filter(|r| r.person.is_some() || (conn.is_none() && r.entry.taqrib.is_some()))
                    .count(),
                stated: stated(t),
            }
        })
        .collect()
}

/// (century AH, how many huffaz died in it), counting only those with a known year.
pub fn by_century(conn: Option<&Connection>) -> Vec<(i64, usize)> {
    let mut counts: HashMap<i64, usize> = HashMap::new();
    for r in people(conn) {
        if let Some(y) = death_of(&r).0 {
            *counts.entry((y - 1).div_euclid(100) + 1).or_default() += 1;
        }
    }
    let mut out: Vec<(i64, usize)> = counts.into_iter().collect();
    out.sort();
    out
}

fn arabic_word(w: &str) -> bool {
    w.chars().any(|c| ('\u{0600}'..='\u{06FF}').contains(&c))
}

/// The entries whose Arabic heading (or Taqrib name, or reading) holds every word of the text; in one tabaqa.
pub fn find(conn: Option<&Connection>, text: &str, tabaqa: Option<i64>) -> Vec<Hafiz> {
    let rows: Vec<Hafiz> = people(conn)
        .into_iter()
        .filter(|r| tabaqa.map_or(true, |t| r.entry.tabaqa == t))
        .collect();
    let words: Vec<&str> = text
        .split_whitespace()
        .map(|w| {
            if w == "ابن" || w == "إبن" {
                "بن"
            } else {
                w
            }
        })
        .collect();
    if words.is_empty() {
        return rows;
    }
    rows.into_iter()
        .filter(|r| {
            let arabic = normalize(&format!(
                "{} {}",
                r.entry.head,
                r.taqrib_name.as_deref().unwrap_or("")
            ));
            let mut names_of = vec![r.entry.name.clone()];
            if let Some(t) = &r.taqrib_name {
                names_of.push(display_name(t));
            }
            let read: Vec<String> = names_of
                .iter()
                .flat_map(|n| [names::latin(n, "tr", 8), names::latin(n, "en", 8)])
                .flatten()
                .filter(|x| !x.is_empty())
                .collect();
            let latin_hay = fold_latin(&read.join(" "));
            words.iter().all(|w| {
                if arabic_word(w) {
                    arabic.contains(&normalize(w))
                } else {
                    latin_hay.contains(&fold_latin(w))
                }
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_index() {
        assert_eq!(index().len(), 1176);
        let gens = generations(None);
        assert_eq!(gens.len(), 21);
        assert_eq!(gens[0].count, 23);
        assert_eq!(stated(9), Some(106));
        assert_eq!(
            rumuz_text(&["ع".to_string()]),
            vec!["the six books".to_string()]
        );
        assert_eq!(find(None, "الزهري", Some(4)).len(), 3);
        let zuhri: Vec<i64> = find(None, "zuhri", None)
            .iter()
            .map(|r| r.entry.n)
            .collect();
        assert_eq!(zuhri, vec![97, 124, 952]);
    }
}
