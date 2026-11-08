//! Where isnady keeps its data — the same folder as the Python isnady (`isnady/data/paths.py`), so the two read and
//! write the same database, settings and bookmarks.
//!
//! Order of precedence:
//!   1. the ISNADY_DATA_DIR environment variable (tests, portable use);
//!   2. a folder named isnady-data beside the portable application;
//!   3. the folder chosen by the user (location.json in the platform's configuration folder);
//!   4. the platform default.

use std::env;
use std::path::{Path, PathBuf};

pub const APP_DIR_NAME: &str = "isnady";
pub const DB_FILE_NAME: &str = "isnady.db";
pub const POINTER_FILE: &str = "location.json";
pub const PORTABLE_DIR_NAME: &str = "isnady-data";

fn home() -> PathBuf {
    env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// `Path(p).expanduser()`.
pub fn expand_user(p: &str) -> PathBuf {
    expand(p)
}

fn expand(p: &str) -> PathBuf {
    if let Some(rest) = p.strip_prefix("~/").or_else(|| p.strip_prefix("~\\")) {
        return home().join(rest);
    }
    if p == "~" {
        return home();
    }
    PathBuf::from(p)
}

pub fn default_data_dir() -> PathBuf {
    if cfg!(windows) {
        let base = env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| home().join("AppData").join("Local"));
        return base.join(APP_DIR_NAME);
    }
    if cfg!(target_os = "macos") {
        return home()
            .join("Library")
            .join("Application Support")
            .join(APP_DIR_NAME);
    }
    match env::var_os("XDG_DATA_HOME") {
        Some(x) if !x.is_empty() => PathBuf::from(x).join(APP_DIR_NAME),
        _ => home().join(".local").join("share").join(APP_DIR_NAME),
    }
}

pub fn config_dir() -> PathBuf {
    let base = if cfg!(windows) {
        env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| home().join("AppData").join("Roaming"))
    } else if cfg!(target_os = "macos") {
        home().join("Library").join("Preferences")
    } else {
        match env::var_os("XDG_CONFIG_HOME") {
            Some(x) if !x.is_empty() => PathBuf::from(x),
            _ => home().join(".config"),
        }
    };
    base.join(APP_DIR_NAME)
}

/// The folder the user chose (location.json), when there is one.
pub fn chosen_data_dir() -> Option<PathBuf> {
    let raw = std::fs::read_to_string(config_dir().join(POINTER_FILE)).ok()?;
    let value: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let dir = value.get("data_dir")?.as_str()?;
    if dir.is_empty() {
        None
    } else {
        Some(expand(dir))
    }
}

/// A folder named isnady-data beside this program, for a portable copy (a USB stick).
pub fn portable_data_dir() -> Option<PathBuf> {
    let exe = env::current_exe().ok()?;
    let candidate = exe.parent()?.join(PORTABLE_DIR_NAME);
    if candidate.is_dir() {
        Some(candidate)
    } else {
        None
    }
}

/// Why the data folder is where it is: environment, portable, chosen, or default.
pub fn data_dir_source() -> &'static str {
    if env::var_os("ISNADY_DATA_DIR")
        .map(|v| !v.is_empty())
        .unwrap_or(false)
    {
        "environment"
    } else if portable_data_dir().is_some() {
        "portable"
    } else if chosen_data_dir().is_some() {
        "chosen"
    } else {
        "default"
    }
}

pub fn data_dir() -> PathBuf {
    match env::var("ISNADY_DATA_DIR") {
        Ok(v) if !v.is_empty() => expand(&v),
        _ => portable_data_dir()
            .or_else(chosen_data_dir)
            .unwrap_or_else(default_data_dir),
    }
}

pub fn db_path() -> PathBuf {
    data_dir().join(DB_FILE_NAME)
}

/// Shown as Python shows a path (no trailing separator, as the system writes it).
pub fn shown(p: &Path) -> String {
    p.display().to_string()
}

/// Use another data folder from the next start (`paths.relocate`): "copy" copies the current data there (the old
/// folder is kept), "as-is" uses the folder as it is; None goes back to the platform default. The folder used, or
/// the message for the user.
pub fn relocate(new: Option<&str>, mode: &str) -> Result<PathBuf, String> {
    if env::var_os("ISNADY_DATA_DIR").is_some_and(|v| !v.is_empty()) {
        return Err(
            "The data folder is set by the ISNADY_DATA_DIR environment variable; unset it first."
                .into(),
        );
    }
    let current = data_dir();
    let target = match new {
        Some(p) => {
            let p = expand(p);
            let abs = if p.is_absolute() {
                p
            } else {
                env::current_dir().unwrap_or_default().join(p)
            };
            std::fs::canonicalize(&abs).unwrap_or(abs)
        }
        None => default_data_dir(),
    };
    if Some(&target) == std::fs::canonicalize(&current).ok().as_ref() || target == current {
        return Err(format!("isnady already uses {}.", shown(&target)));
    }
    std::fs::create_dir_all(&target)
        .map_err(|_| format!("{} cannot be written to.", shown(&target)))?;
    let probe = target.join(".isnady-write-test");
    if std::fs::write(&probe, b"").is_err() {
        return Err(format!("{} cannot be written to.", shown(&target)));
    }
    let _ = std::fs::remove_file(&probe);
    if mode == "copy" {
        if target.join(DB_FILE_NAME).exists() {
            return Err(format!(
                "{} already holds an isnady database. Choose 'use as it is', or another folder.",
                shown(&target)
            ));
        }
        let source_db = current.join(DB_FILE_NAME);
        if source_db.exists() {
            if let Ok(c) = rusqlite::Connection::open(&source_db) {
                let _ = c.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()));
            }
        }
        copy_tree(&current, &target, true).map_err(|e| e.to_string())?;
    } else if mode != "as-is" {
        return Err("mode must be copy or as-is".into());
    }
    let pointer = config_dir().join(POINTER_FILE);
    if let Some(parent) = pointer.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if new.is_some() {
        let mut m = serde_json::Map::new();
        m.insert("data_dir".into(), serde_json::Value::from(shown(&target)));
        let mut buf = Vec::new();
        let fmt = serde_json::ser::PrettyFormatter::with_indent(b"  ");
        let mut ser = serde_json::Serializer::with_formatter(&mut buf, fmt);
        serde::Serialize::serialize(&serde_json::Value::Object(m), &mut ser)
            .map_err(|e| e.to_string())?;
        std::fs::write(&pointer, buf).map_err(|e| e.to_string())?;
    } else if pointer.exists() {
        let _ = std::fs::remove_file(&pointer);
    }
    Ok(target)
}

/// Every file and folder of `from` into `to` (the top level skips the write-ahead files and hidden ones).
pub fn copy_tree(from: &Path, to: &Path, top: bool) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for item in std::fs::read_dir(from)? {
        let item = item?;
        let name = item.file_name().to_string_lossy().to_string();
        if top && (name.ends_with("-wal") || name.ends_with("-shm") || name.starts_with('.')) {
            continue;
        }
        let dest = to.join(&name);
        if item.file_type()?.is_dir() {
            copy_tree(&item.path(), &dest, false)?;
        } else {
            std::fs::copy(item.path(), dest)?;
        }
    }
    Ok(())
}
