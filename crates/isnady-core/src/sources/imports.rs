//! One way to import a source, for the command line and the window alike (`core/imports.py`). After the importer
//! has written its rows, everything derived from them is brought up to date in the same order every time: orphan
//! rows are removed, the search index is extended, the chains are read and the narrators identified.

use rusqlite::Connection;
use serde_json::{json, Value};
use std::path::PathBuf;

use super::importers::{self, ImportError, ImportReport, Options, SourceInfo};
use super::{forget_prefetched, is_url, prefetch, store, Auth};
use crate::paths;

/// What to import and how (`ImportRequest`).
#[derive(Debug, Clone)]
pub struct ImportRequest {
    pub format_id: String,
    pub location: String,
    pub name: Option<String>,
    pub license: Option<String>,
    pub tier: Option<String>,
    pub origin: String,
    pub auth: Auth,
    pub options: Options,
}

impl ImportRequest {
    fn source(&self) -> SourceInfo {
        SourceInfo {
            location: self.location.clone(),
            name: self.name.clone(),
            license: self.license.clone(),
            origin: self.origin.clone(),
            tier: self.tier.clone(),
            auth: self.auth.clone(),
        }
    }
}

/// The narrators identified after an import: (identified, links) (`link_narrators`' statistics).
pub type Narrators = Option<(i64, i64)>;

// ------------------------------------------------------------------ an import cut short (ROB1)
pub const RUNNING_FILE: &str = "import-running.json";

fn running_path() -> PathBuf {
    paths::data_dir().join(RUNNING_FILE)
}

/// A note in the data folder while an import runs, removed when it ends (`_running`).
struct Running;

impl Running {
    fn start(titles: &[String]) -> Running {
        let note = json!({"titles": titles, "pid": std::process::id(),
                          "started": crate::py::now_local("%Y-%m-%dT%H:%M:%S")});
        let _ = std::fs::write(running_path(), crate::stats::pyjson::dumps(&note));
        Running
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(running_path());
    }
}

/// The books of an import that did not finish (`interrupted`): (titles, started), or None.
pub fn interrupted() -> Option<(Vec<String>, String)> {
    let data: Value = serde_json::from_str(&std::fs::read_to_string(running_path()).ok()?).ok()?;
    let obj = data.as_object()?;
    if let Some(pid) = obj.get("pid").and_then(Value::as_u64) {
        if pid != std::process::id() as u64
            && !cfg!(windows)
            && std::process::Command::new("kill")
                .args(["-0", &pid.to_string()])
                .stderr(std::process::Stdio::null())
                .status()
                .is_ok_and(|st| st.success())
        {
            return None; // another isnady is importing right now
        }
    }
    let titles = obj
        .get("titles")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|t| t.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let started = obj
        .get("started")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    Some((titles, started))
}

pub fn forget_interrupted() {
    let _ = std::fs::remove_file(running_path());
}

// ------------------------------------------------------------------ importing
/// What comes after the importers, once (`run_import`, `run_many`): cleanup, the search index, the chains, the
/// narrators.
fn bring_up_to_date(conn: &Connection, say: &dyn Fn(&str)) -> Result<Narrators, String> {
    store::cleanup(conn).map_err(|e| e.to_string())?;
    crate::search::ensure_index(conn, say).map_err(|e| e.to_string())?;
    crate::reader::store::ensure_isnads(conn, say, false).map_err(|e| e.to_string())?;
    let stats = crate::reader::matcher::link_narrators(conn, say, false)?;
    crate::cache::clear();
    Ok((stats.links > 0).then_some((stats.identified as i64, stats.links as i64)))
}

/// Import and bring every derived table up to date (`run_import`).
pub fn run_import(
    conn: &Connection,
    request: &ImportRequest,
    say: &dyn Fn(&str),
) -> Result<(ImportReport, Narrators), ImportError> {
    let label = request
        .name
        .clone()
        .unwrap_or_else(|| request.location.clone());
    let report = {
        let _note = Running::start(std::slice::from_ref(&label));
        crate::log::timed(
            "import",
            &format!("import {label} ({})", request.format_id),
            || {
                importers::run(
                    &request.format_id,
                    conn,
                    &request.source(),
                    &request.options,
                    say,
                )
            },
        )?
    };
    let narrators = bring_up_to_date(conn, say).map_err(ImportError::Resource)?;
    Ok((report, narrators))
}

/// Books downloaded ahead of the one being written.
pub const AHEAD: usize = 3;

fn ahead(request: &ImportRequest) {
    if request.auth.kind != "none" || !is_url(&request.location) {
        return;
    }
    if request.format_id == "isnady" {
        importers::isnady_book::prefetch(&request.location, &request.options);
    } else {
        prefetch(&request.location, None);
    }
}

pub type Imported = Vec<(String, ImportReport)>;
pub type Failed = Vec<(String, String)>;

/// Import several sources, then bring the derived tables up to date ONCE (`run_many`). A source that fails does
/// not stop the others.
pub fn run_many(
    conn: &Connection,
    requests: &[(String, ImportRequest)],
    say: &dyn Fn(&str),
) -> Result<(Imported, Failed, Narrators), String> {
    let mut imported = vec![];
    let mut failed = vec![];
    let labels: Vec<String> = requests.iter().map(|(l, _)| l.clone()).collect();
    let result = {
        let _note = Running::start(&labels);
        let started = std::time::Instant::now();
        let mut out = Ok(());
        for (i, (label, request)) in requests.iter().enumerate() {
            for (_l, later) in requests.iter().skip(i).take(AHEAD + 1) {
                ahead(later);
            }
            say(&format!("[{}/{}] {label}", i + 1, requests.len()));
            match importers::run(
                &request.format_id,
                conn,
                &request.source(),
                &request.options,
                say,
            ) {
                Ok(report) => imported.push((label.clone(), report)),
                Err(ImportError::Resource(why)) => failed.push((label.clone(), why)),
                Err(ImportError::Db(e)) => {
                    out = Err(e.to_string());
                    break;
                }
            }
        }
        crate::log::info(
            "import",
            &format!(
                "✔ import of {} source(s) in {} ({} imported, {} failed)",
                requests.len(),
                crate::log::since(started),
                imported.len(),
                failed.len()
            ),
        );
        out
    };
    forget_prefetched();
    result?;
    for (label, why) in &failed {
        crate::log::warning("import", &format!("⚠ {label}: {why}"));
    }
    let narrators = bring_up_to_date(conn, say)?;
    Ok((imported, failed, narrators))
}
