//! Where the reader is in each book — the Python isnady's `isnady/core/readmarks.py`, in the same state.json of the
//! profile: the place each book was last read ("reader.marks.last"), and the markers the reader names himself
//! ("reader.marks.named"). A place is kept by the book's key, the chapter's label and the hadith's number.

use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Map, Value};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{config, i18n, isnad, works};

const LAST: &str = "reader.marks.last";
const NAMED: &str = "reader.marks.named";
const LIMIT: usize = 200;

#[derive(Debug, Clone, Default)]
pub struct Place {
    pub book: String,
    pub chapter: String,
    pub number: String,
    pub title: String,
    pub place: i64,
    pub at: f64,
    pub id: Option<String>,
    pub name: String,
}

fn now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

fn text(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Null) | None => String::new(),
        Some(other) => other.to_string(),
    }
}

impl Place {
    fn new(book: &str, chapter: &str, number: &str, title: &str, place: i64) -> Place {
        Place {
            book: book.into(),
            chapter: chapter.into(),
            number: number.into(),
            title: title.into(),
            place,
            at: now(),
            id: None,
            name: String::new(),
        }
    }

    fn from_json(raw: &Value) -> Option<Place> {
        let o = raw.as_object()?;
        let book = text(o.get("book"));
        if book.is_empty() {
            return None;
        }
        let place = o
            .get("place")
            .and_then(|p| p.as_i64().or_else(|| p.as_f64().map(|f| f as i64)))
            .unwrap_or(0);
        let at = o.get("at").and_then(Value::as_f64).unwrap_or(0.0);
        let id = o.get("id").map(|v| text(Some(v))).filter(|s| !s.is_empty());
        Some(Place {
            book,
            chapter: text(o.get("chapter")),
            number: text(o.get("number")),
            title: text(o.get("title")),
            place,
            at,
            name: if id.is_some() {
                text(o.get("name"))
            } else {
                String::new()
            },
            id,
        })
    }

    fn to_json(&self) -> Value {
        let mut m = Map::new();
        m.insert("book".into(), json!(self.book));
        m.insert("chapter".into(), json!(self.chapter));
        m.insert("number".into(), json!(self.number));
        m.insert("title".into(), json!(self.title));
        m.insert("place".into(), json!(self.place));
        m.insert("at".into(), json!(self.at));
        if let Some(id) = &self.id {
            m.insert("id".into(), json!(id));
            m.insert("name".into(), json!(self.name));
        }
        Value::Object(m)
    }
}

fn last_map() -> Map<String, Value> {
    config::state_get(LAST)
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}

/// The reader is here now.
pub fn remember(book: &str, chapter: &str, number: &str, title: &str, place: i64) {
    if book.is_empty() {
        return;
    }
    let mut data = last_map();
    data.insert(
        book.into(),
        Place::new(book, chapter, number, title, place).to_json(),
    );
    config::state_set(LAST, Value::Object(data));
}

pub fn last(book: &str) -> Option<Place> {
    last_map().get(book).and_then(Place::from_json)
}

/// The books read, the most recent first, each with its last place.
pub fn recent(limit: usize) -> Vec<Place> {
    let mut places: Vec<Place> = last_map().values().filter_map(Place::from_json).collect();
    places.sort_by(|a, b| b.at.partial_cmp(&a.at).unwrap_or(std::cmp::Ordering::Equal));
    places.truncate(limit);
    places
}

/// Forget the last place of one book, or of every book; how many were forgotten.
pub fn forget(book: Option<&str>) -> usize {
    let mut data = last_map();
    let n = match book {
        None => {
            let n = data.len();
            data.clear();
            n
        }
        Some(b) => usize::from(data.shift_remove(b).is_some()),
    };
    config::state_set(LAST, Value::Object(data));
    n
}

pub fn marks() -> Vec<Place> {
    config::state_get(NAMED)
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(Place::from_json)
        .filter(|m| m.id.is_some())
        .collect()
}

fn fold(text: &str) -> String {
    let t = text.replace(['İ', 'ı', 'I'], "i");
    caseless::default_case_fold_str(&t).replace('\u{0307}', "")
}

fn new_id() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    let mut h = RandomState::new().build_hasher();
    h.write_u128(now().to_bits() as u128 ^ std::process::id() as u128);
    format!("{:016x}", h.finish())[..10].to_string()
}

/// A marker at this place; a marker of the same name is moved here instead of kept twice.
pub fn add(name: &str, book: &str, chapter: &str, number: &str, title: &str, place: i64) -> Place {
    let joined = name.split_whitespace().collect::<Vec<_>>().join(" ");
    let name = if !joined.is_empty() {
        joined
    } else if !title.is_empty() {
        title.to_string()
    } else {
        book.to_string()
    };
    let mut mark = Place::new(book, chapter, number, title, place);
    mark.id = Some(new_id());
    mark.name = name.clone();
    let mut all: Vec<Value> = vec![mark.to_json()];
    all.extend(
        marks()
            .into_iter()
            .filter(|m| fold(&m.name) != fold(&name))
            .map(|m| m.to_json()),
    );
    all.truncate(LIMIT);
    config::state_set(NAMED, Value::Array(all));
    mark
}

/// A marker by its id, or by its name (whole, or the start of it).
pub fn find(text: &str) -> Option<Place> {
    let raw = text.trim();
    let t = fold(raw);
    if t.is_empty() {
        return None;
    }
    let every = marks();
    every
        .iter()
        .find(|m| m.id.as_deref() == Some(raw) || fold(&m.name) == t)
        .or_else(|| every.iter().find(|m| fold(&m.name).starts_with(&t)))
        .cloned()
}

/// A marker named anew (`readmarks.rename`).
pub fn rename(mark_id: &str, name: &str) -> bool {
    let joined = name.split_whitespace().collect::<Vec<_>>().join(" ");
    if joined.is_empty() {
        return false;
    }
    let mut every = marks();
    let Some(m) = every.iter_mut().find(|m| m.id.as_deref() == Some(mark_id)) else {
        return false;
    };
    m.name = joined;
    config::state_set(
        NAMED,
        Value::Array(every.iter().map(Place::to_json).collect()),
    );
    true
}

pub fn remove(mark_id: &str) -> bool {
    let every = marks();
    let kept: Vec<Value> = every
        .iter()
        .filter(|m| m.id.as_deref() != Some(mark_id))
        .map(Place::to_json)
        .collect();
    let removed = kept.len() != every.len();
    config::state_set(NAMED, Value::Array(kept));
    removed
}

/// What is kept for a chapter (and a hadith in it).
pub fn place_of(
    conn: &Connection,
    book: &str,
    chapter_id: Option<i64>,
    hadith_id: Option<i64>,
) -> Place {
    let mut out = Place::new(book, "", "", "", 0);
    if let Some(cid) = chapter_id {
        if let Some(n) = works::node(conn, cid, "en") {
            out.chapter = n.label.clone().unwrap_or_default();
            out.title = n.title.clone();
            if let Ok(Some(work)) = works::find_work(conn, book, "en") {
                if let Ok(order) = works::reading_order(conn, work.id) {
                    out.place = order
                        .iter()
                        .position(|c| *c == cid)
                        .map(|i| i as i64 + 1)
                        .unwrap_or(0);
                }
            }
        }
    }
    if let Some(hid) = hadith_id {
        out.number = conn
            .query_row(
                "SELECT number FROM hadiths WHERE id = ?",
                params![hid],
                |r| r.get(0),
            )
            .optional()
            .ok()
            .flatten()
            .unwrap_or_default();
    }
    out
}

/// (chapter id, hadith id) of a kept place in this database: by the hadith's number first, then the chapter's
/// label, then its place in the reading order.
pub fn resolve(conn: &Connection, mark: &Place) -> (Option<i64>, Option<i64>) {
    let Ok(Some(work)) = works::find_work(conn, &mark.book, "en") else {
        return (None, None);
    };
    let hid = if mark.number.is_empty() {
        None
    } else {
        isnad::find_hadith(conn, &mark.book, &mark.number)
            .ok()
            .flatten()
            .map(|h| h.0)
    };
    if let Some(h) = hid {
        if let Some(ch) = works::chapter_of_hadith(conn, h) {
            return (Some(ch), Some(h));
        }
    }
    let order = works::reading_order(conn, work.id).unwrap_or_default();
    if !mark.chapter.is_empty() {
        let row: Option<i64> = conn
            .query_row(
                "SELECT id FROM work_nodes WHERE work_id = ? AND label = ? ORDER BY id LIMIT 1",
                params![work.id, mark.chapter],
                |r| r.get(0),
            )
            .optional()
            .ok()
            .flatten();
        if let Some(r) = row {
            if order.contains(&r) {
                return (Some(r), hid);
            }
        }
    }
    if mark.place > 0 && (mark.place as usize) <= order.len() {
        return (Some(order[mark.place as usize - 1]), hid);
    }
    (order.first().copied(), hid)
}

/// "Sahih al-Bukhari › 3. Knowledge · hadith 59" — plain words for a place.
pub fn describe(mark: &Place, book_title: &str, lang: &str) -> String {
    let mut parts = vec![if book_title.is_empty() {
        mark.book.clone()
    } else {
        book_title.to_string()
    }];
    let chapter = format!(
        "{}{}",
        if mark.chapter.is_empty() {
            String::new()
        } else {
            format!("{}. ", mark.chapter)
        },
        mark.title
    );
    if !chapter.trim().is_empty() {
        parts.push(chapter.trim().to_string());
    }
    let mut text = parts.join(" › ");
    if !mark.number.is_empty() {
        text += &format!(
            " · {}",
            i18n::t(lang, "hadith {number}", &[("number", mark.number.clone())])
        );
    }
    text
}
