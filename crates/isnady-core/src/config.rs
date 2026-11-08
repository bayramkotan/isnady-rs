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

/// A script's (label, sample, default family, default size, default line height).
pub type Script = (String, String, String, f64, i64);

/// What isnady/config.py knows, read from the Python isnady (data/settings.json, by tools/gen_tables.py).
#[derive(serde::Deserialize)]
pub struct Known {
    pub defaults: Vec<(String, Value)>,
    /// script → (label, sample, default family, default size, default line height)
    pub scripts: Vec<(String, Script)>,
    pub script_of_language: Map<String, Value>,
    pub color_tokens: Vec<(String, String)>,
    pub sidebar: Vec<String>,
    pub themes: Vec<String>,
}

pub fn known() -> &'static Known {
    static K: std::sync::OnceLock<Known> = std::sync::OnceLock::new();
    K.get_or_init(|| {
        serde_json::from_str(include_str!("../data/settings.json")).expect("data/settings.json")
    })
}

/// The default of a setting (`config.DEFAULTS`).
pub fn default_of(key: &str) -> Option<Value> {
    known()
        .defaults
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.clone())
}

/// settings.json of the profile in use (`config.settings_path`).
pub fn settings_path() -> PathBuf {
    profile_folder().join(SETTINGS_FILE)
}

/// Python's `str(value)` of a setting: a text as it is, a number as Python prints it.
pub fn py_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "None".into(),
        Value::Bool(b) => {
            if *b {
                "True".into()
            } else {
                "False".into()
            }
        }
        Value::Number(n) => {
            if n.is_f64() {
                crate::py::float_repr(n.as_f64().unwrap_or(0.0))
            } else {
                n.to_string()
            }
        }
        other => other.to_string(),
    }
}

fn is_empty(v: &Value) -> bool {
    v.is_null() || matches!(v, Value::String(s) if s.is_empty())
}

/// Python's `==` between two settings: 20 == 20.0.
fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        (x, y) => x == y,
    }
}

/// The value converted to its stored type, or the message for the user (`config.validate`).
pub fn validate(key: &str, value: &Value) -> Result<Value, String> {
    let Some(default) = default_of(key) else {
        return Err(format!("Unknown setting '{key}'. See: iy config list"));
    };
    if is_empty(value) {
        return Ok(Value::from(""));
    }
    let text = py_str(value);
    let parts = |t: &str| -> Vec<String> {
        t.split(',')
            .map(|p| p.trim().to_string())
            .filter(|p| !p.is_empty())
            .collect()
    };
    match key {
        "view.theme" => {
            let k = known();
            return if k.themes.contains(&text) {
                Ok(Value::from(text))
            } else {
                Err(format!(
                    "view.theme must be one of: {}",
                    k.themes.join(", ")
                ))
            };
        }
        "view.ui_language" => {
            let langs = crate::i18n::languages();
            return if langs.iter().any(|l| l.0 == text) {
                Ok(Value::from(text))
            } else {
                Err(format!(
                    "view.ui_language must be one of: {}",
                    langs
                        .iter()
                        .map(|l| l.0.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
            };
        }
        "books.disabled" => {
            let mut set: Vec<String> = parts(&text);
            set.sort();
            set.dedup();
            return Ok(Value::from(set.join(",")));
        }
        "view.other_languages" => {
            return if text == "fold" || text == "omit" {
                Ok(Value::from(text))
            } else {
                Err("view.other_languages must be fold or omit".into())
            };
        }
        "sources.repository" => {
            let low = text.trim().to_lowercase();
            return Ok(Value::from(
                low.chars()
                    .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || ".-_".contains(*c))
                    .collect::<String>(),
            ));
        }
        "view.sidebar" => {
            let sidebar = &known().sidebar;
            let keys = parts(&text);
            if keys.iter().any(|k| !sidebar.contains(k)) {
                return Err(format!(
                    "view.sidebar takes these sections: {}",
                    sidebar.join(", ")
                ));
            }
            let mut order: Vec<String> = vec![];
            for k in keys.iter().chain(sidebar.iter()) {
                if !order.contains(k) {
                    order.push(k.clone());
                }
            }
            return Ok(Value::from(if order == *sidebar {
                String::new()
            } else {
                order.join(",")
            }));
        }
        "view.content_languages" => return Ok(Value::from(parts(&text).join(","))),
        "log.level" => {
            let v = text.trim().to_lowercase();
            return if v == "info" || v == "debug" {
                Ok(Value::from(v))
            } else {
                Err("log.level is info or debug".into())
            };
        }
        "view.language_order" => {
            let mut out: Vec<String> = vec![];
            for p in parts(&text) {
                if p != "Arabic" && !out.contains(&p) {
                    out.push(p);
                }
            }
            return Ok(Value::from(out.join(",")));
        }
        _ => {}
    }
    if key.ends_with(".color") || key.starts_with("colors.") {
        let ok = text.len() == 7
            && text.starts_with('#')
            && text[1..].chars().all(|c| c.is_ascii_hexdigit());
        return if ok {
            Ok(Value::from(text.to_uppercase()))
        } else {
            Err(format!(
                "{key} must be a colour like #1D4777 (or empty for the default)"
            ))
        };
    }
    if key.ends_with(".family") {
        return Ok(Value::from(text.trim()));
    }
    let number: f64 = match value {
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        _ => crate::py::parse_float(&text).ok_or_else(|| format!("{key} must be a number"))?,
    };
    let (low, high) = if key.ends_with(".size") {
        match key {
            "ui.size" => (6.0, 24.0),
            _ => (6.0, 48.0),
        }
    } else if key.ends_with(".line_height") {
        (80.0, 250.0)
    } else if key == "view.text_scale" {
        (0.8, 1.6)
    } else {
        (f64::NEG_INFINITY, f64::INFINITY)
    };
    if !(low <= number && number <= high) {
        return Err(format!(
            "{key} must be between {} and {}",
            crate::py::fmt_g(low, 6),
            crate::py::fmt_g(high, 6)
        ));
    }
    Ok(if default.is_i64() || default.is_u64() {
        Value::from(number.trunc() as i64)
    } else {
        Value::from(number)
    })
}

fn write_settings(data: &Map<String, Value>) -> Result<(), String> {
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
    write_atomically(
        &settings_path(),
        &String::from_utf8(buf).map_err(|e| e.to_string())?,
    );
    Ok(())
}

/// A setting stored as `config.set` stores it: validated, settings.json of the profile, keys sorted, indented by
/// two; "" or the default removes the key.
pub fn set(key: &str, value: Value) -> Result<(), String> {
    let value = validate(key, &value)?;
    let default = default_of(key).unwrap_or(Value::Null);
    let mut data = read_json(&settings_path());
    if is_empty(&value) || same(&value, &default) {
        data.remove(key);
    } else {
        data.insert(key.to_string(), value);
    }
    write_settings(&data)
}

/// The value in effect: the stored one, else the default (`config.get`).
pub fn effective(key: &str) -> Value {
    match read_json(&settings_path()).get(key) {
        Some(v) if !is_empty(v) => v.clone(),
        _ => default_of(key).unwrap_or(Value::Null),
    }
}

/// Forget the stored values under a prefix ("" = everything); how many (`config.reset`).
pub fn reset(prefix: &str) -> Result<usize, String> {
    let mut data = read_json(&settings_path());
    let under = format!("{}.", prefix.trim_end_matches('.'));
    let keys: Vec<String> = data
        .keys()
        .filter(|k| prefix.is_empty() || k.as_str() == prefix || k.starts_with(&under))
        .cloned()
        .collect();
    for k in &keys {
        data.remove(k);
    }
    write_settings(&data)?;
    Ok(keys.len())
}

/// (key, value in effect, default) of every setting (`config.all_values`).
pub fn all_values() -> Vec<(String, Value, Value)> {
    let stored = read_json(&settings_path());
    known()
        .defaults
        .iter()
        .map(|(k, d)| {
            let v = match stored.get(k) {
                Some(v) if !is_empty(v) => v.clone(),
                _ => d.clone(),
            };
            (k.clone(), v, d.clone())
        })
        .collect()
}

/// The sections in the order chosen: the saved ones first, the others after them in isnady's order.
pub fn sidebar_order() -> Vec<String> {
    let sidebar = &known().sidebar;
    let mut seen: Vec<String> = vec![];
    for k in py_str(&effective("view.sidebar")).split(',') {
        if sidebar.iter().any(|s| s == k) && !seen.iter().any(|s| s == k) {
            seen.push(k.to_string());
        }
    }
    for k in sidebar {
        if !seen.contains(k) {
            seen.push(k.clone());
        }
    }
    seen
}

/// Forget every setting and every remembered place; how many settings were stored (`reset.settings`).
pub fn reset_settings() -> Result<usize, String> {
    let n = reset("")?;
    state_forget("");
    Ok(n)
}

pub fn same_value(a: &Value, b: &Value) -> bool {
    same(a, b)
}

/// A setting as stored in settings.json, None when it is not set (the default holds).
pub fn get(key: &str) -> Option<Value> {
    read_json(&profile_folder().join(SETTINGS_FILE))
        .get(key)
        .cloned()
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

// ------------------------------------------------------------------ profiles (core/profiles.py, PRF1)

/// What a profile holds: its settings, where the reader was, its bookmarks.
pub const PROFILE_FILES: [&str; 3] = ["settings.json", "state.json", "bookmarks.json"];
pub const PROFILE_EXTENSION: &str = ".isnady-profile.zip";
pub const PROFILE_FORMAT: &str = "isnady-profile/1";

fn write_profiles(active: &str, items: Vec<Map<String, Value>>) {
    let mut out = Map::new();
    out.insert("active".into(), Value::from(active));
    out.insert(
        "profiles".into(),
        Value::Array(items.into_iter().map(Value::Object).collect()),
    );
    write_atomically(
        &data_dir().join(PROFILES_FILE),
        &to_python_json(&Value::Object(out)),
    );
}

fn id_of(p: &Map<String, Value>) -> &str {
    p.get("id").and_then(|i| i.as_str()).unwrap_or("")
}

fn name_in(p: &Map<String, Value>) -> &str {
    p.get("name").and_then(|i| i.as_str()).unwrap_or("")
}

/// A profile by its id or its name, case not counted (`profiles.find`).
pub fn find_profile(text: &str) -> Option<String> {
    let want = text.trim().to_lowercase();
    for p in profiles_data().1 {
        if id_of(&p).to_lowercase() == want || name_in(&p).to_lowercase() == want {
            return Some(id_of(&p).to_string());
        }
    }
    if ["default", "varsayılan", "الافتراضي"].contains(&want.as_str()) {
        return Some(DEFAULT_PROFILE.to_string());
    }
    None
}

/// The folder of a profile's files: the data folder for the first, profiles/<id>/ for the others (made).
pub fn folder_of(profile_id: &str) -> PathBuf {
    if profile_id == DEFAULT_PROFILE {
        return data_dir();
    }
    let p = data_dir().join("profiles").join(profile_id);
    let _ = std::fs::create_dir_all(&p);
    p
}

fn new_id(name: &str, taken: &[String]) -> String {
    let folded: String = name
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'ç' => 'c',
            'ğ' => 'g',
            'ı' => 'i',
            'ö' => 'o',
            'ş' => 's',
            'ü' => 'u',
            'â' => 'a',
            'î' => 'i',
            'û' => 'u',
            c => c,
        })
        .collect();
    let mut base = String::new();
    let mut dash = false;
    for c in folded.chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            base.push(c);
            dash = false;
        } else if !dash {
            base.push('-');
            dash = true;
        }
    }
    let base: String = base.trim_matches('-').chars().take(32).collect();
    let base = if base.is_empty() {
        "profile".to_string()
    } else {
        base
    };
    let (mut pid, mut n) = (base.clone(), 2);
    while taken.contains(&pid) || pid == DEFAULT_PROFILE {
        pid = format!("{base}-{n}");
        n += 1;
    }
    pid
}

/// A new profile, its settings copied from another one or isnady's defaults; its id (`profiles.create`).
pub fn create_profile(name: &str, copy_from: Option<&str>) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("A profile needs a name.".into());
    }
    if find_profile(name).is_some() {
        return Err(format!("There is already a profile named {name}."));
    }
    let (active, mut items) = profiles_data();
    let taken: Vec<String> = items.iter().map(|p| id_of(p).to_string()).collect();
    let pid = new_id(name, &taken);
    let mut p = Map::new();
    p.insert("id".into(), Value::from(pid.clone()));
    p.insert("name".into(), Value::from(name));
    items.push(p);
    write_profiles(&active, items);
    let target = folder_of(&pid);
    if let Some(from) = copy_from {
        let source = folder_of(from);
        for f in PROFILE_FILES {
            if source.join(f).exists() {
                let _ = std::fs::copy(source.join(f), target.join(f));
            }
        }
    }
    Ok(pid)
}

pub fn rename_profile(profile_id: &str, name: &str) -> Result<(), String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("A profile needs a name.".into());
    }
    if let Some(other) = find_profile(name) {
        if other != profile_id {
            return Err(format!("There is already a profile named {name}."));
        }
    }
    let (active, mut items) = profiles_data();
    for p in items.iter_mut() {
        if id_of(p) == profile_id {
            p.insert("name".into(), Value::from(name));
            write_profiles(&active, items);
            return Ok(());
        }
    }
    Err(format!("No profile {profile_id}."))
}

/// Remove a profile and its files; never the first one, never the one in use (`profiles.delete`).
pub fn delete_profile(profile_id: &str) -> Result<(), String> {
    if profile_id == DEFAULT_PROFILE {
        return Err(
            "The first profile cannot be deleted: its files are isnady's own data folder.".into(),
        );
    }
    if profile_id == active_profile() {
        return Err("The profile in use cannot be deleted: switch to another one first.".into());
    }
    let (active, items) = profiles_data();
    let n = items.len();
    let left: Vec<Map<String, Value>> = items
        .into_iter()
        .filter(|p| id_of(p) != profile_id)
        .collect();
    if left.len() == n {
        return Err(format!("No profile {profile_id}."));
    }
    write_profiles(&active, left);
    let _ = std::fs::remove_dir_all(data_dir().join("profiles").join(profile_id));
    Ok(())
}

/// One file with the profile's settings, place and bookmarks; the file written (`profiles.export`).
pub fn export_profile(profile_id: &str, target: &str) -> Result<PathBuf, String> {
    use std::io::Write;
    let mut target = crate::paths::expand_user(target);
    if target.is_dir() {
        let mut label = profile_name(profile_id);
        if label.is_empty() {
            label = if profile_id == DEFAULT_PROFILE {
                "default".into()
            } else {
                profile_id.into()
            };
        }
        let mut safe = String::new();
        let mut run = false;
        for c in label.chars() {
            if c.is_alphanumeric() || c == '_' || c == '.' || c == '-' {
                safe.push(c);
                run = false;
            } else if !run {
                safe.push('_');
                run = true;
            }
        }
        target = target.join(format!("{safe}{PROFILE_EXTENSION}"));
    }
    let source = folder_of(profile_id);
    if let Some(parent) = target.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let file = std::fs::File::create(&target).map_err(|e| e.to_string())?;
    let mut z = zip::ZipWriter::new(file);
    let opts =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut files = vec![];
    for f in PROFILE_FILES {
        if let Ok(raw) = std::fs::read(source.join(f)) {
            z.start_file(f, opts).map_err(|e| e.to_string())?;
            z.write_all(&raw).map_err(|e| e.to_string())?;
            files.push(Value::from(f));
        }
    }
    let mut manifest = Map::new();
    manifest.insert("format".into(), Value::from(PROFILE_FORMAT));
    manifest.insert("name".into(), Value::from(profile_name(profile_id)));
    manifest.insert("id".into(), Value::from(profile_id));
    manifest.insert(
        "exported".into(),
        Value::from(crate::py::now_local("%Y-%m-%dT%H:%M:%S")),
    );
    manifest.insert("files".into(), Value::Array(files));
    z.start_file("profile.json", opts)
        .map_err(|e| e.to_string())?;
    z.write_all(to_python_json(&Value::Object(manifest)).as_bytes())
        .map_err(|e| e.to_string())?;
    z.finish().map_err(|e| e.to_string())?;
    Ok(target)
}

/// A profile file made by export, as a new profile (named as in the file, or name); its id.
pub fn import_profile(path: &str, name: Option<&str>) -> Result<String, String> {
    use std::io::Read;
    let path = crate::paths::expand_user(path);
    let fname = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let not_one = |e: String| format!("{fname} is not an isnady profile ({e}).");
    let file = std::fs::File::open(&path).map_err(|e| not_one(e.to_string()))?;
    let mut z = zip::ZipArchive::new(file).map_err(|e| not_one(e.to_string()))?;
    let mut read = |n: &str| -> Option<Vec<u8>> {
        let mut f = z.by_name(n).ok()?;
        let mut buf = vec![];
        f.read_to_end(&mut buf).ok()?;
        Some(buf)
    };
    let manifest: Value = read("profile.json")
        .and_then(|b| serde_json::from_slice(&b).ok())
        .ok_or_else(|| not_one("'There is no item named 'profile.json' in the archive'".into()))?;
    if manifest.get("format").and_then(|f| f.as_str()) != Some(PROFILE_FORMAT) {
        return Err(format!("{fname} is not an isnady profile."));
    }
    let mut contents = vec![];
    for f in PROFILE_FILES {
        if let Some(raw) = read(f) {
            if serde_json::from_slice::<Value>(&raw).is_err() {
                return Err(format!("{fname}: {f} is damaged."));
            }
            contents.push((f, raw));
        }
    }
    let given = name
        .map(String::from)
        .filter(|n| !n.is_empty())
        .or_else(|| {
            manifest
                .get("name")
                .and_then(|n| n.as_str())
                .filter(|n| !n.is_empty())
                .map(String::from)
        })
        .or_else(|| {
            fname
                .split('.')
                .next()
                .filter(|n| !n.is_empty())
                .map(String::from)
        })
        .unwrap_or_else(|| "Imported".into());
    let wanted = given.trim().to_string();
    let (mut fin, mut n) = (wanted.clone(), 2);
    while find_profile(&fin).is_some() {
        fin = format!("{wanted} ({n})");
        n += 1;
    }
    let pid = create_profile(&fin, None)?;
    let target = folder_of(&pid);
    for (f, raw) in contents {
        std::fs::write(target.join(f), raw).map_err(|e| e.to_string())?;
    }
    Ok(pid)
}
