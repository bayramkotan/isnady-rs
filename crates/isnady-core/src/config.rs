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
    write_atomically(&path, &to_python_json(&Value::Object(data)));
}

/// Forget every remembered value whose key begins with prefix (`config.state_forget`).
pub fn state_forget(prefix: &str) {
    let path = profile_folder().join(STATE_FILE);
    if !path.exists() {
        return;
    }
    let data = read_json(&path);
    let kept: Map<String, Value> = data
        .iter()
        .filter(|(k, _)| !k.starts_with(prefix))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    if kept.len() != data.len() {
        write_atomically(&path, &to_python_json(&Value::Object(kept)));
    }
}

fn write_atomically(path: &PathBuf, text: &str) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let tmp = path.with_extension("tmp");
    if std::fs::write(&tmp, text).is_ok() {
        let _ = std::fs::rename(&tmp, path);
    }
}

/// The settings the window changes, with the Python isnady's defaults (`config.defaults`): a value equal to its
/// default is not stored, as `config.set` does.
pub fn default_of(key: &str) -> Option<Value> {
    Some(match key {
        "view.theme" => Value::from("system"),
        "view.text_scale" => Value::from(1.0),
        "view.ui_language" => Value::from("en"),
        "view.content_languages" => Value::from(""),
        _ => return None,
    })
}

/// A setting stored as `config.set` stores it: settings.json of the profile, keys sorted, indented by two; "" or
/// the default removes the key. Only the settings `default_of` knows.
pub fn set(key: &str, value: Value) -> Result<(), String> {
    let default =
        default_of(key).ok_or_else(|| format!("Unknown setting '{key}'. See: iy config list"))?;
    let path = profile_folder().join(SETTINGS_FILE);
    let mut data = read_json(&path);
    let empty = matches!(&value, Value::String(s) if s.is_empty()) || value.is_null();
    let same = match (&value, &default) {
        (Value::Number(a), Value::Number(b)) => a.as_f64() == b.as_f64(),
        (a, b) => a == b,
    };
    if empty || same {
        data.remove(key);
    } else {
        data.insert(key.to_string(), value);
    }
    let mut keys: Vec<&String> = data.keys().collect();
    keys.sort();
    let sorted: Map<String, Value> = keys
        .into_iter()
        .map(|k| (k.clone(), data[k].clone()))
        .collect();
    let mut buf = Vec::new();
    let fmt = serde_json::ser::PrettyFormatter::with_indent(b"  ");
    let mut ser = serde_json::Serializer::with_formatter(&mut buf, fmt);
    serde::Serialize::serialize(&Value::Object(sorted), &mut ser).map_err(|e| e.to_string())?;
    write_atomically(&path, &String::from_utf8(buf).map_err(|e| e.to_string())?);
    Ok(())
}

/// A number setting (view.text_scale), its default when unset.
pub fn get_f64(key: &str, default: f64) -> f64 {
    let data = read_json(&profile_folder().join(SETTINGS_FILE));
    match data.get(key) {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(default),
        Some(Value::String(s)) => s.parse().unwrap_or(default),
        _ => default,
    }
}

/// One profile of profiles.json (`profiles.all_profiles`).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub active: bool,
}

/// profiles.json as the Python isnady reads it: the first profile always there, "active" defaulting to it.
fn profiles_data() -> (String, Vec<Map<String, Value>>) {
    let data = read_json(&data_dir().join(PROFILES_FILE));
    let mut items: Vec<Map<String, Value>> = data
        .get("profiles")
        .and_then(|p| p.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|p| p.as_object().cloned())
                .filter(|p| {
                    p.get("id")
                        .and_then(|i| i.as_str())
                        .is_some_and(|i| !i.is_empty())
                })
                .collect()
        })
        .unwrap_or_default();
    if !items
        .iter()
        .any(|p| p.get("id").and_then(|i| i.as_str()) == Some(DEFAULT_PROFILE))
    {
        let mut first = Map::new();
        first.insert("id".into(), Value::from(DEFAULT_PROFILE));
        first.insert("name".into(), Value::from(""));
        items.insert(0, first);
    }
    let active = data
        .get("active")
        .and_then(|a| a.as_str())
        .filter(|a| !a.is_empty())
        .unwrap_or(DEFAULT_PROFILE)
        .to_string();
    (active, items)
}

/// Every profile, the first one first; the first one's name is "" until it is given one.
pub fn all_profiles() -> Vec<Profile> {
    let now = active_profile();
    profiles_data()
        .1
        .iter()
        .map(|p| {
            let id = p
                .get("id")
                .and_then(|i| i.as_str())
                .unwrap_or("")
                .to_string();
            Profile {
                active: id == now,
                name: p
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("")
                    .to_string(),
                id,
            }
        })
        .collect()
}

/// The name of a profile ("" for the first one until it is named).
pub fn profile_name(id: &str) -> String {
    all_profiles()
        .into_iter()
        .find(|p| p.id == id)
        .map(|p| p.name)
        .unwrap_or_default()
}

/// Make a profile the active one (`profiles.switch`): profiles.json written as Python writes it.
pub fn switch_profile(id: &str) -> Result<(), String> {
    let (_, items) = profiles_data();
    if !items
        .iter()
        .any(|p| p.get("id").and_then(|i| i.as_str()) == Some(id))
    {
        return Err(format!("No profile {id}."));
    }
    let mut out = Map::new();
    out.insert("active".into(), Value::from(id));
    out.insert(
        "profiles".into(),
        Value::Array(items.into_iter().map(Value::Object).collect()),
    );
    write_atomically(
        &data_dir().join(PROFILES_FILE),
        &to_python_json(&Value::Object(out)),
    );
    Ok(())
}

/// JSON as Python's json.dumps(..., ensure_ascii=False, indent=1) writes it.
pub fn to_python_json(v: &Value) -> String {
    let mut buf = Vec::new();
    let fmt = serde_json::ser::PrettyFormatter::with_indent(b" ");
    let mut ser = serde_json::Serializer::with_formatter(&mut buf, fmt);
    serde::Serialize::serialize(v, &mut ser).expect("JSON");
    String::from_utf8(buf).expect("UTF-8")
}
