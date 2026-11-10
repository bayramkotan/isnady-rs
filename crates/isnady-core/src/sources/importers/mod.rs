//! The importers, one per data format (`data/importers/`): each turns one format into rows of the database, every
//! row tied to the source it came from. Here: `isnady` (a book of isnady-data, `isnady_book.py`) and `isnady-sahifa`
//! (a written source as a book, `sahifa_book.py`); the other formats say they come next (`import_later`).

pub mod fawazahmed0;
pub mod isnady_book;
pub mod openiti_work;
pub mod rijal_works;
pub mod sahifa_book;
pub mod table;

use rusqlite::Connection;
use serde_json::{Map, Value};

use super::{import_later, Auth, ResourceError};

/// Where a source is read from and how (`base.SourceInfo`).
#[derive(Debug, Clone)]
pub struct SourceInfo {
    pub location: String,
    pub name: Option<String>,
    pub license: Option<String>,
    /// builtin | user
    pub origin: String,
    /// A | B | C; None = from the licence
    pub tier: Option<String>,
    pub auth: Auth,
}

/// What an import wrote (`base.ImportReport`).
#[derive(Debug, Clone, Default)]
pub struct ImportReport {
    pub source_key: String,
    pub editions: Vec<String>,
    pub skipped_editions: Vec<String>,
    pub texts: i64,
    pub new_hadiths: i64,
    pub grades: i64,
    pub warnings: Vec<String>,
}

/// Why an import stopped: the resource (said to the user, the other books go on) or the database.
#[derive(Debug)]
pub enum ImportError {
    Resource(String),
    Db(rusqlite::Error),
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImportError::Resource(m) => f.write_str(m),
            ImportError::Db(e) => write!(f, "{e}"),
        }
    }
}

impl From<rusqlite::Error> for ImportError {
    fn from(e: rusqlite::Error) -> Self {
        ImportError::Db(e)
    }
}

impl From<ResourceError> for ImportError {
    fn from(e: ResourceError) -> Self {
        ImportError::Resource(e.0)
    }
}

pub type Options = Map<String, Value>;

/// Import a resource of this format (`get_importer(format).run`).
pub fn run(
    format_id: &str,
    conn: &Connection,
    source: &SourceInfo,
    options: &Options,
    say: &dyn Fn(&str),
) -> Result<ImportReport, ImportError> {
    match format_id {
        "isnady" => isnady_book::run(conn, source, options, say),
        "isnady-sahifa" => sahifa_book::run(conn, source, say),
        "taqrib" => rijal_works::taqrib(conn, source, say),
        "najashi" => rijal_works::by_profile("najashi", conn, source, say),
        "openiti-work" => openiti_work::run(conn, source, options, say),
        "fawazahmed0" => fawazahmed0::run(conn, source, options, say),
        "table" => table::run(conn, source, options, say),
        f if super::has_importer(f) => Err(ImportError::Resource(import_later(f))),
        f => {
            let mut known: Vec<&str> = super::importers()
                .iter()
                .map(|(id, _, _)| id.as_str())
                .collect();
            known.sort();
            Err(ImportError::Resource(format!(
                "Unknown format '{f}'. Known formats: {}",
                if known.is_empty() {
                    "none".into()
                } else {
                    known.join(", ")
                }
            )))
        }
    }
}

/// A JSON file of a source (`_json`): the text read without its byte-order mark.
pub(crate) fn read_json(
    location: &str,
    auth: &Auth,
    timeout: Option<u64>,
) -> Result<Value, ResourceError> {
    let bytes = super::read_bytes_with(location, auth, timeout)?;
    let text = String::from_utf8_lossy(&bytes);
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    serde_json::from_str(text)
        .map_err(|e| ResourceError(format!("Not valid JSON: {location} ({e})")))
}

/// `str(value)` of a JSON value as Python writes it.
pub(crate) fn py_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "None".into(),
        Value::Bool(b) => if *b { "True" } else { "False" }.into(),
        Value::Number(n) => match (n.as_i64(), n.as_f64()) {
            (Some(i), _) => i.to_string(),
            (None, Some(f)) => crate::py::float_repr(f),
            _ => n.to_string(),
        },
        other => other.to_string(),
    }
}

/// A JSON value as sqlite3 binds the Python value.
pub(crate) fn sql(v: &Value) -> rusqlite::types::Value {
    use rusqlite::types::Value as V;
    match v {
        Value::Null => V::Null,
        Value::Bool(b) => V::Integer(*b as i64),
        Value::Number(n) => match n.as_i64() {
            Some(i) => V::Integer(i),
            None => V::Real(n.as_f64().unwrap_or(0.0)),
        },
        Value::String(s) => V::Text(s.clone()),
        other => V::Text(other.to_string()),
    }
}

/// Whether a JSON value is true for Python (`if value:`).
pub(crate) fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}
