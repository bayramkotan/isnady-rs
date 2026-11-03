//! The interface in the reader's language — the Python isnady's catalogs (`isnady/assets/i18n/<code>.json`, written
//! here by `tools/gen_tables.py`): the English text is the key, a text a catalog does not have is shown in English.

use crate::config;
use std::collections::HashMap;
use std::sync::OnceLock;

/// The languages with a catalog, besides English.
pub const TRANSLATED: [&str; 2] = ["ar", "tr"];

/// The catalog of a language as JSON ({"English text": "translation"}); "{}" for English or a language without one.
pub fn catalog_json(code: &str) -> &'static str {
    match code {
        "tr" => include_str!("../data/i18n/tr.json"),
        "ar" => include_str!("../data/i18n/ar.json"),
        _ => "{}",
    }
}

/// The interface language chosen (view.ui_language), "en" by default.
pub fn ui_language() -> String {
    let v = config::get_str("view.ui_language", "en");
    if v.is_empty() {
        "en".into()
    } else {
        v
    }
}

/// Written right to left?
pub fn rtl(code: &str) -> bool {
    matches!(code, "ar" | "ur" | "fa" | "prs" | "ps" | "ug")
}

fn catalog(code: &str) -> &'static HashMap<String, String> {
    static TR: OnceLock<HashMap<String, String>> = OnceLock::new();
    static AR: OnceLock<HashMap<String, String>> = OnceLock::new();
    static NONE: OnceLock<HashMap<String, String>> = OnceLock::new();
    let cell = match code {
        "tr" => &TR,
        "ar" => &AR,
        _ => &NONE,
    };
    cell.get_or_init(|| serde_json::from_str(catalog_json(code)).unwrap_or_default())
}

/// A text of the interface in a language, its {fields} filled (`i18n.t` with the language given).
pub fn t(code: &str, text: &str, values: &[(&str, String)]) -> String {
    let template = catalog(code).get(text).map(String::as_str).unwrap_or(text);
    let mut out = template.to_string();
    for (k, v) in values {
        out = out.replace(&format!("{{{k}}}"), v);
    }
    out
}
