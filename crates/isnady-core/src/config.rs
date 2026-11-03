//! The settings, the remembered state and the profiles — the same files as the Python isnady (`isnady/config.py`,
//! `isnady/core/profiles.py`): settings.json and state.json in the folder of the profile in use (the data folder for
//! the first profile, profiles/<id>/ for the others), profiles.json in the data folder.

use serde_json::{Map, Value};
use std::path::PathBuf;

use crate::paths::data_dir;

pub const SETTINGS_FILE: &str = "settings.json";
pub const STATE_FILE: &str = "state.json";
pub const PROFILES_FILE: &str = "profiles.json";
pub const DEFAULT_PROFILE: &str = "default";

fn read_json(path: &PathBuf) -> Map<String, Value> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}

/// The profile in use: ISNADY_PROFILE (by id or name), else the one profiles.json names, else the first.
pub fn active_profile() -> String {
    let data = read_json(&data_dir().join(PROFILES_FILE));
    let profiles: Vec<(String, String)> = data
        .get("profiles")
        .and_then(|p| p.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|p| {
                    let id = p.get("id")?.as_str()?.to_string();
                    let name = p
                        .get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or("")
                        .to_string();
                    Some((id, name))
                })
                .collect()
        })
        .unwrap_or_default();
    if let Ok(wanted) = std::env::var("ISNADY_PROFILE") {
        if !wanted.is_empty() {
            let w = wanted.to_lowercase();
            return profiles
                .iter()
                .find(|(id, name)| id.to_lowercase() == w || name.to_lowercase() == w)
                .map(|(id, _)| id.clone())
                .unwrap_or_else(|| DEFAULT_PROFILE.to_string());
        }
    }
    match data.get("active").and_then(|a| a.as_str()) {
        Some(a) if profiles.iter().any(|(id, _)| id == a) => a.to_string(),
        _ => DEFAULT_PROFILE.to_string(),
    }
}

pub fn profile_folder() -> PathBuf {
    let pid = active_profile();
    if pid == DEFAULT_PROFILE {
        data_dir()
    } else {
        data_dir().join("profiles").join(pid)
    }
}

/// A setting as stored ("" or missing: the default the caller gives).
pub fn get_str(key: &str, default: &str) -> String {
    let data = read_json(&profile_folder().join(SETTINGS_FILE));
    match data.get(key) {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        Some(Value::Null) | None => default.to_string(),
        Some(Value::String(_)) => default.to_string(),
        Some(other) => other.to_string(),
    }
}

pub fn state_get(key: &str) -> Option<Value> {
    read_json(&profile_folder().join(STATE_FILE))
        .get(key)
        .cloned()
}

/// Remember a value in state.json (written as Python writes it: indented by one, UTF-8, replaced atomically).
pub fn state_set(key: &str, value: Value) {
    let path = profile_folder().join(STATE_FILE);
    let mut data = read_json(&path);
    data.insert(key.to_string(), value);
    let text = to_python_json(&Value::Object(data));
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let tmp = path.with_extension("tmp");
    if std::fs::write(&tmp, text).is_ok() {
        let _ = std::fs::rename(&tmp, &path);
    }
}

/// JSON as Python's json.dumps(..., ensure_ascii=False, indent=1) writes it.
pub fn to_python_json(v: &Value) -> String {
    let mut buf = Vec::new();
    let fmt = serde_json::ser::PrettyFormatter::with_indent(b" ");
    let mut ser = serde_json::Serializer::with_formatter(&mut buf, fmt);
    serde::Serialize::serialize(v, &mut ser).expect("JSON");
    String::from_utf8(buf).expect("UTF-8")
}
