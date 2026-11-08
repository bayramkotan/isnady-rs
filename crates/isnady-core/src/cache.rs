//! What is counted once while the database is unchanged (the Python isnady's `core/cache.py`, in memory): a value is
//! kept with the database's stamp — a few counts and highest ids that move whenever anything is imported, read again
//! or identified — and counted again when the stamp moves.

use rusqlite::Connection;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Mutex;

type Kept = HashMap<String, (String, Box<dyn Any + Send>)>;

static KEPT: Mutex<Option<Kept>> = Mutex::new(None);

/// The database's stamp (`cache._full_stamp`, the parts isnady-rs reads).
pub fn stamp(conn: &Connection) -> rusqlite::Result<String> {
    let version: i64 = conn.query_row("PRAGMA data_version", [], |r| r.get(0))?;
    let row: [Option<i64>; 8] = conn.query_row(
        "SELECT (SELECT COUNT(*) FROM collections), (SELECT MAX(id) FROM hadiths), (SELECT MAX(id) FROM isnads),
                (SELECT COUNT(*) FROM isnads), (SELECT MAX(id) FROM grades), (SELECT COUNT(*) FROM grades),
                (SELECT SUM(person_id) FROM isnad_links), (SELECT MAX(id) FROM persons)",
        [],
        |r| {
            Ok([
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
                r.get(7)?,
            ])
        },
    )?;
    Ok(format!("{:?}|{version}|{row:?}", conn.path()))
}

/// The value kept under a name for this database, or counted now and kept (`cache.kept`).
pub fn kept<T: Clone + Send + 'static>(
    conn: &Connection,
    name: &str,
    count: impl FnOnce() -> rusqlite::Result<T>,
) -> rusqlite::Result<T> {
    let now = stamp(conn)?;
    if let Ok(guard) = KEPT.lock() {
        if let Some((s, v)) = guard.as_ref().and_then(|m| m.get(name)) {
            if *s == now {
                if let Some(v) = v.downcast_ref::<T>() {
                    return Ok(v.clone());
                }
            }
        }
    }
    let v = count()?;
    if let Ok(mut guard) = KEPT.lock() {
        guard
            .get_or_insert_with(HashMap::new)
            .insert(name.to_string(), (now, Box::new(v.clone())));
    }
    Ok(v)
}

/// Whether a value is kept under a name for the database as it is now (`cache.ready`).
pub fn is_kept(conn: &Connection, name: &str) -> bool {
    let Ok(now) = stamp(conn) else {
        return false;
    };
    KEPT.lock()
        .ok()
        .and_then(|g| g.as_ref().and_then(|m| m.get(name)).map(|(s, _)| *s == now))
        .unwrap_or(false)
}

/// Forget every value kept (`cache.clear`, in memory).
pub fn clear() {
    if let Ok(mut g) = KEPT.lock() {
        *g = None;
    }
}
