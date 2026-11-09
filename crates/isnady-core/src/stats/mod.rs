//! Statistics (ST7): the Python isnady's `core/stats_corpus.py`, `stats_graders.py`, `stats_models.py`,
//! `stats_lab.py` and `stats_words.py`, step for step — the same measures on the same data, the same bootstrap
//! samples (Python's Mersenne Twister, the same seed), the same sums, so the two programs print the same figures.
//! The results are kept where the Python isnady keeps them (data_dir()/derived/*.json, with the same fingerprint of
//! their inputs): either program reads what the other computed.
//!
//! Two roads (SH1): the Sunni books on Ibn Hajar's ranks and the five Sunni grade groups; the Shia books apart, on
//! al-Najashi's four Imami categories and the four Imami grades — never together, never on each other's scale.

pub mod corpus;
pub mod graders;
pub mod lab;
pub mod models;
pub mod num;
pub mod pyjson;
pub mod rng;
pub mod words;

use rusqlite::Connection;
pub use serde_json::Value;

use std::collections::HashMap;
use std::hash::Hash;
use std::path::PathBuf;

use crate::{books, grades, paths, works};

pub const SHIA_SCOPE: &str = "@shia";
pub const ALL_SCOPE: &str = "@all";

/// The grade groups of the Sunni road (`grades.GROUP_LABELS`) and the Imami categories (`grades.IMAMI_LABELS`).
pub fn group_label(k: i64) -> &'static str {
    match k {
        4 => "sahih",
        3 => "hasan",
        2 => "da'if",
        1 => "very weak",
        0 => "fabricated",
        _ => "",
    }
}

pub fn imami_label(k: i64) -> &'static str {
    match k {
        4 => "sahih",
        3 => "hasan",
        2 => "muwaththaq",
        1 => "da'if",
        _ => "",
    }
}

/// A counter that remembers the order its keys came in (Python's Counter / dict).
#[derive(Clone, Debug)]
pub struct Counter<K: Eq + Hash + Clone> {
    pub keys: Vec<K>,
    pub counts: Vec<i64>,
    index: HashMap<K, usize>,
}

impl<K: Eq + Hash + Clone> Default for Counter<K> {
    fn default() -> Self {
        Counter {
            keys: vec![],
            counts: vec![],
            index: HashMap::new(),
        }
    }
}

impl<K: Eq + Hash + Clone> Counter<K> {
    pub fn add(&mut self, k: K, n: i64) {
        if let Some(&i) = self.index.get(&k) {
            self.counts[i] += n;
        } else {
            self.index.insert(k.clone(), self.keys.len());
            self.keys.push(k);
            self.counts.push(n);
        }
    }
    pub fn get(&self, k: &K) -> i64 {
        self.index.get(k).map(|&i| self.counts[i]).unwrap_or(0)
    }
    pub fn contains(&self, k: &K) -> bool {
        self.index.contains_key(k)
    }
    pub fn len(&self) -> usize {
        self.keys.len()
    }
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }
    pub fn items(&self) -> impl Iterator<Item = (&K, i64)> {
        self.keys.iter().zip(self.counts.iter().copied())
    }
    /// `most_common(n)`: the largest counts first, equal ones in the order they came.
    pub fn most_common(&self, n: usize) -> Vec<(K, i64)> {
        let mut idx: Vec<usize> = (0..self.keys.len()).collect();
        idx.sort_by_key(|&i| -self.counts[i]);
        idx.into_iter()
            .take(n)
            .map(|i| (self.keys[i].clone(), self.counts[i]))
            .collect()
    }
}

/// One grader's grades of one book, in the order they were read (Python's dict).
#[derive(Clone, Debug, Default)]
pub struct Grades {
    pub name: String,
    pub order: Vec<i64>,
    pub map: HashMap<i64, i64>,
}

impl Grades {
    pub fn get(&self, h: i64) -> Option<i64> {
        self.map.get(&h).copied()
    }
    pub fn len(&self) -> usize {
        self.order.len()
    }
    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }
    pub fn items(&self) -> impl Iterator<Item = (i64, i64)> + '_ {
        self.order.iter().map(|h| (*h, self.map[h]))
    }
    fn insert(&mut self, h: i64, g: i64) {
        if self.map.insert(h, g).is_none() {
            self.order.push(h);
        }
    }
}

pub type Data = Vec<Grades>;

fn grader_entry<'a>(
    out: &'a mut Data,
    at: &mut HashMap<String, usize>,
    name: &str,
) -> &'a mut Grades {
    let i = *at.entry(name.to_string()).or_insert_with(|| {
        out.push(Grades {
            name: name.to_string(),
            ..Default::default()
        });
        out.len() - 1
    });
    &mut out[i]
}

const GRADES_OF_BOOK: &str =
    "SELECT g.grader_name, g.hadith_id, g.grade FROM grades g JOIN hadiths h ON h.id = g.hadith_id
     JOIN collections c ON c.id = h.collection_id WHERE c.key = ? ORDER BY g.id";

/// grader → {hadith: grade group} of one Sunni book, the first grade a grader gives a hadith; grades of the chain only
/// left out and counted per grader (`stats_graders.load`).
pub fn load_sunni(
    conn: &Connection,
    key: &str,
    mut chain_only: Option<&mut Counter<String>>,
) -> rusqlite::Result<Data> {
    let mut out: Data = vec![];
    let mut at = HashMap::new();
    let mut seen = std::collections::HashSet::new();
    let mut stmt = conn.prepare(GRADES_OF_BOOK)?;
    let mut rows = stmt.query([key])?;
    while let Some(r) = rows.next()? {
        let (name, hid, grade): (String, i64, String) = (r.get(0)?, r.get(1)?, r.get(2)?);
        if !seen.insert((name.clone(), hid)) {
            continue;
        }
        if grades::isnad_only(&grade) {
            if let Some(c) = chain_only.as_deref_mut() {
                c.add(name, 1);
            }
            continue;
        }
        if let Some(k) = grades::group(&grade) {
            grader_entry(&mut out, &mut at, &name).insert(hid, k as i64);
        }
    }
    Ok(out)
}

/// grader → {hadith: Imami category} of one Shia book (`stats_graders.load_imami`).
pub fn load_imami(conn: &Connection, key: &str) -> rusqlite::Result<Data> {
    let mut out: Data = vec![];
    let mut at = HashMap::new();
    let mut stmt = conn.prepare(GRADES_OF_BOOK)?;
    let mut rows = stmt.query([key])?;
    while let Some(r) = rows.next()? {
        let (name, hid, grade): (String, i64, String) = (r.get(0)?, r.get(1)?, r.get(2)?);
        let e = grader_entry(&mut out, &mut at, &name);
        if e.map.contains_key(&hid) {
            continue;
        }
        if let Some(k) = grades::imami_group(&grade) {
            e.insert(hid, k as i64);
        }
    }
    out.retain(|g| !g.is_empty());
    Ok(out)
}

pub struct RankGroup {
    pub name: &'static str,
    pub desc: &'static str,
    pub ranks: &'static [i64],
}

const RANK_GROUPS: [RankGroup; 6] = [
    RankGroup {
        name: "Companion",
        desc: "Companions (rank 1): their uprightness is not examined",
        ranks: &[1],
    },
    RankGroup {
        name: "Trustworthy",
        desc: "thiqa and above (ranks 2–3)",
        ranks: &[2, 3],
    },
    RankGroup {
        name: "Truthful",
        desc: "saduq, with or without lapses (ranks 4–5)",
        ranks: &[4, 5],
    },
    RankGroup {
        name: "Acceptable",
        desc: "maqbul — acceptable when followed, weak otherwise (rank 6)",
        ranks: &[6],
    },
    RankGroup {
        name: "Weak / unknown",
        desc: "mastur, da'if, majhul (ranks 7–9)",
        ranks: &[7, 8, 9],
    },
    RankGroup {
        name: "Rejected",
        desc: "abandoned, accused of lying, liar (ranks 10–12)",
        ranks: &[10, 11, 12],
    },
];

const RANK_GROUPS_SHIA: [RankGroup; 4] = [
    RankGroup {
        name: "Trustworthy",
        desc: "thiqa — an Imami the critic calls trustworthy",
        ranks: &[1],
    },
    RankGroup {
        name: "Praised",
        desc: "mamduh — praised (jalil, wajh, 'ayn …) without the word thiqa",
        ranks: &[2],
    },
    RankGroup {
        name: "Muwaththaq",
        desc: "a thiqa of another school (Waqifi, Fathi, 'ammi …)",
        ranks: &[3],
    },
    RankGroup {
        name: "Weak",
        desc: "da'if — weak, confused, accused, extremist",
        ranks: &[4],
    },
];

/// Everything that differs between the two roads (`stats_corpus.Road`).
pub struct Road {
    pub tradition: &'static str,
    pub shia: bool,
    pub rank_groups: &'static [RankGroup],
    pub weak_from: i64,
    pub grade_order: Vec<i64>,
    pub grade_labels: Vec<String>,
    pub levels: Vec<String>,
    pub reference: &'static str,
    pub critic: &'static str,
}

impl Road {
    pub fn new(tradition: &str) -> Road {
        let shia = tradition == "shia";
        let rank_groups: &'static [RankGroup] = if shia {
            &RANK_GROUPS_SHIA
        } else {
            &RANK_GROUPS
        };
        let grade_order: Vec<i64> = if shia {
            vec![4, 3, 2, 1]
        } else {
            vec![4, 3, 2, 1, 0]
        };
        let grade_labels = grade_order
            .iter()
            .map(|&g| if shia { imami_label(g) } else { group_label(g) }.to_string())
            .collect();
        let mut levels: Vec<String> = rank_groups
            .iter()
            .enumerate()
            .map(|(i, g)| {
                if !shia && i == 0 {
                    "Companion only".to_string()
                } else {
                    g.name.to_string()
                }
            })
            .collect();
        levels.push("None ranked".into());
        Road {
            tradition: if shia { "shia" } else { "sunni" },
            shia,
            rank_groups,
            weak_from: if shia { 4 } else { 7 },
            grade_order,
            grade_labels,
            levels,
            reference: "Trustworthy",
            critic: if shia { "al-Najashi" } else { "Ibn Hajar" },
        }
    }

    /// The group of a rank (`group_of`), None when the rank is not one of this road's.
    pub fn group_of(&self, rank: Option<i64>) -> Option<usize> {
        let r = rank?;
        self.rank_groups.iter().position(|g| g.ranks.contains(&r))
    }

    pub fn grade_name(&self, g: i64) -> &'static str {
        if self.shia {
            imami_label(g)
        } else {
            group_label(g)
        }
    }

    pub fn grade_index(&self, g: i64) -> Option<usize> {
        self.grade_order.iter().position(|x| *x == g)
    }

    pub fn load(&self, conn: &Connection, key: &str) -> rusqlite::Result<Data> {
        if self.shia {
            load_imami(conn, key)
        } else {
            load_sunni(conn, key, None)
        }
    }
}

/// "sunni" for all books or a Sunni book; "shia" for "@shia" or a Shia book.
pub fn tradition_of_scope(conn: &Connection, collection: Option<&str>) -> &'static str {
    match collection {
        Some(SHIA_SCOPE) => "shia",
        None | Some("") => "sunni",
        Some(key) => {
            let t: Option<String> = conn
                .query_row(
                    "SELECT tradition FROM collections WHERE key = ?",
                    [key],
                    |r| r.get(0),
                )
                .ok()
                .flatten();
            if t.as_deref() == Some("shia") {
                "shia"
            } else {
                "sunni"
            }
        }
    }
}

pub fn road_of(conn: &Connection, collection: Option<&str>) -> Road {
    Road::new(tradition_of_scope(conn, collection))
}

/// (id, key, English title) of the books measured: one; every active Sunni book; or every active Shia book.
pub fn books_of(
    conn: &Connection,
    collection: Option<&str>,
) -> rusqlite::Result<Vec<(i64, String, String)>> {
    if let Some(key) = collection.filter(|c| !c.is_empty() && *c != SHIA_SCOPE) {
        let row: Option<(i64, String, String)> = conn
            .query_row(
                "SELECT id, key, name FROM collections WHERE key = ?",
                [key],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .ok();
        return Ok(row
            .map(|(id, k, n)| vec![(id, k.clone(), works::book_title(&k, &n, "en"))])
            .unwrap_or_default());
    }
    let shia = collection == Some(SHIA_SCOPE);
    let mut ids = HashMap::new();
    {
        let mut stmt = conn.prepare("SELECT key, id FROM collections")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
        for r in rows {
            let (k, id) = r?;
            ids.insert(k, id);
        }
    }
    Ok(books::all_books(conn, false)?
        .into_iter()
        .filter(|b| (b.tradition == "shia") == shia)
        .map(|b| {
            let t = works::book_title(&b.key, &b.name, "en");
            (ids[&b.key], b.key, t)
        })
        .collect())
}

#[derive(Clone, Debug)]
pub struct Person {
    pub name: Option<String>,
    pub tabaqa: Option<i64>,
    pub death: Option<i64>,
    pub rank: Option<i64>,
}

/// Every narrator of one tradition with his critic's first rank (`stats_corpus._persons`).
pub fn persons(conn: &Connection, tradition: &str) -> rusqlite::Result<HashMap<i64, Person>> {
    let mut stmt = conn.prepare(
        "SELECT p.id, p.name_ar, p.tabaqa, p.death_year_ah,
                (SELECT v.rank FROM verdicts v WHERE v.person_id = p.id ORDER BY v.id LIMIT 1)
         FROM persons p WHERE COALESCE(p.tradition, 'sunni') = ?",
    )?;
    let rows = stmt.query_map([tradition], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            Person {
                name: r.get(1)?,
                tabaqa: r.get(2)?,
                death: r.get(3)?,
                rank: r.get(4)?,
            },
        ))
    })?;
    rows.collect()
}

#[derive(Clone, Debug)]
pub struct Chain {
    pub hadith: i64,
    pub book: i64,
    pub reaches: bool,
    pub links: Vec<Option<i64>>,
}

/// Every chain of the books with its links in order; a chain not split into names has none (`stats_corpus._chains`).
pub fn chains(conn: &Connection, book_ids: &[i64]) -> rusqlite::Result<Vec<Chain>> {
    if book_ids.is_empty() {
        return Ok(vec![]);
    }
    let marks = vec!["?"; book_ids.len()].join(",");
    let sql = format!(
        "SELECT i.id, i.hadith_id, h.collection_id, i.reaches_prophet, l.position, l.person_id
         FROM isnads i JOIN hadiths h ON h.id = i.hadith_id
         LEFT JOIN isnad_links l ON l.isnad_id = i.id AND i.problem IS NULL
         WHERE h.collection_id IN ({marks}) AND (i.problem IS NULL OR i.ordinal = 1)
         ORDER BY i.id, l.position"
    );
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query(rusqlite::params_from_iter(book_ids))?;
    let mut out: Vec<Chain> = vec![];
    let mut last: Option<i64> = None;
    while let Some(r) = rows.next()? {
        let id: i64 = r.get(0)?;
        if last != Some(id) {
            let reaches: Option<i64> = r.get(3)?;
            out.push(Chain {
                hadith: r.get(1)?,
                book: r.get(2)?,
                reaches: reaches.unwrap_or(0) != 0,
                links: vec![],
            });
            last = Some(id);
        }
        let position: Option<i64> = r.get(4)?;
        if position.is_some() {
            let person: Option<i64> = r.get(5)?;
            out.last_mut().expect("chain").links.push(person);
        }
    }
    Ok(out)
}

/// The fingerprint of what the corpus statistics are computed from (`stats_corpus.fingerprint`, the same text).
pub fn fingerprint(conn: &Connection, collection: Option<&str>) -> rusqlite::Result<String> {
    use rusqlite::types::Value as V;
    let one = |sql: &str| -> rusqlite::Result<String> {
        let row: Vec<V> = conn.query_row(sql, [], |r| {
            (0..r.as_ref().column_count())
                .map(|i| r.get::<_, V>(i))
                .collect()
        })?;
        let parts: Vec<String> = row
            .iter()
            .map(|v| match v {
                V::Null => "null".to_string(),
                V::Integer(i) => i.to_string(),
                V::Real(f) => crate::py::float_repr(*f),
                V::Text(t) => serde_json::to_string(t).unwrap_or_default(),
                V::Blob(_) => "null".to_string(),
            })
            .collect();
        Ok(format!("[{}]", parts.join(", ")))
    };
    let off: String = if collection.is_none() {
        let mut d: Vec<String> = books::disabled().into_iter().collect();
        d.sort();
        format!(
            "[{}]",
            d.iter()
                .map(|k| ascii_json(k))
                .collect::<Vec<_>>()
                .join(", ")
        )
    } else {
        "[]".into()
    };
    Ok(format!(
        "[\"isnady-corpus-3\", {}, {off}, {}, {}, {}, {}, {}]",
        ascii_json(collection.unwrap_or("*")),
        one("SELECT COUNT(*), COALESCE(MAX(id), 0) FROM hadiths")?,
        one("SELECT COUNT(*), SUM(person_id IS NOT NULL), SUM(COALESCE(person_id, 0)) FROM isnad_links")?,
        one("SELECT COUNT(*), COALESCE(MAX(id), 0) FROM verdicts")?,
        one("SELECT COUNT(*), COALESCE(MAX(id), 0) FROM grades")?,
        one("SELECT COUNT(*), COALESCE(MAX(id), 0), COALESCE(SUM(death_year_ah), 0) FROM persons")?,
    ))
}

/// A string as `json.dumps` writes it with its default ensure_ascii.
fn ascii_json(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 || (c as u32) > 0x7f => {
                let mut buf = [0u16; 2];
                for u in c.encode_utf16(&mut buf) {
                    out.push_str(&format!("\\u{:04x}", u));
                }
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The file a result is kept in (`stats_corpus.cache_name`): one per book, one per set of active books.
pub fn cache_name(prefix: &str, collection: Option<&str>) -> String {
    let mut off: Vec<String> = books::disabled().into_iter().collect();
    off.sort();
    let all = matches!(collection, None | Some(SHIA_SCOPE));
    let tail = if !all || off.is_empty() {
        String::new()
    } else {
        format!("-{}", &format!("{:x}", md5::compute(off.join(",")))[..8])
    };
    let scope = match collection {
        None => "all_sunni_books".to_string(),
        Some(SHIA_SCOPE) => "all_shia_books".to_string(),
        Some(k) => works::full_name(k),
    };
    format!("{prefix}-{scope}{tail}.json")
}

pub fn derived() -> PathBuf {
    paths::data_dir().join("derived")
}

/// A saved result whose fingerprint is this one (`cached`): its path written in "from_file".
pub fn saved(path: &std::path::Path, stamp: &str) -> Option<Value> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut v = pyjson::loads(&text)?;
    if v.get("fingerprint").and_then(Value::as_str) != Some(stamp) {
        return None;
    }
    v["from_file"] = Value::String(path.display().to_string());
    Some(v)
}

/// The result written for the next time (and for the Python isnady), then marked as just computed.
pub fn save(path: &std::path::Path, mut result: Value, stamp: &str) -> Value {
    result["fingerprint"] = Value::String(stamp.to_string());
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, pyjson::dumps(&result));
    result["from_file"] = Value::Null;
    result
}

/// The statistics are not saved for this data (`stats_jobs.NotSaved`).
#[derive(Debug)]
pub struct NotSaved;

/// A float for the results: Python's json writes a non-finite one as NaN or Infinity; here null.
pub fn f(x: f64) -> Value {
    serde_json::Number::from_f64(x)
        .map(Value::Number)
        .unwrap_or(Value::Null)
}

pub fn fo(x: Option<f64>) -> Value {
    x.map(f).unwrap_or(Value::Null)
}

/// What Python's `str()` writes for a value in a CSV cell (None: nothing).
pub fn csv_cell(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::Bool(b) => if *b { "True" } else { "False" }.into(),
        Value::Number(x) => match x.as_i64() {
            Some(k) => k.to_string(),
            None => crate::py::float_repr(x.as_f64().unwrap_or(0.0)),
        },
        Value::String(t) => t.clone(),
        other => other.to_string(),
    }
}

/// A CSV file as Python's csv module writes it, UTF-8 with BOM (spreadsheets read the Arabic right).
pub fn csv_text(header: &[Value], rows: &[Vec<Value>]) -> String {
    let quote = |t: String| -> String {
        if t.contains(',') || t.contains('"') || t.contains('\n') || t.contains('\r') {
            format!("\"{}\"", t.replace('"', "\"\""))
        } else {
            t
        }
    };
    let mut out = String::from("\u{feff}");
    for row in std::iter::once(header).chain(rows.iter().map(|r| r.as_slice())) {
        out.push_str(
            &row.iter()
                .map(|c| quote(csv_cell(c)))
                .collect::<Vec<_>>()
                .join(","),
        );
        out.push_str("\r\n");
    }
    out
}
