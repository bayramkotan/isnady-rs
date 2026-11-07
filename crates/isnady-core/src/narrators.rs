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
pub fn known_aliases(conn: &Connection, person_id: i64) -> Vec<String> {
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

// ------------------------------------------------------------------ finding narrators (the Search page, S3)

/// One narrator of a list or of a search (`narrators.browse` / `find` rows).
#[derive(Debug, Clone, serde::Serialize)]
pub struct Row {
    pub id: i64,
    pub name: String,
    pub tabaqa: Option<i64>,
    pub death: Option<i64>,
    pub verdict: String,
    pub rank: Option<i64>,
    pub madhhab: Option<String>,
    pub in_chains: i64,
    pub view: name_parts::View,
    pub tradition: String,
    pub source: String,
    /// found by a close spelling, not the one typed (S4)
    pub close: bool,
    pub tier: i64,
    /// the consonants typed, but a reading that sounds different
    pub weak: bool,
    /// the consonants typed, and no reading to compare the vowels with
    pub unread: bool,
    /// the words of the entry the query matched, as written there
    pub matched: Vec<String>,
    pub closeness: f64,
    /// "Hz. Ebû Bekir es-Sıddîk (r.a.)": a famous one's title, above his entry's own name
    pub famous: Option<String>,
}

/// The name of a narrator laid out (`describe`'s name_view): the parts, the one he is known by.
pub fn name_view(
    conn: &Connection,
    person_id: i64,
    name_ar: &str,
    langs: Langs,
) -> name_parts::View {
    let shown = display_name(name_ar);
    let kunya: Option<String> = conn
        .query_row("SELECT kunya FROM persons WHERE id = ?", [person_id], |r| {
            r.get(0)
        })
        .optional()
        .ok()
        .flatten()
        .flatten();
    let parts = name_parts::parse(
        &shown,
        kunya.as_deref(),
        &known_aliases(conn, person_id),
        Some(name_ar),
        None,
    );
    name_parts::present(&parts, langs.reading, langs.words)
}

/// The narrators of one tradition, and how many names in the chains of THAT tradition's books are identified
/// (`narrators.overview`): (persons, links, identified, works).
pub fn overview(
    conn: &Connection,
    tradition: &str,
) -> rusqlite::Result<(i64, i64, i64, Vec<String>)> {
    let persons: i64 = conn.query_row(
        "SELECT COUNT(*) FROM persons WHERE COALESCE(tradition, 'sunni') = ?",
        [tradition],
        |r| r.get(0),
    )?;
    let (links, identified): (Option<i64>, Option<i64>) = conn.query_row(
        "SELECT COUNT(*), SUM(l.person_id IS NOT NULL) FROM isnad_links l JOIN isnads i ON i.id = l.isnad_id \
         JOIN hadiths h ON h.id = i.hadith_id JOIN collections c ON c.id = h.collection_id \
         WHERE COALESCE(c.tradition, 'sunni') = ?",
        [tradition],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let mut st = conn.prepare(
        "SELECT DISTINCT s.name FROM persons p JOIN sources s ON s.id = p.source_id \
         WHERE COALESCE(p.tradition, 'sunni') = ? ORDER BY s.name",
    )?;
    let works = st
        .query_map([tradition], |r| r.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok((persons, links.unwrap_or(0), identified.unwrap_or(0), works))
}

/// The languages a list is read in: names in `reading` (the interface's, `language.ui_language`), the interface's own
/// words in `words` (`i18n.language`).
#[derive(Debug, Clone, Copy)]
pub struct Langs<'a> {
    pub reading: &'a str,
    pub words: &'a str,
}

/// Narrators for a list (`narrators.browse`): (rows, how many before the limit), most often in the chains first, or
/// a search's own order (best match first); tradition None: both. `close` gets the ids found by a close spelling.
#[allow(clippy::too_many_arguments)]
pub fn browse(
    conn: &Connection,
    text: &str,
    tradition: Option<&str>,
    only: Option<&[i64]>,
    close: Option<&mut std::collections::HashSet<i64>>,
    limit: usize,
    offset: usize,
    langs: Langs,
) -> rusqlite::Result<(Vec<Row>, usize)> {
    browse_by(
        conn,
        text,
        &Filters::default(),
        tradition,
        only,
        close,
        limit,
        offset,
        langs,
    )
}

/// The Narrators page's filters (`browse`'s tabaqa, rank, book): a generation, a rank (-1: no judgment), the
/// narrators of one of the six books by Ibn Hajar's marks.
#[derive(Debug, Clone, Default)]
pub struct Filters {
    pub tabaqa: Option<i64>,
    pub rank: Option<i64>,
    pub book: Option<String>,
}

/// browse with the page's filters.
#[allow(clippy::too_many_arguments)]
pub fn browse_by(
    conn: &Connection,
    text: &str,
    filters: &Filters,
    tradition: Option<&str>,
    only: Option<&[i64]>,
    close: Option<&mut std::collections::HashSet<i64>>,
    limit: usize,
    offset: usize,
    langs: Langs,
) -> rusqlite::Result<(Vec<Row>, usize)> {
    use rusqlite::types::Value as Sql;
    let mut wheres: Vec<String> = Vec::new();
    let mut args: Vec<Sql> = Vec::new();
    if let Some(t) = tradition.filter(|t| !t.is_empty()) {
        wheres.push("COALESCE(p.tradition, 'sunni') = ?".into());
        args.push(Sql::Text(t.to_string()));
    }
    let mut ids: Vec<i64> = Vec::new();
    let searched = !text.trim().is_empty();
    if searched {
        ids = crate::people::search_persons_close(conn, text, 5000, close)?
            .into_iter()
            .map(|f| f.0)
            .collect();
        if ids.is_empty() {
            return Ok((Vec::new(), 0));
        }
        wheres.push(format!(
            "p.id IN ({})",
            ids.iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(",")
        ));
    }
    if let Some(only) = only {
        let list = only
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(",");
        wheres.push(format!(
            "p.id IN ({})",
            if list.is_empty() { "NULL".into() } else { list }
        ));
    }
    if let Some(tb) = filters.tabaqa.filter(|x| *x != 0) {
        wheres.push("p.tabaqa = ?".into());
        args.push(Sql::Integer(tb));
    }
    match filters.rank {
        Some(-1) => wheres.push("v.rank IS NULL".into()),
        Some(r) if r != 0 => {
            wheres.push("v.rank = ?".into());
            args.push(Sql::Integer(r));
        }
        _ => {}
    }
    if let Some(marks) = filters
        .book
        .as_ref()
        .and_then(|b| crate::tables::get().narrators.collection_marks.get(b))
    {
        wheres.push(format!(
            "EXISTS (SELECT 1 FROM person_marks m WHERE m.person_id = p.id AND m.mark IN ({}))",
            vec!["?"; marks.len()].join(",")
        ));
        args.extend(marks.iter().map(|m| Sql::Text(m.clone())));
    }
    let sql = format!(
        "SELECT p.id, p.name_ar, p.tabaqa, p.death_year_ah, v.phrase, v.rank, v.madhhab, \
         (SELECT COUNT(*) FROM isnad_links l WHERE l.person_id = p.id) AS in_chains, \
         COALESCE(p.tradition, 'sunni'), (SELECT name FROM sources WHERE id = p.source_id), p.kunya \
         FROM persons p LEFT JOIN verdicts v ON v.id = (SELECT MIN(id) FROM verdicts WHERE person_id = p.id) \
         {} ORDER BY in_chains DESC, p.name_ar",
        if wheres.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", wheres.join(" AND "))
        }
    );
    type Raw = (
        i64,
        String,
        Option<i64>,
        Option<i64>,
        Option<String>,
        Option<i64>,
        Option<String>,
        i64,
        String,
        Option<String>,
        Option<String>,
    );
    let mut st = conn.prepare(&sql)?;
    let mut rows: Vec<Raw> = st
        .query_map(rusqlite::params_from_iter(args.iter()), |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
                r.get(7)?,
                r.get(8)?,
                r.get(9)?,
                r.get(10)?,
            ))
        })?
        .collect::<Result<_, _>>()?;
    if searched {
        let rank_of: HashMap<i64, usize> = ids.iter().enumerate().map(|(i, p)| (*p, i)).collect();
        rows.sort_by_key(|r| rank_of.get(&r.0).copied().unwrap_or(ids.len()));
    }
    let total = rows.len();
    let out = rows
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|r| {
            let shown = display_name(&r.1);
            let parts = name_parts::parse(
                &shown,
                r.10.as_deref(),
                &known_aliases(conn, r.0),
                Some(&r.1),
                None,
            );
            Row {
                id: r.0,
                name: shown,
                tabaqa: r.2,
                death: r.3,
                verdict: r.4.unwrap_or_default(),
                rank: r.5,
                madhhab: r.6,
                in_chains: r.7,
                view: name_parts::present(&parts, langs.reading, langs.words),
                tradition: r.8,
                source: r.9.unwrap_or_default(),
                close: false,
                tier: 0,
                weak: false,
                unread: false,
                matched: Vec::new(),
                closeness: 0.0,
                famous: None,
            }
        })
        .collect();
    Ok((out, total))
}

use crate::people::{
    arabic_letter, arabic_skeleton, exact, fold_latin, latin_skeleton, loose, one_off,
    ordered_score, same_loose, split_abd, tokens,
};
use crate::{difflib, name_parts};

/// The words of a query as search_persons compares them, and whether they are Latin (`_query_words`).
fn query_words(text: &str) -> (Vec<String>, bool) {
    if arabic_letter(text) {
        return (
            tokens(text).into_iter().filter(|w| w != "بن").collect(),
            false,
        );
    }
    (
        py::words(text)
            .into_iter()
            .map(latin_skeleton)
            .filter(|k| !k.is_empty() && k != "bn" && k != "bnt")
            .collect(),
        true,
    )
}

/// The words of a name, comparable with query_words; keep_ibn keeps "b." / "ibn" (`_name_words`).
fn name_words(name: &str, latin: bool, keep_ibn: bool) -> Vec<String> {
    let words: Vec<String> = if latin {
        py::words(name).into_iter().map(arabic_skeleton).collect()
    } else {
        tokens(name)
            .into_iter()
            .map(|w| if w == "ابن" { "بن".to_string() } else { w })
            .collect()
    };
    if keep_ibn {
        words
    } else {
        words
            .into_iter()
            .filter(|w| !["bn", "bnt", "بن", "بنت"].contains(&w.as_str()))
            .collect()
    }
}

fn whole_query(text: &str, latin: bool) -> Vec<String> {
    if latin {
        return py::words(text)
            .into_iter()
            .map(latin_skeleton)
            .filter(|k| !k.is_empty())
            .collect();
    }
    tokens(text)
        .into_iter()
        .map(|w| if w == "ابن" { "بن".to_string() } else { w })
        .collect()
}

/// How alike a Latin query and a name's reading are, letter by letter, over as many words as the query has.
pub fn closeness(query: &str, reading: &str) -> f64 {
    let q = fold_latin(query);
    let n = py::words(&q).len().max(1);
    let folded = fold_latin(reading);
    let r = py::words(&folded)
        .into_iter()
        .take(n)
        .collect::<Vec<_>>()
        .join(" ");
    if q.is_empty() || r.is_empty() {
        0.0
    } else {
        difflib::ratio(&q, &r)
    }
}

/// A Latin query is a weak match below this: only the consonants in common (`WEAK_BELOW`).
pub const WEAK_BELOW: f64 = 0.7;
/// The best matches whose names are read to put the closest first (`SEARCH_RERANK`).
pub const SEARCH_RERANK: usize = 400;

/// How alike a Latin query is to the readings of the parts of the name whose consonants it matched (`_part_closeness`):
/// None when a matched part has no reading.
fn part_closeness(query: &str, view: &name_parts::View) -> Option<f64> {
    let q = fold_latin(query);
    let n = py::words(&q).len().max(1);
    let wanted: Vec<String> = whole_query(query, true)
        .into_iter()
        .filter(|w| w != "bn" && w != "bnt")
        .map(|w| loose(&w))
        .collect();
    let mut parts: Vec<(String, String)> = vec![(view.arabic.clone(), view.reading.clone())];
    for (_key, values, _star) in &view.rows {
        parts.extend(values.iter().cloned());
    }
    let (mut best, mut unread) = (0.0f64, false);
    for (arabic, reading) in parts {
        let words: Vec<String> = name_words(&arabic, true, false)
            .iter()
            .map(|w| loose(w))
            .collect();
        if words.is_empty()
            || !wanted
                .iter()
                .any(|qw| words.iter().any(|w| same_loose(qw, w) || one_off(qw, w)))
        {
            continue;
        }
        if reading.is_empty() {
            if wanted.iter().any(|qw| words.iter().any(|w| qw == w)) {
                unread = true;
            }
            continue;
        }
        let head = reading.split(" (").next().unwrap_or("");
        let folded_text = fold_latin(head);
        let folded = py::words(&folded_text);
        let stretches = folded.len().saturating_sub(n) + 1;
        for i in 0..stretches.max(1) {
            let stretch = folded
                .iter()
                .skip(i)
                .take(n)
                .copied()
                .collect::<Vec<_>>()
                .join(" ");
            if !stretch.is_empty() {
                best = best.max(difflib::ratio(&q, &stretch));
            }
        }
    }
    if best < WEAK_BELOW && unread {
        return None;
    }
    Some(best)
}

/// The words of an entry the query matched, as written there (`_matched_words`).
fn matched_words(query: &str, entry: &str, latin: bool) -> Vec<String> {
    let wanted: Vec<String> = whole_query(query, latin)
        .into_iter()
        .filter(|w| !["bn", "bnt", "بن"].contains(&w.as_str()))
        .map(|w| loose(&w))
        .collect();
    let mut out: Vec<String> = Vec::new();
    for q in &wanted {
        for word in py::words(entry) {
            let w = if latin {
                arabic_skeleton(word)
            } else {
                normalize(word)
            };
            if same_loose(q, &w) || one_off(q, &w) {
                if !out.iter().any(|o| o == word) {
                    out.push(word.to_string());
                }
                break;
            }
        }
    }
    out
}

/// Python's round(x, 1).
fn round1(x: f64) -> f64 {
    format!("{x:.1}").parse().unwrap_or(x)
}

/// Narrators for the Search page, both traditions unless one is asked for, the closest first (`narrators.find`):
/// the name he is known by, then the query in his name, then only in the words about him; the scholars known by the
/// name typed and the famous Companions first of all. (rows, how many match in all); only the first SEARCH_RERANK.
pub fn find(
    conn: &Connection,
    text: &str,
    tradition: Option<&str>,
    langs: Langs,
) -> rusqlite::Result<(Vec<Row>, usize)> {
    let text = split_abd(text);
    let (wanted, latin) = query_words(&text);
    if wanted.is_empty() {
        return Ok((Vec::new(), 0));
    }
    let mut close = std::collections::HashSet::new();
    let (mut rows, mut total) = browse(
        conn,
        &text,
        tradition,
        None,
        Some(&mut close),
        SEARCH_RERANK,
        0,
        langs,
    )?;
    let whole = whole_query(&text, latin);
    let tier = |row: &Row, close_spelling: bool| -> i64 {
        let m: fn(&str, &str) -> bool = if close_spelling { one_off } else { exact };
        let w_whole: Vec<String> = if close_spelling {
            whole.iter().map(|w| loose(w)).collect()
        } else {
            whole.clone()
        };
        let w_wanted: Vec<String> = if close_spelling {
            wanted.iter().map(|w| loose(w)).collect()
        } else {
            wanted.clone()
        };
        let known = name_words(&row.view.arabic, latin, true);
        let first = w_whole.first().cloned().unwrap_or_default();
        let begins = !known.is_empty() && m(&first, &known[0]);
        if begins
            && ordered_score(&w_whole, &known, &w_whole, &known, m).is_some()
            && (close_spelling
                || !latin
                || row.view.reading.is_empty()
                || closeness(&text, &row.view.reading) >= 0.8)
        {
            return 0;
        }
        let parts = row
            .view
            .rows
            .iter()
            .flat_map(|(_k, values, _s)| values.iter().map(|(ar, _l)| ar.clone()))
            .collect::<Vec<_>>()
            .join(" ");
        let name = name_words(&parts, latin, false);
        if !name.is_empty() && ordered_score(&w_wanted, &name, &w_wanted, &name, m).is_some() {
            return 1;
        }
        2
    };
    for row in rows.iter_mut() {
        row.close = close.contains(&row.id);
        row.tier = tier(row, row.close) + if row.close { 10 } else { 0 };
        let part = if latin {
            part_closeness(&text, &row.view)
        } else {
            None
        };
        row.weak = part.is_some_and(|p| p < WEAK_BELOW);
        row.unread = latin && part.is_none();
        if row.weak || row.close || row.unread {
            row.matched = matched_words(&text, &row.name, latin);
        }
        row.closeness = if latin {
            closeness(&text, &row.view.reading).max(closeness(&text, &row.view.full_reading))
        } else {
            1.0
        };
    }
    // a scholar known by the name typed ("Bukhari", "Abu Dawud") is first, when he is also a narrator here
    let famous: Vec<i64> = {
        let mut v: Vec<i64> = crate::scholars::named(&text)
            .into_iter()
            .filter_map(|s| crate::scholars::as_narrator(conn, s))
            .collect();
        v.sort_unstable();
        v.dedup();
        v
    };
    for row in rows.iter_mut() {
        if famous.contains(&row.id) {
            row.tier = -1;
        }
    }
    let have: Vec<i64> = rows.iter().map(|r| r.id).collect();
    let missing: Vec<i64> = famous
        .iter()
        .copied()
        .filter(|p| !have.contains(p))
        .collect();
    if !missing.is_empty() {
        let (more, _n) = browse(
            conn,
            "",
            tradition,
            Some(&missing),
            None,
            missing.len(),
            0,
            langs,
        )?;
        total += more.len();
        rows.extend(more.into_iter().map(|mut r| {
            r.tier = -1;
            r.closeness = 1.0;
            r.close = false;
            r
        }));
    }
    // a famous Companion named any way people write him comes before everyone (FS1)
    let first = crate::people::famous_ids(conn, &crate::people::famous_named(&text, tradition));
    let place: HashMap<i64, i64> = first
        .iter()
        .enumerate()
        .map(|(i, p)| (*p, i as i64))
        .collect();
    for row in rows.iter_mut() {
        if let Some(i) = place.get(&row.id) {
            row.tier = -100 + i;
            row.weak = false;
            row.closeness = 1.0;
            row.close = false;
            row.unread = false;
        }
    }
    let have: Vec<i64> = rows.iter().map(|r| r.id).collect();
    let missing: Vec<i64> = first
        .iter()
        .copied()
        .filter(|p| !have.contains(p))
        .collect();
    if !missing.is_empty() {
        let (more, _n) = browse(
            conn,
            "",
            tradition,
            Some(&missing),
            None,
            missing.len(),
            0,
            langs,
        )?;
        total += more.len();
        rows.extend(more.into_iter().map(|mut r| {
            r.tier = -100 + place[&r.id];
            r.closeness = 1.0;
            r.close = false;
            r
        }));
    }
    let shown = crate::people::famous_titles(conn, langs.words);
    for r in rows.iter_mut() {
        if let Some(title) = shown.get(&r.id) {
            r.famous = Some(title.clone());
        }
    }
    let mut keyed: Vec<(usize, Row)> = rows.into_iter().enumerate().collect();
    keyed.sort_by(|(ia, a), (ib, b)| {
        (a.weak, a.tier)
            .cmp(&(b.weak, b.tier))
            .then(round1(b.closeness).total_cmp(&round1(a.closeness)))
            .then(ia.cmp(ib))
    });
    Ok((keyed.into_iter().map(|(_, r)| r).collect(), total))
}

/// The matches after the first ones find() returned (`narrators.find_more`, NR-B1), in search_persons' own order.
pub fn find_more(
    conn: &Connection,
    text: &str,
    tradition: Option<&str>,
    offset: usize,
    limit: usize,
    langs: Langs,
) -> rusqlite::Result<Vec<Row>> {
    let mut close = std::collections::HashSet::new();
    let text = split_abd(text);
    let (mut rows, _total) = browse(
        conn,
        &text,
        tradition,
        None,
        Some(&mut close),
        limit,
        offset,
        langs,
    )?;
    let (_wanted, latin) = query_words(&text);
    for row in rows.iter_mut() {
        row.close = close.contains(&row.id);
        row.weak = false;
        row.tier = 20;
        row.closeness = 1.0;
        if row.close {
            row.matched = matched_words(&text, &row.name, latin);
        }
    }
    Ok(rows)
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
