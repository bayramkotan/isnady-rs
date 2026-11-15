//! How isnady identifies the narrators of a chain, every way of it with its parameters (core/narrator_rules.py,
//! 0.8.0): Tools → Narrator Identification and `iy-rs narrator-rules`. The ways, their defaults and ranges and their
//! texts come from the Python isnady (`gen_tables.py` → isnad_parser.json); the values are kept in the database
//! (core_meta 'narrator_rules', only what differs from the defaults, compact JSON with sorted keys) — the two
//! programs read and write the same text.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

use super::matcher::m;

#[derive(Deserialize, Serialize)]
pub struct ParamDef {
    pub name: String,
    pub default: Value,
    pub lo: f64,
    pub hi: f64,
    pub step: f64,
}

#[derive(Deserialize, Serialize)]
pub struct RuleDef {
    pub key: String,
    pub group: String,
    pub switch: bool,
    pub params: Vec<ParamDef>,
}

#[derive(Deserialize, Serialize)]
pub struct RuleTexts {
    pub titles: HashMap<String, String>,
    pub explained: HashMap<String, String>,
    pub parameters: HashMap<String, String>,
    pub groups: HashMap<String, String>,
}

/// A value of a way: on or off, a whole number, a number with decimals.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum V {
    B(bool),
    I(i64),
    F(f64),
}

impl V {
    pub fn json(&self) -> String {
        match self {
            V::B(b) => b.to_string(),
            V::I(i) => i.to_string(),
            V::F(f) => serde_json::to_string(f).unwrap_or_default(),
        }
    }
    /// As the command shows it: 9, 0.7, 3 (Python's "{:g}")
    pub fn shown(&self) -> String {
        match self {
            V::B(b) => if *b { "on" } else { "off" }.into(),
            V::I(i) => i.to_string(),
            V::F(f) => f.to_string(),
        }
    }
}

pub type Rules = BTreeMap<String, BTreeMap<String, V>>;

pub fn defs() -> &'static [RuleDef] {
    &m().rules
}

pub fn texts() -> &'static RuleTexts {
    &m().rule_texts
}

fn def(key: &str) -> Option<&'static RuleDef> {
    defs().iter().find(|d| d.key == key)
}

fn default_of(p: &ParamDef) -> V {
    match p.default.as_i64() {
        Some(i) if !p.default.is_f64() => V::I(i),
        _ => V::F(p.default.as_f64().unwrap_or(0.0)),
    }
}

/// Every way with its parameters as isnady ships it (`defaults`).
pub fn defaults() -> Rules {
    defs()
        .iter()
        .map(|d| {
            let mut v = BTreeMap::new();
            v.insert("on".to_string(), V::B(true));
            for p in &d.params {
                v.insert(p.name.clone(), default_of(p));
            }
            (d.key.clone(), v)
        })
        .collect()
}

/// The values kept in the database ("" when nothing differs from the defaults).
pub fn stored(conn: &Connection) -> String {
    let v: Option<String> = conn
        .query_row(
            "SELECT value FROM core_meta WHERE key = 'narrator_rules'",
            [],
            |r| r.get(0),
        )
        .optional()
        .ok()
        .flatten();
    match v {
        Some(s) if !s.is_empty() && s != "{}" => s,
        _ => String::new(),
    }
}

/// A value made right for its parameter (`_fit`): the type of the default, within its range, half up, two decimals.
fn fit(key: &str, name: &str, value: &Value) -> Option<V> {
    let d = def(key)?;
    if name == "on" {
        return match value {
            Value::Bool(b) if d.switch => Some(V::B(*b)),
            _ => None,
        };
    }
    let p = d.params.iter().find(|p| p.name == name)?;
    if value.is_boolean() {
        return None;
    }
    let x = value.as_f64()?;
    fit_number(p, x)
}

fn fit_number(p: &ParamDef, x: f64) -> Option<V> {
    if !x.is_finite() {
        return None;
    }
    let x = x.max(p.lo).min(p.hi);
    Some(match default_of(p) {
        V::I(_) => V::I((x + 0.5).floor() as i64),
        _ => V::F((x * 100.0 + 0.5).floor() / 100.0),
    })
}

fn to_value(v: &V) -> Value {
    match v {
        V::B(b) => Value::Bool(*b),
        V::I(i) => Value::from(*i),
        V::F(f) => Value::from(*f),
    }
}

/// The rules in force: the defaults with the database's values over them (`load`).
pub fn load(conn: &Connection) -> Rules {
    let mut out = defaults();
    let text = stored(conn);
    if text.is_empty() {
        return out;
    }
    let Ok(Value::Object(changed)) = serde_json::from_str::<Value>(&text) else {
        return out;
    };
    for (key, values) in &changed {
        let Value::Object(values) = values else {
            continue;
        };
        if !out.contains_key(key) {
            continue;
        }
        for (name, value) in values {
            if let Some(fixed) = fit(key, name, value) {
                out.get_mut(key).unwrap().insert(name.clone(), fixed);
            }
        }
    }
    out
}

/// What differs from the defaults, as kept in the database (`canonical`): sorted keys, no spaces ("" for none).
pub fn canonical(rules: &Rules) -> String {
    let base = defaults();
    let mut parts = Vec::new();
    for (key, values) in rules {
        let Some(b) = base.get(key) else { continue };
        let mut inner = Vec::new();
        for (name, v) in values {
            let Some(bv) = b.get(name) else { continue };
            let Some(v) = fit(key, name, &to_value(v)) else {
                continue;
            };
            if v != *bv {
                inner.push(format!(
                    "{}:{}",
                    serde_json::to_string(name).unwrap(),
                    v.json()
                ));
            }
        }
        if !inner.is_empty() {
            parts.push(format!(
                "{}:{{{}}}",
                serde_json::to_string(key).unwrap(),
                inner.join(",")
            ));
        }
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!("{{{}}}", parts.join(","))
    }
}

pub fn save(conn: &Connection, rules: &Rules) -> rusqlite::Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS core_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
        [],
    )?;
    let text = canonical(rules);
    if text.is_empty() {
        conn.execute("DELETE FROM core_meta WHERE key = 'narrator_rules'", [])?;
    } else {
        conn.execute(
            "INSERT OR REPLACE INTO core_meta (key, value) VALUES ('narrator_rules', ?)",
            params![text],
        )?;
    }
    Ok(())
}

/// The window's dialog saved: the guesses on or off, and the ways kept.
pub fn save_with_guess(conn: &Connection, rules: &Rules, guess: bool) -> rusqlite::Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS core_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
        [],
    )?;
    conn.execute(
        "INSERT OR REPLACE INTO core_meta (key, value) VALUES ('narrator_guess', ?)",
        [if guess { "on" } else { "off" }],
    )?;
    save(conn, rules)
}

/// One setting as the command writes it — "famous=off", "neighbours.rounds=4" — put into rules (`apply`).
pub fn apply(rules: &mut Rules, setting: &str) -> Option<String> {
    let Some((left, value)) = setting.split_once('=') else {
        return Some(format!("not KEY=on|off or KEY.PARAMETER=VALUE: {setting}"));
    };
    let (left, value) = (py_strip(left), py_strip(value));
    let (key, name) = match left.split_once('.') {
        Some((k, n)) => (k, n),
        None => (left, ""),
    };
    let Some(d) = def(key) else {
        return Some(format!("no such way: {key} (see iy narrator-rules)"));
    };
    if name.is_empty() {
        if !d.switch {
            return Some(format!("{key} cannot be turned off"));
        }
        if value != "on" && value != "off" {
            return Some(format!("{key}: on or off, not {value}"));
        }
        rules
            .get_mut(key)
            .unwrap()
            .insert("on".into(), V::B(value == "on"));
        return None;
    }
    let Some(p) = d.params.iter().find(|p| p.name == name) else {
        return Some(format!("{key} has no parameter {name}"));
    };
    let number = py_float(value).unwrap_or(f64::NAN);
    match fit_number(p, number) {
        Some(v) => {
            rules.get_mut(key).unwrap().insert(name.into(), v);
            None
        }
        None => Some(format!("{key}.{name}: a number, not {value}")),
    }
}

fn py_strip(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace())
}

/// Python's float(): "4", " 2.5 ", "1e1", "inf" (not a number here: refused by fit_number).
fn py_float(s: &str) -> Option<f64> {
    let t = py_strip(s).replace('_', "");
    if t.is_empty() {
        return None;
    }
    let lower = t.to_ascii_lowercase();
    let body = lower.trim_start_matches(['+', '-']);
    if matches!(body, "inf" | "infinity" | "nan") {
        return Some(f64::NAN);
    }
    t.parse::<f64>().ok()
}

/// How many links each way decided at the last identification: {"sunni": {key: n}, "shia": {...}}.
pub fn counts(conn: &Connection) -> BTreeMap<String, BTreeMap<String, i64>> {
    conn.query_row(
        "SELECT value FROM core_meta WHERE key = 'narrator_rule_counts'",
        [],
        |r| r.get::<_, String>(0),
    )
    .ok()
    .and_then(|s| serde_json::from_str(&s).ok())
    .unwrap_or_default()
}

pub fn save_counts(
    conn: &Connection,
    found: &BTreeMap<String, BTreeMap<String, i64>>,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO core_meta (key, value) VALUES ('narrator_rule_counts', ?)",
        params![serde_json::to_string(found).unwrap_or_default()],
    )?;
    Ok(())
}

/// The value of a way's parameter in force.
pub fn on(rules: &Rules, key: &str) -> bool {
    matches!(rules.get(key).and_then(|v| v.get("on")), Some(V::B(true)))
}

pub fn int(rules: &Rules, key: &str, name: &str) -> i64 {
    match rules.get(key).and_then(|v| v.get(name)) {
        Some(V::I(i)) => *i,
        Some(V::F(f)) => *f as i64,
        _ => 0,
    }
}

pub fn float(rules: &Rules, key: &str, name: &str) -> f64 {
    match rules.get(key).and_then(|v| v.get(name)) {
        Some(V::F(f)) => *f,
        Some(V::I(i)) => *i as f64,
        _ => 0.0,
    }
}

/// The rules as JSON for the window: {key: {"on": true, parameter: value}}.
pub fn to_json(rules: &Rules) -> Value {
    Value::Object(
        rules
            .iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    Value::Object(v.iter().map(|(n, x)| (n.clone(), to_value(x))).collect()),
                )
            })
            .collect(),
    )
}

/// The rules the window sends back, made right (each value through `fit`).
pub fn from_json(value: &Value) -> Rules {
    let mut out = defaults();
    if let Value::Object(map) = value {
        for (key, values) in map {
            if let (Some(slot), Value::Object(values)) = (out.get_mut(key), values) {
                for (name, v) in values {
                    if let Some(fixed) = fit(key, name, v) {
                        slot.insert(name.clone(), fixed);
                    }
                }
            }
        }
    }
    out
}
