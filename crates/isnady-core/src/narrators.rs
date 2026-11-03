//! What the rijal works say about a narrator, as the chain shows it — the Python isnady's `narrators.describe`
//! (`isnady/core/narrators.py`), with the name he is known by (`name_parts`) and his name without Ibn Hajar's reading
//! notes (`rijal.display_name`).

use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashMap;
use std::sync::OnceLock;

use crate::normalize::normalize;
use crate::py;
use crate::tables;

#[derive(Debug, Clone)]
pub struct Verdict {
    pub critic_name: String,
    pub phrase: String,
    pub rank_scheme: Option<String>,
    pub rank: Option<i64>,
    /// the rank in words, for a Shia verdict (al-Najashi's categories)
    pub rank_label: String,
}

#[derive(Debug, Clone)]
pub struct Person {
    pub id: i64,
    pub name_ar: String,
    pub tabaqa: Option<i64>,
    pub death_year_ah: Option<i64>,
    pub display_name: String,
    /// the Arabic of the name he is known by ("الأعمش", "ابن عمر", "أبو هريرة")
    pub known_arabic: String,
    pub verdicts: Vec<Verdict>,
}

/// Is any rijal work imported?
pub fn loaded(conn: &Connection) -> bool {
    conn.query_row("SELECT 1 FROM persons LIMIT 1", [], |_| Ok(()))
        .optional()
        .ok()
        .flatten()
        .is_some()
}

/// Every narrator of the chains looked up by the Python isnady's matcher, on these narrators and these chains
/// (`narrators.links_ready`).
pub fn links_ready(conn: &Connection) -> bool {
    let t = tables::get();
    let run = || -> rusqlite::Result<bool> {
        let persons: i64 = conn.query_row("SELECT COUNT(*) FROM persons", [], |r| r.get(0))?;
        if persons == 0 {
            return Ok(true);
        }
        let death: Option<String> = conn
            .query_row(
                "SELECT value FROM core_meta WHERE key = 'death_rule'",
                [],
                |r| r.get(0),
            )
            .optional()?;
        if death.as_deref() != Some(t.death_rule.as_str()) {
            return Ok(false);
        }
        let (n, max): (i64, i64) = conn.query_row(
            "SELECT COUNT(*), COALESCE(MAX(id), 0) FROM persons",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        let stamp = format!("{}|({n}, {max})|{}", t.matcher_id, t.parser_id);
        let row: Option<String> = conn
            .query_row(
                "SELECT value FROM core_meta WHERE key = 'narrator_matcher'",
                [],
                |r| r.get(0),
            )
            .optional()?;
        if row.as_deref() != Some(stamp.as_str()) {
            return Ok(false);
        }
        let pending: bool = conn.query_row(
            "SELECT EXISTS (SELECT 1 FROM isnad_links WHERE candidates IS NULL)",
            [],
            |r| r.get(0),
        )?;
        Ok(!pending)
    };
    run().unwrap_or(false)
}

/// The aliases that may name a person by themselves: his own, or one shared by several only for the one the chains
/// cite most, at least twice as often as the next (`narrators.known_aliases`).
fn known_aliases(conn: &Connection, person_id: i64) -> Vec<String> {
    static ALLOWED: OnceLock<HashMap<i64, Vec<String>>> = OnceLock::new();
    let allowed = ALLOWED.get_or_init(|| {
        let mut out: HashMap<i64, Vec<String>> = HashMap::new();
        let mut read = || -> rusqlite::Result<()> {
            let mut uses: HashMap<i64, i64> = HashMap::new();
            let mut st = conn.prepare(
                "SELECT person_id, COUNT(*) FROM isnad_links WHERE person_id IS NOT NULL GROUP BY person_id",
            )?;
            for r in st.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)))? {
                let (p, n) = r?;
                uses.insert(p, n);
            }
            let mut owners: Vec<(String, Vec<i64>)> = Vec::new();
            let mut at: HashMap<String, usize> = HashMap::new();
            let mut st =
                conn.prepare("SELECT person_id, name FROM person_names WHERE kind = 'variant'")?;
            for r in st.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))? {
                let (pid, name) = r?;
                let i = *at.entry(name.clone()).or_insert_with(|| {
                    owners.push((name.clone(), Vec::new()));
                    owners.len() - 1
                });
                owners[i].1.push(pid);
            }
            for (name, pids) in owners {
                let mut pids = pids;
                pids.sort_unstable();
                pids.dedup();
                pids.sort_by_key(|p| -uses.get(p).copied().unwrap_or(0));
                if pids.len() == 1 {
                    out.entry(pids[0]).or_default().push(name);
                } else {
                    let first = uses.get(&pids[0]).copied().unwrap_or(0);
                    let second = uses.get(&pids[1]).copied().unwrap_or(0);
                    if first >= 10 && first >= 2 * second {
                        out.entry(pids[0]).or_default().push(name);
                    }
                }
            }
            Ok(())
        };
        let _ = read();
        out
    });
    allowed.get(&person_id).cloned().unwrap_or_default()
}

/// A name without Ibn Hajar's reading notes ("بفتح المعجمة وسكون الزاي"): `rijal.display_name`.
pub fn display_name(name: &str) -> String {
    static RES: OnceLock<(fancy_regex::Regex, fancy_regex::Regex)> = OnceLock::new();
    let (start, notes) = RES.get_or_init(|| {
        let t = tables::get();
        (
            tables::pattern(&t.rijal.note_start),
            tables::pattern(&t.rijal.note_words),
        )
    });
    let hit = |re: &fancy_regex::Regex, w: &str| re.is_match(w).unwrap_or(false);
    let mut out: Vec<&str> = Vec::new();
    let mut skipping = false;
    for word in py::words(name) {
        if hit(start, word) {
            skipping = true;
            continue;
        }
        if skipping {
            if ["بن", "ابن", "أبو", "أبي", "أم", "بنت"].contains(&word)
                || (word.starts_with("ال") && !hit(start, word) && !hit(notes, word))
            {
                skipping = false;
            } else {
                continue;
            }
        }
        out.push(word);
    }
    out.join(" ")
}

const KUNYA: [&str; 4] = ["أبو", "أبي", "أبا", "أم"];

/// How many words the name starting at i takes: عبد الله, عبيد الله, أبي بكر are one name.
fn take_name(words: &[String], i: usize) -> usize {
    if i + 1 < words.len() && (words[i] == "عبد" || words[i] == "عبيد" || words[i + 1] == "الله")
    {
        return 2;
    }
    if i + 1 < words.len() && KUNYA.contains(&words[i].as_str()) {
        return if i + 2 < words.len() && words[i + 1] == "عبد" {
            3
        } else {
            2
        };
    }
    1
}

/// The Arabic of the name a person is known by — `name_parts.present(name_parts.parse(...))["arabic"]`: an alias of
/// a recognisable shape, else his kunya when the entry opens with it, else the name the entry says he is known as,
/// else his name and his father's.
fn known_arabic(name: &str, aliases: &[String], text: &str) -> String {
    let t = tables::get();
    let cleaned: String = name
        .chars()
        .map(|c| if "،,.:؛()[]".contains(c) { ' ' } else { c })
        .collect();
    let mut words: Vec<String> = py::words(&cleaned).iter().map(|s| s.to_string()).collect();
    if let Some(cut) = words
        .iter()
        .position(|w| ["يلقب", "ويلقب", "الملقب", "لقبه", "ولقبه"].contains(&w.as_str()))
    {
        words.truncate(cut);
    }
    let mut ism: Option<Vec<String>> = None;
    let mut nasab: Vec<Vec<String>> = Vec::new();
    let mut kunya: Option<Vec<String>> = None;
    let mut i = 0;
    let ubayy = words.first().is_some_and(|w| w == "أبي")
        && words.get(1).is_some_and(|w| w == "بن" || w == "ابن");
    if !words.is_empty() && KUNYA.contains(&words[0].as_str()) && !ubayy {
        let n = take_name(&words, 0);
        kunya = Some(words[..n.min(words.len())].to_vec());
        i = n;
        if i + 1 < words.len()
            && !words[i].starts_with("ال")
            && (words[i + 1] == "بن" || words[i + 1] == "بنت")
        {
            let m = take_name(&words, i);
            ism = Some(words[i..(i + m).min(words.len())].to_vec());
            i += m;
        }
    } else if ubayy {
        ism = Some(vec!["أبيّ".to_string()]);
        i = 1;
    } else if !words.is_empty() {
        let n = take_name(&words, 0);
        ism = Some(words[..n.min(words.len())].to_vec());
        i = n;
    }
    while ism.is_some()
        && i < words.len()
        && !["بن", "بنت", "ابن", "مولى", "مولاهم"].contains(&words[i].as_str())
        && !words[i].starts_with("ال")
        && !KUNYA.contains(&words[i].as_str())
        && !(i + 1 < words.len() && words[i + 1] == "الدين")
    {
        let n = take_name(&words, i);
        let mut more = ism.take().unwrap_or_default();
        more.extend_from_slice(&words[i..(i + n).min(words.len())]);
        ism = Some(more);
        i += n;
    }
    while i + 1 < words.len() && ["بن", "بنت", "ابن"].contains(&words[i].as_str()) {
        let n = take_name(&words, i + 1);
        nasab.push(words[i..(i + 1 + n).min(words.len())].to_vec());
        i += 1 + n;
    }
    // the name the entry says he is KNOWN as ("المعروف ب…")
    static KNOWN: OnceLock<fancy_regex::Regex> = OnceLock::new();
    let known_re = KNOWN.get_or_init(|| tables::pattern(&t.name_parts.known));
    let stated: Option<Vec<String>> = known_re
        .captures(text)
        .ok()
        .flatten()
        .and_then(|c| c.get(1))
        .map(|m| {
            py::words(&m.as_str().replace("بابن", "ابن"))
                .iter()
                .map(|s| s.to_string())
                .collect()
        });
    let mut clean: Vec<Vec<String>> = Vec::new();
    for a in aliases {
        let aw: Vec<String> = py::words(a).iter().map(|s| s.to_string()).collect();
        if !(1..=3).contains(&aw.len()) || aw.iter().any(|x| x == "عن" || x == "هو" || x == "و")
        {
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
    let mut known: Option<(String, Vec<String>)> = None;
    let mut ranked: Vec<(u8, (&str, Vec<String>))> = Vec::new();
    for aw in &clean {
        if (aw[0] == "بن" || aw[0] == "ابن") && aw.len() >= 2 {
            ranked.push((1, ("ibn", aw.clone())));
        } else if aw.len() == 1 && aw[0].starts_with("ال") && words.contains(&aw[0]) {
            ranked.push(if aw[0].ends_with('ي') {
                (2, ("nisba", aw.clone()))
            } else {
                (0, ("laqab", aw.clone()))
            });
        }
    }
    if let Some(best) = ranked.iter().map(|r| r.0).min() {
        let (kind, w) = &ranked.iter().find(|r| r.0 == best).expect("min").1;
        known = Some((kind.to_string(), w.clone()));
    }
    if known.is_none()
        && words.first().is_some_and(|w| KUNYA.contains(&w.as_str()))
        && kunya.as_ref().is_some_and(|k| !k.is_empty())
        && ism.as_deref() != Some(&["أبيّ".to_string()][..])
    {
        known = Some(("kunya".into(), kunya.clone().unwrap_or_default()));
    }
    if known.is_none() {
        if let Some(s) = &stated {
            if !s.is_empty() {
                known = Some((
                    if s[0] == "ابن" { "ibn" } else { "laqab" }.into(),
                    s.clone(),
                ));
            }
        }
    }
    if known.is_none() {
        if let Some(n) = ism.as_ref().filter(|n| !n.is_empty()) {
            let mut w = n.clone();
            if let Some(first) = nasab.first() {
                w.extend(first.iter().cloned());
            }
            known = Some(("ism".into(), w));
        }
    }
    let (kind, kw) = known.unwrap_or_else(|| ("ism".into(), Vec::new()));
    let mut arabic = kw.join(" ");
    if kind == "ibn" {
        arabic = format!("ابن {}", kw.get(1..).unwrap_or(&[]).join(" "));
    }
    if arabic.is_empty() {
        let mut full = ism.unwrap_or_default();
        for p in &nasab {
            full.extend(p.iter().cloned());
        }
        arabic = full.join(" ");
    }
    arabic
}

/// What the rijal works say about one person, as far as the chain shows it (`narrators.describe`).
pub fn describe(conn: &Connection, person_id: i64) -> Option<Person> {
    let t = tables::get();
    let row = conn
        .query_row(
            "SELECT p.id, p.name_ar, p.kunya, p.tabaqa, p.death_year_ah FROM persons p WHERE p.id = ?",
            params![person_id],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, Option<i64>>(3)?,
                    r.get::<_, Option<i64>>(4)?,
                ))
            },
        )
        .optional()
        .ok()
        .flatten()?;
    let (id, name_ar, _kunya, tabaqa, death) = row;
    let mut verdicts = Vec::new();
    if let Ok(mut st) = conn.prepare(
        "SELECT critic_name, phrase, rank_scheme, rank FROM verdicts WHERE person_id = ? ORDER BY id",
    ) {
        if let Ok(rows) = st.query_map(params![person_id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, Option<i64>>(3)?,
            ))
        }) {
            for (critic_name, phrase, rank_scheme, rank) in rows.flatten() {
                let rank_label = match (rank_scheme.as_deref(), rank) {
                    (Some("shia"), Some(r)) if r != 0 => t
                        .rijal
                        .shia_ranks
                        .get(&r.to_string())
                        .cloned()
                        .unwrap_or_default(),
                    _ => String::new(),
                };
                verdicts.push(Verdict {
                    critic_name,
                    phrase,
                    rank_scheme,
                    rank,
                    rank_label,
                });
            }
        }
    }
    let display = display_name(&name_ar);
    let aliases = known_aliases(conn, person_id);
    let known_arabic = known_arabic(&display, &aliases, &name_ar);
    Some(Person {
        id,
        name_ar,
        tabaqa,
        death_year_ah: death,
        display_name: display,
        known_arabic,
        verdicts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_names() {
        assert_eq!(known_arabic("أبو هريرة الدوسي", &[], ""), "أبو هريرة");
        assert_eq!(
            known_arabic("عبد الله بن عمر بن الخطاب", &[], ""),
            "عبد الله بن عمر"
        );
        assert_eq!(
            known_arabic(
                "سليمان بن مهران الأسدي الكاهلي الأعمش",
                &["الأعمش".to_string()],
                ""
            ),
            "الأعمش"
        );
    }
}
