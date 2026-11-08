//! The scholars of hadith in isnady — the Python isnady's `core/scholars.py`: who they are (the same register,
//! written into `data/tables.json` by tools/gen_tables.py), the roles the imported data gives each (compiler, grader,
//! critic), and finding them by any spelling of a name or a work. Sunni and Shia scholars stand on the same list,
//! each by his own road; the measures of their work come with the Hadith Scholars page (RS5).

use rusqlite::{Connection, OptionalExtension};
use std::collections::{HashMap, HashSet};
use unicode_normalization::char::canonical_combining_class;
use unicode_normalization::UnicodeNormalization;

use crate::normalize::normalize;
use crate::people::{fold_latin, loose, near};
use crate::tables::{self, Scholar};
use crate::{i18n, name_parts, py};

/// Every scholar of the register, Sunni then Shia, in its order.
pub fn all() -> &'static [Scholar] {
    &tables::get().scholars.list
}

pub fn find(id: &str) -> Option<&'static Scholar> {
    all().iter().find(|s| s.id == id)
}

pub fn tradition_of(s: &Scholar) -> &str {
    s.tradition.as_deref().unwrap_or("sunni")
}

/// What a role is called and what it means, in English (`ROLE_HELP`).
pub fn role_help(role: &str) -> (String, String) {
    tables::get()
        .scholars
        .role_help
        .get(role)
        .cloned()
        .unwrap_or_default()
}

/// A scholar's short name in a language: "al-Albani", "Elbânî", "الألباني" (`scholars.short_name`).
pub fn short_name(s: &Scholar, lang: &str) -> String {
    if lang == "tr" {
        if let Some(n) = tables::get().scholars.short_tr.get(&s.id) {
            return n.clone();
        }
        if !s.tr.is_empty() {
            return s.tr.clone();
        }
        return s.short.clone().unwrap_or_else(|| s.name.clone());
    }
    if ["ar", "fa", "prs", "ps", "ur", "ug"].contains(&lang) && !s.known_ar.is_empty() {
        return s.known_ar.clone();
    }
    s.short.clone().unwrap_or_else(|| s.name.clone())
}

/// The scholar's name laid out like a narrator's (`scholars.name_view`): reading names in `reading`, the
/// interface in `lang`.
pub fn name_view(s: &Scholar, reading: &str, lang: &str) -> name_parts::View {
    let known = (!s.known_ar.is_empty()).then_some(s.known_ar.as_str());
    let mut view = name_parts::present(
        &name_parts::parse(&s.arabic, None, &[], None, known),
        reading,
        lang,
    );
    if view.reading.is_empty() {
        view.reading = short_name(s, lang);
    }
    if py::words(&view.full_reading).len() < 2 {
        view.full_reading = if lang == "tr" && !s.tr.is_empty() {
            s.tr.clone()
        } else {
            s.full.clone()
        };
    }
    view
}

fn column(conn: &Connection, sql: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    if let Ok(mut st) = conn.prepare(sql) {
        if let Ok(rows) = st.query_map([], |r| r.get::<_, Option<String>>(0)) {
            out.extend(rows.flatten().flatten());
        }
    }
    out
}

/// The roles the data of this database gives each scholar: id → ["compiler", "grader", "critic"] (`present`).
pub fn roles(conn: &Connection) -> HashMap<String, Vec<String>> {
    let books = column(conn, "SELECT key FROM collections");
    let graders = column(conn, "SELECT DISTINCT grader_name FROM grades");
    let critics = column(conn, "SELECT DISTINCT critic_name FROM verdicts");
    let mut out = HashMap::new();
    for s in all() {
        let mut r = Vec::new();
        if s.compiler_of.iter().any(|b| books.contains(b)) {
            r.push("compiler".to_string());
        }
        if s.grader_names.iter().any(|g| graders.contains(g)) {
            r.push("grader".to_string());
        }
        if s.critic_names.iter().any(|c| critics.contains(c)) {
            r.push("critic".to_string());
        }
        if !r.is_empty() {
            out.insert(s.id.clone(), r);
        }
    }
    out
}

/// Lower case, without accents or the marks of transliteration, Arabic normalized (`scholars._fold`).
fn fold(text: &str) -> String {
    normalize(text)
        .to_lowercase()
        .nfkd()
        .filter(|c| canonical_combining_class(*c) == 0)
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect()
}

/// A scholar found by Search: the register's entry, his roles here, found by a close spelling.
#[derive(Debug, Clone)]
pub struct Found {
    pub scholar: &'static Scholar,
    pub roles: Vec<String>,
    pub close: bool,
}

fn names_of(s: &Scholar) -> String {
    [
        s.name.as_str(),
        s.full.as_str(),
        s.tr.as_str(),
        s.arabic.as_str(),
        s.known_ar.as_str(),
    ]
    .join(" ")
}

/// The scholars whose names or works contain every word typed, those named first and those with something in this
/// database before the others (`scholars.search`).
pub fn search(conn: &Connection, text: &str) -> Vec<Found> {
    let words: Vec<String> = py::words(&fold(text))
        .into_iter()
        .map(|w| {
            let x = w.strip_prefix("al").unwrap_or(w);
            let x = x.strip_prefix("el").unwrap_or(x);
            if x.is_empty() { w } else { x }.to_string()
        })
        .filter(|w| !["b", "bn", "ibn", "bin", "ب", "بن"].contains(&w.as_str()))
        .collect();
    if words.is_empty() {
        return Vec::new();
    }
    let loose_words: Vec<String> = py::words(&fold_latin(text))
        .into_iter()
        .map(String::from)
        .collect();
    let fits = |hay: &str| -> bool {
        let exact = fold(hay).replace(' ', "");
        if words.iter().all(|w| exact.contains(w.as_str())) {
            return true;
        }
        if loose_words.is_empty() {
            return false;
        }
        let folded = fold_latin(&hay.replace('-', " ")).replace(' ', "");
        loose_words.iter().all(|w| folded.contains(w.as_str()))
    };
    let have = roles(conn);
    let mut found: Vec<(Found, bool)> = Vec::new();
    for s in all() {
        let names = names_of(s);
        if fits(&format!("{names} {}", s.works.join(" "))) {
            found.push((
                Found {
                    scholar: s,
                    roles: have.get(&s.id).cloned().unwrap_or_default(),
                    close: false,
                },
                fits(&names),
            ));
        }
    }
    if found.is_empty() {
        found = close_spellings(text, &have);
    }
    found.sort_by_key(|(f, in_name)| (!*in_name, f.roles.is_empty()));
    found.into_iter().map(|(f, _)| f).collect()
}

/// Nothing found as typed: every query word close to a word of his names or works (`scholars._close_spellings`).
fn close_spellings(text: &str, have: &HashMap<String, Vec<String>>) -> Vec<(Found, bool)> {
    let words = |hay: &str| -> Vec<String> {
        let latin = fold_latin(&hay.replace('-', " "));
        let folded = fold(hay);
        py::words(&latin)
            .into_iter()
            .chain(py::words(&folded))
            .filter(|w| !w.is_empty())
            .map(loose)
            .collect()
    };
    let fits = |q: &str, ws: &[String]| -> bool {
        let qn = q.chars().count();
        ws.iter().any(|w| {
            w.starts_with(q)
                || (qn >= 4 && (near(q, w) || near(q, &w.chars().take(qn).collect::<String>())))
        })
    };
    let pick = |t: String| -> Vec<String> {
        py::words(&t)
            .into_iter()
            .map(loose)
            .filter(|q| q.chars().count() >= 3)
            .collect()
    };
    let mut wanted = pick(fold_latin(text));
    if wanted.is_empty() {
        wanted = pick(fold(text));
    }
    if wanted.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for s in all() {
        let names = names_of(s);
        let name_words = words(&names);
        let all_words = words(&format!("{names} {}", s.works.join(" ")));
        if wanted.iter().all(|q| fits(q, &all_words)) {
            out.push((
                Found {
                    scholar: s,
                    roles: have.get(&s.id).cloned().unwrap_or_default(),
                    close: true,
                },
                wanted.iter().all(|q| fits(q, &name_words)),
            ));
        }
    }
    if out.iter().any(|(_, in_name)| *in_name) {
        out.retain(|(_, in_name)| *in_name);
    }
    out
}

const IBN: [&str; 7] = ["b", "bn", "ibn", "bin", "ب", "بن", "ابن"];

/// A name as one string of letters, without the article and without "ibn" (`scholars._squeeze`).
fn squeeze(text: &str) -> String {
    if text.chars().any(|c| ('\u{0600}'..='\u{06ff}').contains(&c)) {
        let n = normalize(text);
        return py::words(&n)
            .into_iter()
            .map(|w| {
                if w.chars().count() > 4 {
                    w.strip_prefix("ال").unwrap_or(w)
                } else {
                    w
                }
            })
            .filter(|w| !IBN.contains(w))
            .collect();
    }
    fold_latin(text).replace(' ', "")
}

/// The scholars KNOWN by the words typed: the name he is known by begins with them (`scholars.named`).
pub fn named(text: &str) -> Vec<&'static Scholar> {
    let wanted = squeeze(text);
    if wanted.chars().count() < 3 {
        return Vec::new();
    }
    let known = |s: &Scholar| -> Vec<String> {
        [s.name.as_str(), s.tr.as_str(), s.known_ar.as_str()]
            .into_iter()
            .filter(|n| !n.is_empty())
            .map(squeeze)
            .collect()
    };
    let exact: Vec<&Scholar> = all()
        .iter()
        .filter(|s| known(s).iter().any(|n| n.starts_with(&wanted)))
        .collect();
    if !exact.is_empty() {
        return exact;
    }
    let wanted = loose(&wanted);
    all()
        .iter()
        .filter(|s| known(s).iter().any(|n| loose(n).starts_with(&wanted)))
        .collect()
}

/// His entry as a narrator in his own tradition's rijal work (the Taqrib, or al-Najashi), when there is one.
pub fn as_narrator(conn: &Connection, s: &Scholar) -> Option<i64> {
    let prefix = s.taqrib.as_deref().or(s.najashi.as_deref())?;
    if prefix.is_empty() {
        return None;
    }
    conn.query_row(
        "SELECT id FROM persons WHERE name_ar LIKE ? AND COALESCE(tradition, 'sunni') = ? ORDER BY id LIMIT 1",
        rusqlite::params![format!("{prefix}%"), tradition_of(s)],
        |r| r.get(0),
    )
    .optional()
    .ok()
    .flatten()
}

/// "Works: …" and the role's title, in the interface language.
pub fn role_title(role: &str, lang: &str) -> String {
    i18n::t(lang, &role_help(role).0, &[])
}

/// The scholars with something in this database, each with the roles the data gives him: the Sunni ones first, the
/// Shia after them, each in the register's order (`scholars.present`).
pub fn present(conn: &Connection) -> Vec<(&'static Scholar, Vec<String>)> {
    let have = roles(conn);
    let mut out: Vec<(&'static Scholar, Vec<String>)> = all()
        .iter()
        .filter_map(|s| have.get(&s.id).map(|r| (s, r.clone())))
        .collect();
    out.sort_by_key(|(s, _)| tradition_of(s) == "shia");
    out
}

/// The grader names of one tradition's scholars, as the sources write them (`scholars.grader_names`).
pub fn grader_names(tradition: &str) -> HashSet<String> {
    all()
        .iter()
        .filter(|s| tradition_of(s) == tradition)
        .flat_map(|s| s.grader_names.iter().cloned())
        .collect()
}

/// A narrator of a compiler's chains and how often (his teachers, the Companions they end with).
#[derive(Debug, Clone)]
pub struct Counted {
    pub id: i64,
    pub name: String,
    pub count: i64,
}

/// One of his books as the data measures it (`scholars.compiler_stats`).
#[derive(Debug, Clone)]
pub struct CompilerStats {
    pub key: String,
    pub book: String,
    pub hadith: i64,
    pub chains: i64,
    pub split: i64,
    pub reach: i64,
    pub links: i64,
    pub identified: i64,
    pub teachers: Vec<Counted>,
    pub companions: Vec<Counted>,
    /// road B: whom the chains reach — (key, label, count)
    pub ends: Vec<(String, String, i64)>,
    pub tradition: String,
}

fn counted(conn: &Connection, sql: &str, cid: i64) -> rusqlite::Result<Vec<Counted>> {
    let mut st = conn.prepare(sql)?;
    let rows = st.query_map([cid], |r| {
        Ok(Counted {
            id: r.get(0)?,
            name: crate::narrators::display_name(&r.get::<_, String>(1)?),
            count: r.get(2)?,
        })
    })?;
    rows.collect()
}

pub fn compiler_stats(conn: &Connection, s: &Scholar) -> rusqlite::Result<Vec<CompilerStats>> {
    crate::cache::kept(conn, &format!("scholar-compiler-{}", s.id), || {
        let mut out = vec![];
        for key in &s.compiler_of {
            let Some((cid, name)) = conn
                .query_row(
                    "SELECT id, name FROM collections WHERE key = ?",
                    [key],
                    |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)),
                )
                .optional()?
            else {
                continue;
            };
            let hadith: i64 = conn.query_row(
                "SELECT COUNT(*) FROM hadiths WHERE collection_id = ?",
                [cid],
                |r| r.get(0),
            )?;
            let (chains, split, reach): (i64, Option<i64>, Option<i64>) = conn.query_row(
                "SELECT COUNT(*), SUM(i.problem IS NULL), SUM(i.reaches_prophet) FROM isnads i \
                 JOIN hadiths h ON h.id = i.hadith_id WHERE h.collection_id = ? AND (i.problem IS NULL OR i.ordinal = 1)",
                [cid],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )?;
            let (links, identified): (i64, Option<i64>) = conn.query_row(
                "SELECT COUNT(*), SUM(l.person_id IS NOT NULL) FROM isnad_links l JOIN isnads i ON i.id = l.isnad_id \
                 JOIN hadiths h ON h.id = i.hadith_id WHERE h.collection_id = ? AND i.problem IS NULL",
                [cid],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            let teachers = counted(
                conn,
                "SELECT l.person_id, p.name_ar, COUNT(*) FROM isnad_links l JOIN isnads i ON i.id = l.isnad_id \
                 JOIN hadiths h ON h.id = i.hadith_id JOIN persons p ON p.id = l.person_id \
                 WHERE h.collection_id = ? AND l.position = 1 AND i.problem IS NULL \
                 GROUP BY l.person_id ORDER BY 3 DESC LIMIT 12",
                cid,
            )?;
            let mut companions = counted(
                conn,
                "SELECT l.person_id, p.name_ar, COUNT(*) FROM isnad_links l JOIN isnads i ON i.id = l.isnad_id \
                 JOIN hadiths h ON h.id = i.hadith_id JOIN persons p ON p.id = l.person_id \
                 WHERE h.collection_id = ? AND p.tabaqa = 1 AND i.problem IS NULL \
                   AND l.position = (SELECT MAX(position) FROM isnad_links x WHERE x.isnad_id = l.isnad_id) \
                 GROUP BY l.person_id ORDER BY 3 DESC LIMIT 12",
                cid,
            )?;
            let mut ends = vec![];
            if tradition_of(s) == "shia" {
                companions.clear();
                let mut st = conn.prepare(
                    "SELECT i.ends_with, COUNT(*) FROM isnads i JOIN hadiths h ON h.id = i.hadith_id \
                     WHERE h.collection_id = ? AND i.problem IS NULL AND i.ends_with IS NOT NULL \
                     GROUP BY 1 ORDER BY 2 DESC",
                )?;
                for r in
                    st.query_map([cid], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?
                {
                    let (k, n) = r?;
                    let label = crate::isnad::masum_label(&k);
                    ends.push((k.clone(), if label.is_empty() { k } else { label }, n));
                }
            }
            out.push(CompilerStats {
                key: key.clone(),
                book: name,
                hadith,
                chains,
                split: split.unwrap_or(0),
                reach: reach.unwrap_or(0),
                links,
                identified: identified.unwrap_or(0),
                teachers,
                companions,
                ends,
                tradition: tradition_of(s).to_string(),
            });
        }
        Ok(out)
    })
}

/// How he and another grader of his tradition judged the same hadith.
#[derive(Debug, Clone)]
pub struct Agreement {
    pub common: usize,
    pub agree: f64,
    pub within_one: f64,
    pub kappa: Option<f64>,
    pub lower: f64,
    pub higher: f64,
}

/// His grades as the data measures them (`scholars.grader_stats`).
#[derive(Debug, Clone)]
pub struct GraderStats {
    pub graded: usize,
    pub books: Vec<String>,
    pub tradition: String,
    /// (label in English, count), from the best group down
    pub distribution: Vec<(String, usize)>,
    pub wordings: Vec<(String, usize)>,
    /// (the other grader, how they agree), in the order the data first names them
    pub agreement: Vec<(String, Agreement)>,
    pub strictness: Option<f64>,
    pub compared: usize,
    pub lower: Option<f64>,
    pub higher: Option<f64>,
}

type Groups = Vec<(String, HashMap<i64, i32>)>;

/// grader name → {hadith: group} (the first grade a grader gives a hadith), for the graders of one tradition, in the
/// order the data first names them (`scholars._grade_groups`).
fn grade_groups(conn: &Connection, tradition: &str) -> rusqlite::Result<Groups> {
    crate::cache::kept(conn, &format!("grade-groups-{tradition}"), || {
        let shia = grader_names("shia");
        let mut order: Vec<String> = vec![];
        let mut out: HashMap<String, HashMap<i64, i32>> = HashMap::new();
        let mut seen: HashSet<(String, i64)> = HashSet::new();
        let mut st =
            conn.prepare("SELECT grader_name, hadith_id, grade FROM grades ORDER BY id")?;
        let rows = st.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, Option<String>>(2)?.unwrap_or_default(),
            ))
        })?;
        for row in rows {
            let (name, hid, grade) = row?;
            if shia.contains(&name) != (tradition == "shia") || seen.contains(&(name.clone(), hid))
            {
                continue;
            }
            seen.insert((name.clone(), hid));
            if tradition != "shia" && crate::grades::isnad_only(&grade) {
                continue;
            }
            let g = if tradition == "shia" {
                crate::grades::imami_group(&grade)
            } else {
                crate::grades::group(&grade)
            };
            if let Some(g) = g {
                if !out.contains_key(&name) {
                    order.push(name.clone());
                }
                out.entry(name).or_default().insert(hid, g);
            }
        }
        Ok(order
            .into_iter()
            .map(|n| {
                let m = out.remove(&n).unwrap_or_default();
                (n, m)
            })
            .collect())
    })
}

fn kappa(pairs: &[(i32, i32)]) -> Option<f64> {
    let n = pairs.len();
    if n == 0 {
        return None;
    }
    let observed = pairs.iter().filter(|(a, b)| a == b).count() as f64 / n as f64;
    let mut ca: HashMap<i32, usize> = HashMap::new();
    let mut cb: HashMap<i32, usize> = HashMap::new();
    for (a, b) in pairs {
        *ca.entry(*a).or_default() += 1;
        *cb.entry(*b).or_default() += 1;
    }
    let keys: HashSet<i32> = ca.keys().chain(cb.keys()).copied().collect();
    let expected = keys
        .iter()
        .map(|k| (ca.get(k).copied().unwrap_or(0) * cb.get(k).copied().unwrap_or(0)) as f64)
        .sum::<f64>()
        / (n * n) as f64;
    if expected == 1.0 {
        None
    } else {
        Some((observed - expected) / (1.0 - expected))
    }
}

pub fn grader_stats(conn: &Connection, s: &Scholar) -> rusqlite::Result<Option<GraderStats>> {
    crate::cache::kept(conn, &format!("scholar-grader-{}", s.id), || {
        let tradition = tradition_of(s).to_string();
        let labels: Vec<(i32, &str)> = if tradition == "shia" {
            vec![(4, "sahih"), (3, "hasan"), (2, "muwaththaq"), (1, "da'if")]
        } else {
            vec![
                (4, "sahih"),
                (3, "hasan"),
                (2, "da'if"),
                (1, "very weak"),
                (0, "fabricated"),
            ]
        };
        let groups = grade_groups(conn, &tradition)?;
        let names: Vec<String> = s
            .grader_names
            .iter()
            .filter(|n| groups.iter().any(|(g, _)| g == *n))
            .cloned()
            .collect();
        if names.is_empty() {
            return Ok(None);
        }
        let mut mine: HashMap<i64, i32> = HashMap::new();
        for n in &names {
            if let Some((_, m)) = groups.iter().find(|(g, _)| g == n) {
                mine.extend(m.iter().map(|(k, v)| (*k, *v)));
            }
        }
        let marks = vec!["?"; names.len()].join(",");
        let mut wordings: Vec<(String, usize)> = vec![];
        {
            let mut st = conn.prepare(&format!(
                "SELECT grade FROM grades WHERE grader_name IN ({marks})"
            ))?;
            let mut counts: HashMap<String, usize> = HashMap::new();
            let mut first: Vec<String> = vec![];
            for g in st.query_map(rusqlite::params_from_iter(names.iter()), |r| {
                r.get::<_, Option<String>>(0)
            })? {
                let g = g?.unwrap_or_default();
                if !counts.contains_key(&g) {
                    first.push(g.clone());
                }
                *counts.entry(g).or_default() += 1;
            }
            // Counter.most_common: by count, ties in the order first met
            let mut all: Vec<(String, usize)> = first
                .into_iter()
                .map(|g| {
                    let n = counts[&g];
                    (g, n)
                })
                .collect();
            all.sort_by_key(|b| std::cmp::Reverse(b.1));
            all.truncate(8);
            wordings = all;
        }
        let mut distribution = vec![];
        for (k, label) in &labels {
            distribution.push((
                label.to_string(),
                mine.values().filter(|v| **v == *k).count(),
            ));
        }
        let mut agreement = vec![];
        for (other, theirs) in &groups {
            if names.contains(other) {
                continue;
            }
            let pairs: Vec<(i32, i32)> = mine
                .iter()
                .filter_map(|(h, a)| theirs.get(h).map(|b| (*a, *b)))
                .collect();
            if pairs.is_empty() {
                continue;
            }
            let n = pairs.len() as f64;
            agreement.push((
                other.clone(),
                Agreement {
                    common: pairs.len(),
                    agree: pairs.iter().filter(|(a, b)| a == b).count() as f64 / n,
                    within_one: pairs.iter().filter(|(a, b)| (a - b).abs() <= 1).count() as f64 / n,
                    kappa: kappa(&pairs),
                    lower: pairs.iter().filter(|(a, b)| a < b).count() as f64 / n,
                    higher: pairs.iter().filter(|(a, b)| a > b).count() as f64 / n,
                },
            ));
        }
        let (mut lower, mut higher, mut compared) = (0usize, 0usize, 0usize);
        for (h, g) in &mine {
            for (o, theirs) in &groups {
                if names.contains(o) {
                    continue;
                }
                if let Some(b) = theirs.get(h) {
                    compared += 1;
                    lower += usize::from(g < b);
                    higher += usize::from(g > b);
                }
            }
        }
        let mut books: Vec<String> = vec![];
        {
            let mut st = conn.prepare(&format!(
                "SELECT DISTINCT c.name FROM grades g JOIN hadiths h ON h.id = g.hadith_id \
                 JOIN collections c ON c.id = h.collection_id WHERE g.grader_name IN ({marks})"
            ))?;
            for b in st.query_map(rusqlite::params_from_iter(names.iter()), |r| {
                r.get::<_, String>(0)
            })? {
                books.push(b?);
            }
        }
        let c = compared as f64;
        Ok(Some(GraderStats {
            graded: mine.len(),
            books,
            tradition,
            distribution,
            wordings,
            agreement,
            strictness: (compared > 0).then(|| (higher as f64 - lower as f64) / c),
            compared,
            lower: (compared > 0).then(|| lower as f64 / c),
            higher: (compared > 0).then(|| higher as f64 / c),
        }))
    })
}

/// The creeds al-Najashi and al-Tusi name, in English (`shia_rijal.MADHHAB_LABELS`).
pub fn madhhab_label(key: &str) -> &str {
    match key {
        "imami" => "Imami",
        "waqifi" => "Waqifi",
        "fathi" => "Fathi",
        "zaydi" => "Zaydi",
        "ammi" => "'Ammi (Sunni)",
        "nawusi" => "Nawusi",
        "kaysani" => "Kaysani",
        "ghali" => "Ghali (extremist)",
        _ => key,
    }
}

/// His verdicts on narrators (`scholars.critic_stats`): ranks as (rank, the English label in full — the interface translates it and cuts it at " (", count).
#[derive(Debug, Clone)]
pub struct CriticStats {
    pub verdicts: usize,
    pub works: Vec<String>,
    pub tradition: String,
    pub ranks: Vec<(i64, String, usize)>,
    /// road B: the creed where he states it — (English label, count), the most first
    pub creeds: Vec<(String, usize)>,
    pub unranked: usize,
}

pub fn critic_stats(conn: &Connection, s: &Scholar) -> rusqlite::Result<Option<CriticStats>> {
    crate::cache::kept(conn, &format!("scholar-critic-{}", s.id), || {
        if s.critic_names.is_empty() {
            return Ok(None);
        }
        let marks = vec!["?"; s.critic_names.len()].join(",");
        let mut st = conn.prepare(&format!(
            "SELECT rank, work, madhhab FROM verdicts WHERE critic_name IN ({marks})"
        ))?;
        let rows: Vec<(Option<i64>, Option<String>, Option<String>)> = st
            .query_map(rusqlite::params_from_iter(s.critic_names.iter()), |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            })?
            .collect::<rusqlite::Result<_>>()?;
        if rows.is_empty() {
            return Ok(None);
        }
        let count = |k: Option<i64>| rows.iter().filter(|r| r.0 == k).count();
        let mut works: Vec<String> = rows
            .iter()
            .filter_map(|r| r.1.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        works.sort();
        let t = crate::tables::get();
        if tradition_of(s) == "shia" {
            let mut keys: Vec<i64> = t
                .rijal
                .shia_ranks
                .keys()
                .filter_map(|k| k.parse().ok())
                .collect();
            keys.sort();
            let ranks = keys
                .iter()
                .map(|k| {
                    let label = t
                        .rijal
                        .shia_ranks
                        .get(&k.to_string())
                        .cloned()
                        .unwrap_or_default();
                    (*k, label, count(Some(*k)))
                })
                .collect();
            let mut first: Vec<String> = vec![];
            let mut counts: HashMap<String, usize> = HashMap::new();
            for r in &rows {
                if let Some(m) = r.2.as_ref().filter(|m| !m.is_empty()) {
                    if !counts.contains_key(m) {
                        first.push(m.clone());
                    }
                    *counts.entry(m.clone()).or_default() += 1;
                }
            }
            let mut creeds: Vec<(String, usize)> = first
                .into_iter()
                .map(|m| {
                    let n = counts[&m];
                    (madhhab_label(&m).to_string(), n)
                })
                .collect();
            creeds.sort_by_key(|b| std::cmp::Reverse(b.1));
            return Ok(Some(CriticStats {
                verdicts: rows.len(),
                works,
                tradition: "shia".into(),
                ranks,
                creeds,
                unranked: count(None),
            }));
        }
        let ranks = (1..=12)
            .map(|k: i64| {
                let label = t
                    .rijal
                    .ranks
                    .get(&k.to_string())
                    .cloned()
                    .unwrap_or_default();
                (k, label, count(Some(k)))
            })
            .collect();
        Ok(Some(CriticStats {
            verdicts: rows.len(),
            works,
            tradition: "sunni".into(),
            ranks,
            creeds: vec![],
            unranked: count(None),
        }))
    })
}

/// His written life, when isnady has one (`lives.of_scholar`).
pub fn written_life(scholar_id: &str) -> Option<&'static serde_json::Value> {
    crate::lives::written()
        .iter()
        .rev()
        .find(|l| l["scholar"].as_str() == Some(scholar_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_ones() {
        let ids = |t: &str| named(t).iter().map(|s| s.id.clone()).collect::<Vec<_>>();
        assert_eq!(ids("Bukhari"), vec!["bukhari"]);
        assert_eq!(ids("Buhârî"), vec!["bukhari"]);
        assert_eq!(ids("ابن حجر"), vec!["ibnhajar"]);
        assert!(ids("Anas").is_empty());
        assert_eq!(short_name(find("albani").unwrap(), "tr"), "Elbânî");
    }
}
