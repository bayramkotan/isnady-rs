//! Meaning search (YZ1a, A4): cross-lingual latent semantic analysis, learnt from isnady's own data
//! (`core/semantic.py`).
//!
//! Every hadith is held in Arabic AND in its translations, side by side: a parallel corpus. Latent semantic analysis
//! (a truncated singular value decomposition of the TF-IDF matrix) over documents that join all the languages of one
//! hadith learns concepts shared by the languages, so a Turkish or English query finds an Arabic hadith that shares
//! none of its words. Nothing is downloaded; the model is learnt on this computer from the sources imported here.
//!
//! The index is the Python isnady's own files (`ai/lsa.npz`, `ai/lsa.vocab`, `ai/lsa.json`): built by either
//! program, read by both. The Python isnady finds the concepts with ARPACK (scipy's svds); here a randomized subspace
//! iteration finds the same subspace — the scores depend only on that subspace, not on the vectors chosen in it.

use rusqlite::{Connection, OptionalExtension};
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Mutex;

use crate::py::thousands;
use crate::search::{SearchQuery, SearchResult};

pub const METHOD_ID: &str = "isnady-lsa-2";
pub const DEFAULT_DIMS: usize = 200;
const MIN_DF: usize = 3;
const MAX_DF_SHARE: f64 = 0.4;
const LATIN_STEM: usize = 5;
pub const LEXICAL_WEIGHT: f32 = 0.5;
const PREFIXES: [&str; 8] = ["وبال", "فبال", "وال", "فال", "بال", "كال", "لل", "ال"];

pub const NOT_BUILT: &str =
    "The meaning index is not built yet: iy ai build  (or Tools → Build Meaning Index)";

fn say_t(say: &dyn Fn(&str), text: &str, values: &[(&str, String)]) {
    say(&crate::i18n::t(&crate::i18n::ui_language(), text, values));
}

fn model_dir() -> PathBuf {
    let d = crate::paths::data_dir().join("ai");
    let _ = std::fs::create_dir_all(&d);
    d
}

/// Python's str.isdigit: every character a digit (decimal digits of any script, and the superscripts).
fn py_isdigit(w: &str) -> bool {
    !w.is_empty()
        && w.chars().all(|c| {
            c.is_ascii_digit()
                || matches!(c, '\u{b2}' | '\u{b3}' | '\u{b9}' | '\u{2070}'..='\u{2079}' | '\u{2080}'..='\u{2089}')
                || (c.is_numeric() && c.to_digit(10).is_none() && is_decimal(c))
        })
}

/// A decimal digit of another script (Unicode Nd): Arabic-Indic, Persian, Devanagari … — each block starts at 0.
fn is_decimal(c: char) -> bool {
    const ZEROS: [u32; 22] = [
        0x660, 0x6F0, 0x7C0, 0x966, 0x9E6, 0xA66, 0xAE6, 0xB66, 0xBE6, 0xC66, 0xCE6, 0xD66, 0xDE6,
        0xE50, 0xED0, 0xF20, 0x1040, 0x1090, 0x17E0, 0x1810, 0xFF10, 0x1946,
    ];
    let u = c as u32;
    ZEROS.iter().any(|z| (*z..z + 10).contains(&u))
}

/// Words after isnady's normalization; Arabic words lose the article and joined particles (و ف ب ك ل + ال), so
/// بالنيات, والنية and النيات meet at نيات / نيه (`tokens`).
pub fn tokens(text: &str) -> Vec<String> {
    let mut out = vec![];
    for w in crate::reader::pyre::split(&crate::normalize::normalize(text)) {
        let mut w: String = w.to_string();
        if w.chars().any(|c| ('\u{600}'..='\u{6FF}').contains(&c)) {
            let n = w.chars().count();
            let mut cut = false;
            for p in PREFIXES {
                if w.starts_with(p) && n - p.chars().count() >= 3 {
                    w = w[p.len()..].to_string();
                    cut = true;
                    break;
                }
            }
            if !cut && (w.starts_with('و') || w.starts_with('ف')) && n > 4 {
                w = w.chars().skip(1).collect();
            }
        } else if w.chars().count() > LATIN_STEM {
            // Turkish is agglutinative; keeping the first five letters stands in for a stemmer ("F5")
            w = w.chars().take(LATIN_STEM).collect();
        }
        if w.chars().count() >= 2 && !py_isdigit(&w) {
            out.push(w);
        }
    }
    out
}

fn stamp(conn: &Connection) -> rusqlite::Result<String> {
    let (n, m): (i64, i64) = conn.query_row(
        "SELECT COUNT(*), COALESCE(MAX(id), 0) FROM texts",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    Ok(format!("{METHOD_ID}|{n}|{m}"))
}

/// The index as `lsa.json` says it, with "built" and "stale" (`status`).
pub fn status(conn: &Connection) -> Value {
    let Some(mut meta) = std::fs::read_to_string(model_dir().join("lsa.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .filter(Value::is_object)
    else {
        return json!({"built": false});
    };
    let now = stamp(conn).unwrap_or_default();
    let stale = meta.get("stamp").and_then(Value::as_str) != Some(now.as_str());
    meta["built"] = json!(true);
    meta["stale"] = json!(stale);
    meta
}

// ------------------------------------------------------------------ numpy's .npy and .npz

/// One array of an .npz: its type ("<f2", "<f4", "<i4", "<i8") and shape, and its values.
pub enum Array {
    F32(Vec<f32>),
    I64(Vec<i64>),
}

fn f16_to_f32(h: u16) -> f32 {
    let sign = ((h >> 15) as u32) << 31;
    let exp = ((h >> 10) & 0x1f) as u32;
    let frac = (h & 0x3ff) as u32;
    let bits = if exp == 0 {
        if frac == 0 {
            sign
        } else {
            // subnormal: frac × 2^-24
            let v = frac as f32 * (1.0 / 16_777_216.0);
            return if sign != 0 { -v } else { v };
        }
    } else if exp == 31 {
        sign | 0x7f80_0000 | (frac << 13)
    } else {
        sign | ((exp + 112) << 23) | (frac << 13)
    };
    f32::from_bits(bits)
}

/// numpy's float32 → float16: round to nearest, ties to even.
fn f32_to_f16(v: f32) -> u16 {
    let x = v.to_bits();
    let sign = ((x >> 16) & 0x8000) as u16;
    let abs = x & 0x7fff_ffff;
    if abs >= 0x7f80_0000 {
        return sign | 0x7c00 | if abs > 0x7f80_0000 { 0x200 } else { 0 };
    }
    if abs >= 0x477f_f000 {
        return sign | 0x7c00; // overflows to infinity
    }
    if abs < 0x3880_0000 {
        // subnormal or zero in half precision
        if abs < 0x3300_0000 {
            return sign;
        }
        let e = abs >> 23;
        let m = (abs & 0x7f_ffff) | 0x80_0000;
        let shift = 126 - e; // value × 2^24 = m × 2^(e − 126)
        let half = 1u32 << (shift - 1);
        let mut r = m >> shift;
        let rest = m & ((1 << shift) - 1);
        if rest > half || (rest == half && r & 1 == 1) {
            r += 1;
        }
        return sign | r as u16;
    }
    let e = (abs >> 23) - 112;
    let m = abs & 0x7f_ffff;
    let mut h = (e << 10) | (m >> 13);
    let rest = m & 0x1fff;
    if rest > 0x1000 || (rest == 0x1000 && h & 1 == 1) {
        h += 1;
    }
    sign | h as u16
}

fn parse_npy(data: &[u8]) -> Result<(Array, Vec<usize>), String> {
    if data.len() < 10 || &data[..6] != b"\x93NUMPY" {
        return Err("not a numpy array".into());
    }
    let (len, start) = if data[6] == 1 {
        (u16::from_le_bytes([data[8], data[9]]) as usize, 10)
    } else {
        (
            u32::from_le_bytes([data[8], data[9], data[10], data[11]]) as usize,
            12,
        )
    };
    let header = String::from_utf8_lossy(&data[start..start + len]).to_string();
    let descr = header
        .split("'descr':")
        .nth(1)
        .and_then(|s| s.split('\'').nth(1))
        .ok_or("no descr")?
        .to_string();
    let shape: Vec<usize> = header
        .split("'shape':")
        .nth(1)
        .and_then(|s| s.split('(').nth(1))
        .and_then(|s| s.split(')').next())
        .ok_or("no shape")?
        .split(',')
        .filter_map(|p| p.trim().parse().ok())
        .collect();
    let fortran = header.contains("'fortran_order': True");
    let body = &data[start + len..];
    let arr = match descr.as_str() {
        "<f2" => Array::F32(
            body.chunks_exact(2)
                .map(|b| f16_to_f32(u16::from_le_bytes([b[0], b[1]])))
                .collect(),
        ),
        "<f4" => Array::F32(
            body.chunks_exact(4)
                .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                .collect(),
        ),
        "<f8" => Array::F32(
            body.chunks_exact(8)
                .map(|b| f64::from_le_bytes(b.try_into().expect("8 bytes")) as f32)
                .collect(),
        ),
        "<i4" => Array::I64(
            body.chunks_exact(4)
                .map(|b| i32::from_le_bytes([b[0], b[1], b[2], b[3]]) as i64)
                .collect(),
        ),
        "<i8" => Array::I64(
            body.chunks_exact(8)
                .map(|b| i64::from_le_bytes(b.try_into().expect("8 bytes")))
                .collect(),
        ),
        other => return Err(format!("numpy type {other}")),
    };
    // a 2-D array kept column by column (svds gives one): its values put row by row
    if fortran && shape.len() == 2 {
        let (r, c) = (shape[0], shape[1]);
        let reorder = |v: &[f32]| -> Vec<f32> {
            let mut out = vec![0f32; v.len()];
            for i in 0..r {
                for j in 0..c {
                    out[i * c + j] = v[j * r + i];
                }
            }
            out
        };
        return Ok((
            match arr {
                Array::F32(v) => Array::F32(reorder(&v)),
                Array::I64(v) => {
                    let f: Vec<f32> = v.iter().map(|x| *x as f32).collect();
                    Array::I64(reorder(&f).into_iter().map(|x| x as i64).collect())
                }
            },
            shape,
        ));
    }
    Ok((arr, shape))
}

fn npy(descr: &str, shape: &[usize], body: &[u8]) -> Vec<u8> {
    let dims: Vec<String> = shape.iter().map(|d| d.to_string()).collect();
    let shape_text = if dims.len() == 1 {
        format!("({},)", dims[0])
    } else {
        format!("({})", dims.join(", "))
    };
    let mut header =
        format!("{{'descr': '{descr}', 'fortran_order': False, 'shape': {shape_text}, }}");
    let total = 10 + header.len() + 1;
    header.push_str(&" ".repeat((64 - total % 64) % 64));
    header.push('\n');
    let mut out = b"\x93NUMPY\x01\x00".to_vec();
    out.extend((header.len() as u16).to_le_bytes());
    out.extend(header.as_bytes());
    out.extend(body);
    out
}

fn read_npz(path: &std::path::Path) -> Result<HashMap<String, (Array, Vec<usize>)>, String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut z = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    let mut out = HashMap::new();
    for i in 0..z.len() {
        let mut f = z.by_index(i).map_err(|e| e.to_string())?;
        let name = f.name().trim_end_matches(".npy").to_string();
        let mut data = vec![];
        f.read_to_end(&mut data).map_err(|e| e.to_string())?;
        out.insert(name, parse_npy(&data)?);
    }
    Ok(out)
}

// ------------------------------------------------------------------ the model in memory

/// A sparse matrix, rows compressed (scipy's csr_matrix).
pub struct Csr {
    pub rows: usize,
    pub cols: usize,
    pub indptr: Vec<usize>,
    pub indices: Vec<usize>,
    pub data: Vec<f32>,
}

pub struct Model {
    index: HashMap<String, usize>,
    idf: Vec<f32>,
    /// k × terms
    components: Vec<f32>,
    k: usize,
    ids: Vec<i64>,
    /// documents × k
    vectors: Vec<f32>,
    x: Csr,
}

static MODEL: Mutex<Option<std::sync::Arc<Model>>> = Mutex::new(None);

/// Forget the model read (after a build).
pub fn clear() {
    if let Ok(mut m) = MODEL.lock() {
        *m = None;
    }
}

fn take(
    map: &mut HashMap<String, (Array, Vec<usize>)>,
    name: &str,
) -> Result<(Array, Vec<usize>), String> {
    map.remove(name)
        .ok_or_else(|| format!("lsa.npz has no {name}"))
}

fn floats(a: Array) -> Vec<f32> {
    match a {
        Array::F32(v) => v,
        Array::I64(v) => v.into_iter().map(|x| x as f32).collect(),
    }
}

fn ints(a: Array) -> Vec<i64> {
    match a {
        Array::I64(v) => v,
        Array::F32(v) => v.into_iter().map(|x| x as i64).collect(),
    }
}

fn load() -> Result<std::sync::Arc<Model>, String> {
    if let Some(m) = MODEL.lock().map_err(|e| e.to_string())?.as_ref() {
        return Ok(m.clone());
    }
    let folder = model_dir();
    let path = folder.join("lsa.npz");
    if !path.exists() {
        return Err(NOT_BUILT.into());
    }
    let mut data = read_npz(&path)?;
    let vocab = std::fs::read_to_string(folder.join("lsa.vocab")).map_err(|e| e.to_string())?;
    let index: HashMap<String, usize> = vocab
        .split('\n')
        .enumerate()
        .map(|(i, t)| (t.to_string(), i))
        .collect();
    let (idf, _) = take(&mut data, "idf")?;
    let (components, cshape) = take(&mut data, "components")?;
    let (ids, _) = take(&mut data, "doc_ids")?;
    let (vectors, _) = take(&mut data, "doc_vectors")?;
    let (x_data, _) = take(&mut data, "x_data")?;
    let (x_indices, _) = take(&mut data, "x_indices")?;
    let (x_indptr, _) = take(&mut data, "x_indptr")?;
    let (x_shape, _) = take(&mut data, "x_shape")?;
    let shape = ints(x_shape);
    let model = Model {
        index,
        idf: floats(idf),
        components: floats(components),
        k: cshape.first().copied().unwrap_or(0),
        ids: ints(ids),
        vectors: floats(vectors),
        x: Csr {
            rows: shape.first().copied().unwrap_or(0) as usize,
            cols: shape.get(1).copied().unwrap_or(0) as usize,
            indptr: ints(x_indptr).into_iter().map(|v| v as usize).collect(),
            indices: ints(x_indices).into_iter().map(|v| v as usize).collect(),
            data: floats(x_data),
        },
    };
    let model = std::sync::Arc::new(model);
    *MODEL.lock().map_err(|e| e.to_string())? = Some(model.clone());
    Ok(model)
}

/// The text's TF-IDF vector over the learnt vocabulary (unit length) as (term, weight), or None (`_weights`).
fn weights(m: &Model, text: &str) -> Option<Vec<(usize, f32)>> {
    let mut counts: Vec<(usize, usize)> = vec![];
    for t in tokens(text) {
        if let Some(&j) = m.index.get(&t) {
            match counts.iter_mut().find(|c| c.0 == j) {
                Some(c) => c.1 += 1,
                None => counts.push((j, 1)),
            }
        }
    }
    if counts.is_empty() {
        return None;
    }
    let q: Vec<(usize, f32)> = counts
        .into_iter()
        .map(|(j, c)| (j, (1.0 + (c as f64).ln()) as f32 * m.idf[j]))
        .collect();
    let norm = q
        .iter()
        .map(|(_, w)| (*w as f64) * (*w as f64))
        .sum::<f64>()
        .sqrt() as f32;
    let norm = if norm == 0.0 { 1.0 } else { norm };
    Some(q.into_iter().map(|(j, w)| (j, w / norm)).collect())
}

/// Hybrid score for every indexed hadith: shared words and shared concepts (`scores`).
fn scores(m: &Model, text: &str) -> Option<Vec<f32>> {
    let q = weights(m, text)?;
    let k = m.k;
    let terms = m.idf.len();
    let mut v = vec![0f64; k];
    for (r, out) in v.iter_mut().enumerate() {
        let row = &m.components[r * terms..(r + 1) * terms];
        *out = q
            .iter()
            .map(|(j, w)| row[*j] as f64 * *w as f64)
            .sum::<f64>();
    }
    let v: Vec<f32> = v.iter().map(|x| *x as f32).collect();
    let norm = v
        .iter()
        .map(|x| (*x as f64) * (*x as f64))
        .sum::<f64>()
        .sqrt() as f32;
    let norm = if norm == 0.0 { 1.0 } else { norm };
    let v: Vec<f32> = v.iter().map(|x| x / norm).collect();
    let qd: HashMap<usize, f32> = q.into_iter().collect();
    let n = m.ids.len();
    let mut out = vec![0f32; n];
    for (d, o) in out.iter_mut().enumerate() {
        let vec = &m.vectors[d * k..(d + 1) * k];
        let concept = vec
            .iter()
            .zip(&v)
            .map(|(a, b)| *a as f64 * *b as f64)
            .sum::<f64>() as f32;
        let mut lexical = 0f64;
        for p in m.x.indptr[d]..m.x.indptr[d + 1] {
            if let Some(w) = qd.get(&m.x.indices[p]) {
                lexical += m.x.data[p] as f64 * *w as f64;
            }
        }
        *o = LEXICAL_WEIGHT * lexical as f32 + (1.0 - LEXICAL_WEIGHT) * concept;
    }
    Some(out)
}

/// A meaning search's page (`MeaningPage`).
pub struct MeaningPage {
    pub total: usize,
    pub results: Vec<SearchResult>,
    pub scores: Vec<f32>,
    pub elapsed_ms: u128,
    pub unknown_words: Vec<String>,
}

/// Hadith closest in meaning to the query, best first; scores are cosine similarities (`search`).
pub fn search(
    conn: &Connection,
    query: &SearchQuery,
    min_score: f32,
) -> Result<MeaningPage, String> {
    let started = std::time::Instant::now();
    let m = load()?;
    let toks = tokens(&query.text);
    let known = toks.iter().any(|t| m.index.contains_key(t));
    let unknown: Vec<String> = toks
        .iter()
        .filter(|t| !m.index.contains_key(*t))
        .cloned()
        .collect();
    let Some(all) = scores(&m, &query.text) else {
        return Ok(MeaningPage {
            total: 0,
            results: vec![],
            scores: vec![],
            elapsed_ms: 0,
            unknown_words: unknown,
        });
    };
    let mut order: Vec<usize> = (0..all.len()).collect();
    order.sort_by(|a, b| {
        all[*b]
            .partial_cmp(&all[*a])
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.cmp(b))
    });
    let keys = if query.collections.is_empty() {
        crate::books::active_keys(conn).map_err(|e| e.to_string())?
    } else {
        Some(query.collections.clone())
    };
    let allowed: Option<HashSet<i64>> = match keys {
        None => None,
        Some(keys) => {
            let sql = format!(
                "SELECT h.id FROM hadiths h JOIN collections c ON c.id = h.collection_id WHERE c.key IN ({})",
                if keys.is_empty() { "NULL".to_string() } else { vec!["?"; keys.len()].join(",") }
            );
            let mut st = conn.prepare(&sql).map_err(|e| e.to_string())?;
            let ids = st
                .query_map(rusqlite::params_from_iter(keys.iter()), |r| {
                    r.get::<_, i64>(0)
                })
                .map_err(|e| e.to_string())?
                .collect::<Result<HashSet<i64>, _>>()
                .map_err(|e| e.to_string())?;
            Some(ids)
        }
    };
    let graded = crate::search::grade_filter(conn, query).map_err(|e| e.to_string())?;
    let mut picked: Vec<(i64, f32)> = vec![];
    for i in order {
        if all[i] < min_score {
            break;
        }
        let hid = m.ids[i];
        if allowed.as_ref().is_some_and(|a| !a.contains(&hid))
            || graded.as_ref().is_some_and(|g| !g.contains(&hid))
        {
            continue;
        }
        picked.push((hid, all[i]));
    }
    let page: Vec<&(i64, f32)> = picked.iter().skip(query.offset).take(query.limit).collect();
    let mut results = vec![];
    for (hid, _) in &page {
        results
            .push(crate::search::load_result(conn, *hid, &[], query).map_err(|e| e.to_string())?);
    }
    Ok(MeaningPage {
        total: picked.len(),
        results,
        scores: page.iter().map(|p| p.1).collect(),
        elapsed_ms: started.elapsed().as_millis(),
        unknown_words: if known { vec![] } else { unknown },
    })
}

// ------------------------------------------------------------------ building

fn threads() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .clamp(1, 16)
}

/// y = A · m for a dense m (cols × l, rows first): rows × l.
fn mul(a: &Csr, m: &[f64], l: usize) -> Vec<f64> {
    let mut out = vec![0f64; a.rows * l];
    let chunk = a.rows.div_ceil(threads()).max(1);
    std::thread::scope(|s| {
        for (c, part) in out.chunks_mut(chunk * l).enumerate() {
            s.spawn(move || {
                for (r, row) in part.chunks_mut(l).enumerate() {
                    let i = c * chunk + r;
                    for p in a.indptr[i]..a.indptr[i + 1] {
                        let v = a.data[p] as f64;
                        let src = &m[a.indices[p] * l..(a.indices[p] + 1) * l];
                        for (o, x) in row.iter_mut().zip(src) {
                            *o += v * x;
                        }
                    }
                }
            });
        }
    });
    out
}

fn transpose(a: &Csr) -> Csr {
    let mut count = vec![0usize; a.cols + 1];
    for &j in &a.indices {
        count[j + 1] += 1;
    }
    for j in 0..a.cols {
        count[j + 1] += count[j];
    }
    let mut next = count.clone();
    let mut indices = vec![0usize; a.indices.len()];
    let mut data = vec![0f32; a.data.len()];
    for i in 0..a.rows {
        for p in a.indptr[i]..a.indptr[i + 1] {
            let j = a.indices[p];
            indices[next[j]] = i;
            data[next[j]] = a.data[p];
            next[j] += 1;
        }
    }
    Csr {
        rows: a.cols,
        cols: a.rows,
        indptr: count,
        indices,
        data,
    }
}

/// G = Yᵀ Y (l × l).
fn gram(y: &[f64], l: usize) -> Vec<f64> {
    let n = y.len() / l;
    let chunk = n.div_ceil(threads()).max(1);
    let parts: Vec<Vec<f64>> = std::thread::scope(|s| {
        let handles: Vec<_> = y
            .chunks(chunk * l)
            .map(|part| {
                s.spawn(move || {
                    let mut g = vec![0f64; l * l];
                    for row in part.chunks(l) {
                        for a in 0..l {
                            let ra = row[a];
                            if ra == 0.0 {
                                continue;
                            }
                            let ga = &mut g[a * l..a * l + l];
                            for b in a..l {
                                ga[b] += ra * row[b];
                            }
                        }
                    }
                    g
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("thread"))
            .collect()
    });
    let mut g = vec![0f64; l * l];
    for p in parts {
        for (o, v) in g.iter_mut().zip(p) {
            *o += v;
        }
    }
    for a in 0..l {
        for b in 0..a {
            g[a * l + b] = g[b * l + a];
        }
    }
    g
}

/// The columns of Y made orthonormal in place (Cholesky QR, twice).
fn orthonormalize(y: &mut [f64], l: usize) {
    for _ in 0..2 {
        let mut g = gram(y, l);
        let trace: f64 = (0..l).map(|i| g[i * l + i]).sum();
        let shift = trace * 1e-13 + 1e-300;
        // Cholesky: G = Rᵀ R, R upper triangular
        let mut r = vec![0f64; l * l];
        for i in 0..l {
            g[i * l + i] += shift;
            for j in i..l {
                let mut s = g[i * l + j];
                for k in 0..i {
                    s -= r[k * l + i] * r[k * l + j];
                }
                if i == j {
                    r[i * l + i] = s.max(shift).sqrt();
                } else {
                    r[i * l + j] = s / r[i * l + i];
                }
            }
        }
        // Y ← Y R⁻¹, each row solving x R = y
        let chunk = (y.len() / l).div_ceil(threads()).max(1);
        let r = &r;
        std::thread::scope(|s| {
            for part in y.chunks_mut(chunk * l) {
                s.spawn(move || {
                    for row in part.chunks_mut(l) {
                        for j in 0..l {
                            let mut v = row[j];
                            for k in 0..j {
                                v -= row[k] * r[k * l + j];
                            }
                            row[j] = v / r[j * l + j];
                        }
                    }
                });
            }
        });
    }
}

/// Eigenvalues and eigenvectors (columns) of a symmetric matrix, by cyclic Jacobi rotations.
fn jacobi(mut a: Vec<f64>, l: usize) -> (Vec<f64>, Vec<f64>) {
    let mut v = vec![0f64; l * l];
    for i in 0..l {
        v[i * l + i] = 1.0;
    }
    for _sweep in 0..60 {
        let off: f64 = (0..l)
            .flat_map(|i| (0..l).filter(move |j| *j != i).map(move |j| (i, j)))
            .map(|(i, j)| a[i * l + j].powi(2))
            .sum();
        let diag: f64 = (0..l).map(|i| a[i * l + i].powi(2)).sum();
        if off <= diag * 1e-30 {
            break;
        }
        for p in 0..l {
            for q in p + 1..l {
                let apq = a[p * l + q];
                if apq.abs() < 1e-300 {
                    continue;
                }
                let theta = (a[q * l + q] - a[p * l + p]) / (2.0 * apq);
                let t = theta.signum() / (theta.abs() + (theta * theta + 1.0).sqrt());
                let t = if theta == 0.0 { 1.0 } else { t };
                let c = 1.0 / (t * t + 1.0).sqrt();
                let s = t * c;
                for k in 0..l {
                    let akp = a[k * l + p];
                    let akq = a[k * l + q];
                    a[k * l + p] = c * akp - s * akq;
                    a[k * l + q] = s * akp + c * akq;
                }
                for k in 0..l {
                    let apk = a[p * l + k];
                    let aqk = a[q * l + k];
                    a[p * l + k] = c * apk - s * aqk;
                    a[q * l + k] = s * apk + c * aqk;
                }
                for k in 0..l {
                    let vkp = v[k * l + p];
                    let vkq = v[k * l + q];
                    v[k * l + p] = c * vkp - s * vkq;
                    v[k * l + q] = s * vkp + c * vkq;
                }
            }
        }
    }
    ((0..l).map(|i| a[i * l + i]).collect(), v)
}

/// The k right singular vectors of A with the largest singular values (k × cols, rows first): a randomized
/// subspace iteration (Halko, Martinsson and Tropp 2011), with more power steps and oversampling than usual so the
/// subspace is the one ARPACK finds.
fn top_right_vectors(a: &Csr, k: usize, say: &dyn Fn(&str)) -> Vec<f64> {
    let extra: usize = std::env::var("ISNADY_LSA_EXTRA")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(60);
    let l = (k + extra).min(a.rows.min(a.cols));
    let at = transpose(a);
    // a fixed start: the same index every time on the same data
    let mut seed: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut omega = vec![0f64; a.cols * l];
    for v in omega.iter_mut() {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        *v = (seed >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0;
    }
    let mut y = mul(a, &omega, l);
    orthonormalize(&mut y, l);
    let steps: usize = std::env::var("ISNADY_LSA_STEPS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(16);
    for step in 0..steps {
        let mut z = mul(&at, &y, l);
        orthonormalize(&mut z, l);
        y = mul(a, &z, l);
        orthonormalize(&mut y, l);
        say(&format!("{} / {steps}", step + 1));
    }
    // B = Qᵀ A (l × cols), kept as Bᵀ = Aᵀ Q (cols × l)
    let bt = mul(&at, &y, l);
    let g = gram(&bt, l); // B Bᵀ
    let (values, vectors) = jacobi(g, l);
    let mut order: Vec<usize> = (0..l).collect();
    order.sort_by(|x, y| {
        values[*y]
            .partial_cmp(&values[*x])
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut out = vec![0f64; k * a.cols];
    for (r, &e) in order.iter().take(k).enumerate() {
        let s = values[e].max(0.0).sqrt();
        if s == 0.0 {
            continue;
        }
        // v = Bᵀ w / s
        let row = &mut out[r * a.cols..(r + 1) * a.cols];
        for (c, o) in row.iter_mut().enumerate() {
            let b = &bt[c * l..(c + 1) * l];
            *o = (0..l).map(|i| b[i] * vectors[i * l + e]).sum::<f64>() / s;
        }
    }
    out
}

/// Learn the concepts from the imported texts and write the index (`build`).
pub fn build(conn: &Connection, dims: usize, say: &dyn Fn(&str)) -> Result<Value, String> {
    let started = std::time::Instant::now();
    say_t(say, "Reading texts", &[]);
    let mut ids: Vec<i64> = vec![];
    let mut docs: HashMap<i64, Vec<String>> = HashMap::new();
    {
        let mut st = conn
            .prepare("SELECT hadith_id, text FROM texts ORDER BY hadith_id")
            .map_err(|e| e.to_string())?;
        let rows = st
            .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
            .map_err(|e| e.to_string())?;
        for row in rows {
            let (hid, text) = row.map_err(|e| e.to_string())?;
            let entry = docs.entry(hid).or_insert_with(|| {
                ids.push(hid);
                vec![]
            });
            entry.extend(tokens(&text));
        }
    }
    let n = ids.len();
    if n < 50 {
        return Err(
            "Import some hadith first: meaning search learns from the imported texts.".into(),
        );
    }
    say_t(
        say,
        "Learning the vocabulary of {n:,} hadith",
        &[("n:,", thousands(n as i64))],
    );
    let mut df: HashMap<&str, usize> = HashMap::new();
    for toks in docs.values() {
        let set: HashSet<&str> = toks.iter().map(String::as_str).collect();
        for t in set {
            *df.entry(t).or_default() += 1;
        }
    }
    let mut vocab: Vec<&str> = df
        .iter()
        .filter(|(_, c)| **c >= MIN_DF && **c as f64 <= MAX_DF_SHARE * n as f64)
        .map(|(t, _)| *t)
        .collect();
    vocab.sort_unstable();
    let index: HashMap<&str, usize> = vocab.iter().enumerate().map(|(i, t)| (*t, i)).collect();
    let idf: Vec<f32> = vocab
        .iter()
        .map(|t| (((1 + n) as f64 / (1 + df[t]) as f64).ln() + 1.0) as f32)
        .collect();
    let mut indptr = vec![0usize];
    let mut indices = vec![];
    let mut data: Vec<f32> = vec![];
    for hid in &ids {
        let mut counts: Vec<(&str, usize)> = vec![];
        let mut at: HashMap<&str, usize> = HashMap::new();
        for t in &docs[hid] {
            match at.get(t.as_str()) {
                Some(&p) => counts[p].1 += 1,
                None => {
                    at.insert(t.as_str(), counts.len());
                    counts.push((t.as_str(), 1));
                }
            }
        }
        let mut row: Vec<(usize, f32)> = counts
            .into_iter()
            .filter_map(|(t, c)| {
                index
                    .get(t)
                    .map(|&j| (j, (1.0 + (c as f64).ln()) as f32 * idf[j]))
            })
            .collect();
        // as scipy keeps the row (the terms from the last to the first), the squares summed in float32 in that
        // order, then × (1 / norm)
        row.sort_unstable_by_key(|b| std::cmp::Reverse(b.0));
        let norm = row.iter().map(|(_, v)| *v * *v).sum::<f32>().sqrt();
        let inverse = 1.0f32 / if norm == 0.0 { 1.0 } else { norm };
        for (j, v) in row {
            indices.push(j);
            data.push(v * inverse);
        }
        indptr.push(indices.len());
    }
    let x = Csr {
        rows: n,
        cols: vocab.len(),
        indptr,
        indices,
        data,
    };
    let k = 10.max(dims.min(n.min(x.cols).saturating_sub(1)));
    say_t(
        say,
        "Finding {k} shared concepts across the languages",
        &[("k", k.to_string())],
    );
    let vt = top_right_vectors(&x, k, &|_| {});
    let vt32: Vec<f32> = vt.iter().map(|v| *v as f32).collect();
    // the documents in the concept space, unit length
    let vt_cols: Vec<f64> = {
        // cols × k, for x · vtᵀ
        let mut t = vec![0f64; x.cols * k];
        for r in 0..k {
            for c in 0..x.cols {
                t[c * k + r] = vt32[r * x.cols + c] as f64;
            }
        }
        t
    };
    let vecs = mul(&x, &vt_cols, k);
    let mut doc_vectors: Vec<u16> = Vec::with_capacity(n * k);
    for row in vecs.chunks(k) {
        let row32: Vec<f32> = row.iter().map(|v| *v as f32).collect();
        let len = row32.iter().map(|v| v * v).sum::<f32>().sqrt();
        let len = if len == 0.0 { 1.0 } else { len };
        doc_vectors.extend(row32.iter().map(|v| f32_to_f16(v / len)));
    }
    let folder = model_dir();
    let le16 = |v: &[u16]| v.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>();
    let arrays: Vec<(&str, Vec<u8>)> = vec![
        (
            "idf",
            npy(
                "<f4",
                &[vocab.len()],
                &idf.iter()
                    .flat_map(|v| v.to_le_bytes())
                    .collect::<Vec<u8>>(),
            ),
        ),
        (
            "components",
            npy(
                "<f2",
                &[k, vocab.len()],
                &le16(&vt32.iter().map(|v| f32_to_f16(*v)).collect::<Vec<u16>>()),
            ),
        ),
        (
            "doc_ids",
            npy(
                "<i8",
                &[n],
                &ids.iter()
                    .flat_map(|v| v.to_le_bytes())
                    .collect::<Vec<u8>>(),
            ),
        ),
        ("doc_vectors", npy("<f2", &[n, k], &le16(&doc_vectors))),
        (
            "x_data",
            npy(
                "<f2",
                &[x.data.len()],
                &le16(&x.data.iter().map(|v| f32_to_f16(*v)).collect::<Vec<u16>>()),
            ),
        ),
        (
            "x_indices",
            npy(
                "<i4",
                &[x.indices.len()],
                &x.indices
                    .iter()
                    .flat_map(|v| (*v as i32).to_le_bytes())
                    .collect::<Vec<u8>>(),
            ),
        ),
        (
            "x_indptr",
            npy(
                "<i8",
                &[x.indptr.len()],
                &x.indptr
                    .iter()
                    .flat_map(|v| (*v as i64).to_le_bytes())
                    .collect::<Vec<u8>>(),
            ),
        ),
        (
            "x_shape",
            npy(
                "<i8",
                &[2],
                &[(n as i64).to_le_bytes(), (x.cols as i64).to_le_bytes()].concat(),
            ),
        ),
    ];
    let tmp = folder.join("lsa.npz.part");
    {
        let mut z = zip::ZipWriter::new(std::fs::File::create(&tmp).map_err(|e| e.to_string())?);
        for (name, body) in arrays {
            z.start_file(
                format!("{name}.npy"),
                zip::write::FileOptions::default()
                    .compression_method(zip::CompressionMethod::Deflated)
                    .large_file(body.len() > 0x7fff_0000),
            )
            .map_err(|e| e.to_string())?;
            z.write_all(&body).map_err(|e| e.to_string())?;
        }
        z.finish().map_err(|e| e.to_string())?;
    }
    std::fs::rename(&tmp, folder.join("lsa.npz")).map_err(|e| e.to_string())?;
    std::fs::write(folder.join("lsa.vocab"), vocab.join("\n")).map_err(|e| e.to_string())?;
    let seconds = (started.elapsed().as_secs_f64() * 10.0).round() / 10.0;
    let mut meta = Map::new();
    meta.insert("method".into(), json!(METHOD_ID));
    meta.insert("dims".into(), json!(k));
    meta.insert("documents".into(), json!(n));
    meta.insert("terms".into(), json!(vocab.len()));
    meta.insert(
        "stamp".into(),
        json!(stamp(conn).map_err(|e| e.to_string())?),
    );
    meta.insert(
        "built_at".into(),
        json!(crate::py::now_local("%Y-%m-%d %H:%M")),
    );
    meta.insert("seconds".into(), json!(seconds));
    let meta = Value::Object(meta);
    std::fs::write(
        folder.join("lsa.json"),
        crate::stats::pyjson::dumps_indent(&meta, 2),
    )
    .map_err(|e| e.to_string())?;
    clear();
    say_t(
        say,
        "Meaning index ready: {n:,} hadith, {vocab:,} words, {k} concepts",
        &[
            ("n:,", thousands(n as i64)),
            ("vocab:,", thousands(vocab.len() as i64)),
            ("k", k.to_string()),
        ],
    );
    Ok(meta)
}

/// Whether the index there is (`lsa.npz`): for the window's "build it first".
pub fn built() -> bool {
    model_dir().join("lsa.npz").exists()
}

/// One hadith's number in its book, for messages.
pub fn hadith_named(conn: &Connection, hid: i64) -> Option<String> {
    conn.query_row(
        "SELECT c.key || ' ' || h.number FROM hadiths h JOIN collections c ON c.id = h.collection_id WHERE h.id = ?",
        [hid],
        |r| r.get(0),
    )
    .optional()
    .ok()
    .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn half_floats_round_trip() {
        for v in [
            0.0f32, 1.0, -2.5, 0.1, 0.333_333, 1e-5, 6.0e-8, 65504.0, -0.000_123,
        ] {
            let h = f32_to_f16(v);
            let back = f16_to_f32(h);
            assert!((back - v).abs() <= v.abs() * 1e-3 + 6e-8, "{v} → {back}");
        }
        assert_eq!(f32_to_f16(1.0), 0x3c00);
        assert_eq!(f32_to_f16(0.1), 0x2e66);
    }

    #[test]
    fn tokens_as_python() {
        assert_eq!(
            tokens("بالنيات والنية"),
            vec!["نيات".to_string(), "نيه".to_string()]
        );
    }
}
