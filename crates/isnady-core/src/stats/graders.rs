//! The graders of a book measured in depth (`stats_graders.py`): every pair's agreement (share, Cohen's kappa, the
//! ordinal weighted kappa, with 95% intervals), Krippendorff's ordinal alpha, a Dawid–Skene model of each grader's
//! confusion matrix, strictness from the model (mutashaddid / mutasahil, measured), and the disputed hadith.

use rusqlite::Connection;
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};

use super::corpus::CachedError;
use super::num::fsum;
use super::rng::Random;
use super::{
    f, fo, group_label, imami_label, load_imami, load_sunni, Counter, Data, Grades, NotSaved,
};

pub const METHOD: &str = "isnady-graders-4";
pub const MIN_GRADES: usize = 20;
pub const HIGH_AGREEMENT: f64 = 0.95;
pub const K: usize = 5;
pub const BOOT: usize = 300;
pub const SEED: u32 = 1405;

pub fn labels() -> Vec<&'static str> {
    (0..K as i64).map(group_label).collect()
}

pub fn is_shia_book(conn: &Connection, key: &str) -> bool {
    conn.query_row(
        "SELECT tradition FROM collections WHERE key = ?",
        [key],
        |r| r.get::<_, Option<String>>(0),
    )
    .ok()
    .flatten()
    .as_deref()
        == Some("shia")
}

/// (key, name, graders) of every book with grades, most graders first (`books_with_grades`).
pub fn books_with_grades(conn: &Connection) -> rusqlite::Result<Vec<(String, String, i64)>> {
    let mut stmt = conn.prepare(
        "SELECT key, name, COUNT(*) FROM (
             SELECT c.key AS key, c.name AS name, g.grader_name FROM grades g JOIN hadiths h ON h.id = g.hadith_id
             JOIN collections c ON c.id = h.collection_id GROUP BY c.key, g.grader_name HAVING COUNT(*) >= ?)
         GROUP BY key ORDER BY 3 DESC, name",
    )?;
    let rows = stmt.query_map([MIN_GRADES as i64], |r| {
        Ok((r.get(0)?, r.get(1)?, r.get(2)?))
    })?;
    rows.collect()
}

pub fn fingerprint(conn: &Connection, key: &str) -> rusqlite::Result<String> {
    let (n, top): (i64, i64) = conn.query_row(
        "SELECT COUNT(*), COALESCE(MAX(g.id), 0) FROM grades g JOIN hadiths h ON h.id = g.hadith_id
         JOIN collections c ON c.id = h.collection_id WHERE c.key = ?",
        [key],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    Ok(format!("{METHOD}|{key}|{n}|{top}|{BOOT}"))
}

fn kappas(pairs: &[(usize, usize)]) -> (f64, f64, f64) {
    let n = pairs.len() as i64;
    let mut obs = [[0i64; K]; K];
    for &(a, b) in pairs {
        obs[a][b] += 1;
    }
    let ra: Vec<i64> = (0..K).map(|i| obs[i].iter().sum()).collect();
    let rb: Vec<i64> = (0..K).map(|j| (0..K).map(|i| obs[i][j]).sum()).collect();
    let po = (0..K).map(|i| obs[i][i]).sum::<i64>() as f64 / n as f64;
    let pe = (0..K).map(|i| ra[i] * rb[i]).sum::<i64>() as f64 / (n * n) as f64;
    let kappa = if pe < 1.0 {
        (po - pe) / (1.0 - pe)
    } else {
        1.0
    };
    let w = |i: usize, j: usize| {
        let x = (i as f64 - j as f64) / (K - 1) as f64;
        x * x
    };
    let cells = || (0..K).flat_map(|i| (0..K).map(move |j| (i, j)));
    let d_o = fsum(cells().map(|(i, j)| w(i, j) * obs[i][j] as f64)) / n as f64;
    let d_e = fsum(cells().map(|(i, j)| w(i, j) * ra[i] as f64 * rb[j] as f64)) / (n * n) as f64;
    let wkappa = if d_e > 0.0 { 1.0 - d_o / d_e } else { 1.0 };
    (po, kappa, wkappa)
}

fn order_diff(pairs: &[(usize, usize)]) -> f64 {
    let lower = pairs.iter().filter(|(a, b)| a < b).count() as i64;
    let higher = pairs.iter().filter(|(a, b)| a > b).count() as i64;
    (lower - higher) as f64 / pairs.len() as f64
}

fn interval(values: &[f64]) -> Value {
    let mut v = values.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let last = (v.len() - 1) as f64;
    json!([f(v[(0.025 * last) as usize]), f(v[(0.975 * last) as usize])])
}

/// Krippendorff's alpha with the ordinal metric.
fn alpha(units: &[&Vec<usize>]) -> Option<f64> {
    let mut coinc = [[0.0f64; K]; K];
    for u in units {
        let m = u.len();
        if m < 2 {
            continue;
        }
        for (i, &a) in u.iter().enumerate() {
            for (j, &b) in u.iter().enumerate() {
                if i != j {
                    coinc[a][b] += 1.0 / (m - 1) as f64;
                }
            }
        }
    }
    let nc: Vec<f64> = (0..K).map(|c| fsum(coinc[c].iter().copied())).collect();
    let n = fsum(nc.iter().copied());
    if n <= 1.0 {
        return None;
    }
    let delta = |c: usize, k: usize| {
        let (lo, hi) = (c.min(k), c.max(k));
        let x = fsum((lo..=hi).map(|g| nc[g])) - (nc[c] + nc[k]) / 2.0;
        x * x
    };
    let cells = || (0..K).flat_map(|c| (0..K).map(move |k| (c, k)));
    let d_o = fsum(cells().map(|(c, k)| coinc[c][k] * delta(c, k)));
    let d_e = fsum(cells().map(|(c, k)| nc[c] * nc[k] * delta(c, k))) / (n - 1.0);
    if d_e > 0.0 {
        Some(1.0 - d_o / d_e)
    } else {
        None
    }
}

pub struct Ds {
    pub graders: Vec<String>,
    pub prior: Vec<f64>,
    pub theta: Vec<Vec<Vec<f64>>>,
    pub items: Vec<i64>,
    pub posterior: Vec<Vec<f64>>,
    pub strictness: Vec<f64>,
    pub loglik: f64,
}

fn normalize(m: &[Vec<f64>]) -> Vec<Vec<f64>> {
    m.iter()
        .map(|row| {
            let s = fsum(row.iter().copied());
            row.iter().map(|x| x / s).collect()
        })
        .collect()
}

fn strict_of(prior: &[f64], theta: &[Vec<f64>]) -> f64 {
    let cells = || (0..K).flat_map(|k| (0..K).map(move |l| (k, l)));
    let below = fsum(
        cells()
            .filter(|(k, l)| l < k)
            .map(|(k, l)| prior[k] * theta[k][l]),
    );
    let above = fsum(
        cells()
            .filter(|(k, l)| l > k)
            .map(|(k, l)| prior[k] * theta[k][l]),
    );
    above - below
}

/// The Dawid–Skene model by EM (`dawid_skene`): the input graders with their grades {item: grade}.
pub fn dawid_skene(data: &[(&str, &HashMap<i64, i64>)], iterations: usize, tol: f64) -> Ds {
    let mut graders: Vec<&str> = data.iter().map(|(g, _)| *g).collect();
    graders.sort();
    let by_name: HashMap<&str, &HashMap<i64, i64>> = data.iter().map(|(g, d)| (*g, *d)).collect();
    let mut items: Vec<i64> = data
        .iter()
        .flat_map(|(_, d)| d.keys().copied())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    items.sort_unstable();
    let votes: Vec<Vec<(usize, usize)>> = items
        .iter()
        .map(|h| {
            graders
                .iter()
                .enumerate()
                .filter_map(|(j, g)| by_name[g].get(h).map(|l| (j, *l as usize)))
                .collect()
        })
        .collect();
    let mut post: Vec<Vec<f64>> = votes
        .iter()
        .map(|vs| {
            let mut c = [0i64; K];
            for &(_, k) in vs {
                c[k] += 1;
            }
            (0..K).map(|k| c[k] as f64 / vs.len() as f64).collect()
        })
        .collect();
    let mut loglik_prev: Option<f64> = None;
    let mut prior = vec![0.0; K];
    let mut theta: Vec<Vec<Vec<f64>>> = vec![];
    let mut loglik = 0.0;
    let ni = items.len() as f64;
    for _ in 0..iterations {
        prior = (0..K)
            .map(|k| fsum(post.iter().map(|p| p[k])) / ni)
            .collect();
        let mut m = vec![vec![vec![0.5f64; K]; K]; graders.len()];
        for (h, vs) in votes.iter().enumerate() {
            for &(jj, l) in vs {
                for k in 0..K {
                    m[jj][k][l] += post[h][k];
                }
            }
        }
        theta = m.iter().map(|mj| normalize(mj)).collect();
        loglik = 0.0;
        for (h, vs) in votes.iter().enumerate() {
            let w: Vec<f64> = (0..K)
                .map(|k| {
                    let mut p = prior[k].max(1e-12);
                    for &(j, l) in vs {
                        p *= theta[j][k][l];
                    }
                    p
                })
                .collect();
            let s = fsum(w.iter().copied());
            loglik += s.ln();
            post[h] = w.iter().map(|x| x / s).collect();
        }
        if let Some(prev) = loglik_prev {
            if (loglik - prev).abs() < tol * prev.abs() {
                break;
            }
        }
        loglik_prev = Some(loglik);
    }
    let strictness = theta.iter().map(|t| strict_of(&prior, t)).collect();
    Ds {
        graders: graders.iter().map(|g| g.to_string()).collect(),
        prior,
        theta,
        items,
        posterior: post,
        strictness,
        loglik,
    }
}

/// A grader left out of the model, measured against its result without moving it.
fn measured_against(
    ds: &Ds,
    at: &HashMap<i64, usize>,
    grades: impl Iterator<Item = (i64, i64)>,
) -> Vec<Vec<f64>> {
    let mut m = vec![vec![0.5f64; K]; K];
    for (h, l) in grades {
        if let Some(&i) = at.get(&h) {
            for k in 0..K {
                m[k][l as usize] += ds.posterior[i][k];
            }
        }
    }
    normalize(&m)
}

fn theta_json(t: &[Vec<f64>]) -> Value {
    Value::Array(
        t.iter()
            .map(|r| Value::Array(r.iter().map(|x| f(*x)).collect()))
            .collect(),
    )
}

pub fn analyse(conn: &Connection, key: &str, say: &dyn Fn(&str)) -> rusqlite::Result<Value> {
    let ui = crate::i18n::ui_language();
    let tr = |s: &str, v: &[(&str, String)]| crate::i18n::t(&ui, s, v);
    let mut chain_only: Counter<String> = Counter::default();
    let shia = is_shia_book(conn, key);
    let data: Data = if shia {
        load_imami(conn, key)?
    } else {
        load_sunni(conn, key, Some(&mut chain_only))?
    };
    let mut few = Map::new();
    for g in &data {
        if g.len() < MIN_GRADES {
            few.insert(g.name.clone(), json!(g.len()));
        }
    }
    let mut kept: Vec<&Grades> = data.iter().filter(|g| g.len() >= MIN_GRADES).collect();
    kept.sort_by_key(|g| -(g.len() as i64));
    let mut rng = Random::new(SEED);
    let mut out = Map::new();
    out.insert("collection".into(), json!(key));
    out.insert(
        "tradition".into(),
        json!(if shia { "shia" } else { "sunni" }),
    );
    out.insert(
        "graders".into(),
        json!(kept.iter().map(|g| &g.name).collect::<Vec<_>>()),
    );
    out.insert(
        "counts".into(),
        Value::Object(
            kept.iter()
                .map(|g| (g.name.clone(), json!(g.len())))
                .collect(),
        ),
    );
    out.insert(
        "distribution".into(),
        Value::Object(
            kept.iter()
                .map(|g| {
                    let d: Vec<i64> = (0..K as i64)
                        .map(|c| g.items().filter(|(_, k)| *k == c).count() as i64)
                        .collect();
                    (g.name.clone(), json!(d))
                })
                .collect(),
        ),
    );
    let labels: Vec<&str> = if shia {
        (0..K as i64).map(imami_label).collect()
    } else {
        labels()
    };
    out.insert("labels".into(), json!(labels));
    out.insert(
        "categories".into(),
        if shia {
            json!([1, 2, 3, 4])
        } else {
            json!([0, 1, 2, 3, 4])
        },
    );
    out.insert("boot".into(), json!(BOOT));
    out.insert(
        "chain_only".into(),
        Value::Object(
            kept.iter()
                .map(|g| (g.name.clone(), json!(chain_only.get(&g.name))))
                .collect(),
        ),
    );
    out.insert("too_few".into(), Value::Object(few));
    // a grader who gives one grade only is a selection, not a grading
    let single: Vec<&Grades> = kept
        .iter()
        .copied()
        .filter(|g| g.map.values().collect::<HashSet<_>>().len() == 1)
        .collect();
    let mut selections = vec![];
    for g in &single {
        let grade = g.get(g.order[0]).unwrap_or(0);
        let mut others = Map::new();
        for o in &kept {
            if o.name == g.name {
                continue;
            }
            let common: Vec<i64> = g
                .order
                .iter()
                .copied()
                .filter(|h| o.map.contains_key(h))
                .collect();
            if !common.is_empty() {
                let dist: Vec<i64> = (0..K as i64)
                    .map(|c| common.iter().filter(|h| o.get(**h) == Some(c)).count() as i64)
                    .collect();
                others.insert(
                    o.name.clone(),
                    json!({"common": common.len(), "distribution": dist}),
                );
            }
        }
        selections
            .push(json!({"grader": g.name, "grade": grade, "count": g.len(), "others": others}));
    }
    out.insert("selections".into(), Value::Array(selections));
    let singles: HashSet<&str> = single.iter().map(|g| g.name.as_str()).collect();
    let graders: Vec<&Grades> = kept
        .into_iter()
        .filter(|g| !singles.contains(g.name.as_str()))
        .collect();
    out.insert(
        "graders".into(),
        json!(graders.iter().map(|g| &g.name).collect::<Vec<_>>()),
    );
    if graders.len() < 2 {
        return Ok(Value::Object(out));
    }

    say(&tr("Agreement between every pair of graders", &[]));
    let mut pairs_out = Map::new();
    for (i, a) in graders.iter().enumerate() {
        for b in &graders[i + 1..] {
            let mut common: Vec<i64> = a
                .order
                .iter()
                .copied()
                .filter(|h| b.map.contains_key(h))
                .collect();
            common.sort_unstable();
            if common.len() < 20 {
                continue;
            }
            let pairs: Vec<(usize, usize)> = common
                .iter()
                .map(|h| (a.map[h] as usize, b.map[h] as usize))
                .collect();
            let (po, kap, wk) = kappas(&pairs);
            let od = order_diff(&pairs);
            let mut boots: [Vec<f64>; 4] = Default::default();
            let mut s = Vec::with_capacity(pairs.len());
            for _ in 0..BOOT {
                s.clear();
                for _ in 0..pairs.len() {
                    s.push(pairs[rng.randrange(pairs.len())]);
                }
                let (bpo, bk, bwk) = kappas(&s);
                boots[0].push(bpo);
                boots[1].push(bk);
                boots[2].push(bwk);
                boots[3].push(order_diff(&s));
            }
            let within = pairs.iter().filter(|(x, y)| x.abs_diff(*y) <= 1).count() as f64
                / pairs.len() as f64;
            let table: Vec<Vec<i64>> = (0..K)
                .map(|r| {
                    (0..K)
                        .map(|c| pairs.iter().filter(|(x, y)| *x == r && *y == c).count() as i64)
                        .collect()
                })
                .collect();
            pairs_out.insert(
                format!("{}|{}", a.name, b.name),
                json!({"common": common.len(), "agree": f(po), "kappa": f(kap), "wkappa": f(wk), "order": f(od),
                       "within_one": f(within),
                       "ci": {"agree": interval(&boots[0]), "kappa": interval(&boots[1]),
                              "wkappa": interval(&boots[2]), "order": interval(&boots[3])},
                       "table": table}),
            );
        }
    }
    say(&tr("Krippendorff's alpha", &[]));
    let mut all: Vec<i64> = graders
        .iter()
        .flat_map(|g| g.order.iter().copied())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    all.sort_unstable();
    let units: Vec<Vec<usize>> = all
        .iter()
        .map(|h| {
            graders
                .iter()
                .filter_map(|g| g.get(*h).map(|k| k as usize))
                .collect::<Vec<_>>()
        })
        .filter(|u| u.len() >= 2)
        .collect();
    let refs: Vec<&Vec<usize>> = units.iter().collect();
    let alpha_value = alpha(&refs);
    let mut alpha_boots = vec![];
    for _ in 0..BOOT / 3 {
        let sample: Vec<&Vec<usize>> = (0..units.len())
            .map(|_| &units[rng.randrange(units.len())])
            .collect();
        if let Some(a) = alpha(&sample) {
            alpha_boots.push(a);
        }
    }
    out.insert("pairs".into(), Value::Object(pairs_out.clone()));
    out.insert(
        "alpha".into(),
        json!({"value": fo(alpha_value), "ci": interval(&alpha_boots), "units": units.len()}),
    );

    say(&tr(
        "Dawid–Skene model (each grader's confusion matrix)",
        &[],
    ));
    let by_name: HashMap<&str, &Grades> = graders.iter().map(|g| (g.name.as_str(), *g)).collect();
    let mut one_voice = vec![];
    let mut left_out: Vec<String> = vec![];
    for (k, p) in &pairs_out {
        let (a, b) = k.split_once('|').unwrap_or((k, ""));
        let agree = p["agree"].as_f64().unwrap_or(0.0);
        if agree >= HIGH_AGREEMENT {
            let (keep, drop) = if by_name[a].len() >= by_name[b].len() {
                (a, b)
            } else {
                (b, a)
            };
            one_voice.push(json!({"kept": keep, "left_out": drop, "agree": f(agree)}));
            if !left_out.iter().any(|x| x == drop) {
                left_out.push(drop.to_string());
            }
        }
    }
    out.insert("one_voice".into(), Value::Array(one_voice));
    let voices: Vec<(&str, &HashMap<i64, i64>)> = graders
        .iter()
        .filter(|g| !left_out.contains(&g.name))
        .map(|g| (g.name.as_str(), &g.map))
        .collect();
    let ds = dawid_skene(&voices, 200, 1e-6);
    let at: HashMap<i64, usize> = ds.items.iter().enumerate().map(|(i, h)| (*h, i)).collect();
    let mut theta_of: Vec<(String, Vec<Vec<f64>>)> = ds
        .graders
        .iter()
        .cloned()
        .zip(ds.theta.iter().cloned())
        .collect();
    let mut strict_of_g: HashMap<String, f64> = ds
        .graders
        .iter()
        .cloned()
        .zip(ds.strictness.iter().copied())
        .collect();
    for g in &left_out {
        let th = measured_against(&ds, &at, by_name[g.as_str()].items());
        strict_of_g.insert(g.clone(), strict_of(&ds.prior, &th));
        theta_of.push((g.clone(), th));
    }
    let mut strict_boot: HashMap<String, Vec<f64>> = HashMap::new();
    for b in 0..40 {
        say(&tr(
            "Strictness intervals ({value} of 40)",
            &[("value", (b + 1).to_string())],
        ));
        let sample: Vec<i64> = (0..ds.items.len())
            .map(|_| ds.items[rng.randrange(ds.items.len())])
            .collect();
        let mut resampled: Vec<(String, HashMap<i64, i64>)> = vec![];
        let mut where_: HashMap<String, usize> = HashMap::new();
        for (n, h) in sample.iter().enumerate() {
            for g in &graders {
                if let Some(v) = g.get(*h) {
                    let i = *where_.entry(g.name.clone()).or_insert_with(|| {
                        resampled.push((g.name.clone(), HashMap::new()));
                        resampled.len() - 1
                    });
                    resampled[i].1.insert(n as i64, v);
                }
            }
        }
        let input: Vec<(&str, &HashMap<i64, i64>)> = resampled
            .iter()
            .filter(|(g, _)| !left_out.contains(g))
            .map(|(g, d)| (g.as_str(), d))
            .collect();
        let fit = dawid_skene(&input, 60, 1e-5);
        let fat: HashMap<i64, usize> = fit.items.iter().enumerate().map(|(i, h)| (*h, i)).collect();
        for (g, v) in fit.graders.iter().zip(&fit.strictness) {
            strict_boot.entry(g.clone()).or_default().push(*v);
        }
        for g in &left_out {
            let mut items: Vec<(i64, i64)> = resampled
                .iter()
                .find(|(name, _)| name == g)
                .map(|(_, d)| d.iter().map(|(n, l)| (*n, *l)).collect())
                .unwrap_or_default();
            items.sort_unstable();
            let th = measured_against(&fit, &fat, items.into_iter());
            strict_boot
                .entry(g.clone())
                .or_default()
                .push(strict_of(&fit.prior, &th));
        }
    }
    let theta_obj: Map<String, Value> = theta_of
        .iter()
        .map(|(g, t)| (g.clone(), theta_json(t)))
        .collect();
    let strictness: Map<String, Value> = graders
        .iter()
        .map(|g| {
            let boots = strict_boot.get(&g.name).cloned().unwrap_or_default();
            (
                g.name.clone(),
                json!({"value": f(strict_of_g[&g.name]), "ci": if boots.is_empty() { Value::Null } else { interval(&boots) }}),
            )
        })
        .collect();
    out.insert(
        "model".into(),
        json!({"prior": ds.prior.iter().map(|x| f(*x)).collect::<Vec<_>>(), "theta": theta_obj,
               "loglik": f(ds.loglik), "strictness": strictness}),
    );

    let mut consensus = [0i64; K];
    let mut disputed: Vec<(i64, Vec<(String, i64)>, usize, f64, i64, f64)> = vec![];
    for (i, h) in ds.items.iter().enumerate() {
        let p = &ds.posterior[i];
        let mut best = 0;
        for k in 1..K {
            if p[k] > p[best] {
                best = k;
            }
        }
        consensus[best] += 1;
        let given: Vec<(String, i64)> = graders
            .iter()
            .filter_map(|g| g.get(*h).map(|v| (g.name.clone(), v)))
            .collect();
        let spread = if given.len() >= 2 {
            given.iter().map(|x| x.1).max().unwrap() - given.iter().map(|x| x.1).min().unwrap()
        } else {
            0
        };
        let entropy = -fsum(p.iter().filter(|x| **x > 0.0).map(|x| x * x.ln())) / (K as f64).ln();
        if spread >= 2 || (given.len() >= 2 && p[best] < 0.6) {
            disputed.push((*h, given, best, p[best], spread, entropy));
        }
    }
    disputed.sort_by(|a, b| {
        (-a.4)
            .cmp(&-b.4)
            .then(a.3.partial_cmp(&b.3).unwrap_or(std::cmp::Ordering::Equal))
    });
    let first: Vec<_> = disputed.iter().take(300).collect();
    let mut numbers: HashMap<i64, Value> = HashMap::new();
    if !first.is_empty() {
        let ids: Vec<String> = first.iter().map(|d| d.0.to_string()).collect();
        let mut stmt = conn.prepare(&format!(
            "SELECT id, number FROM hadiths WHERE id IN ({})",
            ids.join(",")
        ))?;
        let rows = stmt.query_map([], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, rusqlite::types::Value>(1)?))
        })?;
        for row in rows {
            let (id, v) = row?;
            numbers.insert(id, sql_json(v));
        }
    }
    let disputed_json: Vec<Value> = first
        .iter()
        .map(|(h, given, best, cert, spread, entropy)| {
            let grades: Map<String, Value> = given.iter().map(|(g, v)| (g.clone(), json!(v))).collect();
            json!({"hadith_id": h, "grades": grades, "consensus": best, "certainty": f(*cert), "spread": spread,
                   "entropy": f(*entropy), "number": numbers.get(h).cloned().unwrap_or(Value::Null)})
        })
        .collect();
    let maxes: Vec<f64> = ds
        .posterior
        .iter()
        .map(|p| p.iter().copied().fold(f64::MIN, f64::max))
        .collect();
    out.insert("consensus".into(), json!(consensus));
    out.insert("disputed_total".into(), json!(disputed.len()));
    out.insert("disputed".into(), Value::Array(disputed_json));
    out.insert(
        "certainty".into(),
        json!({"high": maxes.iter().filter(|m| **m >= 0.95).count(),
               "middle": maxes.iter().filter(|m| **m >= 0.6 && **m < 0.95).count(),
               "low": maxes.iter().filter(|m| **m < 0.6).count(), "total": maxes.len()}),
    );
    Ok(Value::Object(out))
}

pub fn sql_json(v: rusqlite::types::Value) -> Value {
    use rusqlite::types::Value as V;
    match v {
        V::Null | V::Blob(_) => Value::Null,
        V::Integer(i) => json!(i),
        V::Real(x) => f(x),
        V::Text(t) => json!(t),
    }
}

pub fn cached(
    conn: &Connection,
    key: &str,
    say: &dyn Fn(&str),
    recompute: bool,
    saved_only: bool,
) -> Result<Value, CachedError> {
    let path = super::derived().join(format!("graders-{}.json", crate::works::full_name(key)));
    let stamp = fingerprint(conn, key)?;
    if !recompute {
        if let Some(v) = super::saved(&path, &stamp) {
            return Ok(v);
        }
    }
    if saved_only {
        return Err(CachedError::NotSaved(NotSaved));
    }
    let what = format!(
        "graders computed for {}",
        if key.is_empty() { "all books" } else { key }
    );
    let result = crate::log::timed("statistics", &what, || analyse(conn, key, say))?;
    Ok(super::save(&path, result, &stamp))
}
