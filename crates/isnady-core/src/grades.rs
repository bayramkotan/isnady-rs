//! Grades as the scholars gave them, grouped so that they can be compared, and the search by grade (GR-FLT):
//! the Python isnady's `core/grades.py`, word for word.
//!
//! The wording is always kept and shown; the GROUP is only for counting, comparing and choosing. The Sunni groups
//! follow the classical order (4 sahih, 3 hasan, 2 da'if, 1 very weak, 0 fabricated); the Imami categories of the
//! Shia books have their own scale (sahih, hasan, muwaththaq, da'if) and are NEVER mapped onto the Sunni ones.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use fancy_regex::Regex as FancyRegex;
use regex::Regex;
use rusqlite::Connection;

/// The Sunni group of a grade: 4 sahih (also "hasan sahih"), 3 hasan, 2 da'if, 1 very weak, 0 fabricated; None when
/// the wording fits none (`grades.group`).
pub fn group(grade: &str) -> Option<i32> {
    static RES: OnceLock<[Regex; 5]> = OnceLock::new();
    let [fabricated, very_weak, weak, sahih, hasan] = RES.get_or_init(|| {
        [
            Regex::new(r" (mawdu|mawdoo|maudu|fabricated) ").unwrap(),
            Regex::new(r" (very daif|daif jiddan|da if jiddan|munkar|matruk) ").unwrap(),
            Regex::new(r" (daif|da if|zaif|shadh|shaadh|sanad daif) ").unwrap(),
            Regex::new(r" sahih ").unwrap(),
            Regex::new(r" hasan ").unwrap(),
        ]
    });
    let lowered: String = grade
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_lowercase() || c == ' ' {
                c
            } else {
                ' '
            }
        })
        .collect();
    let g = format!(" {lowered} ");
    if fabricated.is_match(&g) {
        Some(0)
    } else if very_weak.is_match(&g) {
        Some(1)
    } else if weak.is_match(&g) {
        Some(2)
    } else if sahih.is_match(&g) {
        Some(4) // " hasan sahih " is sahih too
    } else if hasan.is_match(&g) {
        Some(3)
    } else {
        None
    }
}

const IMAMI_WORDS: [(&str, i32); 8] = [
    ("صحيح", 4),
    ("حسن", 3),
    ("موثق", 2),
    ("ضعيف", 1),
    ("مجهول", 1),
    ("مهول", 1),
    ("مرسل", 1),
    ("مرفوع", 1),
];

/// The Imami category of a grade as al-Majlisi or al-Behbudi wrote it, by its first word of grade
/// (`grades.imami_group`): "حسن كالصحيح" is hasan, "ضعيف على المشهور" da'if; "كالسابق" none.
pub fn imami_group(grade: &str) -> Option<i32> {
    static RE: OnceLock<FancyRegex> = OnceLock::new();
    static MARKS: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        let words: Vec<&str> = IMAMI_WORDS.iter().map(|(w, _)| *w).collect();
        FancyRegex::new(&format!(r"(?<![\w])({})(?![\w])", words.join("|"))).unwrap()
    });
    let marks = MARKS.get_or_init(|| Regex::new("[\u{064B}-\u{0652}\u{0670}]").unwrap());
    let bare = marks.replace_all(grade, "");
    let found = re.captures(&bare).ok().flatten()?;
    let word = found.get(1)?.as_str();
    IMAMI_WORDS
        .iter()
        .find(|(w, _)| *w == word)
        .map(|(_, g)| *g)
}

/// (code, group, label) of the Sunni choices, in their order.
pub const SUNNI_FILTERS: [(&str, Option<i32>, &str); 6] = [
    ("sahih", Some(4), "sahih"),
    ("hasan", Some(3), "hasan"),
    ("daif", Some(2), "da'if"),
    ("very-weak", Some(1), "very weak"),
    ("fabricated", Some(0), "fabricated"),
    ("ungraded", None, "not graded"),
];

/// (code, category, label) of the Shia choices, in their own scale.
pub const SHIA_FILTERS: [(&str, Option<i32>, &str); 5] = [
    ("imami-sahih", Some(4), "sahih"),
    ("imami-hasan", Some(3), "hasan"),
    ("muwaththaq", Some(2), "muwaththaq"),
    ("imami-daif", Some(1), "da'if"),
    ("imami-ungraded", None, "not graded"),
];

/// Every code `--grade` takes, Sunni then Shia.
pub fn filter_codes() -> Vec<&'static str> {
    SUNNI_FILTERS
        .iter()
        .chain(SHIA_FILTERS.iter())
        .map(|(c, _, _)| *c)
        .collect()
}

/// The two books no grader grades: sahih by their compiler's condition and the community's acceptance.
pub const SAHIHAYN: [&str; 2] = ["bukhari", "muslim"];

/// The filter code of one grade in its own tradition's scale ("sahih", "imami-sahih", "muwaththaq" …).
pub fn code_of(grade: &str, tradition: Option<&str>) -> Option<&'static str> {
    if tradition.unwrap_or("sunni") == "shia" {
        let g = imami_group(grade)?;
        SHIA_FILTERS
            .iter()
            .find(|(_, x, _)| *x == Some(g))
            .map(|(c, _, _)| *c)
    } else {
        let g = group(grade)?;
        SUNNI_FILTERS
            .iter()
            .find(|(_, x, _)| *x == Some(g))
            .map(|(c, _, _)| *c)
    }
}

/// Does a hadith with these grade codes pass the choice (`grades.passes`)? every: each grade it was given is among
/// those chosen (the graders agree), else one of them is enough.
pub fn passes(
    codes: &[Option<&str>],
    tradition: Option<&str>,
    book: &str,
    wanted: &HashSet<String>,
    every: bool,
    sahihayn: bool,
) -> bool {
    let shia = tradition.unwrap_or("sunni") == "shia";
    let known: Vec<&str> = codes.iter().flatten().copied().collect();
    if known.is_empty() {
        if sahihayn && !shia && SAHIHAYN.contains(&book) && codes.is_empty() {
            return wanted.contains("sahih");
        }
        return wanted.contains(if shia { "imami-ungraded" } else { "ungraded" });
    }
    if every {
        known.iter().all(|c| wanted.contains(*c))
    } else {
        known.iter().any(|c| wanted.contains(*c))
    }
}

/// The hadith that pass the choice, in every book (`grades.allowed_ids`) — kept for the last choice while the
/// database is unchanged, as the Python isnady keeps it (core.cache): a window searching page after page counts once.
pub fn allowed_ids(
    conn: &Connection,
    wanted: &[String],
    every: bool,
    sahihayn: bool,
) -> rusqlite::Result<HashSet<i64>> {
    use std::sync::Mutex;
    type Kept = Option<(String, HashSet<i64>)>;
    static LAST: Mutex<Kept> = Mutex::new(None);
    let mut sorted: Vec<&String> = wanted.iter().collect();
    sorted.sort();
    let version: i64 = conn.query_row("PRAGMA data_version", [], |r| r.get(0))?;
    let changes = conn.changes();
    let key = format!(
        "{:?}|{every}|{sahihayn}|{version}|{changes}|{:?}",
        sorted,
        conn.path()
    );
    if let Ok(last) = LAST.lock() {
        if let Some((k, set)) = last.as_ref() {
            if *k == key {
                return Ok(set.clone());
            }
        }
    }
    let set = count_allowed(conn, wanted, every, sahihayn)?;
    if let Ok(mut last) = LAST.lock() {
        *last = Some((key, set.clone()));
    }
    Ok(set)
}

fn count_allowed(
    conn: &Connection,
    wanted: &[String],
    every: bool,
    sahihayn: bool,
) -> rusqlite::Result<HashSet<i64>> {
    let wanted: HashSet<String> = wanted.iter().cloned().collect();
    let mut codes: HashMap<i64, Vec<Option<&'static str>>> = HashMap::new();
    let mut stmt = conn.prepare(
        "SELECT g.hadith_id, c.tradition, g.grade FROM grades g JOIN hadiths h ON h.id = g.hadith_id \
         JOIN collections c ON c.id = h.collection_id",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, Option<String>>(1)?,
            r.get::<_, Option<String>>(2)?,
        ))
    })?;
    for row in rows {
        let (hid, tradition, grade) = row?;
        codes.entry(hid).or_default().push(code_of(
            grade.as_deref().unwrap_or(""),
            tradition.as_deref(),
        ));
    }
    let mut out = HashSet::new();
    let mut stmt = conn.prepare(
        "SELECT h.id, c.key, c.tradition FROM hadiths h JOIN collections c ON c.id = h.collection_id",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, Option<String>>(2)?,
        ))
    })?;
    let none = vec![];
    for row in rows {
        let (hid, key, tradition) = row?;
        let given = codes.get(&hid).unwrap_or(&none);
        if passes(given, tradition.as_deref(), &key, &wanted, every, sahihayn) {
            out.insert(hid);
        }
    }
    Ok(out)
}

/// "sahih, hasan (Sunni) · sahih (Shia)" — the choice in words (`grades.describe_choice`), each word through tr.
pub fn describe_choice(wanted: &[String], every: bool, tr: &dyn Fn(&str) -> String) -> String {
    let mut parts = vec![];
    for (filters, road) in [(&SUNNI_FILTERS[..], "Sunni"), (&SHIA_FILTERS[..], "Shia")] {
        let names: Vec<String> = filters
            .iter()
            .filter(|(c, _, _)| wanted.iter().any(|w| w == c))
            .map(|(_, _, label)| tr(label))
            .collect();
        if !names.is_empty() {
            parts.push(format!("{} ({})", names.join(", "), tr(road)));
        }
    }
    let text = parts.join(" · ");
    if every && !parts.is_empty() {
        format!("{text} — {}", tr("every grader agrees"))
    } else {
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sunni_groups() {
        assert_eq!(group("Sahih"), Some(4));
        assert_eq!(group("Hasan Sahih"), Some(4));
        assert_eq!(group("Isnaad Hasan"), Some(3));
        assert_eq!(group("Da'if Jiddan"), Some(1));
        assert_eq!(group("Mawdu'"), Some(0));
        assert_eq!(group("Shadh"), Some(2));
        assert_eq!(group("?"), None);
    }

    #[test]
    fn imami_categories() {
        assert_eq!(imami_group("حسن كالصحيح"), Some(3));
        assert_eq!(imami_group("ضَعِيفٌ على المشهور"), Some(1));
        assert_eq!(imami_group("موثق"), Some(2));
        assert_eq!(imami_group("كالسابق"), None);
        assert_eq!(imami_group("الصحيحة"), None);
    }

    #[test]
    fn choosing() {
        let wanted: HashSet<String> = ["sahih".to_string()].into();
        assert!(passes(&[], Some("sunni"), "bukhari", &wanted, false, true));
        assert!(!passes(
            &[],
            Some("sunni"),
            "bukhari",
            &wanted,
            false,
            false
        ));
        assert!(passes(
            &[Some("sahih"), Some("daif")],
            None,
            "abudawud",
            &wanted,
            false,
            true
        ));
        assert!(!passes(
            &[Some("sahih"), Some("daif")],
            None,
            "abudawud",
            &wanted,
            true,
            true
        ));
        assert_eq!(
            describe_choice(
                &["hasan".into(), "sahih".into(), "muwaththaq".into()],
                true,
                &|s| s.to_string()
            ),
            "sahih, hasan (Sunni) · muwaththaq (Shia) — every grader agrees"
        );
    }
}
