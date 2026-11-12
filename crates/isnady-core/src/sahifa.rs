//! Written sources (SHF1): the Python isnady's `core/sahifa.py` — the hadith that were written down early, and how
//! writing shows in the chains, each road on its own (SH1).
//!
//! 1. A sahifa that came down whole: Hammam b. Munabbih's, found in the imported Musnad as the runs under
//!    Ma'mar ← Hammam that go on with "And the Messenger of God ﷺ said", each hadith beside its parallels.
//! 2. Writings that live in later books ('Abdullah b. 'Amr's al-Sahifa al-Sadiqa, 'Ali's sahifa, the letters; on the
//!    Shia road the Book of 'Ali, al-Jami'a, Mushaf Fatima), found by their wordings.
//! 3. Writing in the chains: "he wrote to me", "I found in the book of …".
//!
//! The sources are written for isnady and read from the Python isnady (data/sahifa.json, by tools/gen_tables.py).

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use rusqlite::Connection;
use serde::Deserialize;

use crate::cache;
use crate::normalize::normalize;

pub type Text = HashMap<String, String>;

#[derive(Deserialize, Clone, Debug)]
pub struct Source {
    pub id: String,
    pub book: String,
    pub road: String,
    pub layer: i64,
    pub title: Text,
    pub lead: Text,
    pub how: Text,
    #[serde(default)]
    pub patterns: Vec<String>,
    #[serde(default)]
    pub all: Vec<String>,
    #[serde(default)]
    pub head: Vec<String>,
    #[serde(default)]
    pub chain: Vec<String>,
}

#[derive(Deserialize)]
struct Data {
    hammam_name: String,
    sources: Vec<Source>,
}

fn data() -> &'static Data {
    static DATA: OnceLock<Data> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../data/sahifa.json")).expect("data/sahifa.json")
    })
}

/// A text in a language, else the English (`sahifa.text`).
pub fn text(values: &Text, lang: &str) -> String {
    values
        .get(lang)
        .filter(|v| !v.is_empty())
        .or_else(|| values.get("en"))
        .cloned()
        .unwrap_or_default()
}

/// The sources of one road, or of both, in their order.
pub fn sources(road: Option<&str>) -> Vec<&'static Source> {
    data()
        .sources
        .iter()
        .filter(|s| road.map_or(true, |r| s.road == r))
        .collect()
}

pub fn find(source_id: &str) -> Option<&'static Source> {
    data().sources.iter().find(|s| s.id == source_id)
}

// ------------------------------------------------------------------ reading the books
struct Marks {
    prophet: String,
    from_many: String,
    go_on: [String; 2],
}

fn marks() -> &'static Marks {
    static M: OnceLock<Marks> = OnceLock::new();
    M.get_or_init(|| Marks {
        prophet: normalize("صلى الله عليه وسلم"),
        from_many: normalize("فذكر أحاديث منها"),
        go_on: [normalize("وقال"), normalize("وبإسناده")],
    })
}

/// The words after the chain, as near as the text lets them be found (normalised text).
fn matn(n: &str) -> &str {
    let m = marks();
    let mut n = n;
    if let Some(k) = n.find(&m.from_many) {
        n = &n[k + m.from_many.len()..];
    }
    match n.find(&m.prophet) {
        Some(k) => &n[k + m.prophet.len()..],
        None => n,
    }
}

/// `re.findall(r"[ء-ي]+", …)` longer than two letters.
fn words(n: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    let mut cur = String::new();
    let mut count = 0;
    for c in matn(n).chars().chain(std::iter::once(' ')) {
        if ('\u{0621}'..='\u{064A}').contains(&c) {
            cur.push(c);
            count += 1;
        } else {
            if count > 2 {
                out.insert(std::mem::take(&mut cur));
            } else {
                cur.clear();
            }
            count = 0;
        }
    }
    out
}

fn starts_on(head: &str) -> bool {
    marks().go_on.iter().any(|g| head.starts_with(g.as_str()))
}

fn head(t: &str, n: usize) -> String {
    normalize(&t.chars().take(n).collect::<String>())
}

/// (hadith id, book key, number, road, Arabic text), in the order of the books (`sahifa._arabic`).
type Row = (i64, String, String, String, String);

fn arabic(conn: &Connection, filter: &str, args: &[i64]) -> rusqlite::Result<Vec<Row>> {
    let sql = format!(
        "SELECT h.id, c.key, h.number, COALESCE(c.tradition, 'sunni'), t.text FROM texts t \
         JOIN editions e ON e.id = t.edition_id JOIN hadiths h ON h.id = t.hadith_id \
         JOIN collections c ON c.id = h.collection_id \
         WHERE e.language = 'Arabic' {filter} ORDER BY c.id, h.number_sort"
    );
    let mut st = conn.prepare(&sql)?;
    let rows = st.query_map(rusqlite::params_from_iter(args.iter()), |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            number(r, 2)?,
            r.get::<_, String>(3)?,
            r.get::<_, Option<String>>(4)?.unwrap_or_default(),
        ))
    })?;
    rows.collect()
}

use crate::db::number_text as number;

/// Hammam's sahifa in the Musnad and each hadith's parallels elsewhere.
#[derive(Clone, Default, Debug)]
pub struct Hammam {
    pub items: Vec<i64>,
    /// hadith of the sahifa → the hadith of the other books matched to it, in the order they were found
    pub parallels: Vec<(i64, Vec<i64>)>,
}

impl Hammam {
    pub fn parallels_of(&self, hadith_id: i64) -> &[i64] {
        self.parallels
            .iter()
            .find(|(k, _)| *k == hadith_id)
            .map(|(_, v)| v.as_slice())
            .unwrap_or(&[])
    }
}

fn hammam(conn: &Connection) -> rusqlite::Result<Hammam> {
    let rows = arabic(conn, "AND c.key = 'ahmad'", &[])?;
    let (hm, mm) = (normalize("همام"), normalize("معمر"));
    let mut items: Vec<i64> = Vec::new();
    let mut wordsof: HashMap<i64, HashSet<String>> = HashMap::new();
    let mut i = 0;
    while i < rows.len() {
        let h = head(&rows[i].4, 220);
        let next = if i + 1 < rows.len() {
            head(&rows[i + 1].4, 40)
        } else {
            String::new()
        };
        if h.contains(&hm) && h.contains(&mm) && starts_on(&next) {
            let mut j = i + 1;
            while j < rows.len() && starts_on(&head(&rows[j].4, 40)) {
                j += 1;
            }
            for r in &rows[i..j] {
                items.push(r.0);
                wordsof.insert(r.0, words(&normalize(&r.4)));
            }
            i = j;
        } else {
            i += 1;
        }
    }
    let mut parallels: Vec<(i64, Vec<i64>)> = Vec::new();
    let person: Option<i64> = conn
        .query_row(
            "SELECT id FROM persons WHERE name_ar LIKE ? ORDER BY id LIMIT 1",
            [format!("{}%", data().hammam_name)],
            |r| r.get(0),
        )
        .ok();
    if let (false, Some(person)) = (items.is_empty(), person) {
        let mut st = conn.prepare(
            "SELECT DISTINCT i.hadith_id FROM isnad_links l JOIN isnads i ON i.id = l.isnad_id \
             JOIN hadiths h ON h.id = i.hadith_id JOIN collections c ON c.id = h.collection_id \
             WHERE l.person_id = ? AND c.key != 'ahmad' AND COALESCE(c.tradition, 'sunni') = 'sunni'",
        )?;
        let others: Vec<i64> = st
            .query_map([person], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        if !others.is_empty() {
            let marks = vec!["?"; others.len()].join(",");
            for (hid, _key, _num, _road, t) in
                arabic(conn, &format!("AND h.id IN ({marks})"), &others)?
            {
                let w = words(&normalize(&t));
                let mut best = 0.0_f64;
                let mut at: Option<i64> = None;
                for item in &items {
                    let s = &wordsof[item];
                    let inter = w.intersection(s).count();
                    let score = if inter >= 4 {
                        inter as f64 / (w.len().min(s.len()).max(1)) as f64
                    } else {
                        0.0
                    };
                    if score > best {
                        best = score;
                        at = Some(*item);
                    }
                }
                if let (Some(at), true) = (at, best >= 0.6) {
                    match parallels.iter_mut().find(|(k, _)| *k == at) {
                        Some((_, v)) => v.push(hid),
                        None => parallels.push((at, vec![hid])),
                    }
                }
            }
        }
    }
    Ok(Hammam { items, parallels })
}

/// Everything the page shows, read once per change of the database.
#[derive(Clone, Default, Debug)]
pub struct Counted {
    pub hammam: Hammam,
    /// every source but Hammam's → its hadith, in the order of the books
    pub found: HashMap<String, Vec<i64>>,
}

fn count_all(conn: &Connection) -> rusqlite::Result<Counted> {
    let hammam = hammam(conn)?;
    let mut found: HashMap<String, Vec<i64>> = data()
        .sources
        .iter()
        .filter(|s| s.id != "hammam")
        .map(|s| (s.id.clone(), Vec::new()))
        .collect();
    enum How {
        Any,
        All,
        Head,
    }
    let mut plans: Vec<(&Source, How, Vec<String>)> = Vec::new();
    for s in &data().sources {
        if !s.patterns.is_empty() {
            plans.push((
                s,
                How::Any,
                s.patterns.iter().map(|p| normalize(p)).collect(),
            ));
        } else if !s.all.is_empty() {
            plans.push((s, How::All, s.all.iter().map(|p| normalize(p)).collect()));
        } else if !s.head.is_empty() {
            plans.push((s, How::Head, s.head.iter().map(|p| normalize(p)).collect()));
        }
    }
    for (hid, _key, _num, road, t) in arabic(conn, "", &[])? {
        let n = normalize(&t);
        let mut short: Option<String> = None;
        for (s, how, pats) in &plans {
            if s.road != road {
                continue;
            }
            let hit = match how {
                How::All => pats.iter().all(|p| n.contains(p.as_str())),
                How::Any => pats.iter().any(|p| n.contains(p.as_str())),
                How::Head => {
                    let hay = short.get_or_insert_with(|| n.chars().take(400).collect());
                    pats.iter().any(|p| hay.contains(p.as_str()))
                }
            };
            if hit {
                found.get_mut(&s.id).expect("source").push(hid);
            }
        }
    }
    for s in &data().sources {
        if s.chain.is_empty() {
            continue;
        }
        let pats: Vec<String> = s.chain.iter().map(|p| normalize(p)).collect();
        let mut st = conn.prepare(
            "SELECT i.hadith_id, i.raw_text FROM isnads i JOIN hadiths h ON h.id = i.hadith_id \
             JOIN collections c ON c.id = h.collection_id WHERE COALESCE(c.tradition, 'sunni') = ? \
             ORDER BY c.id, h.number_sort",
        )?;
        let mut seen: Vec<i64> = Vec::new();
        let mut have: HashSet<i64> = HashSet::new();
        let mut rows = st.query([&s.road])?;
        while let Some(r) = rows.next()? {
            let hid: i64 = r.get(0)?;
            let n = normalize(&r.get::<_, Option<String>>(1)?.unwrap_or_default());
            if pats.iter().any(|p| n.contains(p.as_str())) && have.insert(hid) {
                seen.push(hid);
            }
        }
        found.insert(s.id.clone(), seen);
    }
    Ok(Counted { hammam, found })
}

/// Everything this page shows, read once per change of the database (`sahifa.counted`, `cache`).
pub fn counted(conn: &Connection) -> rusqlite::Result<Counted> {
    cache::kept(conn, "sahifa", || count_all(conn))
}

/// Whether the count is ready for the database as it is now (`sahifa.is_counted`).
pub fn is_counted(conn: &Connection) -> bool {
    cache::is_kept(conn, "sahifa")
}

/// The hadith ids of a written source, in the order of the books.
pub fn hadith(c: &Counted, source_id: &str) -> Vec<i64> {
    if source_id == "hammam" {
        return c.hammam.items.clone();
    }
    c.found.get(source_id).cloned().unwrap_or_default()
}

/// The hadith of the other books matched to one hadith of Hammam's sahifa.
pub fn parallels(c: &Counted, hadith_id: i64) -> Vec<i64> {
    c.hammam.parallels_of(hadith_id).to_vec()
}

/// (book key, book name, how many) of these hadith, in the order of the books.
pub fn by_book(conn: &Connection, ids: &[i64]) -> rusqlite::Result<Vec<(String, String, i64)>> {
    let mut out: Vec<(String, String, i64)> = Vec::new();
    for part in ids.chunks(900) {
        let sql = format!(
            "SELECT c.key, c.name, COUNT(*) FROM hadiths h JOIN collections c ON c.id = h.collection_id \
             WHERE h.id IN ({}) GROUP BY c.id ORDER BY c.id",
            vec!["?"; part.len()].join(",")
        );
        let mut st = conn.prepare(&sql)?;
        let mut rows = st.query(rusqlite::params_from_iter(part.iter()))?;
        while let Some(r) = rows.next()? {
            let key: String = r.get(0)?;
            let name: String = r.get::<_, Option<String>>(1)?.unwrap_or_default();
            let n: i64 = r.get(2)?;
            match out.iter_mut().find(|x| x.0 == key) {
                Some(x) => x.2 += n,
                None => out.push((key, name, n)),
            }
        }
    }
    Ok(out)
}

/// (hadith id, book key, number) of these hadith, in their order.
pub fn refs(conn: &Connection, ids: &[i64]) -> rusqlite::Result<Vec<(i64, String, String)>> {
    let mut found: HashMap<i64, (String, String)> = HashMap::new();
    for part in ids.chunks(900) {
        let sql = format!(
            "SELECT h.id, c.key, h.number FROM hadiths h JOIN collections c ON c.id = h.collection_id \
             WHERE h.id IN ({})",
            vec!["?"; part.len()].join(",")
        );
        let mut st = conn.prepare(&sql)?;
        let mut rows = st.query(rusqlite::params_from_iter(part.iter()))?;
        while let Some(r) = rows.next()? {
            found.insert(r.get(0)?, (r.get(1)?, number(r, 2)?));
        }
    }
    Ok(ids
        .iter()
        .filter_map(|i| found.get(i).map(|(k, n)| (*i, k.clone(), n.clone())))
        .collect())
}

/// One row of the summary: a source with how many hadith the imported books hold of it.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Summary {
    pub id: String,
    pub road: String,
    pub count: usize,
    /// Hammam's: how many of its hadith have parallels, and how many parallels
    pub with_parallels: usize,
    pub parallels: usize,
}

pub fn summary(c: &Counted) -> Vec<Summary> {
    data()
        .sources
        .iter()
        .map(|s| {
            let ids = hadith(c, &s.id);
            let (w, p) = if s.id == "hammam" {
                (
                    c.hammam.parallels.len(),
                    c.hammam.parallels.iter().map(|(_, v)| v.len()).sum(),
                )
            } else {
                (0, 0)
            };
            Summary {
                id: s.id.clone(),
                road: s.road.clone(),
                count: ids.len(),
                with_parallels: w,
                parallels: p,
            }
        })
        .collect()
}

/// A text in its languages as the Python isnady keeps them (en, tr, ar first).
fn text_value(t: &Text) -> serde_json::Value {
    let mut keys: Vec<&String> = t.keys().collect();
    keys.sort_by_key(|k| {
        (
            ["en", "tr", "ar"].iter().position(|x| x == k).unwrap_or(9),
            k.as_str(),
        )
    });
    let mut m = serde_json::Map::new();
    for k in keys {
        m.insert(k.clone(), serde_json::Value::String(t[k].clone()));
    }
    serde_json::Value::Object(m)
}

/// Every written source as a book for isnady-data (sahifa/<road>/<book>.json): its texts in three languages and its
/// hadith as [book, number] of the books they stand in — read from the database imported here (`sahifa.export`).
pub fn export(conn: &Connection) -> rusqlite::Result<Vec<serde_json::Value>> {
    use serde_json::json;
    let c = counted(conn)?;
    let mut out = vec![];
    for s in sources(None) {
        let ids = hadith(&c, &s.id);
        let rows = refs(conn, &ids)?;
        let mut item = json!({
            "schema": "isnady-sahifa/1", "id": s.book, "source": s.id, "road": s.road, "layer": s.layer,
            "title": text_value(&s.title), "lead": text_value(&s.lead), "how": text_value(&s.how),
            "hadith": rows.iter().map(|(_h, k, n)| json!([k, n])).collect::<Vec<_>>(),
        });
        if s.id == "hammam" {
            let mut parallels_of = serde_json::Map::new();
            for (hid, _key, num) in &rows {
                let p = parallels(&c, *hid);
                if !p.is_empty() {
                    let found = refs(conn, &p)?;
                    parallels_of.insert(
                        num.clone(),
                        json!(found
                            .iter()
                            .map(|(_x, k, n)| json!([k, n]))
                            .collect::<Vec<_>>()),
                    );
                }
            }
            item["parallels"] = serde_json::Value::Object(parallels_of);
        }
        out.push(item);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sources() {
        assert_eq!(sources(None).len(), 10);
        assert_eq!(sources(Some("shia")).len(), 4);
        let s = find("jamia").expect("jamia");
        assert_eq!(s.all.len(), 2);
        assert_eq!(text(&s.title, "tr"), "el-Câmia");
        let w = words(&normalize(
            "حدثنا صلى الله عليه وسلم قال إنما الأعمال بالنيات",
        ));
        assert!(w.contains(&normalize("الأعمال")) && !w.contains(&normalize("حدثنا")));
    }
}
