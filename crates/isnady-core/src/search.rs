//! Hadith text search — the Python isnady's `isnady/core/search.py`, rule for rule, over the same search index
//! (`text_norm`, the FTS5 trigram index `texts_fts` and the word list), built here as it builds it
//! (`ensure_index`, `rebuild_index`).

use regex::Regex;
use rusqlite::{params_from_iter, Connection};
use std::collections::{HashMap, HashSet};
use std::time::Instant;

use crate::books;
use crate::normalize::{normalize, normalize_with_map, NORMALIZER_VERSION};

pub const MODES: [&str; 3] = ["all", "any", "phrase"];
const MIN_TRIGRAM: usize = 3;
pub const NEAR_MAX: usize = 30;
pub const NEAR_MIN_LENGTH: usize = 3;
const WILDCARDS: [char; 3] = ['?', '؟', '*'];
const PUNCT: &str = "«»“”„\"‘’:;؛،,.!()[]{}<>…–—-/\\|~﴿﴾";

#[derive(Debug, Clone)]
pub struct SearchQuery {
    pub text: String,
    pub mode: String,
    pub whole_words: bool,
    pub collections: Vec<String>,
    pub languages: Vec<String>,
    /// grade codes (grades::filter_codes); empty = every grade
    pub grades: Vec<String>,
    /// with grades: each grade the hadith was given among those chosen
    pub every_grader: bool,
    /// with grades: al-Bukhari's and Muslim's own hadith count as sahih
    pub sahihayn: bool,
    /// chapter ids (work_nodes): only the hadith in them and in the chapters under them
    pub chapters: Vec<i64>,
    pub limit: usize,
    pub offset: usize,
}

impl SearchQuery {
    pub fn new(text: &str) -> Self {
        SearchQuery {
            text: text.to_string(),
            mode: "all".into(),
            whole_words: false,
            collections: vec![],
            languages: vec![],
            grades: vec![],
            every_grader: false,
            sahihayn: true,
            chapters: vec![],
            limit: 25,
            offset: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TextHit {
    pub edition_key: String,
    pub language: String,
    pub direction: String,
    pub text: String,
    /// highlight ranges in `text`, in characters
    pub spans: Vec<(usize, usize)>,
    pub matched: bool,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub hadith_id: i64,
    pub collection_key: String,
    pub collection_name: String,
    pub number: String,
    pub texts: Vec<TextHit>,
    pub grades: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct SearchPage {
    pub terms: Vec<String>,
    pub total: usize,
    pub results: Vec<SearchResult>,
    pub used_index: bool,
    pub elapsed_ms: u128,
    /// a word as typed → the spellings found for it (wildcards, near spellings), in the order found
    pub spellings: Vec<(String, Vec<String>)>,
    pub hadith_ids: Vec<i64>,
    /// with grades or chapters chosen: how many the words found in all (GR-FLT); else the total
    pub before_grades: usize,
}

#[derive(Debug)]
pub enum SearchError {
    NoIndex,
    BadMode(String),
    Db(rusqlite::Error),
}

impl std::fmt::Display for SearchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchError::NoIndex => write!(
                f,
                "The search index is not built yet (or was built by another version of the rules): iy-rs search \
                 WORD builds it once, or Tools → Rebuild Search Index."
            ),
            SearchError::BadMode(m) => write!(f, "mode must be one of all, any, phrase (not {m})"),
            SearchError::Db(e) => write!(f, "{e}"),
        }
    }
}

impl From<rusqlite::Error> for SearchError {
    fn from(e: rusqlite::Error) -> Self {
        SearchError::Db(e)
    }
}

/// Python's `\w`: a letter or digit of any script, or "_".
pub fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

pub fn has_fts(conn: &Connection) -> bool {
    conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'texts_fts'",
        [],
        |_| Ok(()),
    )
    .is_ok()
}

/// Whether the index the Python isnady built covers every text, with the same normalisation rules: the stamp it
/// leaves in core_meta ("index_ready"), read from the small indexes alone — no text is read.
pub fn index_ready(conn: &Connection) -> bool {
    let stamp: Option<String> = conn
        .query_row(
            "SELECT value FROM core_meta WHERE key = 'index_ready'",
            [],
            |r| r.get(0),
        )
        .ok();
    let now: Option<String> = conn
        .query_row(
            "SELECT (SELECT COUNT(*) FROM texts), (SELECT MAX(id) FROM texts), (SELECT MAX(text_id) FROM text_norm)",
            [],
            |r| {
                let n: i64 = r.get(0)?;
                let a: Option<i64> = r.get(1)?;
                let b: Option<i64> = r.get(2)?;
                let py = |v: Option<i64>| v.map(|x| x.to_string()).unwrap_or_else(|| "None".into());
                Ok(format!("{NORMALIZER_VERSION}|{n}|{}|{}", py(a), py(b)))
            },
        )
        .ok();
    stamp.is_some() && stamp == now
}

// ------------------------------------------------------------------ the words asked for
fn strip_wildcards(w: &str) -> &str {
    w.trim_matches(|c| WILDCARDS.contains(&c))
}

/// A question mark ending a word is punctuation: "(?<=\w)[?؟]+(?=\s|$)" → " ".
fn question_end(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if (c == '?' || c == '؟') && i > 0 && is_word(chars[i - 1]) {
            let mut j = i;
            while j < chars.len() && (chars[j] == '?' || chars[j] == '؟') {
                j += 1;
            }
            if j == chars.len() || chars[j].is_whitespace() {
                out.push(' ');
                i = j;
                continue;
            }
            out.extend(&chars[i..j]);
            i = j;
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

pub fn terms(query: &SearchQuery) -> Vec<String> {
    let text = question_end(&normalize(&query.text));
    let spaced: String = text
        .chars()
        .map(|c| if PUNCT.contains(c) { ' ' } else { c })
        .collect();
    let words: Vec<&str> = spaced
        .split_whitespace()
        .filter(|w| !strip_wildcards(w).is_empty())
        .collect();
    if words.is_empty() {
        return vec![];
    }
    if query.mode == "phrase" {
        return vec![words.join(" ")];
    }
    let mut seen = HashSet::new();
    words
        .into_iter()
        .filter(|w| seen.insert(*w))
        .map(String::from)
        .collect()
}

// ------------------------------------------------------------------ spellings: wildcards and near ones
struct Vocabulary<'a> {
    conn: &'a Connection,
    loaded: std::cell::OnceCell<(String, HashMap<String, i64>)>,
}

impl<'a> Vocabulary<'a> {
    fn new(conn: &'a Connection) -> Self {
        Vocabulary {
            conn,
            loaded: std::cell::OnceCell::new(),
        }
    }

    /// Every word of the texts, one per line, and how often each is — read only when a word needs it.
    fn get(&self) -> &(String, HashMap<String, i64>) {
        self.loaded
            .get_or_init(|| load_vocabulary(self.conn).unwrap_or_default())
    }
}

fn load_vocabulary(conn: &Connection) -> rusqlite::Result<(String, HashMap<String, i64>)> {
    let mut stmt = conn.prepare("SELECT word, n FROM words")?;
    let mut lines = String::new();
    let mut counts = HashMap::new();
    for row in stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))? {
        let (w, n) = row?;
        if !lines.is_empty() {
            lines.push('\n');
        }
        lines.push_str(&w);
        counts.insert(w, n);
    }
    Ok((lines, counts))
}

fn in_vocabulary(v: &Vocabulary, pattern: &str, limit: usize) -> Vec<String> {
    let Ok(re) = Regex::new(&format!("(?m)^(?:{pattern})$")) else {
        return vec![];
    };
    let (lines, counts) = v.get();
    let mut found: Vec<String> = re
        .find_iter(lines)
        .map(|m| m.as_str().to_string())
        .collect();
    found.sort();
    found.dedup();
    // the most frequent first (Python sorts a set: equal counts in no fixed order; here by the word)
    found.sort_by(|a, b| {
        counts
            .get(b)
            .unwrap_or(&0)
            .cmp(counts.get(a).unwrap_or(&0))
            .then(a.cmp(b))
    });
    found.truncate(limit);
    found
}

fn wildcard_pattern(word: &str) -> String {
    word.chars()
        .map(|c| match c {
            '?' | '؟' => ".".to_string(),
            '*' => ".*".to_string(),
            _ => regex::escape(&c.to_string()),
        })
        .collect()
}

/// Every spelling one letter away: a letter changed, left out, or added.
fn near_pattern(word: &str) -> String {
    let chars: Vec<char> = word.chars().collect();
    let esc = |s: &[char]| regex::escape(&s.iter().collect::<String>());
    let mut pats = std::collections::BTreeSet::new();
    for i in 0..chars.len() {
        pats.insert(format!("{}.{}", esc(&chars[..i]), esc(&chars[i + 1..])));
        pats.insert(format!("{}{}", esc(&chars[..i]), esc(&chars[i + 1..])));
    }
    for i in 0..=chars.len() {
        pats.insert(format!("{}.{}", esc(&chars[..i]), esc(&chars[i..])));
    }
    pats.into_iter().collect::<Vec<_>>().join("|")
}

fn spell(
    v: &Vocabulary,
    terms: &[String],
    mode: &str,
    near: &str,
) -> (Vec<String>, Vec<(String, Vec<String>)>) {
    let mut spellings: Vec<(String, Vec<String>)> = vec![];
    let mut word = |w: &str| -> String {
        let alts: Vec<String> = if w.chars().any(|c| WILDCARDS.contains(&c)) {
            in_vocabulary(v, &wildcard_pattern(w), NEAR_MAX)
        } else if !near.is_empty()
            && w.chars().count() >= NEAR_MIN_LENGTH
            && (near == "all" || !v.get().1.contains_key(w))
        {
            let mut a = vec![w.to_string()];
            a.extend(
                in_vocabulary(v, &near_pattern(w), NEAR_MAX)
                    .into_iter()
                    .filter(|x| x != w && x.chars().count() >= MIN_TRIGRAM),
            );
            a
        } else {
            return w.to_string();
        };
        let alts = if alts.is_empty() {
            let s = strip_wildcards(w);
            vec![if s.is_empty() {
                w.to_string()
            } else {
                s.to_string()
            }]
        } else {
            alts
        };
        if alts != vec![w.to_string()] && !spellings.iter().any(|(k, _)| k == w) {
            spellings.push((
                w.to_string(),
                alts.iter().filter(|a| *a != w).cloned().collect(),
            ));
        }
        alts.join("|")
    };
    if mode == "phrase" {
        let phrase: Vec<String> = terms[0].split(' ').map(&mut word).collect();
        return (vec![phrase.join(" ")], spellings);
    }
    let out = terms.iter().map(|t| word(t)).collect();
    (out, spellings)
}

// ------------------------------------------------------------------ matching
fn fts_literal(term: &str) -> String {
    format!("\"{}\"", term.replace('"', ""))
}

/// Occurrences of `needle` standing as a whole word (no word character just before or after), as byte ranges.
fn whole_word_finds(hay: &str, needle: &str) -> Vec<(usize, usize)> {
    let mut out = vec![];
    if needle.is_empty() {
        return out;
    }
    let mut start = 0;
    while let Some(p) = hay[start..].find(needle) {
        let s = start + p;
        let e = s + needle.len();
        let before = hay[..s].chars().next_back().map(is_word).unwrap_or(false);
        let after = hay[e..].chars().next().map(is_word).unwrap_or(false);
        if !before && !after {
            out.push((s, e));
            start = e; // Python's finditer: the next match after this one
        } else {
            start = s + hay[s..].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
        }
        if start > hay.len() {
            break;
        }
    }
    out
}

fn alt_re(word: &str) -> String {
    let alts: Vec<&str> = word.split('|').collect();
    if alts.len() == 1 {
        regex::escape(alts[0])
    } else {
        format!(
            "(?:{})",
            alts.iter()
                .map(|a| regex::escape(a))
                .collect::<Vec<_>>()
                .join("|")
        )
    }
}

/// The words of a phrase in order, whatever punctuation or spaces stand between them in the text.
fn phrase_finds(hay: &str, phrase: &str, whole_words: bool) -> Vec<(usize, usize)> {
    let body = phrase
        .split_whitespace()
        .map(alt_re)
        .collect::<Vec<_>>()
        .join(r"[^\w]+");
    let Ok(re) = Regex::new(&body) else {
        return vec![];
    };
    let mut out = vec![];
    let mut at = 0;
    while at <= hay.len() {
        let Some(m) = re.find_at(hay, at) else { break };
        let ok = !whole_words
            || (!hay[..m.start()]
                .chars()
                .next_back()
                .map(is_word)
                .unwrap_or(false)
                && !hay[m.end()..].chars().next().map(is_word).unwrap_or(false));
        if ok {
            out.push((m.start(), m.end()));
            at = if m.end() > m.start() {
                m.end()
            } else {
                m.end() + 1
            };
        } else {
            at = m.start()
                + hay[m.start()..]
                    .chars()
                    .next()
                    .map(|c| c.len_utf8())
                    .unwrap_or(1);
        }
    }
    out
}

fn text_matches(norm: &str, terms: &[String], mode: &str, whole_words: bool) -> bool {
    if mode == "phrase" {
        return terms
            .iter()
            .all(|t| !phrase_finds(norm, t, whole_words).is_empty());
    }
    let found = terms.iter().map(|t| {
        t.split('|').any(|a| {
            if whole_words {
                !whole_word_finds(norm, a).is_empty()
            } else {
                norm.contains(a)
            }
        })
    });
    if mode == "any" {
        found.into_iter().any(|x| x)
    } else {
        found.into_iter().all(|x| x)
    }
}

fn char_index(s: &str, byte: usize) -> usize {
    s[..byte].chars().count()
}

/// The ranges (in characters of the normalised text) of the words found, merged.
fn spans(norm: &str, terms: &[String], whole_words: bool, mode: &str) -> Vec<(usize, usize)> {
    let mut found: Vec<(usize, usize)> = vec![];
    let mut words: Vec<&str> = vec![];
    if mode == "phrase" {
        for t in terms {
            found.extend(phrase_finds(norm, t, whole_words));
        }
    } else {
        words = terms.iter().flat_map(|t| t.split('|')).collect();
    }
    for w in words {
        if w.is_empty() {
            continue;
        }
        if whole_words {
            found.extend(whole_word_finds(norm, w));
        } else {
            let mut start = 0;
            while let Some(p) = norm[start..].find(w) {
                let s = start + p;
                found.push((s, s + w.len()));
                start = s + norm[s..].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
            }
        }
    }
    let mut chars: Vec<(usize, usize)> = found
        .into_iter()
        .map(|(s, e)| (char_index(norm, s), char_index(norm, e)))
        .collect();
    chars.sort();
    let mut merged: Vec<(usize, usize)> = vec![];
    for (s, e) in chars {
        if let Some(last) = merged.last_mut() {
            if s <= last.1 {
                last.1 = last.1.max(e);
                continue;
            }
        }
        merged.push((s, e));
    }
    merged
}

fn to_original(
    spans: &[(usize, usize)],
    map: &[usize],
    original_len: usize,
) -> Vec<(usize, usize)> {
    spans
        .iter()
        .map(|&(s, e)| (map[s], if e < map.len() { map[e] } else { original_len }))
        .collect()
}

// ------------------------------------------------------------------ search
pub fn search(conn: &Connection, query: &SearchQuery) -> Result<SearchPage, SearchError> {
    let started = Instant::now();
    if !MODES.contains(&query.mode.as_str()) {
        return Err(SearchError::BadMode(query.mode.clone()));
    }
    let typed = terms(query);
    let used_index = has_fts(conn);
    if typed.is_empty() {
        return Ok(SearchPage {
            terms: vec![],
            total: 0,
            results: vec![],
            used_index,
            elapsed_ms: 0,
            spellings: vec![],
            hadith_ids: vec![],
            before_grades: 0,
        });
    }
    if !index_ready(conn) {
        return Err(SearchError::NoIndex);
    }
    let vocab = Vocabulary::new(conn);
    let (mut current, spellings) = spell(&vocab, &typed, &query.mode, "");
    let mut page = search_terms(conn, query, &current, used_index, started, spellings)?;
    for near in ["unknown", "all"] {
        if page.total > 0 || page.before_grades > 0 {
            // found, only not of the grades chosen: no other spellings
            break;
        }
        let (near_terms, near_spellings) = spell(&vocab, &typed, &query.mode, near);
        if near_terms != current {
            current = near_terms;
            page = search_terms(conn, query, &current, used_index, started, near_spellings)?;
        }
    }
    Ok(page)
}

fn search_terms(
    conn: &Connection,
    query: &SearchQuery,
    terms: &[String],
    use_index: bool,
    started: Instant,
    spellings: Vec<(String, Vec<String>)>,
) -> Result<SearchPage, SearchError> {
    let joiner = if query.mode == "any" { " OR " } else { " AND " };
    let groups: Vec<String> = if query.mode == "phrase" {
        let mut seen = HashSet::new();
        terms[0]
            .split_whitespace()
            .filter(|g| seen.insert(g.to_string()))
            .map(String::from)
            .collect()
    } else {
        terms.to_vec()
    };
    let mut conditions = vec![];
    let mut params: Vec<String> = vec![];
    for group in &groups {
        let alts: Vec<&str> = group.split('|').collect();
        let long_: Vec<&str> = alts
            .iter()
            .copied()
            .filter(|a| use_index && a.chars().count() >= MIN_TRIGRAM)
            .collect();
        let short: Vec<&str> = alts
            .iter()
            .copied()
            .filter(|a| !long_.contains(a))
            .collect();
        let mut parts = vec![];
        if !long_.is_empty() {
            parts.push(
                "n.text_id IN (SELECT rowid FROM texts_fts WHERE texts_fts MATCH ?)".to_string(),
            );
            params.push(
                long_
                    .iter()
                    .map(|a| fts_literal(a))
                    .collect::<Vec<_>>()
                    .join(" OR "),
            );
        }
        for a in short {
            parts.push("instr(n.norm, ?) > 0".to_string());
            params.push(a.to_string());
        }
        conditions.push(format!("({})", parts.join(" OR ")));
    }
    let mut wheres = vec![format!("({})", conditions.join(joiner))];
    let keys = if query.collections.is_empty() {
        books::active_keys(conn)?
    } else {
        Some(query.collections.clone())
    };
    if let Some(keys) = keys {
        let marks = vec!["?"; keys.len()].join(",");
        wheres.push(format!(
            "c.key IN ({})",
            if marks.is_empty() {
                "NULL".to_string()
            } else {
                marks
            }
        ));
        params.extend(keys);
    }
    if !query.languages.is_empty() {
        wheres.push(format!(
            "e.language IN ({})",
            vec!["?"; query.languages.len()].join(",")
        ));
        params.extend(query.languages.iter().cloned());
    }
    let sql = format!(
        "SELECT t.hadith_id, n.norm
         FROM text_norm n
         JOIN texts t ON t.id = n.text_id
         JOIN editions e ON e.id = t.edition_id
         JOIN hadiths h ON h.id = t.hadith_id
         JOIN collections c ON c.id = h.collection_id
         WHERE {}
         ORDER BY {}, h.number_sort, h.number",
        wheres.join(" AND "),
        books::order_sql(conn, "c.key")?
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(params.iter()), |r| {
        Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
    })?;
    // the exact check, in the hadith's order; the grades and the chapter chosen (GR-FLT, SRCH-SEC1)
    let allowed = grade_filter(conn, query)?;
    let mut hadith_ids = vec![];
    let mut seen = HashSet::new();
    let mut before = 0;
    for row in rows {
        let (hid, norm) = row?;
        if seen.contains(&hid) {
            continue;
        }
        if text_matches(&norm, terms, &query.mode, query.whole_words) {
            seen.insert(hid);
            before += 1;
            if allowed.as_ref().map_or(true, |a| a.contains(&hid)) {
                hadith_ids.push(hid);
            }
        }
    }
    let total = hadith_ids.len();
    let page_ids: Vec<i64> = hadith_ids
        .iter()
        .skip(query.offset)
        .take(query.limit)
        .copied()
        .collect();
    let mut results = vec![];
    for hid in page_ids {
        results.push(load_result(conn, hid, terms, query)?);
    }
    let mut kept = vec![];
    if !spellings.is_empty() && total > 0 {
        // only the spellings the shown hadith really have
        let word = Regex::new(r"\w+").expect("pattern");
        let mut words = HashSet::new();
        for r in &results {
            for t in r.texts.iter().filter(|t| t.matched) {
                for m in word.find_iter(&normalize(&t.text)) {
                    words.insert(m.as_str().to_string());
                }
            }
        }
        for (w, alts) in spellings {
            let alts: Vec<String> = alts.into_iter().filter(|a| words.contains(a)).collect();
            if !alts.is_empty() {
                kept.push((w, alts));
            }
        }
    }
    Ok(SearchPage {
        terms: terms.to_vec(),
        total,
        results,
        used_index: use_index,
        elapsed_ms: started.elapsed().as_millis(),
        spellings: kept,
        hadith_ids,
        before_grades: if allowed.is_some() { before } else { total },
    })
}

/// The hadith the grades and the chapters chosen allow, or None when neither was chosen (`search.grade_filter`).
pub fn grade_filter(
    conn: &Connection,
    query: &SearchQuery,
) -> rusqlite::Result<Option<HashSet<i64>>> {
    let mut allowed: Option<HashSet<i64>> = None;
    if !query.grades.is_empty() {
        allowed = Some(crate::grades::allowed_ids(
            conn,
            &query.grades,
            query.every_grader,
            query.sahihayn,
        )?);
    }
    if !query.chapters.is_empty() {
        let inside = crate::works::hadith_under(conn, &query.chapters)?;
        allowed = Some(match allowed {
            None => inside,
            Some(a) => a.intersection(&inside).copied().collect(),
        });
    }
    Ok(allowed)
}

/// A hadith with its texts (the words found marked) and its grades.
pub fn load_result(
    conn: &Connection,
    hadith_id: i64,
    terms: &[String],
    query: &SearchQuery,
) -> rusqlite::Result<SearchResult> {
    let (number, key, name): (String, String, String) = conn.query_row(
        "SELECT h.number, c.key, c.name FROM hadiths h JOIN collections c ON c.id = h.collection_id WHERE h.id = ?",
        [hadith_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    let mut params: Vec<String> = vec![hadith_id.to_string()];
    let mut filter = String::new();
    if !query.languages.is_empty() {
        filter = format!(
            " AND e.language IN ({})",
            vec!["?"; query.languages.len()].join(",")
        );
        params.extend(query.languages.iter().cloned());
    }
    let sql = format!(
        "SELECT e.key, e.language, e.direction, t.text FROM texts t JOIN editions e ON e.id = t.edition_id
         WHERE t.hadith_id = CAST(? AS INTEGER){filter}
         ORDER BY CASE e.language WHEN 'Arabic' THEN 0 ELSE 1 END, e.language, e.key"
    );
    let mut stmt = conn.prepare(&sql)?;
    let mut texts = vec![];
    for row in stmt.query_map(params_from_iter(params.iter()), |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, Option<String>>(1)?,
            r.get::<_, Option<String>>(2)?,
            r.get::<_, String>(3)?,
        ))
    })? {
        let (ekey, lang, dir, text) = row?;
        let lang = lang.unwrap_or_default();
        let dir = dir
            .filter(|d| !d.is_empty())
            .unwrap_or_else(|| "ltr".into());
        if terms.is_empty() {
            texts.push(TextHit {
                edition_key: ekey,
                language: lang,
                direction: dir,
                text,
                spans: vec![],
                matched: false,
            });
            continue;
        }
        let (norm, map) = normalize_with_map(&text);
        let found = spans(&norm, terms, query.whole_words, &query.mode);
        let n = text.chars().count();
        texts.push(TextHit {
            edition_key: ekey,
            language: lang,
            direction: dir,
            matched: !found.is_empty(),
            spans: to_original(&found, &map, n),
            text,
        });
    }
    let mut g = conn.prepare(
        "SELECT grader_name, grade FROM grades WHERE hadith_id = ? ORDER BY grader_name",
    )?;
    let grades = g
        .query_map([hadith_id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let chosen = books::language_order();
    texts.sort_by_key(|t| books::language_rank(&t.language, &chosen)); // stable, as Python's sort
    Ok(SearchResult {
        hadith_id,
        collection_key: key,
        collection_name: name,
        number,
        texts,
        grades,
    })
}

// ------------------------------------------------------------------ recent searches (the same list as the app)
pub const RECENT_KEY: &str = "search.recent";
pub const RECENT_LIMIT: usize = 20;

pub fn add_recent(text: &str, kind: &str) {
    use serde_json::{json, Value};
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.is_empty() {
        return;
    }
    let kind = if ["hadith", "narrators", "scholars"].contains(&kind) {
        kind
    } else {
        "hadith"
    };
    let folded = caseless::default_case_fold_str(&text);
    let old: Vec<Value> = crate::config::state_get(RECENT_KEY)
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default();
    let mut kept = vec![json!({"text": text, "kind": kind})];
    for r in old
        .into_iter()
        .filter(|r| {
            r.get("text")
                .and_then(|t| t.as_str())
                .map(|t| !t.trim().is_empty())
                .unwrap_or(false)
        })
        .take(RECENT_LIMIT)
    {
        let t = r
            .get("text")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let k = r
            .get("kind")
            .and_then(|k| k.as_str())
            .filter(|k| ["hadith", "narrators", "scholars"].contains(k))
            .unwrap_or("hadith");
        if caseless::default_case_fold_str(&t) == folded && k == kind {
            continue;
        }
        kept.push(json!({"text": t, "kind": k}));
    }
    kept.truncate(RECENT_LIMIT);
    crate::config::state_set(RECENT_KEY, Value::Array(kept));
}

/// The recent searches, the newest first: (text, kind) (`recent.items`).
pub fn recent_items() -> Vec<(String, String)> {
    let raw = crate::config::state_get(RECENT_KEY)
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default();
    raw.iter()
        .filter_map(|r| {
            let text = r.get("text")?.as_str()?.trim().to_string();
            if text.is_empty() {
                return None;
            }
            let kind = r
                .get("kind")
                .and_then(|k| k.as_str())
                .filter(|k| ["hadith", "narrators", "scholars"].contains(k))
                .unwrap_or("hadith");
            Some((text, kind.to_string()))
        })
        .take(RECENT_LIMIT)
        .collect()
}

/// Forget every recent search; how many there were (`recent.clear`).
pub fn clear_recent() -> usize {
    let n = recent_items().len();
    crate::config::state_set(RECENT_KEY, serde_json::Value::Array(vec![]));
    n
}

// ------------------------------------------------------------------ building the index (search.ensure_index)

const BATCH: usize = 2000;
/// this many texts or more to index: their search index built without the trigger
const BULK: usize = 5000;

fn drop_index(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "DROP TRIGGER IF EXISTS text_norm_ai;
         DROP TRIGGER IF EXISTS text_norm_ad;
         DROP TABLE IF EXISTS texts_fts;
         DROP TABLE IF EXISTS text_norm;
         DROP TABLE IF EXISTS words;",
    )
}

fn index_stamp(conn: &Connection) -> rusqlite::Result<String> {
    conn.query_row(
        "SELECT (SELECT COUNT(*) FROM texts), (SELECT MAX(id) FROM texts), (SELECT MAX(text_id) FROM text_norm)",
        [],
        |r| {
            let n: i64 = r.get(0)?;
            let a: Option<i64> = r.get(1)?;
            let b: Option<i64> = r.get(2)?;
            Ok(format!(
                "{NORMALIZER_VERSION}|{n}|{}|{}",
                crate::py::opt(a),
                crate::py::opt(b)
            ))
        },
    )
}

fn exists(conn: &Connection, sql: &str) -> rusqlite::Result<bool> {
    use rusqlite::OptionalExtension;
    Ok(conn.query_row(sql, [], |_| Ok(())).optional()?.is_some())
}

/// Index every text that is not indexed yet; how many were added (`search.ensure_index`).
pub fn ensure_index(conn: &Connection, say: &dyn Fn(&str)) -> rusqlite::Result<usize> {
    if index_ready(conn) {
        return Ok(0);
    }
    let added = build_index(conn, say)?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS core_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
        [],
    )?;
    conn.execute(
        "INSERT OR REPLACE INTO core_meta (key, value) VALUES ('index_ready', ?)",
        [index_stamp(conn)?],
    )?;
    Ok(added)
}

/// Throw the search index away and build it again from the texts (`search.rebuild_index`).
pub fn rebuild_index(conn: &Connection, say: &dyn Fn(&str)) -> rusqlite::Result<usize> {
    drop_index(conn)?;
    ensure_index(conn, say)
}

fn normalized(rows: Vec<(i64, String)>) -> Vec<(i64, String)> {
    // the texts normalised on every core at once; the rows written in the same order
    let cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .max(1);
    if cores == 1 || rows.len() < 64 {
        return rows
            .into_iter()
            .map(|(id, t)| (id, normalize(&t)))
            .collect();
    }
    let size = rows.len().div_ceil(cores);
    std::thread::scope(|scope| {
        let parts: Vec<_> = rows
            .chunks(size)
            .map(|part| {
                scope.spawn(move || {
                    part.iter()
                        .map(|(id, t)| (*id, normalize(t)))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        parts
            .into_iter()
            .flat_map(|h| h.join().unwrap_or_default())
            .collect()
    })
}

fn build_index(conn: &Connection, say: &dyn Fn(&str)) -> rusqlite::Result<usize> {
    use crate::sources::store::create_index_tables;
    use rusqlite::OptionalExtension;
    create_index_tables(conn)?;
    let have: Option<String> = conn
        .query_row(
            "SELECT value FROM core_meta WHERE key = 'normalizer_version'",
            [],
            |r| r.get(0),
        )
        .optional()?;
    if have.is_some_and(|v| v != NORMALIZER_VERSION.to_string()) {
        say("Normalisation rules changed; rebuilding the search index");
        drop_index(conn)?;
        create_index_tables(conn)?;
    }
    conn.execute(
        "INSERT OR REPLACE INTO core_meta (key, value) VALUES ('normalizer_version', ?)",
        [NORMALIZER_VERSION.to_string()],
    )?;
    let missing: i64 = conn.query_row(
        "SELECT COUNT(*) FROM texts t LEFT JOIN text_norm n ON n.text_id = t.id WHERE n.text_id IS NULL",
        [],
        |r| r.get(0),
    )?;
    if missing == 0 {
        if !exists(conn, "SELECT 1 FROM words LIMIT 1")?
            && exists(conn, "SELECT 1 FROM text_norm LIMIT 1")?
        {
            say("Listing the words of the texts (once)");
            add_words(conn, None)?; // a database indexed before the word list existed
        }
        return Ok(0);
    }
    say(&format!("Indexing {missing} texts for search"));
    let ids: Vec<i64> = {
        let mut st = conn.prepare(
            "SELECT t.id FROM texts t LEFT JOIN text_norm n ON n.text_id = t.id WHERE n.text_id IS NULL ORDER BY t.id",
        )?;
        let v = st.query_map([], |r| r.get(0))?.collect::<Result<_, _>>()?;
        v
    };
    let bulk = ids.len() >= BULK && has_fts(conn);
    // an empty index (a first import): FTS5 builds it from the table in one pass
    let fresh = bulk && !exists(conn, "SELECT 1 FROM text_norm LIMIT 1")?;
    if bulk {
        conn.execute("DROP TRIGGER IF EXISTS text_norm_ai", [])?;
    }
    let run = || -> rusqlite::Result<usize> {
        let mut added = 0;
        for chunk in ids.chunks(BATCH) {
            let marks = vec!["?"; chunk.len()].join(",");
            let rows: Vec<(i64, String)> = {
                let mut st =
                    conn.prepare(&format!("SELECT id, text FROM texts WHERE id IN ({marks})"))?;
                let v = st
                    .query_map(params_from_iter(chunk.iter()), |r| {
                        Ok((r.get(0)?, r.get(1)?))
                    })?
                    .collect::<Result<_, _>>()?;
                v
            };
            let n = rows.len();
            let rows = normalized(rows);
            let tx = conn.unchecked_transaction()?;
            {
                let mut ins = tx.prepare("INSERT INTO text_norm (text_id, norm) VALUES (?, ?)")?;
                for (id, norm) in &rows {
                    ins.execute(rusqlite::params![id, norm])?;
                }
            }
            if bulk && !fresh {
                tx.execute(
                    &format!(
                        "INSERT INTO texts_fts (rowid, norm) SELECT text_id, norm FROM text_norm WHERE text_id IN ({marks})"
                    ),
                    params_from_iter(chunk.iter()),
                )?;
            }
            tx.commit()?;
            added += n;
        }
        if fresh {
            conn.execute("INSERT INTO texts_fts (texts_fts) VALUES ('rebuild')", [])?;
        }
        // every word again on a first import, or when the word list is still empty
        let all = fresh || !exists(conn, "SELECT 1 FROM words LIMIT 1")?;
        add_words(conn, if all { None } else { Some(&ids) })?;
        Ok(added)
    };
    let result = run();
    if bulk {
        create_index_tables(conn)?; // the trigger again, for the texts added one by one later
    }
    result
}

/// `_WORD_RE` (`\w+` of Python: letters, digits and the underscore).
fn word_re() -> &'static Regex {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| Regex::new(r"[\p{L}\p{N}_]+").unwrap())
}

/// The words of the texts with how often each is: all of them (None), or those of new texts added
/// (`search._add_words`).
fn add_words(conn: &Connection, text_ids: Option<&[i64]>) -> rusqlite::Result<()> {
    let mut counts: HashMap<String, i64> = HashMap::new();
    let mut count = |norm: &str| {
        for m in word_re().find_iter(norm) {
            *counts.entry(m.as_str().to_string()).or_insert(0) += 1;
        }
    };
    match text_ids {
        None => {
            conn.execute("DELETE FROM words", [])?;
            let mut st = conn.prepare("SELECT norm FROM text_norm")?;
            let mut rows = st.query([])?;
            while let Some(r) = rows.next()? {
                count(&r.get::<_, String>(0)?);
            }
        }
        Some(ids) => {
            for chunk in ids.chunks(BATCH) {
                let marks = vec!["?"; chunk.len()].join(",");
                let mut st = conn.prepare(&format!(
                    "SELECT norm FROM text_norm WHERE text_id IN ({marks})"
                ))?;
                let mut rows = st.query(params_from_iter(chunk.iter()))?;
                while let Some(r) = rows.next()? {
                    count(&r.get::<_, String>(0)?);
                }
            }
        }
    }
    let tx = conn.unchecked_transaction()?;
    {
        let mut up = tx.prepare(
            "INSERT INTO words (word, n) VALUES (?, ?) ON CONFLICT(word) DO UPDATE SET n = n + excluded.n",
        )?;
        for (w, n) in &counts {
            up.execute(rusqlite::params![w, n])?;
        }
    }
    tx.commit()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terms_drop_punctuation_and_wildcard_alone() {
        let mut q = SearchQuery::new("«قَالَ: لا يومن» ? *");
        assert_eq!(terms(&q), vec!["قال", "لا", "يومن"]);
        q.mode = "phrase".into();
        assert_eq!(terms(&q), vec!["قال لا يومن"]);
        let q = SearchQuery::new("niyet? niyet");
        assert_eq!(terms(&q), vec!["niyet"]);
    }

    #[test]
    fn whole_words() {
        assert_eq!(whole_word_finds("بالنيات النيات", "النيات"), vec![(15, 27)]);
        assert!(text_matches(
            "قال: «لا يومن",
            &["قال لا يومن".into()],
            "phrase",
            false
        ));
    }

    #[test]
    fn near() {
        assert!(near_pattern("ab").contains("a.b"));
    }
}
