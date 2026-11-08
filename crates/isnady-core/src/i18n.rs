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

/// What the frame of the window needs, written from the Python isnady by tools/gen_tables.py (data/window.json): the
/// sections, the themes, the interface languages and their names, the shortcuts, the search tips.
pub fn window_json() -> &'static str {
    include_str!("../data/window.json")
}

/// The interface languages isnady knows: (code, its own name, its English name, right to left).
pub fn languages() -> &'static Vec<(String, String, String, bool)> {
    static LIST: OnceLock<Vec<(String, String, String, bool)>> = OnceLock::new();
    LIST.get_or_init(|| {
        let v: serde_json::Value = serde_json::from_str(window_json()).expect("data/window.json");
        v["languages"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|l| {
                        (
                            l[0].as_str().unwrap_or("").to_string(),
                            l[1].as_str().unwrap_or("").to_string(),
                            l[2].as_str().unwrap_or("").to_string(),
                            l[3].as_bool().unwrap_or(false),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default()
    })
}

/// The interface language chosen (view.ui_language): one isnady knows, else "en" (`language.ui_language`).
pub fn ui_language() -> String {
    let v = config::get_str("view.ui_language", "en");
    if languages().iter().any(|l| l.0 == v) {
        v
    } else {
        "en".into()
    }
}

/// Written right to left?
pub fn rtl(code: &str) -> bool {
    languages().iter().any(|l| l.0 == code && l.3)
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

/// The abbreviations of a date written in English ("d. 279 AH (892 CE)") in a language (`i18n.dates`).
pub fn dates(code: &str, text: &str) -> String {
    static RE: std::sync::OnceLock<fancy_regex::Regex> = std::sync::OnceLock::new();
    if text.is_empty() || code == "en" {
        return text.to_string();
    }
    let re =
        RE.get_or_init(|| fancy_regex::Regex::new(r"(?<![\w.])(d\.|c\.|AH|CE)(?![\w])").unwrap());
    re.replace_all(text, |c: &fancy_regex::Captures| t(code, &c[1], &[]))
        .into_owned()
}
