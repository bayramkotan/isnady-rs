//! Statistics of the whole corpus (`stats_corpus.py`): the narrators, the chains, the grades and the books, measured.
//! Ranks and grades are ordered categories: counted, cross-tabulated and rank-correlated (Kendall's tau-b,
//! Goodman–Kruskal's gamma), never averaged. Intervals: percentile bootstraps over hadith with a fixed seed.

use rusqlite::Connection;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

use super::num::fsum;
use super::rng::Random;
use super::{books_of, chains, f, fo, persons, road_of, Counter, NotSaved, Person, Road};

pub const METHOD: &str = "isnady-corpus-3";
pub const BOOT: usize = 200;
pub const SEED: u32 = 1405;

/// Kendall's tau-b and Goodman–Kruskal's gamma of an ordered r × c table.
pub fn kendall_from_table(table: &[Vec<i64>]) -> (Option<f64>, Option<f64>) {
    let rows = table.len();
    let cols = table.first().map(|r| r.len()).unwrap_or(0);
    let n: i64 = table.iter().map(|r| r.iter().sum::<i64>()).sum();
    if n < 2 {
        return (None, None);
    }
    let (mut concordant, mut discordant) = (0i64, 0i64);
    for i in 0..rows {
        for j in 0..cols {
            let nij = table[i][j];
            if nij == 0 {
                continue;
            }
            for row in table.iter().skip(i + 1) {
                for (m, v) in row.iter().enumerate() {
                    if m > j {
                        concordant += nij * v;
                    } else if m < j {
                        discordant += nij * v;
                    }
                }
            }
        }
    }
    let pairs = (n * (n - 1)) as f64 / 2.0;
    let tie_rows = fsum(table.iter().map(|r| {
        let s: i64 = r.iter().sum();
        (s * (s - 1)) as f64 / 2.0
    }));
    let tie_cols = fsum((0..cols).map(|j| {
        let s: i64 = table.iter().map(|r| r[j]).sum();
        (s * (s - 1)) as f64 / 2.0
    }));
    let denom = ((pairs - tie_rows) * (pairs - tie_cols)).powf(0.5);
    let tau = if denom != 0.0 {
        Some((concordant - discordant) as f64 / denom)
    } else {
        None
    };
    let gamma = if concordant + discordant != 0 {
        Some((concordant - discordant) as f64 / (concordant + discordant) as f64)
    } else {
        None
    };
    (tau, gamma)
}

fn table_of(pairs: &[(usize, usize)], rows: usize, cols: usize) -> Vec<Vec<i64>> {
    let mut t = vec![vec![0i64; cols]; rows];
    for &(a, b) in pairs {
        t[a][b] += 1;
    }
    t
}

/// tau-b and gamma of (row, column) pairs with 95% percentile-bootstrap intervals, and the table itself.
pub fn association(pairs: &[(usize, usize)], rows: usize, cols: usize, rng: &mut Random) -> Value {
    let table = table_of(pairs, rows, cols);
    let (tau, gamma) = kendall_from_table(&table);
    let (mut taus, mut gammas) = (vec![], vec![]);
    if pairs.len() >= 30 {
        let n = pairs.len();
        let mut sample = Vec::with_capacity(n);
        for _ in 0..BOOT {
            sample.clear();
            for _ in 0..n {
                sample.push(pairs[rng.choice_index(n)]);
            }
            let (t, g) = kendall_from_table(&table_of(&sample, rows, cols));
            if let Some(t) = t {
                taus.push(t);
            }
            if let Some(g) = g {
                gammas.push(g);
            }
        }
    }
    let ci = |mut v: Vec<f64>| -> Value {
        if v.len() < 20 {
            return Value::Null;
        }
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let n = v.len() as f64;
        json!([f(v[(0.025 * n) as usize]), f(v[(0.975 * n) as usize - 1])])
    };
    json!({"n": pairs.len(), "table": table, "tau": fo(tau), "tau_ci": ci(taus), "gamma": fo(gamma),
           "gamma_ci": ci(gammas)})
}

/// The Lorenz curve of the links the narrators carry, the Gini coefficient and the top shares.
pub fn lorenz(counts: &[i64], points: usize) -> Value {
    let mut values = counts.to_vec();
    values.sort_unstable();
    let n = values.len();
    let total: i64 = values.iter().sum();
    if n == 0 || total == 0 {
        return json!({"curve": [], "gini": null});
    }
    let mut cum = Vec::with_capacity(n);
    let mut running = 0i64;
    for v in &values {
        running += v;
        cum.push(running);
    }
    let mut curve = vec![json!([0.0, 0.0])];
    for k in 1..=points {
        let i = (super::num::round_int((k * n) as f64 / points as f64) - 1).max(0) as usize;
        curve.push(json!([
            f((i + 1) as f64 / n as f64),
            f(cum[i] as f64 / total as f64)
        ]));
    }
    let gini =
        1.0 - 2.0 * fsum(cum.iter().map(|c| *c as f64 / total as f64)) / n as f64 + 1.0 / n as f64;
    let desc: Vec<i64> = values.iter().rev().copied().collect();
    let top = |share: f64| -> f64 {
        let k = super::num::round_int(share * n as f64).max(1) as usize;
        desc[..k.min(n)].iter().sum::<i64>() as f64 / total as f64
    };
    let (mut half, mut acc) = (0usize, 0i64);
    for v in &desc {
        acc += v;
        half += 1;
        if acc as f64 >= total as f64 / 2.0 {
            break;
        }
    }
    json!({"curve": curve, "gini": f(gini), "top1": f(top(0.01)), "top10": f(top(0.10)), "half_by": half,
           "people": n})
}

/// PageRank on the transmission graph, student → teacher weighted by how often.
pub fn pagerank(edges: &Counter<(i64, i64)>, damping: f64, iterations: usize) -> Vec<(i64, f64)> {
    let mut out_weight: HashMap<i64, f64> = HashMap::new();
    let mut incoming: HashMap<i64, Vec<(i64, i64)>> = HashMap::new();
    let mut nodes: Vec<i64> = vec![];
    let mut seen = HashSet::new();
    for (&(s, t), w) in edges.items() {
        *out_weight.entry(s).or_insert(0.0) += w as f64;
        incoming.entry(t).or_default().push((s, w));
        for v in [s, t] {
            if seen.insert(v) {
                nodes.push(v);
            }
        }
    }
    if nodes.is_empty() {
        return vec![];
    }
    let n = nodes.len() as f64;
    let mut rank: HashMap<i64, f64> = nodes.iter().map(|v| (*v, 1.0 / n)).collect();
    for _ in 0..iterations {
        let sink = fsum(
            nodes
                .iter()
                .filter(|v| out_weight.get(v).copied().unwrap_or(0.0) == 0.0)
                .map(|v| rank[v]),
        );
        let mut new = HashMap::with_capacity(nodes.len());
        for v in &nodes {
            let flow = fsum(
                incoming
                    .get(v)
                    .map(|l| l.as_slice())
                    .unwrap_or(&[])
                    .iter()
                    .map(|(s, w)| rank[s] * *w as f64 / out_weight[s]),
            );
            new.insert(*v, (1.0 - damping) / n + damping * (flow + sink / n));
        }
        rank = new;
    }
    nodes.iter().map(|v| (*v, rank[v])).collect()
}

const GAP_LOW: i64 = -60;
const GAP_HIGH: i64 = 160;

fn person_row(
    persons: &HashMap<i64, Person>,
    links: &Counter<i64>,
    hadith_of: &HashMap<i64, HashSet<i64>>,
    p: i64,
) -> Value {
    let info = &persons[&p];
    json!({"id": p, "name": info.name, "rank": info.rank, "tabaqa": info.tabaqa, "death": info.death,
           "links": links.get(&p), "hadith": hadith_of.get(&p).map(|s| s.len()).unwrap_or(0)})
}

fn with(mut row: Value, key: &str, v: Value) -> Value {
    row[key] = v;
    row
}

struct BookStat {
    name: String,
    chains: i64,
    links: i64,
    identified: i64,
    reaching: i64,
    full: i64,
    lengths: Counter<usize>,
    groups: Vec<i64>,
    narrators: HashSet<i64>,
}

pub fn analyse(
    conn: &Connection,
    collection: Option<&str>,
    say: &dyn Fn(&str),
) -> rusqlite::Result<Value> {
    let tr = |s: &str| crate::i18n::t(&crate::i18n::ui_language(), s, &[]);
    let mut rng = Random::new(SEED);
    let books = books_of(conn, collection)?;
    if books.is_empty() {
        return Ok(json!({"empty": true}));
    }
    let road: Road = road_of(conn, collection);
    let ngroups = road.rank_groups.len();
    say(&tr("Reading the narrators"));
    let persons = persons(conn, road.tradition)?;
    say(&tr("Reading the chains"));
    let ids: Vec<i64> = books.iter().map(|b| b.0).collect();
    let chains = chains(conn, &ids)?;
    let marks = vec!["?"; ids.len()].join(",");
    let hadith_count: i64 = conn.query_row(
        &format!("SELECT COUNT(*) FROM hadiths WHERE collection_id IN ({marks})"),
        rusqlite::params_from_iter(&ids),
        |r| r.get(0),
    )?;

    let mut per_book: Vec<BookStat> = books
        .iter()
        .map(|(_, _, name)| BookStat {
            name: name.clone(),
            chains: 0,
            links: 0,
            identified: 0,
            reaching: 0,
            full: 0,
            lengths: Counter::default(),
            groups: vec![0; ngroups],
            narrators: HashSet::new(),
        })
        .collect();
    let at: HashMap<i64, usize> = books.iter().enumerate().map(|(i, b)| (b.0, i)).collect();
    let mut link_count: Counter<i64> = Counter::default();
    let mut hadith_of: HashMap<i64, HashSet<i64>> = HashMap::new();
    let mut edges: Counter<(i64, i64)> = Counter::default();
    let mut weakest_of_hadith: HashMap<i64, usize> = HashMap::new();
    let mut length_of_hadith: HashMap<i64, usize> = HashMap::new();
    let mut gaps: Counter<i64> = Counter::default();
    let mut gap_pairs: Counter<(i64, i64, i64)> = Counter::default();
    let (mut order_ok, mut order_bad) = (0i64, 0i64);
    let mut order_pairs: Counter<(i64, i64)> = Counter::default();
    let mut weakest_dist = vec![0i64; ngroups];
    let mut weakest_full = vec![0i64; ngroups];

    say(&tr("Measuring the chains"));
    for c in &chains {
        let b = &mut per_book[at[&c.book]];
        let links = &c.links;
        b.chains += 1;
        b.links += links.len() as i64;
        b.lengths.add(links.len(), 1);
        b.reaching += c.reaches as i64;
        let ids: Vec<i64> = links
            .iter()
            .flatten()
            .copied()
            .filter(|p| persons.contains_key(p))
            .collect();
        b.identified += ids.len() as i64;
        if !links.is_empty() && ids.len() == links.len() {
            b.full += 1;
        }
        for &p in &ids {
            link_count.add(p, 1);
            hadith_of.entry(p).or_default().insert(c.hadith);
            b.narrators.insert(p);
            if let Some(g) = road.group_of(persons[&p].rank) {
                b.groups[g] += 1;
            }
        }
        let ranks: Vec<i64> = ids
            .iter()
            .filter_map(|p| {
                persons[p]
                    .rank
                    .filter(|_| road.group_of(persons[p].rank).is_some())
            })
            .collect();
        if let Some(&worst) = ranks.iter().max() {
            let weakest = road.group_of(Some(worst)).expect("group");
            weakest_dist[weakest] += 1;
            if ids.len() == links.len() {
                weakest_full[weakest] += 1;
            }
            let e = weakest_of_hadith.entry(c.hadith).or_insert(99);
            *e = (*e).min(weakest);
        }
        if !links.is_empty() && !length_of_hadith.contains_key(&c.hadith) {
            length_of_hadith.insert(c.hadith, links.len());
        }
        for w in links.windows(2) {
            let (Some(student), Some(teacher)) = (w[0], w[1]) else {
                continue;
            };
            let (Some(ps), Some(pt)) = (persons.get(&student), persons.get(&teacher)) else {
                continue;
            };
            edges.add((student, teacher), 1);
            if let (Some(ds), Some(dt)) =
                (ps.death.filter(|d| *d != 0), pt.death.filter(|d| *d != 0))
            {
                let gap = ds - dt;
                gaps.add(gap.clamp(GAP_LOW, GAP_HIGH).div_euclid(10) * 10, 1);
                if !(-60..=110).contains(&gap) {
                    gap_pairs.add((student, teacher, gap), 1);
                }
            }
            if let (Some(ts), Some(tt)) =
                (ps.tabaqa.filter(|t| *t != 0), pt.tabaqa.filter(|t| *t != 0))
            {
                if ts < tt {
                    order_bad += 1;
                    order_pairs.add((student, teacher), 1);
                } else {
                    order_ok += 1;
                }
            }
        }
    }

    let total_links: i64 = per_book.iter().map(|b| b.links).sum();
    let total_identified: i64 = per_book.iter().map(|b| b.identified).sum();
    let overview = json!({
        "books": books.len(), "hadith": hadith_count, "chains": chains.len(), "links": total_links,
        "identified": total_identified, "reaching": per_book.iter().map(|b| b.reaching).sum::<i64>(),
        "full": per_book.iter().map(|b| b.full).sum::<i64>(), "narrators": link_count.len(),
    });

    say(&tr("Measuring the narrators"));
    let mut rank_all = vec![0i64; ngroups];
    for p in persons.values() {
        if let Some(g) = road.group_of(p.rank) {
            rank_all[g] += 1;
        }
    }
    let mut rank_links = vec![0i64; ngroups];
    let mut rank_people = vec![0i64; ngroups];
    let mut unranked_links = 0i64;
    for (p, n) in link_count.items() {
        if let Some(g) = road.group_of(persons[p].rank) {
            rank_links[g] += n;
            rank_people[g] += 1;
        } else {
            unranked_links += n;
        }
    }
    let mut tabaqa_groups = vec![vec![0i64; ngroups]; 12];
    for p in &link_count.keys {
        let info = &persons[p];
        if let (Some(t), Some(g)) = (info.tabaqa, road.group_of(info.rank)) {
            if (1..=12).contains(&t) {
                tabaqa_groups[(t - 1) as usize][g] += 1;
            }
        }
    }
    let row = |p: i64| person_row(&persons, &link_count, &hadith_of, p);
    let pillars: Vec<Value> = link_count
        .most_common(25)
        .into_iter()
        .map(|(p, _)| row(p))
        .collect();
    let mut weak: Vec<(i64, i64)> = link_count
        .items()
        .filter(|(p, _)| persons[p].rank.unwrap_or(0) >= road.weak_from)
        .map(|(p, n)| (*p, n))
        .collect();
    weak.sort_by_key(|x| -x.1);
    let weak_pillars: Vec<Value> = weak.into_iter().take(15).map(|(p, _)| row(p)).collect();
    let concentration = lorenz(&link_count.counts, 40);
    say(&tr("Ranking the narrators in the network"));
    let mut pr = pagerank(&edges, 0.85, 60);
    pr.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let central: Vec<Value> = pr
        .iter()
        .take(15)
        .map(|(p, s)| with(row(*p), "score", f(*s)))
        .collect();
    let mut students_of: Counter<i64> = Counter::default();
    for &(_s, t) in &edges.keys {
        students_of.add(t, 1);
    }
    let most_students: Vec<Value> = students_of
        .most_common(10)
        .into_iter()
        .map(|(p, n)| with(row(p), "count", json!(n)))
        .collect();

    let mut lengths: Counter<usize> = Counter::default();
    for b in &per_book {
        for (k, n) in b.lengths.items() {
            lengths.add(*k, n);
        }
    }
    let mut lengths_sorted: Vec<(usize, i64)> = lengths.items().map(|(k, n)| (*k, n)).collect();
    lengths_sorted.sort();
    let suspects: Vec<Value> = gap_pairs
        .most_common(15)
        .into_iter()
        .map(|((s, t, g), n)| json!({"student": row(s), "teacher": row(t), "gap": g, "times": n}))
        .collect();
    let order_suspects: Vec<Value> = order_pairs
        .most_common(15)
        .into_iter()
        .map(|((s, t), n)| json!({"student": row(s), "teacher": row(t), "times": n}))
        .collect();

    say(&tr("Comparing the grades with the chains"));
    let mut grades_out = vec![];
    let order = &road.grade_order;
    for (_bid, key, name) in &books {
        let mut data = road.load(conn, key)?;
        data.sort_by(|a, b| a.name.cmp(&b.name));
        for g in &data {
            let pairs: Vec<(usize, usize)> = g
                .items()
                .filter_map(|(h, grade)| {
                    let w = *weakest_of_hadith.get(&h)?;
                    Some((w, road.grade_index(grade)?))
                })
                .collect();
            let length_pairs: Vec<(usize, usize)> = g
                .items()
                .filter_map(|(h, grade)| {
                    let l = *length_of_hadith.get(&h)?;
                    Some((l.min(9) - 1, road.grade_index(grade)?))
                })
                .collect();
            let mut dist: HashMap<i64, i64> = HashMap::new();
            for (_h, grade) in g.items() {
                *dist.entry(grade).or_insert(0) += 1;
            }
            let weakest = if pairs.len() >= 10 {
                association(&pairs, ngroups, order.len(), &mut rng)
            } else {
                Value::Null
            };
            let length = if length_pairs.len() >= 10 {
                association(&length_pairs, 9, order.len(), &mut rng)
            } else {
                Value::Null
            };
            grades_out.push(json!({
                "book": name, "key": key, "grader": g.name, "graded": g.len(),
                "distribution": order.iter().map(|o| dist.get(o).copied().unwrap_or(0)).collect::<Vec<_>>(),
                "weakest": weakest, "length": length,
            }));
        }
    }

    let mut book_rows = vec![];
    for b in &per_book {
        let mut lens: Vec<usize> = b
            .lengths
            .items()
            .flat_map(|(k, n)| std::iter::repeat(*k).take(n as usize))
            .filter(|n| *n != 0)
            .collect();
        lens.sort_unstable();
        book_rows.push(json!({
            "name": b.name, "chains": b.chains, "links": b.links,
            "identified": if b.links != 0 { f(b.identified as f64 / b.links as f64) } else { json!(0) },
            "reaching": if b.chains != 0 { f(b.reaching as f64 / b.chains as f64) } else { json!(0) },
            "median_length": if lens.is_empty() { Value::Null } else { json!(lens[lens.len() / 2]) },
            "groups": b.groups, "narrators": b.narrators.len(),
        }));
    }
    let names: Vec<&String> = per_book.iter().map(|b| &b.name).collect();
    let overlap: Vec<Vec<Value>> = per_book
        .iter()
        .map(|a| {
            per_book
                .iter()
                .map(|b| {
                    let union = a.narrators.union(&b.narrators).count();
                    if union != 0 {
                        f(a.narrators.intersection(&b.narrators).count() as f64 / union as f64)
                    } else {
                        json!(0)
                    }
                })
                .collect()
        })
        .collect();

    Ok(json!({
        "method": METHOD, "collection": collection, "boot": BOOT, "tradition": road.tradition,
        "groups": road.rank_groups.iter().map(|g| json!([g.name, g.desc])).collect::<Vec<_>>(),
        "grade_labels": road.grade_labels,
        "overview": overview,
        "narrators": {"rank_all": rank_all, "rank_links": rank_links, "rank_people": rank_people,
                      "unranked_links": unranked_links, "tabaqa_groups": tabaqa_groups, "pillars": pillars,
                      "weak_pillars": weak_pillars, "concentration": concentration, "central": central,
                      "most_students": most_students},
        "chains": {"lengths": lengths_sorted.iter().map(|(k, n)| json!([k, n])).collect::<Vec<_>>(),
                   "weakest": weakest_dist, "weakest_full": weakest_full,
                   "gaps": (GAP_LOW..=GAP_HIGH).step_by(10).map(|k| json!([k, gaps.get(&k)])).collect::<Vec<_>>(),
                   "gap_suspects": suspects, "order_ok": order_ok, "order_bad": order_bad,
                   "order_suspects": order_suspects},
        "grades": grades_out,
        "books": {"rows": book_rows, "names": names, "overlap": overlap},
    }))
}

/// The analysis from data_dir()/derived/ while the data is unchanged; computed and written otherwise.
pub fn cached(
    conn: &Connection,
    collection: Option<&str>,
    say: &dyn Fn(&str),
    recompute: bool,
    saved_only: bool,
) -> Result<Value, CachedError> {
    let path = super::derived().join(super::cache_name("corpus", collection));
    let stamp = super::fingerprint(conn, collection)?;
    if !recompute {
        if let Some(v) = super::saved(&path, &stamp) {
            return Ok(v);
        }
    }
    if saved_only {
        return Err(CachedError::NotSaved(NotSaved));
    }
    let what = format!("corpus computed for {}", collection.unwrap_or("all books"));
    let result = crate::log::timed("statistics", &what, || analyse(conn, collection, say))?;
    Ok(super::save(&path, result, &stamp))
}

#[derive(Debug)]
pub enum CachedError {
    NotSaved(NotSaved),
    Db(rusqlite::Error),
}

impl From<rusqlite::Error> for CachedError {
    fn from(e: rusqlite::Error) -> Self {
        CachedError::Db(e)
    }
}

impl std::fmt::Display for CachedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CachedError::NotSaved(_) => write!(f, "not saved"),
            CachedError::Db(e) => write!(f, "{e}"),
        }
    }
}
