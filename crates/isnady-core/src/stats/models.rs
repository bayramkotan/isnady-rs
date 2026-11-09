//! The hadith model (`stats_models.py`): what a scholar's grade follows — the proportional-odds (cumulative logit)
//! model of the grade on what the chain shows, fitted by Fisher scoring with standard errors from the observed
//! information; the odds ratios, the likelihood-ratio test of every factor, the fit, and the surprises (hadith graded
//! far higher or lower than their chain foretells).

use rusqlite::Connection;
use serde_json::{json, Map, Value};
use std::collections::HashMap;

use super::corpus::{kendall_from_table, CachedError};
use super::num::{chi2_sf, fsum, holm, inverse, normal_sf, round_to, sigmoid, solve};
use super::{books_of, chains, f, fo, persons, road_of, Counter, NotSaved, Road};

pub const METHOD: &str = "isnady-models-2";
pub const MIN_CATEGORY: i64 = 15;
pub const SURPRISE: f64 = 0.05;

pub const BLOCKS: [(&str, &str, &str); 8] = [
    ("weakest", "Weakest narrator", "the weakest identified narrator of the hadith's best chain, against a chain whose weakest is trustworthy (thiqa)"),
    ("weak_count", "More weak narrators", "each further narrator of rank 7 or weaker (mastur, da'if, majhul …) in that chain beyond the first"),
    ("length", "Length", "each further name in the chain"),
    ("unidentified", "Names not identified", "each name of the chain not matched to a narrator of the Taqrib"),
    ("routes", "Chains in the book", "each further chain the hadith has in the same book (mutaba'at within the book)"),
    ("takhrij", "Narrations elsewhere", "each further narration of the hadith found in another book (takhrij)"),
    ("mawquf", "Stops short of the Prophet", "the chain ends with a Companion or a Successor (mawquf, maqtu')"),
    ("doubtful", "A doubtful link", "the chain has a pair too far apart in time, or out of the order of generations"),
];

/// BLOCKS in the words of the road: road B's critic is al-Najashi, its chains end at a Ma'sum.
pub fn blocks_for(road: &Road) -> Vec<(&'static str, &'static str, &'static str)> {
    if !road.shia {
        return BLOCKS.to_vec();
    }
    BLOCKS
        .iter()
        .map(|&(k, lab, e)| match k {
            "weakest" => (k, "Weakest narrator", "the weakest identified narrator of the hadith's best chain in al-Najashi's categories, against a chain whose weakest is trustworthy (thiqa)"),
            "weak_count" => (k, "More weak narrators", "each further narrator al-Najashi calls weak in that chain beyond the first"),
            "unidentified" => (k, "Names not identified", "each name of the chain not matched to a narrator of al-Najashi"),
            "mawquf" => (k, "Stops short of a Ma'sum", "the chain does not reach the Prophet or an Imam (it ends with a narrator)"),
            _ => (k, lab, e),
        })
        .collect()
}

fn label_of(road: &Road, block: &str) -> &'static str {
    blocks_for(road)
        .into_iter()
        .find(|b| b.0 == block)
        .map(|b| b.1)
        .unwrap_or("")
}

pub struct Fit {
    pub params: Vec<f64>,
    pub cov: Vec<Vec<f64>>,
    pub loglik: f64,
    pub converged: bool,
    pub k: usize,
    pub p: usize,
}

type Group = (Vec<f64>, usize, i64);

fn loglik_and_scores(groups: &[Group], theta: &[f64], k: usize, p: usize) -> (f64, Vec<Vec<f64>>) {
    let (a, beta) = theta.split_at(k);
    let mut ll = 0.0f64;
    let mut scores = Vec::with_capacity(groups.len());
    for (x, y, w) in groups {
        let y = *y;
        let eta = fsum(beta.iter().zip(x).map(|(b, v)| b * v));
        let upper = if y < k { sigmoid(a[y] - eta) } else { 1.0 };
        let lower = if y > 0 { sigmoid(a[y - 1] - eta) } else { 0.0 };
        let prob = (upper - lower).max(1e-300);
        ll += *w as f64 * prob.ln();
        let fu = if y < k { upper * (1.0 - upper) } else { 0.0 };
        let fl = if y > 0 { lower * (1.0 - lower) } else { 0.0 };
        let mut g = vec![0.0f64; k + p];
        if y < k {
            g[y] += fu / prob;
        }
        if y > 0 {
            g[y - 1] -= fl / prob;
        }
        let d_eta = (fl - fu) / prob;
        for (j, v) in x.iter().enumerate() {
            g[k + j] = d_eta * v;
        }
        scores.push(g);
    }
    (ll, scores)
}

/// The proportional-odds model by Fisher scoring; groups = [(x, y, weight)], y = 0 the strongest grade.
pub fn fit_ordinal(groups: &[Group], categories: usize) -> Option<Fit> {
    let iterations = 100;
    let p = groups.first().map(|g| g.0.len()).unwrap_or(0);
    let k = categories - 1;
    let total: i64 = groups.iter().map(|g| g.2).sum();
    let mut counts = vec![0i64; categories];
    for (_x, y, w) in groups {
        counts[*y] += w;
    }
    let mut cum = 0i64;
    let mut params = vec![];
    for c in counts.iter().take(k) {
        cum += c;
        let q = (cum as f64 / total as f64).clamp(1e-4, 1.0 - 1e-4);
        params.push((q / (1.0 - q)).ln());
    }
    params.extend(std::iter::repeat(0.0).take(p));
    let n = k + p;
    let (mut ll, mut scores) = loglik_and_scores(groups, &params, k, p);
    let mut converged = false;
    for _ in 0..iterations {
        let mut info = vec![vec![0.0f64; n]; n];
        let mut grad = vec![0.0f64; n];
        for (g, (_x, _y, w)) in scores.iter().zip(groups) {
            let w = *w as f64;
            for i in 0..n {
                if g[i] != 0.0 {
                    grad[i] += w * g[i];
                    let gi = w * g[i];
                    let row = &mut info[i];
                    for j in 0..n {
                        row[j] += gi * g[j];
                    }
                }
            }
        }
        let step = solve(&info, &grad)?;
        let mut scale = 1.0f64;
        let mut found = None;
        while scale > 1e-4 {
            let trial: Vec<f64> = params
                .iter()
                .zip(&step)
                .map(|(v, s)| v + scale * s)
                .collect();
            if (0..k.saturating_sub(1)).all(|j| trial[j] < trial[j + 1]) {
                let (new_ll, new_scores) = loglik_and_scores(groups, &trial, k, p);
                if new_ll >= ll - 1e-9 {
                    found = Some((trial, new_ll, new_scores));
                    break;
                }
            }
            scale /= 2.0;
        }
        let Some((trial, new_ll, new_scores)) = found else {
            break;
        };
        let improvement = new_ll - ll;
        params = trial;
        ll = new_ll;
        scores = new_scores;
        if improvement.abs() < 1e-7 {
            converged = true;
            break;
        }
    }
    let total_score = |theta: &[f64]| -> Vec<f64> {
        let (_ll, sc) = loglik_and_scores(groups, theta, k, p);
        let mut out = vec![0.0f64; n];
        for (g, (_x, _y, w)) in sc.iter().zip(groups) {
            for i in 0..n {
                out[i] += *w as f64 * g[i];
            }
        }
        out
    };
    let mut info = vec![vec![0.0f64; n]; n];
    for i in 0..n {
        let h = 1e-5 * params[i].abs().max(1.0);
        let (mut up, mut down) = (params.clone(), params.clone());
        up[i] += h;
        down[i] -= h;
        let (su, sd) = (total_score(&up), total_score(&down));
        for j in 0..n {
            info[i][j] = -(su[j] - sd[j]) / (2.0 * h);
        }
    }
    let sym: Vec<Vec<f64>> = (0..n)
        .map(|i| (0..n).map(|j| (info[i][j] + info[j][i]) / 2.0).collect())
        .collect();
    let cov = inverse(&sym)?;
    Some(Fit {
        params,
        cov,
        loglik: ll,
        converged,
        k,
        p,
    })
}

pub fn predict(params: &[f64], k: usize, x: &[f64]) -> Vec<f64> {
    let (a, beta) = params.split_at(k);
    let eta = fsum(beta.iter().zip(x).map(|(b, v)| b * v));
    let mut cum: Vec<f64> = (0..k).map(|j| sigmoid(a[j] - eta)).collect();
    cum.push(1.0);
    let mut out = vec![cum[0]];
    for j in 1..=k {
        out.push(cum[j] - cum[j - 1]);
    }
    out
}

/// The factors of a hadith's best chain (`hadith_features`).
#[derive(Clone, Debug)]
pub struct Feat {
    pub weakest: String,
    pub weak_count: i64,
    pub length: i64,
    pub unidentified: i64,
    pub mawquf: i64,
    pub doubtful: i64,
    pub routes: i64,
    pub takhrij: i64,
}

impl Feat {
    pub fn get(&self, key: &str) -> i64 {
        match key {
            "weak_count" => self.weak_count,
            "length" => self.length,
            "unidentified" => self.unidentified,
            "mawquf" => self.mawquf,
            "doubtful" => self.doubtful,
            "routes" => self.routes,
            "takhrij" => self.takhrij,
            _ => 0,
        }
    }
    pub fn json(&self) -> Value {
        json!({"weakest": self.weakest, "weak_count": self.weak_count, "length": self.length,
               "unidentified": self.unidentified, "mawquf": self.mawquf, "doubtful": self.doubtful,
               "routes": self.routes, "takhrij": self.takhrij})
    }
}

/// Every hadith of a book with a chain split into names, with its factors, on the book's road.
pub fn hadith_features(
    conn: &Connection,
    key: &str,
    road: &Road,
) -> rusqlite::Result<HashMap<i64, Feat>> {
    let persons = persons(conn, road.tradition)?;
    let Some(book): Option<i64> = conn
        .query_row("SELECT id FROM collections WHERE key = ?", [key], |r| {
            r.get(0)
        })
        .ok()
    else {
        return Ok(HashMap::new());
    };
    let chains = chains(conn, &[book])?;
    let mut by_hadith: Vec<(i64, Vec<&super::Chain>)> = vec![];
    let mut at: HashMap<i64, usize> = HashMap::new();
    for c in &chains {
        if !c.links.is_empty() {
            let i = *at.entry(c.hadith).or_insert_with(|| {
                by_hadith.push((c.hadith, vec![]));
                by_hadith.len() - 1
            });
            by_hadith[i].1.push(c);
        }
    }
    let mut takhrij: HashMap<i64, i64> = HashMap::new();
    if let Ok(mut stmt) =
        conn.prepare("SELECT hadith_a, COUNT(*) FROM hadith_relations GROUP BY hadith_a")
    {
        if let Ok(rows) = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?))) {
            for (a, n) in rows.flatten() {
                takhrij.insert(a, n);
            }
        }
    }
    let mut out = HashMap::new();
    for (h, cs) in &by_hadith {
        let mut best: Option<((usize, i64), Feat)> = None;
        for c in cs {
            let links = &c.links;
            let ranks: Vec<i64> = links
                .iter()
                .flatten()
                .filter_map(|p| persons.get(p))
                .filter_map(|info| info.rank.filter(|_| road.group_of(info.rank).is_some()))
                .collect();
            let ids = links
                .iter()
                .flatten()
                .filter(|p| persons.contains_key(p))
                .count() as i64;
            let weakest = match ranks.iter().max() {
                Some(&r) => road.levels[road.group_of(Some(r)).expect("group")].clone(),
                None => "None ranked".to_string(),
            };
            let mut doubtful = false;
            for w in links.windows(2) {
                if let (Some(Some(s)), Some(Some(t))) =
                    (w[0].map(|p| persons.get(&p)), w[1].map(|p| persons.get(&p)))
                {
                    if let (Some(ds), Some(dt)) =
                        (s.death.filter(|d| *d != 0), t.death.filter(|d| *d != 0))
                    {
                        if ds - dt > 110 || ds - dt < -60 {
                            doubtful = true;
                        }
                    }
                    if let (Some(ts), Some(tt)) =
                        (s.tabaqa.filter(|d| *d != 0), t.tabaqa.filter(|d| *d != 0))
                    {
                        if ts < tt {
                            doubtful = true;
                        }
                    }
                }
            }
            let weak = ranks.iter().filter(|r| **r >= road.weak_from).count() as i64;
            let unidentified = links.len() as i64 - ids;
            let order = (
                if weakest != "None ranked" {
                    road.levels.iter().position(|l| *l == weakest).unwrap_or(9)
                } else {
                    9
                },
                unidentified,
            );
            let feat = Feat {
                weakest,
                weak_count: (weak - 1).max(0),
                length: links.len() as i64,
                unidentified,
                mawquf: if c.reaches { 0 } else { 1 },
                doubtful: doubtful as i64,
                routes: 0,
                takhrij: 0,
            };
            if best.as_ref().map(|(o, _)| order < *o).unwrap_or(true) {
                best = Some((order, feat));
            }
        }
        let (_, mut feat) = best.expect("a chain");
        feat.routes = cs.len() as i64;
        feat.takhrij = takhrij.get(h).copied().unwrap_or(0);
        out.insert(*h, feat);
    }
    Ok(out)
}

pub struct Design {
    pub columns: Vec<String>,
    pub column_block: Vec<String>,
    levels: Vec<String>,
    numeric: Vec<String>,
}

const CAPS: [(&str, i64); 7] = [
    ("weak_count", 3),
    ("length", 9),
    ("unidentified", 6),
    ("routes", 5),
    ("takhrij", 5),
    ("mawquf", 1),
    ("doubtful", 1),
];

impl Design {
    pub fn new(feats: &[&Feat], blocks: &[String], road: &Road) -> Design {
        let mut columns = vec![];
        let mut column_block = vec![];
        let mut levels = vec![];
        if blocks.iter().any(|b| b == "weakest") {
            let mut seen: Counter<&str> = Counter::default();
            for f in feats {
                seen.add(f.weakest.as_str(), 1);
            }
            levels = road
                .levels
                .iter()
                .filter(|lv| lv.as_str() != road.reference && seen.get(&lv.as_str()) >= 10)
                .cloned()
                .collect();
            for lv in &levels {
                columns.push(format!("Weakest: {lv}"));
                column_block.push("weakest".to_string());
            }
        }
        let numeric: Vec<String> = blocks.iter().filter(|b| *b != "weakest").cloned().collect();
        for b in &numeric {
            columns.push(label_of(road, b).to_string());
            column_block.push(b.clone());
        }
        Design {
            columns,
            column_block,
            levels,
            numeric,
        }
    }

    pub fn row(&self, f: &Feat) -> Vec<f64> {
        let mut values: Vec<f64> = self
            .levels
            .iter()
            .map(|lv| if f.weakest == *lv { 1.0 } else { 0.0 })
            .collect();
        for b in &self.numeric {
            let cap = CAPS
                .iter()
                .find(|c| c.0 == b)
                .map(|c| c.1)
                .unwrap_or(i64::MAX);
            values.push(f.get(b).min(cap) as f64);
        }
        values
    }
}

/// The blocks the data lets the model estimate.
pub fn varying(feats: &[&Feat]) -> Vec<String> {
    BLOCKS
        .iter()
        .filter(|(key, _, _)| {
            if *key == "weakest" {
                let mut v: Vec<&str> = feats.iter().map(|f| f.weakest.as_str()).collect();
                v.sort_unstable();
                v.dedup();
                v.len() > 1
            } else {
                let mut v: Vec<i64> = feats.iter().map(|f| f.get(key)).collect();
                v.sort_unstable();
                v.dedup();
                v.len() > 1
            }
        })
        .map(|b| b.0.to_string())
        .collect()
}

/// Grades given fewer than MIN_CATEGORY times join their weaker neighbour (or stronger, at the end).
pub fn merge_categories(ys: &[usize], road: &Road) -> (HashMap<usize, usize>, Vec<String>) {
    let mut counts = vec![0i64; road.grade_order.len()];
    for y in ys {
        counts[*y] += 1;
    }
    let mut groups: Vec<Vec<usize>> = (0..road.grade_order.len())
        .filter(|c| counts[*c] != 0)
        .map(|c| vec![c])
        .collect();
    let mut changed = true;
    while changed && groups.len() > 2 {
        changed = false;
        for i in 0..groups.len() {
            if groups[i].iter().map(|c| counts[*c]).sum::<i64>() < MIN_CATEGORY {
                let j = if i + 1 < groups.len() { i + 1 } else { i - 1 };
                let mut merged = [groups[i].clone(), groups[j].clone()].concat();
                merged.sort_unstable();
                groups[i.min(j)] = merged;
                groups.remove(i.max(j));
                changed = true;
                break;
            }
        }
    }
    let mapping = groups
        .iter()
        .enumerate()
        .flat_map(|(gi, g)| g.iter().map(move |c| (*c, gi)))
        .collect();
    let labels = groups
        .iter()
        .map(|g| {
            g.iter()
                .map(|c| road.grade_name(road.grade_order[*c]))
                .collect::<Vec<_>>()
                .join(" or ")
        })
        .collect();
    (mapping, labels)
}

fn fit_blocks(
    rows: &[(&Feat, usize)],
    blocks: &[String],
    categories: usize,
    road: &Road,
) -> (Option<Fit>, Design) {
    let feats: Vec<&Feat> = rows.iter().map(|r| r.0).collect();
    let design = Design::new(&feats, blocks, road);
    let groups = grouped(rows.iter().map(|(f, y)| (design.row(f), *y)));
    (fit_ordinal(&groups, categories), design)
}

/// `Counter((x, y) for …)` as (x, y, weight), in the order the rows came.
pub fn grouped(rows: impl Iterator<Item = (Vec<f64>, usize)>) -> Vec<Group> {
    let mut out: Vec<Group> = vec![];
    let mut at: HashMap<(Vec<u64>, usize), usize> = HashMap::new();
    for (x, y) in rows {
        let key = (x.iter().map(|v| v.to_bits()).collect(), y);
        if let Some(&i) = at.get(&key) {
            out[i].2 += 1;
        } else {
            at.insert(key, out.len());
            out.push((x, y, 1));
        }
    }
    out
}

/// Somers' D of the grade on the model's expected grade.
pub fn somers_d(table: &[Vec<i64>]) -> Option<f64> {
    let rows = table.len();
    let cols = table.first().map(|r| r.len()).unwrap_or(0);
    let (mut c, mut d) = (0i64, 0i64);
    for i in 0..rows {
        for j in 0..cols {
            let nij = table[i][j];
            if nij != 0 {
                for row in table.iter().skip(i + 1) {
                    for (b, v) in row.iter().enumerate() {
                        if b > j {
                            c += nij * v;
                        } else if b < j {
                            d += nij * v;
                        }
                    }
                }
            }
        }
    }
    let n: i64 = table.iter().map(|r| r.iter().sum::<i64>()).sum();
    let pairs = (n * (n - 1)) as f64 / 2.0
        - fsum(table.iter().map(|r| {
            let s: i64 = r.iter().sum();
            (s * (s - 1)) as f64 / 2.0
        }));
    if pairs != 0.0 {
        Some((c - d) as f64 / pairs)
    } else {
        None
    }
}

/// The model of one scholar's grades of one book, on the book's road.
pub fn model_grader(
    key: &str,
    grader: &super::Grades,
    features: &HashMap<i64, Feat>,
    numbers: &HashMap<i64, String>,
    road: &Road,
) -> Option<Value> {
    let rows_raw: Vec<(&Feat, usize, i64)> = grader
        .items()
        .filter_map(|(h, g)| Some((features.get(&h)?, road.grade_index(g)?, h)))
        .collect();
    if rows_raw.len() < 100 {
        return None;
    }
    let (mapping, labels) =
        merge_categories(&rows_raw.iter().map(|r| r.1).collect::<Vec<_>>(), road);
    if labels.len() < 2 {
        return None;
    }
    let rows: Vec<(&Feat, usize)> = rows_raw.iter().map(|(f, y, _)| (*f, mapping[y])).collect();
    let blocks = varying(&rows.iter().map(|r| r.0).collect::<Vec<_>>());
    let (fit, design) = fit_blocks(&rows, &blocks, labels.len(), road);
    let fit = fit?;
    let k = fit.k;
    let mut ycount: Counter<usize> = Counter::default();
    for (_, y) in &rows {
        ycount.add(*y, 1);
    }
    let null_groups: Vec<Group> = ycount.items().map(|(y, w)| (vec![], *y, w)).collect();
    let ll0 = fit_ordinal(&null_groups, labels.len()).map(|n| n.loglik);
    let mut coefs = vec![];
    for (j, (name, block)) in design.columns.iter().zip(&design.column_block).enumerate() {
        let b = fit.params[k + j];
        let se = fit.cov[k + j][k + j].max(0.0).sqrt();
        let z = if se != 0.0 { b / se } else { 0.0 };
        coefs.push(
            json!({"name": name, "block": block, "beta": f(b), "se": f(se), "or": f(b.exp()),
                          "or_ci": [f((b - 1.96 * se).exp()), f((b + 1.96 * se).exp())],
                          "p": f(2.0 * normal_sf(z.abs()))}),
        );
    }
    let mut tests: Vec<Map<String, Value>> = vec![];
    let mut test_p = vec![];
    for block in &blocks {
        let reduced: Vec<String> = blocks.iter().filter(|b| *b != block).cloned().collect();
        let ll_r = if !reduced.is_empty() {
            fit_blocks(&rows, &reduced, labels.len(), road)
                .0
                .map(|r| r.loglik)
        } else {
            ll0
        };
        let Some(ll_r) = ll_r else { continue };
        let stat = (2.0 * (fit.loglik - ll_r)).max(0.0);
        let df = design.column_block.iter().filter(|cb| *cb == block).count();
        let p = chi2_sf(stat, df as f64);
        let mut t = Map::new();
        t.insert("block".into(), json!(block));
        t.insert("label".into(), json!(label_of(road, block)));
        t.insert("chi2".into(), f(stat));
        t.insert("df".into(), json!(df));
        t.insert("p".into(), f(p));
        tests.push(t);
        test_p.push(p);
    }
    for (t, adj) in tests.iter_mut().zip(holm(&test_p)) {
        t.insert("p_holm".into(), f(adj));
    }
    let (commonest, count) = ycount.most_common(1)[0];
    let base = count as f64 / rows.len() as f64;
    let mut hits = 0usize;
    let mut table = vec![vec![0i64; labels.len()]; 20];
    let mut calibration: HashMap<usize, (f64, i64, i64)> = HashMap::new();
    let mut above = vec![];
    let mut below = vec![];
    for (feat, y, h) in &rows_raw {
        let yy = mapping[y];
        let probs = predict(&fit.params, k, &design.row(feat));
        let mut likely = 0;
        for i in 1..probs.len() {
            if probs[i] > probs[likely] {
                likely = i;
            }
        }
        if likely == yy {
            hits += 1;
        }
        let expected = fsum(probs.iter().enumerate().map(|(i, q)| i as f64 * q))
            / (probs.len() - 1).max(1) as f64;
        table[((expected * 20.0) as usize).min(19)][yy] += 1;
        let bin = ((probs[0] * 10.0) as usize).min(9);
        let cal = calibration.entry(bin).or_insert((0.0, 0, 0));
        cal.0 += probs[0];
        cal.1 += 1;
        cal.2 += (yy == 0) as i64;
        let number = numbers.get(h).cloned().unwrap_or_else(|| h.to_string());
        let item = |p: f64| {
            json!({"hadith_id": h, "number": number, "grade": labels[yy], "likely": labels[likely],
                   "probs": probs.iter().map(|q| f(round_to(*q, 4))).collect::<Vec<_>>(),
                   "factors": feat.json(), "p": f(p)})
        };
        if yy < likely {
            let p = fsum(probs[..=yy].iter().copied());
            above.push((p, item(p)));
        } else if yy > likely {
            let p = fsum(probs[yy..].iter().copied());
            below.push((p, item(p)));
        }
    }
    let (tau, gamma) = kendall_from_table(&table);
    above.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    below.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let mut bins: Vec<(&usize, &(f64, i64, i64))> = calibration.iter().collect();
    bins.sort_by_key(|b| *b.0);
    let truthy = |x: Option<f64>| x.filter(|v| *v != 0.0);
    let lr = truthy(ll0).map(|l0| 2.0 * (fit.loglik - l0));
    Some(json!({
        "grader": grader.name, "book": key, "n": rows.len(), "labels": labels, "blocks": blocks,
        "coefficients": coefs, "thresholds": fit.params[..k].iter().map(|x| f(*x)).collect::<Vec<_>>(),
        "converged": fit.converged, "loglik": f(fit.loglik), "loglik0": fo(ll0),
        "mcfadden": fo(truthy(ll0).map(|l0| 1.0 - fit.loglik / l0)),
        "aic": f(2.0 * (k + fit.p) as f64 - 2.0 * fit.loglik), "accuracy": f(hits as f64 / rows.len() as f64),
        "base_rate": f(base), "base_label": labels[commonest],
        "somers_d": fo(somers_d(&table)), "tau_b": fo(tau), "gamma": fo(gamma),
        "lr_all": {"chi2": fo(lr), "df": fit.p, "p": fo(lr.map(|x| chi2_sf(x, fit.p as f64)))},
        "tests": tests,
        "calibration": bins.iter().filter(|b| b.1 .1 != 0).map(|(_b, c)| json!([f(round_to(c.0 / c.1 as f64, 4)), f(round_to(c.2 as f64 / c.1 as f64, 4)), c.1])).collect::<Vec<_>>(),
        "graded_higher": above.iter().take(60).map(|x| x.1.clone()).collect::<Vec<_>>(),
        "graded_higher_total": above.len(),
        "graded_higher_strong": above.iter().filter(|d| d.0 <= SURPRISE).count(),
        "graded_lower": below.iter().take(60).map(|x| x.1.clone()).collect::<Vec<_>>(),
        "graded_lower_total": below.len(),
        "graded_lower_strong": below.iter().filter(|d| d.0 <= SURPRISE).count(),
    }))
}

pub fn numbers_of(conn: &Connection, key: &str) -> rusqlite::Result<HashMap<i64, String>> {
    let mut stmt = conn.prepare(
        "SELECT h.id, h.number FROM hadiths h JOIN collections c ON c.id = h.collection_id WHERE c.key = ?",
    )?;
    let rows = stmt.query_map([key], |r| {
        Ok((r.get::<_, i64>(0)?, crate::db::number_text(r, 1)?))
    })?;
    rows.collect()
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
    let mut models: Vec<Value> = vec![];
    for (_id, key, name) in &books {
        let mut data = road.load(conn, key)?;
        if data.is_empty() {
            continue;
        }
        say(&tr(
            "Reading the chains of {name}",
            &[("name", name.clone())],
        ));
        let features = hadith_features(conn, key, &road)?;
        let numbers = numbers_of(conn, key)?;
        data.sort_by(|a, b| a.name.cmp(&b.name));
        for g in &data {
            say(&tr(
                "Fitting the model of {grader} — {name}",
                &[("grader", g.name.clone()), ("name", name.clone())],
            ));
            if let Some(mut m) = model_grader(key, g, &features, &numbers, &road) {
                m["book_name"] = json!(name);
                models.push(m);
            }
        }
    }
    let mut shared: Vec<(usize, f64, Value)> = vec![];
    for (_id, key, name) in &books {
        let mine: Vec<&Value> = models.iter().filter(|m| m["book"] == json!(key)).collect();
        if mine.len() < 2 {
            continue;
        }
        for side in ["graded_higher", "graded_lower"] {
            let mut seen: Vec<(Value, Vec<&Value>, Vec<&Value>)> = vec![];
            for m in &mine {
                for d in m[side].as_array().into_iter().flatten() {
                    let h = d["hadith_id"].clone();
                    if let Some(e) = seen.iter_mut().find(|e| e.0 == h) {
                        e.1.push(m);
                        e.2.push(d);
                    } else {
                        seen.push((h, vec![m], vec![d]));
                    }
                }
            }
            for (h, ms, ds) in seen {
                if ds.len() >= 2 {
                    let graders: Vec<Value> = ms
                        .iter()
                        .zip(&ds)
                        .map(|(m, d)| json!([m["grader"], d["grade"], d["likely"], d["p"]]))
                        .collect();
                    let mean =
                        fsum(ds.iter().map(|d| d["p"].as_f64().unwrap_or(0.0))) / ds.len() as f64;
                    shared.push((
                        ds.len(),
                        mean,
                        json!({"side": side, "book": name, "hadith_id": h, "number": ds[0]["number"],
                               "graders": graders, "factors": ds[0]["factors"]}),
                    ));
                }
            }
        }
    }
    shared.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then(a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
    });
    Ok(json!({
        "method": METHOD, "collection": collection, "surprise": SURPRISE, "tradition": road.tradition,
        "levels": road.levels, "reference": road.reference, "critic": road.critic,
        "blocks": blocks_for(&road).iter().map(|(k, l, e)| json!([k, l, e])).collect::<Vec<_>>(),
        "models": models, "shared": shared.into_iter().take(80).map(|s| s.2).collect::<Vec<_>>(),
    }))
}

pub fn cached(
    conn: &Connection,
    collection: Option<&str>,
    say: &dyn Fn(&str),
    recompute: bool,
    saved_only: bool,
) -> Result<Value, CachedError> {
    let path = super::derived().join(super::cache_name("models", collection));
    let stamp = super::fingerprint(conn, collection)? + METHOD;
    if !recompute {
        if let Some(v) = super::saved(&path, &stamp) {
            return Ok(v);
        }
    }
    if saved_only {
        return Err(CachedError::NotSaved(NotSaved));
    }
    let what = format!("models computed for {}", collection.unwrap_or("all books"));
    let result = crate::log::timed("statistics", &what, || analyse(conn, collection, say))?;
    Ok(super::save(&path, result, &stamp))
}
