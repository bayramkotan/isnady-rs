//! The life of the Prophet ﷺ as a timeline (SIY1): the Python isnady's `core/sira.py`.
//!
//! The events of the sira in order, each with its date as the sira gives it (and where the reports differ, said so),
//! its place, the verses revealed about it, the chapter of Ibn Hisham's Sira that tells it (when imported) and the
//! hadith of the Sunni books that name it. The events are written for isnady (CC BY 4.0, like the lives) and read
//! from the Python isnady (data/sira.json, by tools/gen_tables.py).

use std::collections::HashMap;
use std::sync::OnceLock;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::cache;
use crate::normalize::normalize;

pub type Text = HashMap<String, String>;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Event {
    pub id: String,
    pub period: String,
    /// years after the Hijra; before it, negative
    pub year: i64,
    pub ce: String,
    pub when: Text,
    pub place: Text,
    pub title: Text,
    pub text: Text,
    pub quran: Vec<(u32, u32, u32)>,
    pub sira: String,
    pub phrase: String,
    pub differ: Option<Text>,
}

#[derive(Deserialize)]
struct Data {
    work: String,
    periods: Vec<(String, Text)>,
    events: Vec<Event>,
}

fn data() -> &'static Data {
    static DATA: OnceLock<Data> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../data/sira.json")).expect("data/sira.json")
    })
}

/// Ibn Hisham's Sira, the work whose chapters tell the events.
pub fn work() -> &'static str {
    &data().work
}

/// The periods in order: (key, its name in each language).
pub fn periods() -> &'static [(String, Text)] {
    &data().periods
}

pub fn period_name(key: &str, lang: &str) -> String {
    periods()
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| text(v, lang))
        .unwrap_or_default()
}

/// A text in a language, else the English (`sira.text`).
pub fn text(values: &Text, lang: &str) -> String {
    values
        .get(lang)
        .filter(|v| !v.is_empty())
        .or_else(|| values.get("en"))
        .cloned()
        .unwrap_or_default()
}

pub fn events(period: Option<&str>) -> Vec<&'static Event> {
    data()
        .events
        .iter()
        .filter(|e| period.map_or(true, |p| e.period == p))
        .collect()
}

pub fn all() -> &'static [Event] {
    &data().events
}

pub fn find(event_id: &str) -> Option<&'static Event> {
    data().events.iter().find(|e| e.id == event_id)
}

/// "2 AH · 624 CE"; before the Hijra: "13 years before the Hijra · 610 CE".
pub fn year_label(e: &Event, lang: &str) -> String {
    let y = e.year;
    let ah = if y > 0 {
        match lang {
            "tr" => format!("h. {y}"),
            "ar" => format!("{y} هـ"),
            _ => format!("{y} AH"),
        }
    } else {
        match lang {
            "tr" => format!("hicretten {} yıl önce", -y),
            "ar" => format!("قبل الهجرة بـ{} سنة", -y),
            _ if y == -1 => "1 year before the Hijra".to_string(),
            _ => format!("{} years before the Hijra", -y),
        }
    };
    let ce = match lang {
        "tr" => format!("m. {}", e.ce),
        "ar" => format!("{} م", e.ce),
        _ => format!("{} CE", e.ce),
    };
    format!("{ah} · {ce}")
}

/// His age at the event, in lunar years, from the Year of the Elephant (53 years before the Hijra).
pub fn age(e: &Event) -> i64 {
    e.year + 53 - if e.year <= 0 { 0 } else { 1 }
}

/// The chapter of Ibn Hisham's Sira that tells the event (when it is imported), by its heading.
pub fn sira_chapter(conn: &Connection, e: &Event) -> Option<i64> {
    if e.sira.is_empty() {
        return None;
    }
    let mut st = conn
        .prepare(
            "SELECT n.id, n.title FROM work_nodes n JOIN works w ON w.id = n.work_id \
             WHERE w.key = ? AND n.kind IN ('kitab', 'bab') ORDER BY n.id",
        )
        .ok()?;
    let want = normalize(&e.sira);
    let rows = st
        .query_map([work()], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, Option<String>>(1)?))
        })
        .ok()?;
    for row in rows.flatten() {
        if normalize(row.1.as_deref().unwrap_or("")).contains(&want) {
            return Some(row.0);
        }
    }
    None
}

/// How many hadith of the imported Sunni books hold each event's phrase — counted once per change (`cache`).
pub fn hadith_counts(conn: &Connection) -> rusqlite::Result<HashMap<String, i64>> {
    cache::kept(conn, "sira-events", || {
        let phrases: Vec<(String, String)> = all()
            .iter()
            .filter(|e| !e.phrase.is_empty())
            .map(|e| (e.id.clone(), normalize(&e.phrase)))
            .collect();
        let mut found: HashMap<String, i64> = phrases.iter().map(|(k, _)| (k.clone(), 0)).collect();
        let mut st = conn.prepare(
            "SELECT t.text FROM texts t JOIN editions e ON e.id = t.edition_id JOIN hadiths h ON h.id = t.hadith_id \
             JOIN collections c ON c.id = h.collection_id \
             WHERE e.language = 'Arabic' AND COALESCE(c.tradition, 'sunni') = 'sunni'",
        )?;
        let mut rows = st.query([])?;
        while let Some(r) = rows.next()? {
            let n = normalize(&r.get::<_, Option<String>>(0)?.unwrap_or_default());
            for (k, p) in &phrases {
                if n.contains(p.as_str()) {
                    *found.get_mut(k).expect("phrase") += 1;
                }
            }
        }
        Ok(found)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_timeline() {
        assert_eq!(all().len(), 38);
        let e = find("elephant").expect("elephant");
        assert_eq!(
            year_label(e, "en"),
            "53 years before the Hijra · 570/571 CE"
        );
        assert_eq!(age(e), 0);
        assert_eq!(period_name("medina", "tr"), "Medine dönemi");
    }
}
