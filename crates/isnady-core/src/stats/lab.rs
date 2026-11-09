//! The correlation and test laboratory (`stats_lab.py`): Spearman matrices with Benjamini–Hochberg q-values and the
//! partial correlations, Stuart–Maxwell for the scholars of one book, Brant's test of the hadith model's shape, the
//! narrators' measures, and the books compared (Kruskal–Wallis, Mann–Whitney with Cliff's delta, Holm).

use rusqlite::Connection;
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};

use super::corpus::{pagerank, CachedError};
use super::models::{hadith_features, merge_categories, varying, Design, Feat};
use super::num::{
    bh, chi2_sf, fsum, holm, inverse, normal_sf, pearson, rankdata, sigmoid, solve, t_two_sided,
};
use super::{books_of, chains, f, fo, persons, road_of, Counter, Grades, NotSaved, Road};

pub const METHOD: &str = "isnady-lab-3";
pub const ALPHA: f64 = 0.05;

pub type Column = Vec<Option<f64>>;

fn spearman(x: &Column, y: &Column) -> Map<String, Value> {
    let pairs: Vec<(f64, f64)> = x
        .iter()
        .zip(y)
        .filter_map(|(a, b)| Some(((*a)?, (*b)?)))
        .collect();
    let n = pairs.len();
    let none = |n: usize| {
        let mut m = Map::new();
        m.insert("rho".into(), Value::Null);
        m.insert("n".into(), json!(n));
        m.insert("p".into(), Value::Null);
        m
    };
    if n < 10 {
        return none(n);
    }
    let ra = rankdata(&pairs.iter().map(|p| p.0).collect::<Vec<_>>());
    let rb = rankdata(&pairs.iter().map(|p| p.1).collect::<Vec<_>>());
    let Some(rho) = pearson(&ra, &rb) else {
        return none(n);
    };
    let rho = rho.clamp(-1.0, 1.0);
    let t = rho * ((n as f64 - 2.0) / (1.0 - rho * rho).max(1e-12)).sqrt();
    let mut m = Map::new();
    m.insert("rho".into(), f(rho));
    m.insert("n".into(), json!(n));
    m.insert("p".into(), f(t_two_sided(t, (n - 2) as f64)));
    m
}

fn distinct_known(col: &Column) -> usize {
    super::num::distinct(col.iter().flatten().copied())
}

/// Spearman's rho of every pair, BH over the pairs, and the partial correlations (`correlation_matrix`).
pub fn correlation_matrix(names: &[String], columns: &[Column]) -> Map<String, Value> {
    let k = names.len();
    let mut cells: Vec<Vec<Option<Map<String, Value>>>> = vec![vec![None; k]; k];
    let mut flat = vec![];
    for i in 0..k {
        for j in i + 1..k {
            let s = spearman(&columns[i], &columns[j]);
            if let Some(p) = s["p"].as_f64() {
                flat.push((i, j, p));
            }
            cells[i][j] = Some(s.clone());
            cells[j][i] = Some(s);
        }
    }
    let qs = bh(&flat.iter().map(|x| x.2).collect::<Vec<_>>());
    for ((i, j, _), q) in flat.iter().zip(qs) {
        for (a, b) in [(*i, *j), (*j, *i)] {
            if let Some(c) = cells[a][b].as_mut() {
                c.insert("q".into(), f(q));
            }
        }
    }
    let rows = columns.first().map(|c| c.len()).unwrap_or(0);
    let complete: Vec<usize> = (0..rows)
        .filter(|r| columns.iter().all(|c| c[*r].is_some()))
        .collect();
    let mut partial: Option<Vec<Vec<Option<Map<String, Value>>>>> = None;
    if complete.len() > k + 10 {
        let ranked: Vec<Vec<f64>> = (0..k)
            .map(|c| {
                rankdata(
                    &complete
                        .iter()
                        .map(|r| columns[c][*r].unwrap())
                        .collect::<Vec<_>>(),
                )
            })
            .collect();
        let keep: Vec<usize> = (0..k)
            .filter(|c| super::num::distinct(ranked[*c].iter().copied()) > 1)
            .collect();
        if keep.len() >= 3 {
            let corr: Vec<Vec<f64>> = keep
                .iter()
                .map(|&a| {
                    keep.iter()
                        .map(|&b| {
                            if a == b {
                                1.0
                            } else {
                                pearson(&ranked[a], &ranked[b])
                                    .filter(|x| *x != 0.0)
                                    .unwrap_or(0.0)
                            }
                        })
                        .collect()
                })
                .collect();
            if let Some(inv) = inverse(&corr) {
                let (n, m) = (complete.len(), keep.len());
                let mut grid = vec![vec![None; k]; k];
                let mut pflat = vec![];
                for a in 0..m {
                    for b in a + 1..m {
                        let denom = (inv[a][a] * inv[b][b]).abs().sqrt();
                        let r = if denom != 0.0 {
                            (-inv[a][b] / denom).clamp(-1.0, 1.0)
                        } else {
                            0.0
                        };
                        let df = n as f64 - 2.0 - (m as f64 - 2.0);
                        let t = r * (df / (1.0 - r * r).max(1e-12)).sqrt();
                        let p = t_two_sided(t, df);
                        let mut cell = Map::new();
                        cell.insert("rho".into(), f(r));
                        cell.insert("n".into(), json!(n));
                        cell.insert("p".into(), f(p));
                        grid[keep[a]][keep[b]] = Some(cell.clone());
                        grid[keep[b]][keep[a]] = Some(cell);
                        pflat.push((keep[a], keep[b], p));
                    }
                }
                let qs = bh(&pflat.iter().map(|x| x.2).collect::<Vec<_>>());
                for ((i, j, _), q) in pflat.iter().zip(qs) {
                    for (a, b) in [(*i, *j), (*j, *i)] {
                        if let Some(c) = grid[a][b].as_mut() {
                            c.insert("q".into(), f(q));
                        }
                    }
                }
                partial = Some(grid);
            }
        }
    }
    let grid_json = |g: &Vec<Vec<Option<Map<String, Value>>>>| -> Value {
        Value::Array(
            g.iter()
                .map(|row| {
                    Value::Array(
                        row.iter()
                            .map(|c| c.clone().map(Value::Object).unwrap_or(Value::Null))
                            .collect(),
                    )
                })
                .collect(),
        )
    };
    let mut out = Map::new();
    out.insert("names".into(), json!(names));
    out.insert("cells".into(), grid_json(&cells));
    out.insert(
        "partial".into(),
        partial.as_ref().map(grid_json).unwrap_or(Value::Null),
    );
    out.insert("complete".into(), json!(complete.len()));
    out
}

/// Kruskal–Wallis H (ties corrected), its p and epsilon².
pub fn kruskal(groups: &[&Vec<f64>]) -> Value {
    let groups: Vec<&&Vec<f64>> = groups.iter().filter(|g| !g.is_empty()).collect();
    if groups.len() < 2 {
        return Value::Null;
    }
    let allv: Vec<f64> = groups.iter().flat_map(|g| g.iter().copied()).collect();
    let n = allv.len() as i64;
    let ranks = rankdata(&allv);
    let (mut pos, mut h) = (0usize, 0.0f64);
    for g in &groups {
        let r = &ranks[pos..pos + g.len()];
        pos += g.len();
        let s = fsum(r.iter().copied());
        h += s * s / g.len() as f64;
    }
    h = 12.0 / (n * (n + 1)) as f64 * h - (3 * (n + 1)) as f64;
    let ties = tie_sum(&allv);
    let corr = if n > 1 {
        1.0 - ties as f64 / (n * n * n - n) as f64
    } else {
        1.0
    };
    h = if corr != 0.0 { h / corr } else { 0.0 };
    json!({"H": f(h), "df": groups.len() - 1, "p": f(chi2_sf(h, (groups.len() - 1) as f64)),
           "epsilon2": if n > 1 { f(h / (n - 1) as f64) } else { Value::Null }, "n": n})
}

fn tie_sum(values: &[f64]) -> i64 {
    let mut c: HashMap<u64, i64> = HashMap::new();
    for v in values {
        let key = if *v == 0.0 {
            0f64.to_bits()
        } else {
            v.to_bits()
        };
        *c.entry(key).or_insert(0) += 1;
    }
    c.values().map(|t| t * t * t - t).sum()
}

/// Mann–Whitney U (normal approximation, ties corrected) and Cliff's delta.
pub fn mann_whitney(a: &[f64], b: &[f64]) -> Option<Map<String, Value>> {
    let (n1, n2) = (a.len() as i64, b.len() as i64);
    if n1 < 5 || n2 < 5 {
        return None;
    }
    let both: Vec<f64> = a.iter().chain(b).copied().collect();
    let ranks = rankdata(&both);
    let r1 = fsum(ranks[..n1 as usize].iter().copied());
    let u1 = r1 - (n1 * (n1 + 1)) as f64 / 2.0;
    let n = n1 + n2;
    let ties = tie_sum(&both);
    let var = (n1 * n2) as f64 / 12.0 * ((n + 1) as f64 - ties as f64 / (n * (n - 1)) as f64);
    let z = if var > 0.0 {
        (u1 - (n1 * n2) as f64 / 2.0) / var.sqrt()
    } else {
        0.0
    };
    let mut m = Map::new();
    m.insert("U".into(), f(u1));
    m.insert("z".into(), f(z));
    m.insert("p".into(), f(2.0 * normal_sf(z.abs())));
    m.insert("delta".into(), f(2.0 * u1 / (n1 * n2) as f64 - 1.0));
    m.insert("n1".into(), json!(n1));
    m.insert("n2".into(), json!(n2));
    Some(m)
}

/// Stuart–Maxwell's test of marginal homogeneity of a square table of paired ordered grades.
pub fn stuart_maxwell(table: &[Vec<i64>]) -> Option<Map<String, Value>> {
    let k = table.len();
    let keep: Vec<usize> = (0..k)
        .filter(|&i| table[i].iter().sum::<i64>() + (0..k).map(|r| table[r][i]).sum::<i64>() > 0)
        .collect();
    if keep.len() < 2 {
        return None;
    }
    let t: Vec<Vec<i64>> = keep
        .iter()
        .map(|&i| keep.iter().map(|&j| table[i][j]).collect())
        .collect();
    let m = keep.len();
    let rows: Vec<i64> = (0..m).map(|i| t[i].iter().sum()).collect();
    let cols: Vec<i64> = (0..m).map(|i| (0..m).map(|r| t[r][i]).sum()).collect();
    let d: Vec<f64> = (0..m - 1).map(|i| (rows[i] - cols[i]) as f64).collect();
    let mut s = vec![vec![0.0f64; m - 1]; m - 1];
    for i in 0..m - 1 {
        for j in 0..m - 1 {
            s[i][j] = if i == j {
                (rows[i] + cols[i] - 2 * t[i][i]) as f64
            } else {
                -((t[i][j] + t[j][i]) as f64)
            };
        }
    }
    let x = solve(&s, &d)?;
    let chi2 = fsum(d.iter().zip(&x).map(|(a, b)| a * b));
    let mut out = Map::new();
    out.insert("chi2".into(), f(chi2));
    out.insert("df".into(), json!(m - 1));
    out.insert("p".into(), f(chi2_sf(chi2, (m - 1) as f64)));
    out.insert("n".into(), json!(rows.iter().sum::<i64>()));
    Some(out)
}

type Row = (Vec<f64>, usize, i64);

fn logit_fit(rows: &[(Vec<f64>, f64, i64)]) -> Option<Vec<f64>> {
    let p = rows[0].0.len();
    let mut beta = vec![0.0f64; p];
    for _ in 0..50 {
        let mut grad = vec![0.0f64; p];
        let mut info = vec![vec![0.0f64; p]; p];
        for (x, y, w) in rows {
            let w = *w as f64;
            let z = fsum(x.iter().zip(&beta).map(|(a, b)| a * b));
            let pi = sigmoid(z);
            for i in 0..p {
                grad[i] += w * (y - pi) * x[i];
                let wi = w * pi * (1.0 - pi) * x[i];
                for j in i..p {
                    info[i][j] += wi * x[j];
                }
            }
        }
        for i in 0..p {
            for j in 0..i {
                info[i][j] = info[j][i];
            }
        }
        let step = solve(&info, &grad)?;
        beta = beta.iter().zip(&step).map(|(b, s)| b + s).collect();
        if step.iter().map(|s| s.abs()).fold(f64::MIN, f64::max) < 1e-9 {
            break;
        }
    }
    if beta.iter().any(|b| b.abs() > 30.0) {
        return None;
    }
    Some(beta)
}

fn mul(a: &[Vec<f64>], b: &[Vec<f64>]) -> Vec<Vec<f64>> {
    (0..a.len())
        .map(|i| {
            (0..b[0].len())
                .map(|k| fsum((0..b.len()).map(|t| a[i][t] * b[t][k])))
                .collect()
        })
        .collect()
}

/// Brant's Wald test of the proportional-odds assumption (`brant`).
pub fn brant(
    rows: &[(Vec<f64>, usize)],
    categories: usize,
    names: &[String],
) -> Option<Map<String, Value>> {
    if categories < 3 {
        return None;
    }
    let grouped = super::models::grouped(rows.iter().cloned());
    let data: Vec<Row> = grouped
        .into_iter()
        .map(|(x, y, w)| {
            let mut v = vec![1.0];
            v.extend(x);
            (v, y, w)
        })
        .collect();
    let p = data[0].0.len();
    let mut fits = vec![];
    let mut used = vec![];
    for j in 0..categories - 1 {
        let bin: Vec<(Vec<f64>, f64, i64)> = data
            .iter()
            .map(|(x, y, w)| (x.clone(), if *y > j { 1.0 } else { 0.0 }, *w))
            .collect();
        if let Some(beta) = logit_fit(&bin) {
            fits.push(beta);
            used.push(j);
        }
    }
    let m = fits.len();
    if m < 2 {
        return None;
    }
    let pis: Vec<Vec<f64>> = fits
        .iter()
        .map(|beta| {
            data.iter()
                .map(|(x, _, _)| sigmoid(fsum(x.iter().zip(beta).map(|(a, b)| a * b))))
                .collect()
        })
        .collect();
    let xwx = |weights: &[f64]| -> Vec<Vec<f64>> {
        let mut out = vec![vec![0.0f64; p]; p];
        for ((x, _y, w), v) in data.iter().zip(weights) {
            for i in 0..p {
                for k in 0..p {
                    out[i][k] += *w as f64 * v * x[i] * x[k];
                }
            }
        }
        out
    };
    let mut inv = vec![];
    for pj in &pis {
        let a = inverse(&xwx(&pj.iter().map(|q| q * (1.0 - q)).collect::<Vec<_>>()))?;
        inv.push(a);
    }
    let q = p - 1;
    let mut big = vec![vec![0.0f64; m * q]; m * q];
    for j in 0..m {
        for k in j..m {
            let block = if j == k {
                inv[j].clone()
            } else {
                let w: Vec<f64> = (0..data.len())
                    .map(|r| pis[k][r] - pis[j][r] * pis[k][r])
                    .collect();
                mul(&mul(&inv[j], &xwx(&w)), &inv[k])
            };
            for a in 0..q {
                for b in 0..q {
                    big[j * q + a][k * q + b] = block[a + 1][b + 1];
                    big[k * q + b][j * q + a] = block[a + 1][b + 1];
                }
            }
        }
    }
    let stacked: Vec<f64> = (0..m)
        .flat_map(|j| (0..q).map(move |a| (j, a)))
        .map(|(j, a)| fits[j][a + 1])
        .collect();
    let wald = |cols: &[usize]| -> Option<Map<String, Value>> {
        let mut rows_d = vec![];
        for j in 1..m {
            for &a in cols {
                let mut d = vec![0.0f64; m * q];
                d[a] = 1.0;
                d[j * q + a] = -1.0;
                rows_d.push(d);
            }
        }
        let dv: Vec<f64> = rows_d
            .iter()
            .map(|d| fsum((0..m * q).map(|i| d[i] * stacked[i])))
            .collect();
        let dvd: Vec<Vec<f64>> = rows_d
            .iter()
            .map(|d1| {
                rows_d
                    .iter()
                    .map(|d2| {
                        fsum(
                            (0..m * q).map(|i| d1[i] * fsum((0..m * q).map(|k| big[i][k] * d2[k]))),
                        )
                    })
                    .collect()
            })
            .collect();
        let x = solve(&dvd, &dv)?;
        let chi2 = fsum(dv.iter().zip(&x).map(|(a, b)| a * b)).max(0.0);
        let mut out = Map::new();
        out.insert("chi2".into(), f(chi2));
        out.insert("df".into(), json!(rows_d.len()));
        out.insert("p".into(), f(chi2_sf(chi2, rows_d.len() as f64)));
        Some(out)
    };
    let mut overall = wald(&(0..q).collect::<Vec<_>>())?;
    let mut factors: Vec<Map<String, Value>> = vec![];
    for a in 0..q {
        if let Some(t) = wald(&[a]) {
            let mut fm = Map::new();
            fm.insert("name".into(), json!(names[a]));
            fm.insert(
                "slopes".into(),
                json!(fits.iter().map(|b| f(b[a + 1])).collect::<Vec<_>>()),
            );
            for (k, v) in t {
                fm.insert(k, v);
            }
            factors.push(fm);
        }
    }
    let ps: Vec<f64> = factors
        .iter()
        .map(|x| x["p"].as_f64().unwrap_or(1.0))
        .collect();
    for (fm, adj) in factors.iter_mut().zip(holm(&ps)) {
        fm.insert("p_holm".into(), f(adj));
    }
    let _ = &mut overall;
    let mut out = Map::new();
    out.insert("overall".into(), Value::Object(overall));
    out.insert(
        "factors".into(),
        Value::Array(factors.into_iter().map(Value::Object).collect()),
    );
    out.insert("thresholds".into(), json!(used));
    out.insert("of".into(), json!(categories - 1));
    Some(out)
}

fn brant_for(
    grader: &Grades,
    feats: &HashMap<i64, Feat>,
    road: &Road,
) -> Option<Map<String, Value>> {
    let rows_raw: Vec<(&Feat, usize)> = grader
        .items()
        .filter_map(|(h, g)| Some((feats.get(&h)?, road.grade_index(g)?)))
        .collect();
    if rows_raw.len() < 100 {
        return None;
    }
    let (mapping, labels) =
        merge_categories(&rows_raw.iter().map(|r| r.1).collect::<Vec<_>>(), road);
    if labels.len() < 3 {
        return None;
    }
    let rows: Vec<(&Feat, usize)> = rows_raw.iter().map(|(f, y)| (*f, mapping[y])).collect();
    let fl: Vec<&Feat> = rows.iter().map(|r| r.0).collect();
    let blocks = varying(&fl);
    let design = Design::new(&fl, &blocks, road);
    let mut out = brant(
        &rows
            .iter()
            .map(|(f, y)| (design.row(f), *y))
            .collect::<Vec<_>>(),
        labels.len(),
        &design.columns,
    )?;
    out.insert("grader".into(), json!(grader.name));
    out.insert("labels".into(), json!(labels));
    out.insert("n".into(), json!(rows.len()));
    Some(out)
}

const HADITH_VARS: [(&str, &str, &str); 9] = [
    (
        "weakest",
        "Weakest narrator",
        "the weakest identified narrator of the best chain, as Ibn Hajar's rank group",
    ),
    (
        "weak_count",
        "More weak narrators",
        "weak or unknown narrators beyond the first",
    ),
    ("length", "Length", "names in the chain"),
    (
        "unidentified",
        "Names not identified",
        "names not matched to a narrator",
    ),
    (
        "routes",
        "Chains in the book",
        "the hadith's chains in the same book",
    ),
    (
        "takhrij",
        "Narrations elsewhere",
        "its narrations found in other books",
    ),
    (
        "mawquf",
        "Stops short",
        "the chain ends with a Companion or Successor (1) or reaches the Prophet (0)",
    ),
    (
        "doubtful",
        "Doubtful link",
        "a pair implausible in time or generation (1)",
    ),
    (
        "position",
        "Place in the book",
        "the hadith's place in the book, from first to last",
    ),
];

const NARRATOR_VARS: [(&str, &str, &str); 7] = [
    (
        "rank",
        "Ibn Hajar's rank",
        "1 Companion … 12 liar — a higher number is a WEAKER narrator",
    ),
    (
        "tabaqa",
        "Generation",
        "Ibn Hajar's tabaqa, 1 Companions … 12",
    ),
    ("death", "Year of death", "AH"),
    ("links", "Times in the chains", ""),
    (
        "teachers",
        "Teachers",
        "different narrators he narrates from in these chains",
    ),
    (
        "students",
        "Students",
        "different narrators who narrate from him",
    ),
    (
        "pagerank",
        "PageRank",
        "his share of the flow of transmission",
    ),
];

fn hadith_vars(road: &Road) -> Vec<(&'static str, &'static str, &'static str)> {
    if !road.shia {
        return HADITH_VARS.to_vec();
    }
    HADITH_VARS
        .iter()
        .map(|&(k, l, e)| match k {
            "weakest" => (
                k,
                "Weakest narrator",
                "the weakest identified narrator of the best chain, in al-Najashi's categories",
            ),
            "weak_count" => (
                k,
                "More weak narrators",
                "narrators al-Najashi calls weak beyond the first",
            ),
            "mawquf" => (
                k,
                "Stops short",
                "the chain does not reach a Ma'sum (1) or reaches the Prophet or an Imam (0)",
            ),
            _ => (k, l, e),
        })
        .collect()
}

fn narrator_vars(road: &Road) -> Vec<(&'static str, &'static str, &'static str)> {
    if !road.shia {
        return NARRATOR_VARS.to_vec();
    }
    let mut v = vec![(
        "rank",
        "al-Najashi's category",
        "1 trustworthy, 2 praised, 3 muwaththaq, 4 weak — a higher number is a WEAKER narrator",
    )];
    v.extend(
        NARRATOR_VARS
            .iter()
            .filter(|x| x.0 != "rank" && x.0 != "tabaqa")
            .copied(),
    );
    v
}

struct HadithTable {
    factors: Vec<(String, String, Column)>,
    outcomes: Vec<(String, String, Column)>,
    hadith: usize,
}

fn usable(data: &[Grades]) -> Vec<&Grades> {
    let mut g: Vec<&Grades> = data
        .iter()
        .filter(|d| d.len() >= 100 && d.map.values().collect::<HashSet<_>>().len() > 1)
        .collect();
    g.sort_by(|a, b| a.name.cmp(&b.name));
    g
}

fn hadith_table(
    conn: &Connection,
    key: &str,
    road: &Road,
    feats: &HashMap<i64, Feat>,
    data: &[Grades],
) -> rusqlite::Result<Option<HadithTable>> {
    if feats.len() < 30 {
        return Ok(None);
    }
    let worst = road.grade_labels.last().cloned().unwrap_or_default();
    let mut order: HashMap<i64, usize> = HashMap::new();
    {
        let mut stmt = conn.prepare(
            "SELECT h.id FROM hadiths h JOIN collections c ON c.id = h.collection_id WHERE c.key = ? \
             ORDER BY h.number_sort, h.id",
        )?;
        let rows = stmt.query_map([key], |r| r.get::<_, i64>(0))?;
        for (i, h) in rows.enumerate() {
            order.insert(h?, i);
        }
    }
    let graders = usable(data);
    let mut hadith: Vec<i64> = feats.keys().copied().collect();
    hadith.sort_unstable();
    let total = order.len().saturating_sub(1).max(1) as f64;
    let mut factors = vec![];
    for (k, label, explain) in hadith_vars(road) {
        let col: Column = match k {
            "weakest" => hadith
                .iter()
                .map(|h| {
                    let w = &feats[h].weakest;
                    if w == "None ranked" {
                        None
                    } else {
                        road.levels.iter().position(|l| l == w).map(|i| i as f64)
                    }
                })
                .collect(),
            "position" => hadith
                .iter()
                .map(|h| Some(order.get(h).copied().unwrap_or(0) as f64 / total))
                .collect(),
            _ => hadith
                .iter()
                .map(|h| Some(feats[h].get(k) as f64))
                .collect(),
        };
        if distinct_known(&col) > 1 {
            factors.push((label.to_string(), explain.to_string(), col));
        }
    }
    let last = road.grade_order.len() - 1;
    let mut outcomes: Vec<(String, String, Column)> = graders
        .iter()
        .map(|g| {
            (
                format!("Grade: {}", g.name),
                format!(
                    "{}'s grade, 0 sahih … {last} {worst} (higher = weaker)",
                    g.name
                ),
                hadith
                    .iter()
                    .map(|h| {
                        g.get(*h)
                            .and_then(|v| road.grade_index(v))
                            .map(|i| i as f64)
                    })
                    .collect(),
            )
        })
        .collect();
    if graders.len() >= 2 {
        let spread: Column = hadith
            .iter()
            .map(|h| {
                let gs: Vec<usize> = graders
                    .iter()
                    .filter_map(|g| g.get(*h).and_then(|v| road.grade_index(v)))
                    .collect();
                if gs.len() >= 2 {
                    Some((gs.iter().max().unwrap() - gs.iter().min().unwrap()) as f64)
                } else {
                    None
                }
            })
            .collect();
        outcomes.push((
            "Disagreement".into(),
            "how many grade steps apart the scholars are on this hadith".into(),
            spread,
        ));
    }
    outcomes.retain(|o| distinct_known(&o.2) > 1);
    Ok(Some(HadithTable {
        factors,
        outcomes,
        hadith: hadith.len(),
    }))
}

fn adjusted(factors: &[(String, String, Column)], outcome: &Column) -> Vec<Value> {
    let mut names: Vec<String> = factors.iter().map(|x| x.0.clone()).collect();
    names.push("outcome".into());
    let mut cols: Vec<Column> = factors.iter().map(|x| x.2.clone()).collect();
    cols.push(outcome.clone());
    let m = correlation_matrix(&names, &cols);
    let Some(partial) = m["partial"].as_array() else {
        return vec![Value::Null; factors.len()];
    };
    let mut cells: Vec<Value> = (0..factors.len())
        .map(|i| {
            partial[i]
                .as_array()
                .and_then(|r| r.last())
                .cloned()
                .unwrap_or(Value::Null)
        })
        .collect();
    let known: Vec<usize> = (0..cells.len()).filter(|i| !cells[*i].is_null()).collect();
    let qs = bh(&known
        .iter()
        .map(|i| cells[*i]["p"].as_f64().unwrap_or(1.0))
        .collect::<Vec<_>>());
    for (i, q) in known.iter().zip(qs) {
        cells[*i]["q"] = f(q);
    }
    cells
}

fn narrator_table(
    conn: &Connection,
    collection: Option<&str>,
    road: &Road,
) -> rusqlite::Result<(Vec<String>, Vec<Column>, Vec<String>)> {
    let persons = persons(conn, road.tradition)?;
    let ids: Vec<i64> = books_of(conn, collection)?.iter().map(|b| b.0).collect();
    let chains = chains(conn, &ids)?;
    let mut links: Counter<i64> = Counter::default();
    let mut teachers: HashMap<i64, HashSet<i64>> = HashMap::new();
    let mut students: HashMap<i64, HashSet<i64>> = HashMap::new();
    let mut edges: Counter<(i64, i64)> = Counter::default();
    for c in &chains {
        for p in c.links.iter().flatten() {
            if persons.contains_key(p) {
                links.add(*p, 1);
            }
        }
        for w in c.links.windows(2) {
            if let (Some(s), Some(t)) = (w[0], w[1]) {
                if persons.contains_key(&s) && persons.contains_key(&t) {
                    teachers.entry(s).or_default().insert(t);
                    students.entry(t).or_default().insert(s);
                    edges.add((s, t), 1);
                }
            }
        }
    }
    let pr: HashMap<i64, f64> = pagerank(&edges, 0.85, 60).into_iter().collect();
    let mut people: Vec<i64> = links.keys.clone();
    people.sort_unstable();
    let (mut names, mut cols, mut explain) = (vec![], vec![], vec![]);
    for (k, lab, e) in narrator_vars(road) {
        let col: Column = people
            .iter()
            .map(|p| {
                let info = &persons[p];
                match k {
                    "rank" => info.rank.map(|v| v as f64),
                    "tabaqa" => info.tabaqa.map(|v| v as f64),
                    "death" => info.death.map(|v| v as f64),
                    "links" => Some(links.get(p) as f64),
                    "teachers" => Some(teachers.get(p).map(|s| s.len()).unwrap_or(0) as f64),
                    "students" => Some(students.get(p).map(|s| s.len()).unwrap_or(0) as f64),
                    _ => Some(pr.get(p).copied().unwrap_or(0.0)),
                }
            })
            .collect();
        if distinct_known(&col) > 1 && col.iter().filter(|v| v.is_some()).count() >= 30 {
            names.push(lab.to_string());
            cols.push(col);
            explain.push(e.to_string());
        }
    }
    Ok((names, cols, explain))
}

pub fn analyse(
    conn: &Connection,
    collection: Option<&str>,
    say: &dyn Fn(&str),
) -> rusqlite::Result<Value> {
    let ui = crate::i18n::ui_language();
    let tr = |s: &str, v: &[(&str, String)]| crate::i18n::t(&ui, s, v);
    let books = books_of(conn, collection)?;
    let road = road_of(conn, collection);
    let order_g = road.grade_order.clone();
    let mut hadith_out = vec![];
    let mut graders_out: Vec<Map<String, Value>> = vec![];
    let mut brant_out: Vec<Map<String, Value>> = vec![];
    for (_id, key, name) in &books {
        say(&tr(
            "Correlating the hadith of {name}",
            &[("name", name.clone())],
        ));
        let feats = hadith_features(conn, key, &road)?;
        let data = road.load(conn, key)?;
        if let Some(table) = hadith_table(conn, key, &road, &feats, &data)? {
            if table.factors.len() + table.outcomes.len() >= 3 {
                let all: Vec<&(String, String, Column)> =
                    table.factors.iter().chain(&table.outcomes).collect();
                let mut m = correlation_matrix(
                    &all.iter().map(|x| x.0.clone()).collect::<Vec<_>>(),
                    &all.iter().map(|x| x.2.clone()).collect::<Vec<_>>(),
                );
                m.insert("book".into(), json!(name));
                m.insert("key".into(), json!(key));
                m.insert(
                    "explain".into(),
                    json!(all.iter().map(|x| &x.1).collect::<Vec<_>>()),
                );
                m.insert("hadith".into(), json!(table.hadith));
                m.insert("factors".into(), json!(table.factors.len()));
                let adj: Vec<Value> = table
                    .outcomes
                    .iter()
                    .map(|o| json!({"outcome": o.0, "cells": adjusted(&table.factors, &o.2)}))
                    .collect();
                m.insert("adjusted".into(), Value::Array(adj));
                m.shift_remove("partial");
                let fm = correlation_matrix(
                    &table
                        .factors
                        .iter()
                        .map(|x| x.0.clone())
                        .collect::<Vec<_>>(),
                    &table
                        .factors
                        .iter()
                        .map(|x| x.2.clone())
                        .collect::<Vec<_>>(),
                );
                m.insert("partial".into(), fm["partial"].clone());
                hadith_out.push(Value::Object(m));
            }
        }
        let graders = usable(&data);
        for g in &graders {
            say(&tr(
                "Testing the shape of the model of {g}",
                &[("g", g.name.clone())],
            ));
            if let Some(mut b) = brant_for(g, &feats, &road) {
                b.insert("book".into(), json!(name));
                b.insert("key".into(), json!(key));
                brant_out.push(b);
            }
        }
        for (i, a) in graders.iter().enumerate() {
            for b in &graders[i + 1..] {
                let common: Vec<i64> = a
                    .order
                    .iter()
                    .copied()
                    .filter(|h| b.map.contains_key(h))
                    .collect();
                if common.len() < 30 {
                    continue;
                }
                let mut table = vec![vec![0i64; order_g.len()]; order_g.len()];
                let (mut higher, mut lower) = (0i64, 0i64);
                for h in &common {
                    let x = road.grade_index(a.map[h]).unwrap_or(0);
                    let y = road.grade_index(b.map[h]).unwrap_or(0);
                    table[x][y] += 1;
                    higher += (x < y) as i64;
                    lower += (x > y) as i64;
                }
                if let Some(test) = stuart_maxwell(&table) {
                    let mut t = Map::new();
                    t.insert("book".into(), json!(name));
                    t.insert("a".into(), json!(a.name));
                    t.insert("b".into(), json!(b.name));
                    t.insert("n".into(), json!(common.len()));
                    t.insert("a_stronger".into(), json!(higher));
                    t.insert("a_weaker".into(), json!(lower));
                    t.insert("table".into(), json!(table));
                    for (k, v) in test {
                        t.insert(k, v);
                    }
                    graders_out.push(t);
                }
            }
        }
    }
    let ps: Vec<f64> = graders_out
        .iter()
        .map(|t| t["p"].as_f64().unwrap_or(1.0))
        .collect();
    for (t, adj) in graders_out.iter_mut().zip(holm(&ps)) {
        t.insert("p_holm".into(), f(adj));
    }
    let ps: Vec<f64> = brant_out
        .iter()
        .map(|b| b["overall"]["p"].as_f64().unwrap_or(1.0))
        .collect();
    for (b, adj) in brant_out.iter_mut().zip(holm(&ps)) {
        if let Some(o) = b.get_mut("overall").and_then(Value::as_object_mut) {
            o.insert("p_holm".into(), f(adj));
        }
    }
    let mut out = Map::new();
    out.insert("method".into(), json!(METHOD));
    out.insert("collection".into(), json!(collection));
    out.insert("tradition".into(), json!(road.tradition));
    out.insert("critic".into(), json!(road.critic));
    out.insert("grade_labels".into(), json!(road.grade_labels));
    out.insert("hadith".into(), Value::Array(hadith_out));
    out.insert(
        "graders".into(),
        Value::Array(graders_out.into_iter().map(Value::Object).collect()),
    );
    out.insert(
        "brant".into(),
        Value::Array(brant_out.into_iter().map(Value::Object).collect()),
    );
    out.insert("books".into(), Value::Null);
    say(&tr("Correlating the narrators", &[]));
    let (names, cols, explain) = narrator_table(conn, collection, &road)?;
    if cols.len() >= 2 && cols[0].len() >= 30 {
        let mut m = correlation_matrix(&names, &cols);
        m.insert("explain".into(), json!(explain));
        m.insert("people".into(), json!(cols[0].len()));
        out.insert("narrators".into(), Value::Object(m));
    }
    if books.len() >= 2 {
        say(&tr("Comparing the books", &[]));
        let persons = persons(conn, road.tradition)?;
        let mut measures: Vec<(&str, Vec<(String, Vec<f64>)>)> = vec![
            ("Weakest narrator", vec![]),
            ("Length", vec![]),
            ("Names identified", vec![]),
        ];
        for (bid, _key, name) in &books {
            let (mut weakest, mut length, mut ident) = (vec![], vec![], vec![]);
            for c in chains(conn, &[*bid])? {
                let ls = &c.links;
                if ls.is_empty() {
                    continue;
                }
                length.push(ls.len() as f64);
                let ids: Vec<i64> = ls
                    .iter()
                    .flatten()
                    .copied()
                    .filter(|p| persons.contains_key(p))
                    .collect();
                ident.push(ids.len() as f64 / ls.len() as f64);
                let ranks: Vec<i64> = ids
                    .iter()
                    .filter_map(|p| {
                        persons[p]
                            .rank
                            .filter(|_| road.group_of(persons[p].rank).is_some())
                    })
                    .collect();
                if let Some(&r) = ranks.iter().max() {
                    weakest.push(road.group_of(Some(r)).expect("group") as f64);
                }
            }
            measures[0].1.push((name.clone(), weakest));
            measures[1].1.push((name.clone(), length));
            measures[2].1.push((name.clone(), ident));
        }
        let mut comparisons = vec![];
        for (label, groups) in &measures {
            let kw = kruskal(&groups.iter().map(|g| &g.1).collect::<Vec<_>>());
            let mut pairs: Vec<Map<String, Value>> = vec![];
            for i in 0..groups.len() {
                for j in i + 1..groups.len() {
                    if let Some(mw) = mann_whitney(&groups[i].1, &groups[j].1) {
                        let mut p = Map::new();
                        p.insert("a".into(), json!(groups[i].0));
                        p.insert("b".into(), json!(groups[j].0));
                        for (k, v) in mw {
                            p.insert(k, v);
                        }
                        pairs.push(p);
                    }
                }
            }
            let ps: Vec<f64> = pairs
                .iter()
                .map(|p| p["p"].as_f64().unwrap_or(1.0))
                .collect();
            for (p, adj) in pairs.iter_mut().zip(holm(&ps)) {
                p.insert("p_holm".into(), f(adj));
            }
            let medians: Vec<Value> = groups
                .iter()
                .map(|(n, g)| {
                    let mut s = g.clone();
                    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
                    let med = if s.is_empty() {
                        Value::Null
                    } else {
                        num_json(s[s.len() / 2], label != &"Names identified")
                    };
                    json!([n, med, g.len()])
                })
                .collect();
            comparisons.push(json!({"measure": label, "kruskal": kw,
                                    "pairs": pairs.into_iter().map(Value::Object).collect::<Vec<_>>(),
                                    "medians": medians}));
        }
        out.insert("books".into(), Value::Array(comparisons));
    }
    let _ = fo;
    Ok(Value::Object(out))
}

/// A value Python keeps as an int (a count, a group) written as one.
fn num_json(x: f64, int: bool) -> Value {
    if int && x.fract() == 0.0 {
        json!(x as i64)
    } else {
        f(x)
    }
}

pub fn cached(
    conn: &Connection,
    collection: Option<&str>,
    say: &dyn Fn(&str),
    recompute: bool,
    saved_only: bool,
) -> Result<Value, CachedError> {
    let path = super::derived().join(super::cache_name("lab", collection));
    let stamp = super::fingerprint(conn, collection)? + METHOD;
    if !recompute {
        if let Some(v) = super::saved(&path, &stamp) {
            return Ok(v);
        }
    }
    if saved_only {
        return Err(CachedError::NotSaved(NotSaved));
    }
    let what = format!("lab computed for {}", collection.unwrap_or("all books"));
    let result = crate::log::timed("statistics", &what, || analyse(conn, collection, say))?;
    Ok(super::save(&path, result, &stamp))
}

/// Every pair of a matrix, strongest first: a, b, rho, n, p, q (`pairs_table`).
pub fn pairs_table(
    m: &Value,
    which: &str,
) -> Vec<(String, String, f64, i64, Option<f64>, Option<f64>)> {
    let mut rows = vec![];
    let Some(grid) = m.get(which).and_then(Value::as_array) else {
        return rows;
    };
    let names = m["names"].as_array().cloned().unwrap_or_default();
    for i in 0..grid.len() {
        for j in i + 1..grid.len() {
            let c = &grid[i][j];
            if let Some(rho) = c.get("rho").and_then(Value::as_f64) {
                rows.push((
                    names[i].as_str().unwrap_or("").to_string(),
                    names[j].as_str().unwrap_or("").to_string(),
                    rho,
                    c["n"].as_i64().unwrap_or(0),
                    c["p"].as_f64(),
                    c.get("q").and_then(Value::as_f64),
                ));
            }
        }
    }
    rows.sort_by(|a, b| {
        b.2.abs()
            .partial_cmp(&a.2.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    rows
}

/// outcome, factor, plain rho, adjusted rho, n, q of the adjusted (`adjusted_table`).
pub fn adjusted_table(
    m: &Value,
) -> Vec<(
    String,
    String,
    Option<f64>,
    Option<f64>,
    Option<i64>,
    Option<f64>,
)> {
    let mut rows = vec![];
    let factors = m["factors"].as_u64().unwrap_or(0) as usize;
    let names = m["names"].as_array().cloned().unwrap_or_default();
    for (j, a) in m["adjusted"].as_array().into_iter().flatten().enumerate() {
        let col = factors + j;
        for (i, c) in a["cells"].as_array().into_iter().flatten().enumerate() {
            let plain = &m["cells"][i][col];
            rows.push((
                a["outcome"].as_str().unwrap_or("").to_string(),
                names[i].as_str().unwrap_or("").to_string(),
                plain.get("rho").and_then(Value::as_f64),
                c.get("rho").and_then(Value::as_f64),
                c.get("n").and_then(Value::as_i64),
                c.get("q").and_then(Value::as_f64),
            ));
        }
    }
    rows
}
