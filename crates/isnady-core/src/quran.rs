//! The verses a hadith names (Q2, GEN1-H2): the Python isnady's `core/quran.py` — the marks a translation puts
//! after a verse it quotes ("(33:33)", "[23:99-100]", "(Ahzab 33/51)"), read from the English and Turkish texts, and
//! the names of the suras from the Qur'an the Python isnady downloaded (sources/quran.db). Reading the Qur'an itself
//! comes with the Qur'an in Books.

use regex::Regex;
use rusqlite::{Connection, OpenFlags};
use std::collections::HashMap;
use std::sync::OnceLock;

use crate::paths;

/// Tanzil's simple script: the Arabic text of the Qur'an.
pub const ARABIC: &str = "ara-quransimple";

/// The verses of each sura (Hafs), to check a mark.
pub const VERSES_IN: [u32; 114] = [
    7, 286, 200, 176, 120, 165, 206, 75, 129, 109, 123, 111, 43, 52, 99, 128, 111, 110, 98, 135,
    112, 78, 118, 64, 77, 227, 93, 88, 69, 60, 34, 30, 73, 54, 45, 83, 182, 88, 75, 85, 54, 53, 89,
    59, 37, 35, 38, 29, 18, 45, 60, 49, 62, 55, 78, 96, 29, 22, 24, 13, 14, 11, 11, 18, 12, 12, 30,
    52, 52, 44, 28, 28, 20, 56, 40, 31, 50, 40, 46, 42, 29, 19, 36, 25, 22, 17, 19, 26, 30, 20, 15,
    21, 11, 8, 8, 19, 5, 8, 8, 11, 11, 8, 3, 9, 5, 4, 7, 3, 6, 3, 5, 4, 5, 6,
];

/// The suras' names in Turkish, when the downloaded Qur'an has none (`quran.TR_NAMES`).
pub const TR_NAMES: [&str; 114] = [
    "Fâtiha",
    "Bakara",
    "Âl-i İmrân",
    "Nisâ",
    "Mâide",
    "En'âm",
    "A'râf",
    "Enfâl",
    "Tevbe",
    "Yûnus",
    "Hûd",
    "Yûsuf",
    "Ra'd",
    "İbrâhîm",
    "Hicr",
    "Nahl",
    "İsrâ",
    "Kehf",
    "Meryem",
    "Tâhâ",
    "Enbiyâ",
    "Hac",
    "Mü'minûn",
    "Nûr",
    "Furkân",
    "Şuarâ",
    "Neml",
    "Kasas",
    "Ankebût",
    "Rûm",
    "Lokmân",
    "Secde",
    "Ahzâb",
    "Sebe'",
    "Fâtır",
    "Yâsîn",
    "Sâffât",
    "Sâd",
    "Zümer",
    "Mü'min",
    "Fussilet",
    "Şûrâ",
    "Zuhruf",
    "Duhân",
    "Câsiye",
    "Ahkâf",
    "Muhammed",
    "Fetih",
    "Hucurât",
    "Kâf",
    "Zâriyât",
    "Tûr",
    "Necm",
    "Kamer",
    "Rahmân",
    "Vâkıa",
    "Hadîd",
    "Mücâdele",
    "Haşr",
    "Mümtehine",
    "Saf",
    "Cuma",
    "Münâfikûn",
    "Teğâbün",
    "Talâk",
    "Tahrîm",
    "Mülk",
    "Kalem",
    "Hâkka",
    "Meâric",
    "Nûh",
    "Cin",
    "Müzzemmil",
    "Müddessir",
    "Kıyâme",
    "İnsân",
    "Mürselât",
    "Nebe'",
    "Nâziât",
    "Abese",
    "Tekvîr",
    "İnfitâr",
    "Mutaffifîn",
    "İnşikâk",
    "Bürûc",
    "Târık",
    "A'lâ",
    "Gâşiye",
    "Fecr",
    "Beled",
    "Şems",
    "Leyl",
    "Duhâ",
    "İnşirâh",
    "Tîn",
    "Alak",
    "Kadr",
    "Beyyine",
    "Zilzâl",
    "Âdiyât",
    "Kâria",
    "Tekâsür",
    "Asr",
    "Hümeze",
    "Fîl",
    "Kureyş",
    "Mâûn",
    "Kevser",
    "Kâfirûn",
    "Nasr",
    "Tebbet",
    "İhlâs",
    "Felak",
    "Nâs",
];

pub fn path() -> std::path::PathBuf {
    paths::data_dir().join("sources").join("quran.db")
}

fn open() -> Option<Connection> {
    let p = path();
    if !p.exists() {
        return None;
    }
    Connection::open_with_flags(p, OpenFlags::SQLITE_OPEN_READ_ONLY).ok()
}

/// How many translations are downloaded beside the Arabic (`quran.editions`).
pub fn translations() -> usize {
    open()
        .and_then(|c| {
            c.query_row(
                "SELECT COUNT(*) FROM editions WHERE key != ?",
                [ARABIC],
                |r| r.get::<_, i64>(0),
            )
            .ok()
        })
        .unwrap_or(0) as usize
}

/// The Arabic text is downloaded (`quran.available`).
pub fn available() -> bool {
    open()
        .and_then(|c| {
            c.query_row(
                "SELECT COUNT(*) FROM verses WHERE edition = ?",
                [ARABIC],
                |r| r.get::<_, i64>(0),
            )
            .ok()
        })
        .is_some_and(|n| n > 6000)
}

/// (sura, first verse, last verse) of each verse a translation marks (`quran.marks_in`). A slash counts only after
/// a sura's name (a bare "(1/2)" is a fraction); every number is checked against the sura.
pub fn marks_in(text: &str) -> Vec<(u32, u32, u32)> {
    static MARK: OnceLock<Regex> = OnceLock::new();
    static NAMED: OnceLock<Regex> = OnceLock::new();
    let mark = MARK.get_or_init(|| {
        Regex::new(
            r"[\(\[]\s*(?:(?P<name>[^\d\(\)\[\]\n]{2,30}?)[,]?\s*)?(?P<s>\d{1,3})\s*(?P<sep>[:/])\s*(?P<a>\d{1,3})(?:\s*[-–]\s*(?P<b>\d{1,3}))?\s*[\)\]]",
        )
        .unwrap()
    });
    let named = NAMED.get_or_init(|| Regex::new(r"[A-Za-zÀ-ÿĞğİıŞş]{3}").unwrap());
    let mut out = vec![];
    for m in mark.captures_iter(text) {
        let (Ok(s), Ok(a)) = (m["s"].parse::<u32>(), m["a"].parse::<u32>()) else {
            continue;
        };
        let b = m
            .name("b")
            .and_then(|b| b.as_str().parse::<u32>().ok())
            .unwrap_or(a);
        if &m["sep"] == "/" && !m.name("name").is_some_and(|n| named.is_match(n.as_str())) {
            continue;
        }
        if (1..=114).contains(&s)
            && 1 <= a
            && a <= b
            && b <= VERSES_IN[s as usize - 1]
            && b - a < 30
        {
            out.push((s, a, b));
        }
    }
    out
}

/// {hadith id: [(sura, from, to)]} — every verse the English and Turkish texts mark (`quran._cites_counted`).
/// The verses each hadith names: (sura, from, to), in the order its texts mark them.
pub type Cites = HashMap<i64, Vec<(u32, u32, u32)>>;

pub fn cites(conn: &Connection) -> rusqlite::Result<Cites> {
    let mut out: HashMap<i64, Vec<(u32, u32, u32)>> = HashMap::new();
    let mut stmt = conn.prepare(
        "SELECT t.hadith_id, t.text FROM texts t JOIN editions e ON e.id = t.edition_id \
         WHERE e.language IN ('English', 'Turkish') AND (t.text LIKE '%:%' OR t.text LIKE '%/%')",
    )?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?;
    for row in rows {
        let (hid, text) = row?;
        for reference in marks_in(&text) {
            let list = out.entry(hid).or_default();
            if !list.contains(&reference) {
                list.push(reference);
            }
        }
    }
    Ok(out)
}

/// The name of a sura in a language (`quran.sura_name`): from the downloaded Qur'an, else the Turkish list, else
/// its number.
pub fn sura_name(n: u32, lang: &str) -> String {
    static SURAS: OnceLock<HashMap<u32, (String, String, String)>> = OnceLock::new();
    let suras = SURAS.get_or_init(|| {
        let mut out = HashMap::new();
        if let Some(c) = open() {
            if let Ok(mut stmt) = c.prepare("SELECT n, name, arabic FROM suras") {
                let rows = stmt.query_map([], |r| {
                    Ok((
                        r.get::<_, u32>(0)?,
                        r.get::<_, Option<String>>(1)?.unwrap_or_default(),
                        r.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    ))
                });
                if let Ok(rows) = rows {
                    for (n, name, arabic) in rows.flatten() {
                        let tr = TR_NAMES
                            .get(n as usize - 1)
                            .map(|s| s.to_string())
                            .unwrap_or_default();
                        out.insert(n, (name, tr, arabic));
                    }
                }
            }
        }
        out
    });
    match suras.get(&n) {
        None if lang == "tr" && (1..=114).contains(&n) => TR_NAMES[n as usize - 1].to_string(),
        None => n.to_string(),
        Some((name, tr, arabic)) => {
            let own = match lang {
                "tr" => tr,
                "ar" => arabic,
                _ => name,
            };
            if own.is_empty() {
                name.clone()
            } else {
                own.clone()
            }
        }
    }
}

/// "Ahzâb 33:33", "Mü'minûn 23:99-100" (`quran.label`).
pub fn label(sura: u32, a: u32, b: u32, lang: &str) -> String {
    let range = if b != a {
        format!("-{b}")
    } else {
        String::new()
    };
    format!("{} {sura}:{a}{range}", sura_name(sura, lang))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marks() {
        assert_eq!(
            marks_in("Allah said (33:33) and [23:99-100]."),
            vec![(33, 33, 33), (23, 99, 100)]
        );
        assert_eq!(marks_in("(Ahzab 33/51) but not (1/2)"), vec![(33, 51, 51)]);
        assert_eq!(marks_in("(2:300)"), vec![]);
    }
}
