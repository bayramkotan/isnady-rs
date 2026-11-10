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

// ------------------------------------------------------------------ reading the Qur'an (Q1)

/// The translation each language starts with (Q1d): the one its readers know best, kept in isnady-data.
pub const DEFAULT_OF: [(&str, &str); 18] = [
    ("Turkish", "tur-diyanetisleri"),
    ("English", "eng-ummmuhammad"),
    ("Urdu", "urd-fatehmuhammadja"),
    ("Indonesian", "ind-indonesianislam"),
    ("Bengali", "ben-muhiuddinkhan"),
    ("French", "fra-muhammadhamidul"),
    ("Russian", "rus-elmirkuliev"),
    ("Tamil", "tam-janturstfoundat"),
    ("German", "deu-asfbubenheimand"),
    ("Persian", "fas-hussainansarian"),
    ("Spanish", "spa-juliocortes"),
    ("Bosnian", "bos-besimkorkut"),
    ("Albanian", "sqi-sherifahmeti"),
    ("Azerbaijani", "aze-vasimmammadaliy"),
    ("Uzbek", "uzb-muhammadsodikmu"),
    ("Kazakh", "kaz-khalifahaltai"),
    ("Tatar", "tat-yakubibnnugman"),
    ("Malay", "msa-abdullahmuhamma"),
];

/// Where an edition is read from.
pub const OWN: &str = "isnady-data";
pub const SOURCE: &str = "fawazahmed0/quran-api";

pub fn default_of(language: &str) -> Option<&'static str> {
    DEFAULT_OF
        .iter()
        .find(|(l, _)| *l == language)
        .map(|(_, k)| *k)
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct Sura {
    pub n: u32,
    pub name: String,
    pub english: String,
    pub arabic: String,
    pub place: String,
    pub verses: u32,
    pub tr: String,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct Edition {
    pub key: String,
    pub language: String,
    pub author: String,
    pub direction: String,
    pub source: String,
    pub comments: String,
    pub origin: String,
}

fn text_of(r: &rusqlite::Row, i: usize) -> rusqlite::Result<String> {
    Ok(r.get::<_, Option<String>>(i)?.unwrap_or_default())
}

/// The suras of the downloaded Qur'an (`quran.suras`).
pub fn suras() -> Vec<Sura> {
    let Some(c) = open() else {
        return vec![];
    };
    let Ok(mut st) =
        c.prepare("SELECT n, name, english, arabic, place, verses FROM suras ORDER BY n")
    else {
        return vec![];
    };
    st.query_map([], |r| {
        let n: u32 = r.get(0)?;
        let name = text_of(r, 1)?;
        Ok(Sura {
            n,
            tr: TR_NAMES
                .get((n as usize).wrapping_sub(1))
                .map(|s| s.to_string())
                .unwrap_or_else(|| name.clone()),
            name,
            english: text_of(r, 2)?,
            arabic: text_of(r, 3)?,
            place: text_of(r, 4)?,
            verses: r.get::<_, Option<u32>>(5)?.unwrap_or(0),
        })
    })
    .map(|rows| rows.flatten().collect())
    .unwrap_or_default()
}

/// The editions downloaded, the Arabic first (`quran.editions`).
pub fn editions() -> Vec<Edition> {
    let Some(c) = open() else {
        return vec![];
    };
    let has_origin = c.prepare("SELECT origin FROM editions LIMIT 0").is_ok();
    let sql = format!(
        "SELECT key, language, author, direction, source, comments, {} FROM editions ORDER BY language, author",
        if has_origin { "origin" } else { "NULL" }
    );
    let Ok(mut st) = c.prepare(&sql) else {
        return vec![];
    };
    let mut eds: Vec<Edition> = st
        .query_map([], |r| {
            let origin = text_of(r, 6)?;
            Ok(Edition {
                key: text_of(r, 0)?,
                language: text_of(r, 1)?,
                author: text_of(r, 2)?,
                direction: text_of(r, 3)?,
                source: text_of(r, 4)?,
                comments: text_of(r, 5)?,
                origin: if origin.is_empty() {
                    SOURCE.to_string()
                } else {
                    origin
                },
            })
        })
        .map(|rows| rows.flatten().collect())
        .unwrap_or_default();
    eds.sort_by(|a, b| {
        (a.key != ARABIC, &a.language, &a.author).cmp(&(b.key != ARABIC, &b.language, &b.author))
    });
    eds
}

/// The languages the reader reads (Q1d): the interface language first, then the content languages chosen, Arabic
/// aside (`quran.wanted_languages`).
pub fn wanted_languages() -> Vec<String> {
    let ui = crate::i18n::ui_language();
    let mut out: Vec<String> = vec![crate::i18n::languages()
        .iter()
        .find(|l| l.0 == ui)
        .map(|l| l.2.clone())
        .unwrap_or_else(|| "English".to_string())];
    for part in crate::config::get_str("view.content_languages", "").split(',') {
        if !part.is_empty() {
            out.push(part.to_string());
        }
    }
    let mut seen = Vec::new();
    for l in out {
        if !l.is_empty() && l != "Arabic" && !seen.contains(&l) {
            seen.push(l);
        }
    }
    seen
}

/// The translations downloaded with the Arabic: one in each language the reader reads.
pub fn default_editions() -> Vec<String> {
    let out: Vec<String> = wanted_languages()
        .iter()
        .filter_map(|l| default_of(l).map(String::from))
        .collect();
    if out.is_empty() {
        vec![default_of("English").unwrap_or_default().to_string()]
    } else {
        out
    }
}

/// The languages the reader reads that have no translation downloaded yet (and one isnady-data keeps).
pub fn missing_languages() -> Vec<String> {
    let have: Vec<String> = editions().into_iter().map(|e| e.language).collect();
    wanted_languages()
        .into_iter()
        .filter(|l| default_of(l).is_some() && !have.contains(l))
        .collect()
}

/// The translations shown beside the Arabic (`quran.shown`). saved: the reader's own choice ("quran.editions"; ","
/// = none); None (never chosen): those downloaded in the languages the reader reads, the interface language first.
pub fn shown(saved: Option<&str>) -> Vec<String> {
    let have: Vec<Edition> = editions().into_iter().filter(|e| e.key != ARABIC).collect();
    let keys: Vec<String> = have.iter().map(|e| e.key.clone()).collect();
    if let Some(saved) = saved {
        return saved
            .split(',')
            .filter(|k| keys.iter().any(|x| x == k))
            .map(String::from)
            .collect();
    }
    let mut out = vec![];
    for lang in wanted_languages() {
        let mut mine: Vec<String> = have
            .iter()
            .filter(|e| e.language == lang)
            .map(|e| e.key.clone())
            .collect();
        let first = default_of(&lang);
        mine.sort_by_key(|k| Some(k.as_str()) != first);
        out.extend(mine.into_iter().take(1));
    }
    if out.is_empty() {
        keys.into_iter().take(1).collect()
    } else {
        out
    }
}

/// The reader's own choice of translations, as kept in state.json ("quran.editions"), the way Python reads it.
pub fn saved_choice() -> Option<String> {
    crate::config::state_get("quran.editions").and_then(|v| match v {
        serde_json::Value::Null => None,
        serde_json::Value::String(s) => Some(s),
        other => Some(other.to_string()),
    })
}

/// A verse with its texts: {edition: text}.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Verse {
    pub ayah: u32,
    pub texts: HashMap<String, String>,
}

/// The verses of a sura (or of its verses start..end) in the Arabic and these editions (`quran.verses`).
pub fn verses(sura: u32, editions: &[String], start: u32, end: Option<u32>) -> Vec<Verse> {
    let Some(c) = open() else {
        return vec![];
    };
    let mut keys = vec![ARABIC.to_string()];
    keys.extend(editions.iter().filter(|e| *e != ARABIC).cloned());
    let sql = format!(
        "SELECT ayah, edition, text FROM verses WHERE sura = ? AND ayah BETWEEN ? AND ? AND edition IN ({}) ORDER BY ayah",
        vec!["?"; keys.len()].join(",")
    );
    let mut args: Vec<rusqlite::types::Value> = vec![
        (sura as i64).into(),
        (start as i64).into(),
        (end.unwrap_or(999) as i64).into(),
    ];
    args.extend(keys.iter().map(|k| rusqlite::types::Value::from(k.clone())));
    let Ok(mut st) = c.prepare(&sql) else {
        return vec![];
    };
    let mut out: Vec<Verse> = vec![];
    if let Ok(rows) = st.query_map(rusqlite::params_from_iter(args), |r| {
        Ok((r.get::<_, u32>(0)?, text_of(r, 1)?, text_of(r, 2)?))
    }) {
        for (ayah, edition, text) in rows.flatten() {
            match out.iter_mut().find(|v| v.ayah == ayah) {
                Some(v) => {
                    v.texts.insert(edition, text);
                }
                None => out.push(Verse {
                    ayah,
                    texts: HashMap::from([(edition, text)]),
                }),
            }
        }
    }
    out.sort_by_key(|v| v.ayah);
    out
}

/// A verse found by a search.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Found {
    pub sura: u32,
    pub ayah: u32,
    pub edition: String,
    pub text: String,
}

/// Verses holding every word (Arabic without its vowels; Latin letters without case and accents), in the order of
/// the mushaf (`quran.search`).
pub fn search(text: &str, editions_: &[String], limit: usize) -> Vec<Found> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let Some(c) = (if words.is_empty() { None } else { open() }) else {
        return vec![];
    };
    let mut keys = vec![ARABIC.to_string()];
    if editions_.is_empty() {
        keys.extend(
            editions()
                .into_iter()
                .map(|e| e.key)
                .filter(|k| k != ARABIC),
        );
    } else {
        keys.extend(editions_.iter().filter(|e| *e != ARABIC).cloned());
    }
    let arabic = text
        .chars()
        .any(|ch| ('\u{0600}'..='\u{06FF}').contains(&ch));
    let wanted: Vec<String> = words
        .iter()
        .map(|w| {
            if arabic {
                crate::normalize::normalize(w)
            } else {
                crate::people::fold_latin(w)
            }
        })
        .collect();
    let sql = format!(
        "SELECT sura, ayah, edition, text FROM verses WHERE edition IN ({}) ORDER BY sura, ayah",
        vec!["?"; keys.len()].join(",")
    );
    let Ok(mut st) = c.prepare(&sql) else {
        return vec![];
    };
    let mut out = vec![];
    let Ok(mut rows) = st.query(rusqlite::params_from_iter(keys.iter())) else {
        return vec![];
    };
    while let Ok(Some(r)) = rows.next() {
        let edition = text_of(r, 2).unwrap_or_default();
        if (edition == ARABIC) != arabic {
            continue;
        }
        let t = text_of(r, 3).unwrap_or_default();
        let hay = if arabic {
            crate::normalize::normalize(&t)
        } else {
            crate::people::fold_latin(&t)
        };
        if wanted.iter().all(|w| hay.contains(w.as_str())) {
            out.push(Found {
                sura: r.get(0).unwrap_or(0),
                ayah: r.get(1).unwrap_or(0),
                edition,
                text: t,
            });
            if out.len() >= limit {
                break;
            }
        }
    }
    out
}

/// Remove a translation downloaded (never the Arabic); whether there was one (`quran.remove`).
pub fn remove(edition: &str) -> bool {
    let p = path();
    if edition == ARABIC || !p.exists() {
        return false;
    }
    let Ok(c) = Connection::open(p) else {
        return false;
    };
    let n = c
        .execute("DELETE FROM verses WHERE edition = ?", [edition])
        .unwrap_or(0);
    let _ = c.execute("DELETE FROM editions WHERE key = ?", [edition]);
    n > 0
}

/// Every verse each hadith names, and the hadith that name each verse (`quran.cites`), counted once per change.
#[derive(Clone, Default, Debug)]
pub struct Named {
    pub hadith: HashMap<i64, Vec<(u32, u32, u32)>>,
    /// "s:a" → the hadith that name it, in the order they were read
    pub verse: HashMap<String, Vec<i64>>,
}

pub fn named(conn: &Connection) -> rusqlite::Result<Named> {
    crate::cache::kept(conn, "quran-cites", || {
        let mut out = Named::default();
        let mut stmt = conn.prepare(
            "SELECT t.hadith_id, t.text FROM texts t JOIN editions e ON e.id = t.edition_id \
             WHERE e.language IN ('English', 'Turkish') AND (t.text LIKE '%:%' OR t.text LIKE '%/%')",
        )?;
        let mut rows = stmt.query([])?;
        while let Some(r) = rows.next()? {
            let hid: i64 = r.get(0)?;
            let text = text_of(r, 1)?;
            for (s, a, b) in marks_in(&text) {
                let list = out.hadith.entry(hid).or_default();
                if !list.contains(&(s, a, b)) {
                    list.push((s, a, b));
                    for v in a..=b {
                        out.verse.entry(format!("{s}:{v}")).or_default().push(hid);
                    }
                }
            }
        }
        Ok(out)
    })
}

/// The hadith that name a verse, each once (`quran.hadith_of`).
pub fn hadith_of(n: &Named, sura: u32, ayah: u32) -> Vec<i64> {
    let mut out: Vec<i64> = vec![];
    for h in n.verse.get(&format!("{sura}:{ayah}")).into_iter().flatten() {
        if !out.contains(h) {
            out.push(*h);
        }
    }
    out
}

/// How many hadith name a verse of each sura (`quran.counts_by_sura`).
pub fn counts_by_sura(n: &Named) -> HashMap<u32, usize> {
    let mut sets: HashMap<u32, std::collections::HashSet<i64>> = HashMap::new();
    for (key, hids) in &n.verse {
        let s: u32 = key
            .split(':')
            .next()
            .and_then(|x| x.parse().ok())
            .unwrap_or(0);
        sets.entry(s).or_default().extend(hids.iter().copied());
    }
    sets.into_iter().map(|(s, h)| (s, h.len())).collect()
}

/// "2", "2:255", "2:255-257" → (sura, first verse, last verse or None for the whole sura) — `cli.cmd_quran`.
pub fn parse_ref(text: &str) -> Option<(u32, u32, Option<u32>)> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re =
        RE.get_or_init(|| Regex::new(r"^(\d{1,3})(?::(\d{1,3})(?:-(\d{1,3}))?)?$").expect("verse"));
    let m = re.captures(text)?;
    let sura: u32 = m[1].parse().ok()?;
    if !(1..=114).contains(&sura) {
        return None;
    }
    let start: u32 = m.get(2).and_then(|x| x.as_str().parse().ok()).unwrap_or(1);
    let end = match (m.get(2), m.get(3)) {
        (_, Some(b)) => b.as_str().parse().ok(),
        (Some(_), None) => Some(start),
        _ => None,
    };
    Some((sura, start, end))
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

// ------------------------------------------------------------------ downloading (quran.fetch, quran.catalog; Q1b)
/// The fawazahmed0/quran-api, where a text isnady-data does not keep is read (`quran.BASE`).
pub const BASE: &str = "https://raw.githubusercontent.com/fawazahmed0/quran-api/1";

/// An edition there is to download (`catalog`'s values).
#[derive(Debug, Clone, Default)]
pub struct Offered {
    pub language: String,
    pub author: String,
    pub direction: String,
    pub source: String,
    pub comments: String,
    pub origin: String,
    pub url: String,
    pub sha256: Option<String>,
}

fn connect_rw() -> rusqlite::Result<Connection> {
    let p = path();
    if let Some(d) = p.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let conn = Connection::open(p)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS suras (n INTEGER PRIMARY KEY, name TEXT, english TEXT, arabic TEXT, place TEXT, verses INTEGER);
         CREATE TABLE IF NOT EXISTS editions (key TEXT PRIMARY KEY, language TEXT, author TEXT, direction TEXT, source TEXT, comments TEXT, imported_at TEXT);
         CREATE TABLE IF NOT EXISTS verses (edition TEXT, sura INTEGER, ayah INTEGER, text TEXT, PRIMARY KEY (edition, sura, ayah));",
    )?;
    if conn.prepare("SELECT origin FROM editions LIMIT 0").is_err() {
        conn.execute("ALTER TABLE editions ADD COLUMN origin TEXT", [])?;
    }
    Ok(conn)
}

fn s(v: &serde_json::Value, k: &str) -> String {
    v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

/// (the "quran" section of isnady-data's catalog, what its locations are relative to) (`quran.own`).
fn own() -> Option<(serde_json::Value, String)> {
    use crate::sources::repos;
    let mut seen = std::collections::HashSet::new();
    let mut order = vec![Some(repos::selected())];
    order.push(repos::find(&crate::sources::tables().default_id));
    for repo in order.into_iter().flatten() {
        if !seen.insert(repo.id.clone()) {
            continue;
        }
        let Ok((cat, _)) = repos::catalog(&repo, false) else {
            continue;
        };
        if let Some(q) = cat.get("quran").filter(|q| {
            q.get("editions")
                .and_then(|e| e.as_array())
                .is_some_and(|a| !a.is_empty())
        }) {
            return Some((q.clone(), repo.url.clone().unwrap_or_default()));
        }
    }
    None
}

fn own_editions() -> (Vec<(String, Offered)>, Option<serde_json::Value>, String) {
    let Some((q, base)) = own() else {
        return (vec![], None, String::new());
    };
    let mut out = vec![];
    for e in q["editions"].as_array().into_iter().flatten() {
        let key = s(e, "key");
        out.push((
            key.clone(),
            Offered {
                language: s(e, "language"),
                author: e
                    .get("author")
                    .and_then(|x| x.as_str())
                    .map(str::to_string)
                    .unwrap_or(key.clone()),
                direction: e
                    .get("direction")
                    .and_then(|x| x.as_str())
                    .unwrap_or("ltr")
                    .to_string(),
                source: s(e, "source"),
                comments: s(e, "comments"),
                origin: OWN.into(),
                url: crate::sources::absolute(&base, &s(e, "location")),
                sha256: e.get("sha256").and_then(|x| x.as_str()).map(str::to_string),
            },
        ));
    }
    (out, Some(q), base)
}

fn source_editions() -> Result<Vec<(String, Offered)>, String> {
    let raw =
        crate::sources::read_bytes(&format!("{BASE}/editions.json"), Some(120)).map_err(|e| e.0)?;
    let v: serde_json::Value = serde_json::from_slice(&raw).map_err(|e| e.to_string())?;
    Ok(v.as_object()
        .into_iter()
        .flatten()
        .map(|(_, e)| {
            let name = s(e, "name");
            (
                name.clone(),
                Offered {
                    language: s(e, "language"),
                    author: s(e, "author"),
                    direction: s(e, "direction"),
                    source: s(e, "source"),
                    comments: s(e, "comments"),
                    origin: SOURCE.into(),
                    url: format!("{BASE}/editions/{name}.min.json"),
                    sha256: None,
                },
            )
        })
        .collect())
}

/// The translations downloaded before keep their text but take the names isnady-data gives them (`_rename`).
fn rename(mine: &[(String, Offered)]) {
    if !path().exists() {
        return;
    }
    if let Ok(c) = connect_rw() {
        for (k, v) in mine {
            let _ = c.execute(
                "UPDATE editions SET author = ? WHERE key = ? AND author != ?",
                rusqlite::params![v.author, k, v.author],
            );
        }
    }
}

/// Every edition there is to download: isnady-data's first, then (source) the rest of fawazahmed0/quran-api's
/// (`quran.catalog`).
pub fn catalog(say: &dyn Fn(&str), source: bool) -> Result<Vec<(String, Offered)>, String> {
    say("Reading the list of translations");
    let (mine, _q, _b) = own_editions();
    let mut out = mine.clone();
    if source {
        match source_editions() {
            Ok(list) => {
                for (k, v) in list {
                    if !out.iter().any(|(x, _)| *x == k) {
                        out.push((k, v));
                    }
                }
            }
            Err(e) => {
                crate::log::warning(
                    "quran",
                    &format!("The list of {SOURCE} could not be read: {e}"),
                );
                if mine.is_empty() {
                    return Err(e);
                }
            }
        }
    }
    if !mine.is_empty() {
        rename(&mine);
    }
    Ok(out)
}

type Rows = Vec<(i64, i64, String)>;
/// (n, name, english, arabic, place, verses)
type SuraRow = (i64, String, String, String, String, i64);

fn verses_of(data: &serde_json::Value) -> Option<Rows> {
    if let Some(v) = data.get("verses").and_then(|v| v.as_array()) {
        return v
            .iter()
            .map(|r| {
                let n = |i: usize| match &r[i] {
                    serde_json::Value::String(t) => t.trim().parse().ok(),
                    x => x.as_i64(),
                };
                Some((n(0)?, n(1)?, r[2].as_str()?.to_string()))
            })
            .collect();
    }
    data.get("quran")?
        .as_array()?
        .iter()
        .map(|r| {
            Some((
                r["chapter"].as_i64()?,
                r["verse"].as_i64()?,
                r["text"].as_str()?.to_string(),
            ))
        })
        .collect()
}

fn read_edition(about: &Offered, key: &str) -> Result<(Rows, String), String> {
    let mut tries = vec![(
        if about.url.is_empty() {
            format!("{BASE}/editions/{key}.min.json")
        } else {
            about.url.clone()
        },
        if about.origin.is_empty() {
            SOURCE.to_string()
        } else {
            about.origin.clone()
        },
        about.sha256.clone(),
    )];
    if about.origin == OWN {
        tries.push((
            format!("{BASE}/editions/{key}.min.json"),
            SOURCE.to_string(),
            None,
        ));
    }
    let mut last = String::new();
    for (url, origin, sha) in tries {
        let attempt = || -> Result<Rows, String> {
            let raw = crate::sources::read_bytes(&url, Some(600)).map_err(|e| e.0)?;
            if let Some(sha) = &sha {
                if crate::sources::store::sha256::hex(&raw) != *sha {
                    return Err(format!(
                        "{url}: the file is not the one the catalog lists (sha256)"
                    ));
                }
            }
            let text = String::from_utf8_lossy(&raw);
            let data: serde_json::Value = serde_json::from_str(text.trim_start_matches('\u{feff}'))
                .map_err(|e| e.to_string())?;
            let rows = verses_of(&data).ok_or_else(|| format!("{url}: no verses"))?;
            if rows.len() < 6000 {
                return Err(format!("{url}: {} verses", rows.len()));
            }
            Ok(rows)
        };
        match attempt() {
            Ok(rows) => return Ok((rows, origin)),
            Err(e) => {
                crate::log::warning(
                    "quran",
                    &format!("Qur'an text {key} not read from {url}: {e}"),
                );
                last = e;
            }
        }
    }
    Err(last)
}

fn suras_rows(q: Option<&serde_json::Value>, base: &str) -> Result<Vec<SuraRow>, String> {
    if let Some(loc) = q
        .and_then(|q| q.get("suras"))
        .and_then(|x| x.as_str())
        .filter(|x| !x.is_empty())
    {
        let got = crate::sources::read_bytes(&crate::sources::absolute(base, loc), Some(120))
            .map_err(|e| e.0)
            .and_then(|raw| {
                serde_json::from_slice::<serde_json::Value>(&raw).map_err(|e| e.to_string())
            });
        match got {
            Ok(data) => {
                return Ok(data["suras"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|x| {
                        (
                            x["n"].as_i64().unwrap_or(0),
                            s(x, "name"),
                            s(x, "english"),
                            s(x, "arabic"),
                            s(x, "place"),
                            x["verses"].as_i64().unwrap_or(0),
                        )
                    })
                    .collect())
            }
            Err(e) => crate::log::warning("quran", &format!("The suras not read from {OWN}: {e}")),
        }
    }
    let raw =
        crate::sources::read_bytes(&format!("{BASE}/info.json"), Some(120)).map_err(|e| e.0)?;
    let meta: serde_json::Value = serde_json::from_slice(&raw).map_err(|e| e.to_string())?;
    Ok(meta["chapters"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|c| {
            (
                c["chapter"].as_i64().unwrap_or(0),
                s(c, "name"),
                s(c, "englishname"),
                s(c, "arabicname"),
                s(c, "revelation"),
                c["verses"].as_array().map(|v| v.len() as i64).unwrap_or(0),
            )
        })
        .collect())
}

/// Download the Arabic (once) and these editions; the keys kept. A text that cannot be read is said and the others
/// are still downloaded; an error only when none was (`quran.fetch`).
pub fn fetch(editions_: &[String], say: &dyn Fn(&str)) -> Result<Vec<String>, String> {
    let conn = connect_rw().map_err(|e| e.to_string())?;
    let (mine, q, base) = own_editions();
    let have: i64 = conn
        .query_row("SELECT COUNT(*) FROM suras", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if have == 0 {
        say("Reading the suras");
        let rows = suras_rows(q.as_ref(), &base)?;
        for r in rows {
            conn.execute(
                "INSERT OR REPLACE INTO suras VALUES (?, ?, ?, ?, ?, ?)",
                rusqlite::params![r.0, r.1, r.2, r.3, r.4, r.5],
            )
            .map_err(|e| e.to_string())?;
        }
    }
    let mut wanted: Vec<String> = if available() {
        vec![]
    } else {
        vec![ARABIC.to_string()]
    };
    for e in editions_ {
        if e != ARABIC && !wanted.contains(e) {
            wanted.push(e.clone());
        }
    }
    let mut info: HashMap<String, Offered> = mine.into_iter().collect();
    if wanted.iter().any(|e| !info.contains_key(e)) {
        match source_editions() {
            Ok(list) => {
                for (k, v) in list {
                    info.entry(k).or_insert(v);
                }
            }
            Err(e) => crate::log::warning(
                "quran",
                &format!("The list of {SOURCE} could not be read: {e}"),
            ),
        }
    }
    let (mut kept, mut failed) = (vec![], vec![]);
    for key in &wanted {
        let Some(about) = info.get(key) else {
            say(&format!("No translation '{key}'"));
            failed.push(format!("{key}: not listed"));
            continue;
        };
        say(&format!(
            "Downloading {} — {}",
            about.language, about.author
        ));
        let (rows, origin) = match read_edition(about, key) {
            Ok(x) => x,
            Err(e) => {
                failed.push(format!("{}: {e}", about.author));
                say(&format!("Not downloaded: {} ({e})", about.author));
                continue;
            }
        };
        let write = || -> rusqlite::Result<()> {
            let tx = conn.unchecked_transaction()?;
            tx.execute("DELETE FROM verses WHERE edition = ?", [key])?;
            {
                let mut st = tx.prepare("INSERT OR REPLACE INTO verses VALUES (?, ?, ?, ?)")?;
                for (s_, a, t) in &rows {
                    st.execute(rusqlite::params![key, s_, a, t])?;
                }
            }
            tx.execute(
                "INSERT OR REPLACE INTO editions (key, language, author, direction, source, comments, imported_at, origin) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                rusqlite::params![
                    key,
                    about.language,
                    about.author,
                    if about.direction.is_empty() { "ltr" } else { &about.direction },
                    about.source,
                    about.comments,
                    crate::sources::now_iso(),
                    origin
                ],
            )?;
            tx.commit()
        };
        write().map_err(|e| e.to_string())?;
        kept.push(key.clone());
        crate::log::info(
            "quran",
            &format!("Qur'an text {key} downloaded from {origin}"),
        );
    }
    if !failed.is_empty() && kept.is_empty() {
        return Err(failed.join("; "));
    }
    Ok(kept)
}
