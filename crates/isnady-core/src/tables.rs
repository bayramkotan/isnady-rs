//! The word lists, forms and patterns of the Python isnady, read from `data/tables.json` — written from the Python
//! isnady itself by `tools/gen_tables.py`, never edited by hand, so the two programs read names the same way.

use serde::Deserialize;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::OnceLock;

#[derive(Deserialize)]
pub struct Names {
    /// normalised word (with a shadda when the list keeps one apart) → (Turkish, English)
    pub words: HashMap<String, (String, String)>,
    /// two normalised words, joined by a space → (Turkish, English)
    pub compounds: HashMap<String, (String, String)>,
    pub stops: HashSet<String>,
}

#[derive(Deserialize)]
pub struct Companions {
    pub forms: HashSet<String>,
    pub trim: String,
}

#[derive(Deserialize)]
pub struct Traditions {
    pub group: Vec<String>,
    pub previous: Vec<String>,
    pub vague: Vec<String>,
    pub idda: BTreeMap<String, Vec<String>>,
    pub masum: HashMap<String, String>,
}

#[derive(Deserialize)]
pub struct Isnad {
    /// transmission term → (Arabic, meaning)
    pub terms: HashMap<String, (String, String)>,
    pub templates: HashMap<String, String>,
    pub end: String,
    pub raised: String,
    pub cut: String,
    pub borrowed: String,
    /// language name → code ("Turkish" → "tr")
    pub languages: HashMap<String, String>,
    pub tr_possessive: Vec<String>,
    pub masum_tr: HashMap<String, String>,
    pub honour: String,
    pub kin: HashMap<String, (String, String)>,
    pub kin_alone: HashMap<String, (String, String)>,
    pub kin_third: HashMap<String, String>,
    pub whose: HashSet<String>,
    /// the Turkish of the chain's words (the interface catalog, these texts only)
    pub tr: HashMap<String, String>,
}

#[derive(Deserialize)]
pub struct Rijal {
    pub note_start: String,
    pub note_words: String,
    pub shia_ranks: HashMap<String, String>,
    /// Ibn Hajar's twelve ranks, in English (the interface translates them)
    pub ranks: HashMap<String, String>,
    /// the twelve tabaqat of the Taqrib, in English
    pub tabaqat: HashMap<String, String>,
}

#[derive(Deserialize)]
pub struct NameParts {
    /// the nisbas of places (normalised): shown "(place)"
    pub places: HashSet<String>,
    pub not_names: HashSet<String>,
    pub laqab: String,
    pub known: String,
}

#[derive(Deserialize)]
pub struct Famous {
    /// [key, Turkish, English, Arabic, honour ("m", "f", "2", ""), tradition]
    pub people: Vec<[String; 6]>,
    /// [id, title, tradition, [keys]]
    pub groups: Vec<(String, String, String, Vec<String>)>,
    /// [key, tradition, compact, skeleton]: every spelling of every famous one, folded
    pub keys: Vec<[String; 4]>,
    pub honour: String,
    pub honour_ar: String,
    pub ibn: HashSet<String>,
    pub too_short: HashSet<String>,
}

#[derive(Deserialize)]
pub struct NarratorTables {
    pub book_marks: HashMap<String, String>,
    pub collection_marks: HashMap<String, Vec<String>>,
    pub madhhabs: HashMap<String, String>,
    pub famous: Famous,
}

/// One scholar of hadith (`scholars.SCHOLARS`).
#[derive(Deserialize, Clone, Debug, serde::Serialize)]
pub struct Scholar {
    pub id: String,
    #[serde(default)]
    pub tradition: Option<String>,
    #[serde(default)]
    pub known_ar: String,
    pub tr: String,
    pub name: String,
    pub full: String,
    pub arabic: String,
    pub dates: String,
    pub works: Vec<String>,
    #[serde(default)]
    pub short: Option<String>,
    #[serde(default)]
    pub compiler_of: Vec<String>,
    #[serde(default)]
    pub grader_names: Vec<String>,
    #[serde(default)]
    pub critic_names: Vec<String>,
    #[serde(default)]
    pub taqrib: Option<String>,
    #[serde(default)]
    pub najashi: Option<String>,
}

#[derive(Deserialize)]
pub struct ScholarTables {
    pub list: Vec<Scholar>,
    pub short_tr: HashMap<String, String>,
    /// role → (title, meaning), in English
    pub role_help: HashMap<String, (String, String)>,
}

#[derive(Deserialize)]
pub struct Tables {
    pub parser_id: String,
    pub matcher_id: String,
    pub death_rule: String,
    pub names: Names,
    pub companions: Companions,
    pub traditions: Traditions,
    pub isnad: Isnad,
    pub rijal: Rijal,
    pub narrators: NarratorTables,
    pub name_parts: NameParts,
    pub scholars: ScholarTables,
}

static TABLES: OnceLock<Tables> = OnceLock::new();

pub fn get() -> &'static Tables {
    TABLES.get_or_init(|| {
        serde_json::from_str(include_str!("../data/tables.json")).expect("data/tables.json")
    })
}

/// A pattern of the Python isnady (lookbehind and all), compiled once.
pub fn pattern(p: &str) -> fancy_regex::Regex {
    fancy_regex::Regex::new(p).expect("a pattern of the Python isnady")
}

#[cfg(test)]
mod tests {
    #[test]
    fn tables_read() {
        let t = super::get();
        assert!(t.names.words.len() > 300);
        assert!(t.parser_id.starts_with("isnady-isnad-"));
        super::pattern(&t.companions.trim);
        super::pattern(&t.isnad.honour);
        super::pattern(&t.rijal.note_start);
        super::pattern(&t.rijal.note_words);
        super::pattern(&t.name_parts.laqab);
        super::pattern(&t.name_parts.known);
    }
}
