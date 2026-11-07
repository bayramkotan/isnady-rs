//! Other narrations of a hadith (YZ2, takhrij): the pairs the Python isnady found by comparing the texts
//! (`iy ai build`), read from hadith_relations as `core/takhrij.py` reads them.

use rusqlite::Connection;

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
