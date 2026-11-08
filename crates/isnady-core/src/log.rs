//! isnady's log (LOG1): the Python isnady's `core/log.py` — the same file in the data folder (logs/isnady.log, kept to
//! 2 MB with five older files beside it), the same line ("2026-10-25 07:45:12.345  INFO   search    ✔ …"), read in
//! the window (Help → Log) and on the command line (`iy-rs log`). The two programs write into one log.

use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;

use crate::paths::data_dir;

pub const FILE: &str = "isnady.log";
pub const FOLDER: &str = "logs";
pub const MAX_BYTES: u64 = 2_000_000;
pub const BACKUPS: usize = 5;

static ON: Mutex<Option<bool>> = Mutex::new(None);

pub fn folder() -> PathBuf {
    data_dir().join(FOLDER)
}

pub fn path() -> PathBuf {
    folder().join(FILE)
}

fn debug_on() -> bool {
    crate::config::py_str(&crate::config::effective("log.level")).to_lowercase() == "debug"
}

/// A line written as Python's RotatingFileHandler writes it; never fails (isnady runs without a log when the folder
/// cannot be written).
pub fn write(level: &str, part: &str, message: &str) {
    if level == "DEBUG" && !debug_on() {
        return;
    }
    let Ok(guard) = ON.lock() else { return };
    if *guard != Some(true) {
        return;
    }
    let now = chrono::Local::now();
    let line = format!(
        "{}  {:<6} {:<9} {}\n",
        now.format("%Y-%m-%d %H:%M:%S%.3f"),
        level,
        part,
        message
    );
    let p = path();
    if p.metadata()
        .map(|m| m.len() + line.len() as u64 > MAX_BYTES)
        .unwrap_or(false)
    {
        rotate();
    }
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&p)
    {
        let _ = f.write_all(line.as_bytes());
    }
}

/// isnady.log → isnady.log.1 → … → .5 (the oldest dropped), as RotatingFileHandler does.
fn rotate() {
    let base = path();
    let name = |i: usize| folder().join(format!("{FILE}.{i}"));
    let _ = std::fs::remove_file(name(BACKUPS));
    for i in (1..BACKUPS).rev() {
        if name(i).exists() {
            let _ = std::fs::rename(name(i), name(i + 1));
        }
    }
    let _ = std::fs::rename(&base, name(1));
}

pub fn info(part: &str, message: &str) {
    write("INFO", part, message);
}

pub fn warning(part: &str, message: &str) {
    write("WARNING", part, message);
}

pub fn error(part: &str, message: &str) {
    write("ERROR", part, message);
}

pub fn debug(part: &str, message: &str) {
    write("DEBUG", part, message);
}

fn system() -> &'static str {
    match std::env::consts::OS {
        "linux" => "Linux",
        "windows" => "Windows",
        "macos" => "Darwin",
        other => other,
    }
}

/// Start the log for this run (`log.setup`): the window writes a banner block, a command one line.
pub fn setup(kind: &str) {
    if std::fs::create_dir_all(folder()).is_err() {
        return;
    }
    if let Ok(mut g) = ON.lock() {
        if g.is_some() {
            return;
        }
        *g = Some(true);
    }
    let v = crate::VERSION;
    if kind != "window" {
        info(
            "isnady",
            &format!("──── isnady {v} (Rust) · {kind} · {} ────", system()),
        );
        return;
    }
    let line = "═".repeat(24);
    info(
        "isnady",
        &format!(
            "{line} isnady {v} (Rust) · {kind} · started {} {line}",
            chrono::Local::now().format("%Y-%m-%d %H:%M")
        ),
    );
    info(
        "isnady",
        &format!("▶ Rust · {} ({})", system(), std::env::consts::ARCH),
    );
    let where_ = data_dir();
    info("isnady", &format!("▶ data folder {}", where_.display()));
    info(
        "isnady",
        &format!("▶ profile {}", crate::config::active_profile()),
    );
    let db = where_.join(crate::paths::DB_FILE_NAME);
    if let Ok(m) = db.metadata() {
        info(
            "isnady",
            &format!(
                "▶ database {} ({:.1} MB)",
                db.display(),
                m.len() as f64 / 1e6
            ),
        );
    }
    let content = crate::config::py_str(&crate::config::effective("view.content_languages"));
    info(
        "isnady",
        &format!(
            "▶ interface {} · texts {}",
            crate::i18n::ui_language(),
            if content.is_empty() { "all" } else { &content }
        ),
    );
}

/// "12.3 s", "2 min 5 s" (`log._since`).
pub fn since(start: Instant) -> String {
    let s = start.elapsed().as_secs_f64();
    if s < 60.0 {
        format!("{s:.1} s")
    } else {
        format!("{} min {} s", (s / 60.0) as u64, (s % 60.0) as u64)
    }
}

/// One line with the time a piece of work took (`log.timed`).
pub fn timed<T, E: std::fmt::Display>(
    part: &str,
    what: &str,
    work: impl FnOnce() -> Result<T, E>,
) -> Result<T, E> {
    let start = Instant::now();
    let out = work();
    let ms = start.elapsed().as_secs_f64() * 1000.0;
    let took = if ms < 10000.0 {
        format!("{ms:.0} ms")
    } else {
        format!("{:.1} s", ms / 1000.0)
    };
    match &out {
        Ok(_) => info(part, &format!("✔ {what} in {took}")),
        Err(e) => error(part, &format!("✖ {what} failed after {took}: {e}")),
    }
    out
}

// ------------------------------------------------------------------ reading the log (Help → Log, `iy-rs log`)

/// The log files, newest first: isnady.log, its older parts, then the crash reports.
pub fn files() -> Vec<PathBuf> {
    let where_ = folder();
    if !where_.is_dir() {
        return vec![];
    }
    let mut out: Vec<PathBuf> = std::iter::once(where_.join(FILE))
        .chain((1..=BACKUPS).map(|i| where_.join(format!("{FILE}.{i}"))))
        .filter(|p| p.exists())
        .collect();
    let mut crashes: Vec<(std::time::SystemTime, PathBuf)> = std::fs::read_dir(&where_)
        .map(|d| {
            d.flatten()
                .filter(|e| {
                    let n = e.file_name().to_string_lossy().to_string();
                    n.starts_with("crash_") && n.ends_with(".log")
                })
                .map(|e| {
                    (
                        e.metadata()
                            .and_then(|m| m.modified())
                            .unwrap_or(std::time::UNIX_EPOCH),
                        e.path(),
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    crashes.sort_by_key(|b| std::cmp::Reverse(b.0));
    out.extend(crashes.into_iter().map(|(_, p)| p));
    out
}

/// The log (the last `lines` lines, 0 for all), only the lines of a level and above or holding a text; a traceback
/// stays with its line (`log.read`).
pub fn read(file: &std::path::Path, lines: usize, level: &str, text: &str) -> String {
    let Ok(raw) = std::fs::read(file) else {
        return String::new();
    };
    let content = String::from_utf8_lossy(&raw).to_string();
    let mut rows: Vec<&str> = content.lines().collect();
    if !level.is_empty() || !text.is_empty() {
        let order = |l: &str| match l {
            "debug" => 0,
            "info" => 1,
            "warning" => 2,
            "error" => 3,
            _ => 1,
        };
        let floor = match level.to_lowercase().as_str() {
            "" => 0,
            l => order(l),
        };
        let want = text.to_lowercase();
        let mut kept = vec![];
        let mut keep = false;
        for row in rows {
            let chars: Vec<char> = row.chars().collect();
            let head =
                if chars.len() > 30 && chars[4] == '-' && chars[23] == ' ' && chars[24] == ' ' {
                    Some(
                        chars[25..31]
                            .iter()
                            .collect::<String>()
                            .trim()
                            .to_lowercase(),
                    )
                } else {
                    None
                };
            if let Some(h) = head {
                keep =
                    order(&h) >= floor && (want.is_empty() || row.to_lowercase().contains(&want));
            } else if !want.is_empty() && level.is_empty() {
                keep = keep || row.to_lowercase().contains(&want);
            }
            if keep {
                kept.push(row);
            }
        }
        rows = kept;
    }
    if lines > 0 && rows.len() > lines {
        rows = rows[rows.len() - lines..].to_vec();
    }
    rows.join("\n")
}

/// Empty isnady.log, remove its older parts and the crash reports; how many files were touched.
pub fn clear() -> usize {
    let mut n = 0;
    for p in files() {
        let ok = if p.file_name().map(|f| f == FILE).unwrap_or(false) {
            std::fs::write(&p, "").is_ok()
        } else {
            std::fs::remove_file(&p).is_ok()
        };
        if ok {
            n += 1;
        }
    }
    n
}

/// Remove the lines of isnady.log older than a day ("2026-10-20"), and the older parts and crash reports written
/// before it; the lines removed from isnady.log.
pub fn delete_before(day: &str) -> usize {
    for p in files() {
        if p.file_name().map(|f| f == FILE).unwrap_or(false) {
            continue;
        }
        let modified = p
            .metadata()
            .and_then(|m| m.modified())
            .map(|t| {
                chrono::DateTime::<chrono::Local>::from(t)
                    .format("%Y-%m-%d")
                    .to_string()
            })
            .unwrap_or_default();
        if modified.as_str() < day {
            let _ = std::fs::remove_file(&p);
        }
    }
    let main = path();
    let Ok(raw) = std::fs::read(&main) else {
        return 0;
    };
    let content = String::from_utf8_lossy(&raw).to_string();
    let (mut keep, mut cutting, mut removed) = (String::new(), true, 0);
    for row in content.split_inclusive('\n') {
        let stamp: String = row.chars().take(10).collect();
        let c: Vec<char> = stamp.chars().collect();
        if cutting && c.len() == 10 && c[4] == '-' && c[7] == '-' {
            cutting = stamp.as_str() < day;
        }
        if cutting {
            removed += 1;
        } else {
            keep.push_str(row);
        }
    }
    if std::fs::write(&main, keep).is_err() {
        return 0;
    }
    removed
}
