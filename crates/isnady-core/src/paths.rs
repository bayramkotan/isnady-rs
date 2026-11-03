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
