//! The scholars of hadith in isnady — the Python isnady's `core/scholars.py`: who they are (the same register,
//! written into `data/tables.json` by tools/gen_tables.py), the roles the imported data gives each (compiler, grader,
//! critic), and finding them by any spelling of a name or a work. Sunni and Shia scholars stand on the same list,
//! each by his own road; the measures of their work come with the Hadith Scholars page (RS5).

use rusqlite::{Connection, OptionalExtension};
use std::collections::{HashMap, HashSet};
use unicode_normalization::char::canonical_combining_class;
use unicode_normalization::UnicodeNormalization;

use crate::normalize::normalize;
use crate::people::{fold_latin, loose, near};
use crate::tables::{self, Scholar};
use crate::{i18n, name_parts, py};

/// Every scholar of the register, Sunni then Shia, in its order.
pub fn all() -> &'static [Scholar] {
    &tables::get().scholars.list
}

pub fn find(id: &str) -> Option<&'static Scholar> {
    all().iter().find(|s| s.id == id)
}

pub fn tradition_of(s: &Scholar) -> &str {
    s.tradition.as_deref().unwrap_or("sunni")
}

/// What a role is called and what it means, in English (`ROLE_HELP`).
pub fn role_help(role: &str) -> (String, String) {
    tables::get()
        .scholars
        .role_help
        .get(role)
        .cloned()
        .unwrap_or_default()
}

/// A scholar's short name in a language: "al-Albani", "Elbânî", "الألباني" (`scholars.short_name`).
pub fn short_name(s: &Scholar, lang: &str) -> String {
    if lang == "tr" {
        if let Some(n) = tables::get().scholars.short_tr.get(&s.id) {
            return n.clone();
        }
        if !s.tr.is_empty() {
            return s.tr.clone();
        }
        return s.short.clone().unwrap_or_else(|| s.name.clone());
    }
    if ["ar", "fa", "prs", "ps", "ur", "ug"].contains(&lang) && !s.known_ar.is_empty() {
        return s.known_ar.clone();
    }
    s.short.clone().unwrap_or_else(|| s.name.clone())
}

/// The scholar's name laid out like a narrator's (`scholars.name_view`): reading names in `reading`, the
/// interface in `lang`.
pub fn name_view(s: &Scholar, reading: &str, lang: &str) -> name_parts::View {
    let known = (!s.known_ar.is_empty()).then_some(s.known_ar.as_str());
    let mut view = name_parts::present(
        &name_parts::parse(&s.arabic, None, &[], None, known),
        reading,
        lang,
    );
    if view.reading.is_empty() {
        view.reading = short_name(s, lang);
    }
    if py::words(&view.full_reading).len() < 2 {
        view.full_reading = if lang == "tr" && !s.tr.is_empty() {
            s.tr.clone()
        } else {
            s.full.clone()
        };
    }
    view
}

fn column(conn: &Connection, sql: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    if let Ok(mut st) = conn.prepare(sql) {
        if let Ok(rows) = st.query_map([], |r| r.get::<_, Option<String>>(0)) {
            out.extend(rows.flatten().flatten());
        }
    }
    out
}

/// The roles the data of this database gives each scholar: id → ["compiler", "grader", "critic"] (`present`).
pub fn roles(conn: &Connection) -> HashMap<String, Vec<String>> {
    let books = column(conn, "SELECT key FROM collections");
    let graders = column(conn, "SELECT DISTINCT grader_name FROM grades");
    let critics = column(conn, "SELECT DISTINCT critic_name FROM verdicts");
    let mut out = HashMap::new();
    for s in all() {
        let mut r = Vec::new();
        if s.compiler_of.iter().any(|b| books.contains(b)) {
            r.push("compiler".to_string());
        }
        if s.grader_names.iter().any(|g| graders.contains(g)) {
            r.push("grader".to_string());
        }
        if s.critic_names.iter().any(|c| critics.contains(c)) {
            r.push("critic".to_string());
        }
        if !r.is_empty() {
            out.insert(s.id.clone(), r);
        }
    }
    out
}

/// Lower case, without accents or the marks of transliteration, Arabic normalized (`scholars._fold`).
fn fold(text: &str) -> String {
    normalize(text)
        .to_lowercase()
        .nfkd()
        .filter(|c| canonical_combining_class(*c) == 0)
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect()
}

/// A scholar found by Search: the register's entry, his roles here, found by a close spelling.
#[derive(Debug, Clone)]
pub struct Found {
    pub scholar: &'static Scholar,
    pub roles: Vec<String>,
    pub close: bool,
}

fn names_of(s: &Scholar) -> String {
    [
        s.name.as_str(),
        s.full.as_str(),
        s.tr.as_str(),
        s.arabic.as_str(),
        s.known_ar.as_str(),
    ]
    .join(" ")
}

/// The scholars whose names or works contain every word typed, those named first and those with something in this
/// database before the others (`scholars.search`).
pub fn search(conn: &Connection, text: &str) -> Vec<Found> {
    let words: Vec<String> = py::words(&fold(text))
        .into_iter()
        .map(|w| {
            let x = w.strip_prefix("al").unwrap_or(w);
            let x = x.strip_prefix("el").unwrap_or(x);
            if x.is_empty() { w } else { x }.to_string()
        })
        .filter(|w| !["b", "bn", "ibn", "bin", "ب", "بن"].contains(&w.as_str()))
        .collect();
    if words.is_empty() {
        return Vec::new();
    }
    let loose_words: Vec<String> = py::words(&fold_latin(text))
        .into_iter()
        .map(String::from)
        .collect();
    let fits = |hay: &str| -> bool {
        let exact = fold(hay).replace(' ', "");
        if words.iter().all(|w| exact.contains(w.as_str())) {
            return true;
        }
        if loose_words.is_empty() {
            return false;
        }
        let folded = fold_latin(&hay.replace('-', " ")).replace(' ', "");
        loose_words.iter().all(|w| folded.contains(w.as_str()))
    };
    let have = roles(conn);
    let mut found: Vec<(Found, bool)> = Vec::new();
    for s in all() {
        let names = names_of(s);
        if fits(&format!("{names} {}", s.works.join(" "))) {
            found.push((
                Found {
                    scholar: s,
                    roles: have.get(&s.id).cloned().unwrap_or_default(),
                    close: false,
                },
                fits(&names),
            ));
        }
    }
    if found.is_empty() {
        found = close_spellings(text, &have);
    }
    found.sort_by_key(|(f, in_name)| (!*in_name, f.roles.is_empty()));
    found.into_iter().map(|(f, _)| f).collect()
}

/// Nothing found as typed: every query word close to a word of his names or works (`scholars._close_spellings`).
fn close_spellings(text: &str, have: &HashMap<String, Vec<String>>) -> Vec<(Found, bool)> {
    let words = |hay: &str| -> Vec<String> {
        let latin = fold_latin(&hay.replace('-', " "));
        let folded = fold(hay);
        py::words(&latin)
            .into_iter()
            .chain(py::words(&folded))
            .filter(|w| !w.is_empty())
            .map(loose)
            .collect()
    };
    let fits = |q: &str, ws: &[String]| -> bool {
        let qn = q.chars().count();
        ws.iter().any(|w| {
            w.starts_with(q)
                || (qn >= 4 && (near(q, w) || near(q, &w.chars().take(qn).collect::<String>())))
        })
    };
    let pick = |t: String| -> Vec<String> {
        py::words(&t)
            .into_iter()
            .map(loose)
            .filter(|q| q.chars().count() >= 3)
            .collect()
    };
    let mut wanted = pick(fold_latin(text));
    if wanted.is_empty() {
        wanted = pick(fold(text));
    }
    if wanted.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for s in all() {
        let names = names_of(s);
        let name_words = words(&names);
        let all_words = words(&format!("{names} {}", s.works.join(" ")));
        if wanted.iter().all(|q| fits(q, &all_words)) {
            out.push((
                Found {
                    scholar: s,
                    roles: have.get(&s.id).cloned().unwrap_or_default(),
                    close: true,
                },
                wanted.iter().all(|q| fits(q, &name_words)),
            ));
        }
    }
    if out.iter().any(|(_, in_name)| *in_name) {
        out.retain(|(_, in_name)| *in_name);
    }
    out
}

const IBN: [&str; 7] = ["b", "bn", "ibn", "bin", "ب", "بن", "ابن"];

/// A name as one string of letters, without the article and without "ibn" (`scholars._squeeze`).
fn squeeze(text: &str) -> String {
    if text.chars().any(|c| ('\u{0600}'..='\u{06ff}').contains(&c)) {
        let n = normalize(text);
        return py::words(&n)
            .into_iter()
            .map(|w| {
                if w.chars().count() > 4 {
                    w.strip_prefix("ال").unwrap_or(w)
                } else {
                    w
                }
            })
            .filter(|w| !IBN.contains(w))
            .collect();
    }
    fold_latin(text).replace(' ', "")
}

/// The scholars KNOWN by the words typed: the name he is known by begins with them (`scholars.named`).
pub fn named(text: &str) -> Vec<&'static Scholar> {
    let wanted = squeeze(text);
    if wanted.chars().count() < 3 {
        return Vec::new();
    }
    let known = |s: &Scholar| -> Vec<String> {
        [s.name.as_str(), s.tr.as_str(), s.known_ar.as_str()]
            .into_iter()
            .filter(|n| !n.is_empty())
            .map(squeeze)
            .collect()
    };
    let exact: Vec<&Scholar> = all()
        .iter()
        .filter(|s| known(s).iter().any(|n| n.starts_with(&wanted)))
        .collect();
    if !exact.is_empty() {
        return exact;
    }
    let wanted = loose(&wanted);
    all()
        .iter()
        .filter(|s| known(s).iter().any(|n| loose(n).starts_with(&wanted)))
        .collect()
}

/// His entry as a narrator in his own tradition's rijal work (the Taqrib, or al-Najashi), when there is one.
pub fn as_narrator(conn: &Connection, s: &Scholar) -> Option<i64> {
    let prefix = s.taqrib.as_deref().or(s.najashi.as_deref())?;
    if prefix.is_empty() {
        return None;
    }
    conn.query_row(
        "SELECT id FROM persons WHERE name_ar LIKE ? AND COALESCE(tradition, 'sunni') = ? ORDER BY id LIMIT 1",
        rusqlite::params![format!("{prefix}%"), tradition_of(s)],
        |r| r.get(0),
    )
    .optional()
    .ok()
    .flatten()
}

/// "Works: …" and the role's title, in the interface language.
pub fn role_title(role: &str, lang: &str) -> String {
    i18n::t(lang, &role_help(role).0, &[])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_ones() {
        let ids = |t: &str| named(t).iter().map(|s| s.id.clone()).collect::<Vec<_>>();
        assert_eq!(ids("Bukhari"), vec!["bukhari"]);
        assert_eq!(ids("Buhârî"), vec!["bukhari"]);
        assert_eq!(ids("ابن حجر"), vec!["ibnhajar"]);
        assert!(ids("Anas").is_empty());
        assert_eq!(short_name(find("albani").unwrap(), "tr"), "Elbânî");
    }
}
