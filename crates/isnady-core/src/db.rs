//! The database the Python isnady builds (isnady.db in the data folder). isnady-rs reads it here; `sources::store`
//! opens it to write (removing, resetting, creating and upgrading it); importing stays with the Python isnady until the
//! chain reader is ported (RS17, PARITY.md).

use rusqlite::OpenFlags;
pub use rusqlite::{params_from_iter, Connection, Error};

/// A hadith's number as the Python isnady prints it (the column holds text, or now and then an integer).
pub fn number_text(r: &rusqlite::Row, i: usize) -> rusqlite::Result<String> {
    use rusqlite::types::ValueRef;
    Ok(match r.get_ref(i)? {
        ValueRef::Integer(n) => n.to_string(),
        ValueRef::Real(f) => f.to_string(),
        ValueRef::Text(t) => String::from_utf8_lossy(t).into_owned(),
        _ => "None".to_string(),
    })
}
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::{paths, SCHEMA_VERSION};

#[derive(Debug)]
pub enum OpenError {
    Missing(PathBuf),
    Newer(i64),
    Older(i64),
    Db(rusqlite::Error),
}

impl std::fmt::Display for OpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpenError::Missing(p) => write!(
                f,
                "No database yet at {}. Import the books with the Python isnady first (iy catalog import --all, or \
                 File → Data sources); isnady-rs reads the same database.",
                p.display()
            ),
            OpenError::Newer(v) => write!(
                f,
                "The database is of schema {v}, newer than this isnady-rs reads ({SCHEMA_VERSION}). Update isnady-rs \
                 to the version of your isnady."
            ),
            OpenError::Older(v) => write!(
                f,
                "The database is of schema {v}, older than this isnady-rs reads ({SCHEMA_VERSION}). Open it once with \
                 the Python isnady of the same version: it upgrades it in place."
            ),
            OpenError::Db(e) => write!(f, "{e}"),
        }
    }
}

impl From<rusqlite::Error> for OpenError {
    fn from(e: rusqlite::Error) -> Self {
        OpenError::Db(e)
    }
}

/// Open the database of the data folder (or `path`) for reading.
pub fn connect(path: Option<&Path>) -> Result<Connection, OpenError> {
    if path.is_none() {
        crate::sources::store::first_connect(); // a ready database staged for this start goes in place first
    }
    let path = path.map(PathBuf::from).unwrap_or_else(paths::db_path);
    if !path.exists() {
        return Err(OpenError::Missing(path));
    }
    let conn = Connection::open_with_flags(
        &path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    conn.busy_timeout(Duration::from_secs(30))?;
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if version > SCHEMA_VERSION {
        return Err(OpenError::Newer(version));
    }
    if version < SCHEMA_VERSION {
        return Err(OpenError::Older(version));
    }
    Ok(conn)
}

#[derive(Debug, Clone)]
pub struct SourceRow {
    pub key: String,
    pub name: String,
    pub format: String,
    pub origin: String,
    pub tier: String,
    pub auth_type: String,
    pub license: Option<String>,
    pub location: String,
    pub imported_at: Option<String>,
    pub editions: i64,
    pub texts: i64,
}

/// What is imported: every source with its editions and texts (`iy sources`).
pub fn sources(conn: &Connection) -> rusqlite::Result<Vec<SourceRow>> {
    let mut stmt = conn.prepare(
        "SELECT s.key, s.name, s.format, s.origin, s.tier, s.auth_type, s.license, s.location, s.imported_at,
                COUNT(e.id) AS editions, COALESCE(SUM(e.text_count), 0) AS texts
         FROM sources s LEFT JOIN editions e ON e.source_id = s.id
         GROUP BY s.id ORDER BY s.origin, s.key",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(SourceRow {
            key: r.get(0)?,
            name: r.get(1)?,
            format: r.get(2)?,
            origin: r.get(3)?,
            tier: r.get(4)?,
            auth_type: r.get(5)?,
            license: r.get(6)?,
            location: r.get(7)?,
            imported_at: r.get(8)?,
            editions: r.get(9)?,
            texts: r.get(10)?,
        })
    })?;
    rows.collect()
}
