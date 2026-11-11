//! Takhrij (YZ2, A4): the narrations of the same hadith, found within and across the imported collections
//! (`core/takhrij.py`).
//!
//! Only the TEXT (matn) is compared: the chain is cut off with the isnad parser. Candidates come from a TF-IDF of the
//! matn words; each candidate is then checked by CONTAINMENT of word pairs — how much of the shorter text is found in
//! the longer one. Word pairs made only of the most frequent words are left out.
//!   same         containment >= 0.5
//!   same_report  containment >= 0.2 and the same identified Companion

use rusqlite::Connection;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

use crate::py::thousands;

pub const METHOD_ID: &str = "isnady-takhrij-1";
const SAME: f64 = 0.5;
const SAME_REPORT: f64 = 0.2;
const CANDIDATE: f32 = 0.25;
const COMMON_WORDS: usize = 150;
const MIN_PAIRS: usize = 4;

pub struct Related {
    pub hadith_id: i64,
    /// "same" (the same text) or "same_report" (probably the same report)
    pub kind: String,
    pub score: f64,
    pub book: String,
    pub book_name: String,
    pub number: String,
}

/// Other narrations of this hadith: the same text first, then by book and number (`takhrij.related`). An older
/// database without the table gives none.
pub fn related(conn: &Connection, hadith_id: i64, lang: &str) -> Vec<Related> {
    let run = || -> rusqlite::Result<Vec<Related>> {
        let mut stmt = conn.prepare(
            "SELECT CASE WHEN r.hadith_a = ?1 THEN r.hadith_b ELSE r.hadith_a END AS other, r.kind, r.score,
                    c.key, c.name, h.number, h.number_sort
             FROM hadith_relations r
             JOIN hadiths h ON h.id = CASE WHEN r.hadith_a = ?1 THEN r.hadith_b ELSE r.hadith_a END
             JOIN collections c ON c.id = h.collection_id
             WHERE r.hadith_a = ?1 OR r.hadith_b = ?1
             ORDER BY (r.kind = 'same') DESC, c.key, h.number_sort",
        )?;
        let rows = stmt.query_map([hadith_id], |r| {
            let book: String = r.get(3)?;
            let name: String = r.get(4)?;
            Ok(Related {
                hadith_id: r.get(0)?,
                kind: r.get::<_, Option<String>>(1)?.unwrap_or_default(),
                score: r.get::<_, Option<f64>>(2)?.unwrap_or(0.0),
                book_name: crate::works::book_title(&book, &name, lang),
                book,
                number: r.get(5)?,
            })
        })?;
        rows.collect()
    };
    run().unwrap_or_default()
}

fn say_t(say: &dyn Fn(&str), text: &str, values: &[(&str, String)]) {
    say(&crate::i18n::t(&crate::i18n::ui_language(), text, values));
}

/// The identified last narrator (normally the Companion) of each hadith's first chain (`_companions`).
fn companions(conn: &Connection) -> rusqlite::Result<HashMap<i64, i64>> {
    let mut st = conn.prepare(
        "SELECT i.hadith_id, l.person_id FROM isnad_links l JOIN isnads i ON i.id = l.isnad_id
         WHERE i.ordinal = 1 AND i.problem IS NULL AND l.person_id IS NOT NULL
           AND l.position = (SELECT MAX(position) FROM isnad_links x WHERE x.isnad_id = l.isnad_id)",
    )?;
    let rows = st.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)))?;
    rows.collect()
}

/// Find the narrations of the same hadith and keep them in hadith_relations (`build`).
pub fn build(conn: &Connection, say: &dyn Fn(&str)) -> Result<Value, String> {
    let e = |x: rusqlite::Error| x.to_string();
    let started = std::time::Instant::now();
    say_t(
        say,
        "Reading the texts of the hadith (the chains are left out)",
        &[],
    );
    let mut raw: HashMap<i64, Option<String>> = HashMap::new();
    {
        let mut st = conn
            .prepare("SELECT hadith_id, raw_text FROM isnads WHERE ordinal = 1")
            .map_err(e)?;
        for r in st
            .query_map([], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, Option<String>>(1)?))
            })
            .map_err(e)?
        {
            let (h, t) = r.map_err(e)?;
            raw.insert(h, t);
        }
    }
    let mut order: Vec<i64> = vec![];
    let mut matn: HashMap<i64, Vec<String>> = HashMap::new();
    {
        let mut st = conn
            .prepare(
                "SELECT t.hadith_id, t.text FROM texts t JOIN editions e ON e.id = t.edition_id
                 WHERE e.language = 'Arabic' ORDER BY t.hadith_id, e.id",
            )
            .map_err(e)?;
        for r in st
            .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
            .map_err(e)?
        {
            let (hid, text) = r.map_err(e)?;
            if let std::collections::hash_map::Entry::Vacant(v) = matn.entry(hid) {
                let body =
                    crate::isnad::split_text(&text, raw.get(&hid).and_then(|r| r.as_deref())).1;
                v.insert(crate::semantic::tokens(&body));
                order.push(hid);
            }
        }
    }
    // the most frequent words, ties in the order first seen (Counter.most_common)
    let mut frequency: HashMap<&str, (usize, usize)> = HashMap::new();
    let mut seen = 0usize;
    for hid in &order {
        for t in &matn[hid] {
            let f = frequency.entry(t.as_str()).or_insert_with(|| {
                seen += 1;
                (0, seen)
            });
            f.0 += 1;
        }
    }
    let mut ranked: Vec<(&str, (usize, usize))> = frequency.into_iter().collect();
    ranked.sort_by(|a, b| b.1 .0.cmp(&a.1 .0).then(a.1 .1.cmp(&b.1 .1)));
    let common: HashSet<&str> = ranked.iter().take(COMMON_WORDS).map(|r| r.0).collect();
    let mut pairs: HashMap<i64, HashSet<(&str, &str)>> = HashMap::new();
    let mut ids: Vec<i64> = vec![];
    for hid in &order {
        let toks = &matn[hid];
        let sh: HashSet<(&str, &str)> = toks
            .windows(2)
            .map(|w| (w[0].as_str(), w[1].as_str()))
            .filter(|(a, b)| !(common.contains(a) && common.contains(b)))
            .collect();
        if sh.len() >= MIN_PAIRS {
            pairs.insert(*hid, sh);
            ids.push(*hid);
        }
    }
    let n = ids.len();
    if n < 2 {
        return Err("Import some hadith first.".into());
    }
    say_t(
        say,
        "Finding candidate pairs among {n:,} texts",
        &[("n:,", thousands(n as i64))],
    );
    let mut df: HashMap<&str, usize> = HashMap::new();
    for h in &ids {
        let set: HashSet<&str> = matn[h].iter().map(String::as_str).collect();
        for t in set {
            *df.entry(t).or_default() += 1;
        }
    }
    let mut terms: Vec<&str> = df
        .iter()
        .filter(|(_, d)| **d >= 2 && **d as f64 <= 0.05 * n as f64)
        .map(|(t, _)| *t)
        .collect();
    terms.sort_unstable();
    let vocab: HashMap<&str, usize> = terms.iter().enumerate().map(|(i, t)| (*t, i)).collect();
    // each text as unit TF-IDF weights, and the texts of each word
    let mut rows: Vec<Vec<(usize, f32)>> = Vec::with_capacity(n);
    let mut postings: Vec<Vec<(usize, f32)>> = vec![vec![]; terms.len()];
    for (r, h) in ids.iter().enumerate() {
        let mut counts: HashMap<&str, usize> = HashMap::new();
        for t in &matn[h] {
            *counts.entry(t.as_str()).or_default() += 1;
        }
        let weights: Vec<(usize, f64)> = counts
            .into_iter()
            .filter_map(|(t, c)| {
                vocab
                    .get(t)
                    .map(|j| (*j, (1.0 + (c as f64).ln()) * (n as f64 / df[t] as f64).ln()))
            })
            .collect();
        let norm = weights.iter().map(|(_, w)| w * w).sum::<f64>().sqrt();
        let norm = if norm == 0.0 { 1.0 } else { norm };
        let mut row: Vec<(usize, f32)> = weights
            .into_iter()
            .map(|(j, w)| (j, (w / norm) as f32))
            .collect();
        row.sort_unstable_by_key(|x| x.0);
        for (j, w) in &row {
            postings[*j].push((r, *w));
        }
        rows.push(row);
    }
    let companion = companions(conn).map_err(e)?;
    let threads = std::thread::available_parallelism()
        .map(|t| t.get())
        .unwrap_or(1)
        .clamp(1, 16);
    let done = std::sync::atomic::AtomicUsize::new(0);
    let found: Vec<(i64, i64, &str, f64)> = std::thread::scope(|s| {
        let handles: Vec<_> = (0..threads)
            .map(|t| {
                let (rows, postings, ids, pairs, companion, done) =
                    (&rows, &postings, &ids, &pairs, &companion, &done);
                s.spawn(move || {
                    let mut out = vec![];
                    let mut acc = vec![0f32; n];
                    let mut touched: Vec<usize> = vec![];
                    let mut i = t;
                    while i < n {
                        for (j, w) in &rows[i] {
                            for (other, v) in &postings[*j] {
                                if *other <= i {
                                    continue;
                                }
                                if acc[*other] == 0.0 {
                                    touched.push(*other);
                                }
                                acc[*other] += w * v;
                            }
                        }
                        touched.sort_unstable();
                        for &j in &touched {
                            let cosine = acc[j];
                            acc[j] = 0.0;
                            if cosine < CANDIDATE {
                                continue;
                            }
                            let (a, b) = (ids[i], ids[j]);
                            let (pa, pb) = (&pairs[&a], &pairs[&b]);
                            let shared = if pa.len() <= pb.len() {
                                pa.iter().filter(|p| pb.contains(*p)).count()
                            } else {
                                pb.iter().filter(|p| pa.contains(*p)).count()
                            };
                            let containment = shared as f64 / pa.len().min(pb.len()) as f64;
                            if containment >= SAME {
                                out.push((a, b, "same", containment));
                            } else if containment >= SAME_REPORT
                                && companion.get(&a).is_some()
                                && companion.get(&a) == companion.get(&b)
                            {
                                out.push((a, b, "same_report", containment));
                            }
                        }
                        touched.clear();
                        done.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        i += threads;
                    }
                    out
                })
            })
            .collect();
        let mut last = 0;
        while handles.iter().any(|h| !h.is_finished()) {
            std::thread::sleep(std::time::Duration::from_millis(300));
            let d = done.load(std::sync::atomic::Ordering::Relaxed) / 1000 * 1000;
            if d > last {
                last = d;
                say_t(
                    say,
                    "Compared {value:,} of {n:,} texts",
                    &[
                        ("value:,", thousands(d as i64)),
                        ("n:,", thousands(n as i64)),
                    ],
                );
            }
        }
        let mut all: Vec<(i64, i64, &str, f64)> = handles
            .into_iter()
            .flat_map(|h| h.join().expect("thread"))
            .collect();
        all.sort_by_key(|x| (x.0, x.1));
        all
    });
    say_t(
        say,
        "Compared {value:,} of {n:,} texts",
        &[
            ("value:,", thousands(n as i64)),
            ("n:,", thousands(n as i64)),
        ],
    );
    let tx = conn.unchecked_transaction().map_err(e)?;
    tx.execute(
        "DELETE FROM hadith_relations WHERE method LIKE 'isnady-takhrij%'",
        [],
    )
    .map_err(e)?;
    {
        let mut st = tx
            .prepare(
                "INSERT OR REPLACE INTO hadith_relations (hadith_a, hadith_b, kind, score, method) VALUES (?, ?, ?, ?, ?)",
            )
            .map_err(e)?;
        for (a, b, k, s) in &found {
            st.execute(rusqlite::params![
                a,
                b,
                k,
                crate::stats::num::round_to(*s, 3),
                METHOD_ID
            ])
            .map_err(e)?;
        }
    }
    tx.commit().map_err(e)?;
    let same = found.iter().filter(|f| f.2 == "same").count();
    let report = found.len() - same;
    say_t(
        say,
        "Takhrij ready: {same:,} pairs of the same hadith, {same_report:,} probably the same report",
        &[("same:,", thousands(same as i64)), ("same_report:,", thousands(report as i64))],
    );
    let seconds = (started.elapsed().as_secs_f64() * 10.0).round() / 10.0;
    Ok(
        json!({"method": METHOD_ID, "texts": n, "same": same, "same_report": report, "seconds": seconds}),
    )
}

/// How many pairs there are (`status`).
pub fn status(conn: &Connection) -> (i64, i64, i64) {
    conn.query_row(
        "SELECT COUNT(*), SUM(kind = 'same'), SUM(kind = 'same_report') FROM hadith_relations
         WHERE method LIKE 'isnady-takhrij%'",
        [],
        |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, Option<i64>>(1)?.unwrap_or(0),
                r.get::<_, Option<i64>>(2)?.unwrap_or(0),
            ))
        },
    )
    .unwrap_or((0, 0, 0))
}
