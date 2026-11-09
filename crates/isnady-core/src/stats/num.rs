//! The small numerics of the statistics, as the Python isnady computes them (pure Python, no numpy): its sums (Python's
//! `sum()` of floats adds with Neumaier's compensation), its `math.lgamma` (CPython's own Lanczos sum), `math.erfc`
//! (the C library's), the χ², normal and t tails, the linear solver, Holm and Benjamini–Hochberg, ranks with ties,
//! Pearson and Spearman. Written step for step as there, so the same data gives the same figures.

/// Python's `sum()` of floats (CPython 3.12+: Neumaier's compensated sum).
pub fn fsum<I: IntoIterator<Item = f64>>(values: I) -> f64 {
    let (mut f, mut c) = (0.0f64, 0.0f64);
    for x in values {
        let t = f + x;
        if f.abs() >= x.abs() {
            c += (f - t) + x;
        } else {
            c += (x - t) + f;
        }
        f = t;
    }
    if c != 0.0 && c.is_finite() {
        f += c;
    }
    f
}

extern "C" {
    #[link_name = "erfc"]
    fn c_erfc(x: f64) -> f64;
}

/// `math.erfc` (the C library's, as CPython calls it).
pub fn erfc(x: f64) -> f64 {
    // SAFETY: erfc is a pure function of the C math library every platform links
    unsafe { c_erfc(x) }
}

const LANCZOS_G: f64 = 6.024_680_040_776_729_583_740_234_375;
const LANCZOS_NUM: [f64; 13] = [
    23531376880.410759688572007674451636754734846804940,
    42919803642.649098768957899047001988850926355848959,
    35711959237.355668049440185451547166705960488635843,
    17921034426.037209699919755754458931112671403265390,
    6039542586.3520280050642916443072979210699388420708,
    1439720407.3117216736632230727949123939715485786772,
    248874557.86205415651146038641322942321632125127801,
    31426415.585400194380614231628318205362874684987640,
    2876370.6289353724412254090516208496135991145378768,
    186056.26539522349504029498971604569928220784236328,
    8071.6720023658162106380029022722506138218516325024,
    210.82427775157934587250973392071336271166969580291,
    2.5066282746310002701649081771338373386264310793408,
];
const LANCZOS_DEN: [f64; 13] = [
    0.0,
    39916800.0,
    120543840.0,
    150917976.0,
    105258076.0,
    45995730.0,
    13339535.0,
    2637558.0,
    357423.0,
    32670.0,
    1925.0,
    66.0,
    1.0,
];

fn lanczos_sum(x: f64) -> f64 {
    let (mut num, mut den) = (0.0f64, 0.0f64);
    if x < 5.0 {
        for i in (0..13).rev() {
            num = num * x + LANCZOS_NUM[i];
            den = den * x + LANCZOS_DEN[i];
        }
    } else {
        for i in 0..13 {
            num = num / x + LANCZOS_NUM[i];
            den = den / x + LANCZOS_DEN[i];
        }
    }
    num / den
}

fn sinpi(x: f64) -> f64 {
    // CPython's m_sinpi, for the reflection formula (x > 0 here)
    let y = x.abs() % 2.0;
    let n = (2.0 * y).round() as i64;
    let pi = std::f64::consts::PI;
    let r = match n {
        0 => (pi * y).sin(),
        1 => (pi * (y - 0.5)).cos(),
        2 => (pi * (1.0 - y)).sin(),
        3 => -(pi * (y - 1.5)).cos(),
        4 => (pi * (y - 2.0)).sin(),
        _ => 0.0,
    };
    (1.0f64).copysign(x) * r
}

/// `math.lgamma`: CPython's own Lanczos approximation (not the C library's).
pub fn lgamma(x: f64) -> f64 {
    if !x.is_finite() {
        return if x.is_nan() { x } else { f64::INFINITY };
    }
    if x == x.floor() && x <= 2.0 {
        return if x <= 0.0 { f64::INFINITY } else { 0.0 };
    }
    let absx = x.abs();
    if absx < 1e-20 {
        return -absx.ln();
    }
    let mut r = lanczos_sum(absx).ln() - LANCZOS_G;
    r += (absx - 0.5) * ((absx + LANCZOS_G - 0.5).ln() - 1.0);
    if x < 0.0 {
        let logpi = 1.144_729_885_849_400_174_143_427_351_353_058_711_647;
        r = logpi - sinpi(absx).abs().ln() - absx.ln() - r;
    }
    r
}

pub fn sigmoid(z: f64) -> f64 {
    if z >= 0.0 {
        1.0 / (1.0 + (-z).exp())
    } else {
        let e = z.exp();
        e / (1.0 + e)
    }
}

pub fn normal_sf(z: f64) -> f64 {
    0.5 * erfc(z / 2f64.sqrt())
}

/// P(χ² > x) for df degrees of freedom: the regularized upper incomplete gamma Q(df/2, x/2).
pub fn chi2_sf(x: f64, df: f64) -> f64 {
    if x <= 0.0 {
        return 1.0;
    }
    let (a, x) = (df / 2.0, x / 2.0);
    if x < a + 1.0 {
        let mut term = 1.0 / a;
        let mut total = term;
        let mut n = a;
        for _ in 0..500 {
            n += 1.0;
            term *= x / n;
            total += term;
            if term.abs() < total.abs() * 1e-12 {
                break;
            }
        }
        return (1.0 - total * (-x + a * x.ln() - lgamma(a)).exp()).max(0.0);
    }
    let mut b = x + 1.0 - a;
    let mut c = 1.0 / 1e-300;
    let mut d = 1.0 / b;
    let mut h = d;
    for i in 1..500 {
        let i = i as f64;
        let an = -i * (i - a);
        b += 2.0;
        d = an * d + b;
        if d.abs() < 1e-300 {
            d = 1e-300;
        }
        c = b + an / c;
        if c.abs() < 1e-300 {
            c = 1e-300;
        }
        d = 1.0 / d;
        let delta = d * c;
        h *= delta;
        if (delta - 1.0).abs() < 1e-12 {
            break;
        }
    }
    ((-x + a * x.ln() - lgamma(a)).exp() * h).min(1.0)
}

fn betacf(a: f64, b: f64, x: f64) -> f64 {
    let tiny = 1e-300;
    let (qab, qap, qam) = (a + b, a + 1.0, a - 1.0);
    let mut c = 1.0;
    let mut d = 1.0 - qab * x / qap;
    if d.abs() < tiny {
        d = tiny;
    }
    d = 1.0 / d;
    let mut h = d;
    for m in 1..300 {
        let m = m as f64;
        let m2 = 2.0 * m;
        let aa = m * (b - m) * x / ((qam + m2) * (a + m2));
        d = 1.0 + aa * d;
        if d.abs() < tiny {
            d = tiny;
        }
        c = 1.0 + aa / c;
        if c.abs() < tiny {
            c = tiny;
        }
        d = 1.0 / d;
        h *= d * c;
        let aa = -(a + m) * (qab + m) * x / ((a + m2) * (qap + m2));
        d = 1.0 + aa * d;
        if d.abs() < tiny {
            d = tiny;
        }
        c = 1.0 + aa / c;
        if c.abs() < tiny {
            c = tiny;
        }
        d = 1.0 / d;
        let delta = d * c;
        h *= delta;
        if (delta - 1.0).abs() < 1e-12 {
            break;
        }
    }
    h
}

/// The regularized incomplete beta I_x(a, b).
pub fn betainc(a: f64, b: f64, x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    let front = (lgamma(a + b) - lgamma(a) - lgamma(b) + a * x.ln() + b * (1.0 - x).ln()).exp();
    if x < (a + 1.0) / (a + b + 2.0) {
        return front * betacf(a, b, x) / a;
    }
    1.0 - front * betacf(b, a, 1.0 - x) / b
}

pub fn t_two_sided(t: f64, df: f64) -> f64 {
    if df <= 0.0 {
        return 1.0;
    }
    betainc(df / 2.0, 0.5, df / (df + t * t))
}

/// Holm's step-down adjustment.
pub fn holm(p: &[f64]) -> Vec<f64> {
    let mut order: Vec<usize> = (0..p.len()).collect();
    order.sort_by(|&a, &b| p[a].partial_cmp(&p[b]).unwrap_or(std::cmp::Ordering::Equal));
    let mut out = vec![0.0; p.len()];
    let mut running = 0.0f64;
    for (rank, &i) in order.iter().enumerate() {
        running = running.max(((p.len() - rank) as f64 * p[i]).min(1.0));
        out[i] = running;
    }
    out
}

/// Benjamini–Hochberg q-values.
pub fn bh(p: &[f64]) -> Vec<f64> {
    let m = p.len();
    let mut order: Vec<usize> = (0..m).collect();
    order.sort_by(|&a, &b| p[a].partial_cmp(&p[b]).unwrap_or(std::cmp::Ordering::Equal));
    let mut q = vec![0.0; m];
    let mut running = 1.0f64;
    for rank in (0..m).rev() {
        let i = order[rank];
        running = running.min(p[i] * m as f64 / (rank + 1) as f64);
        q[i] = running;
    }
    q
}

/// a x = b by Gauss–Jordan with partial pivoting; None when a is singular.
pub fn solve(a: &[Vec<f64>], b: &[f64]) -> Option<Vec<f64>> {
    let n = b.len();
    let mut m: Vec<Vec<f64>> = a
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let mut r = row.clone();
            r.push(b[i]);
            r
        })
        .collect();
    for col in 0..n {
        let mut piv = col;
        for r in col + 1..n {
            if m[r][col].abs() > m[piv][col].abs() {
                piv = r;
            }
        }
        if m[piv][col].abs() < 1e-12 {
            return None;
        }
        m.swap(col, piv);
        let p = m[col][col];
        m[col] = m[col].iter().map(|v| v / p).collect();
        for r in 0..n {
            if r != col && m[r][col] != 0.0 {
                let f = m[r][col];
                let pivot = m[col].clone();
                m[r] = m[r].iter().zip(&pivot).map(|(v, w)| v - f * w).collect();
            }
        }
    }
    Some((0..n).map(|i| m[i][n]).collect())
}

pub fn inverse(a: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let n = a.len();
    let mut cols = Vec::with_capacity(n);
    for j in 0..n {
        let mut e = vec![0.0; n];
        e[j] = 1.0;
        cols.push(solve(a, &e)?);
    }
    Some(
        (0..n)
            .map(|i| (0..n).map(|j| cols[j][i]).collect())
            .collect(),
    )
}

/// Ranks 1..n, ties given their average rank.
pub fn rankdata(values: &[f64]) -> Vec<f64> {
    let mut order: Vec<usize> = (0..values.len()).collect();
    order.sort_by(|&a, &b| {
        values[a]
            .partial_cmp(&values[b])
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut ranks = vec![0.0; values.len()];
    let mut i = 0;
    while i < order.len() {
        let mut j = i;
        while j + 1 < order.len() && values[order[j + 1]] == values[order[i]] {
            j += 1;
        }
        let avg = (i + j) as f64 / 2.0 + 1.0;
        for k in i..=j {
            ranks[order[k]] = avg;
        }
        i = j + 1;
    }
    ranks
}

pub fn pearson(x: &[f64], y: &[f64]) -> Option<f64> {
    let n = x.len();
    if n < 3 {
        return None;
    }
    let mx = fsum(x.iter().copied()) / n as f64;
    let my = fsum(y.iter().copied()) / n as f64;
    let sxy = fsum(x.iter().zip(y).map(|(a, b)| (a - mx) * (b - my)));
    let sxx = fsum(x.iter().map(|a| (a - mx) * (a - mx)));
    let syy = fsum(y.iter().map(|b| (b - my) * (b - my)));
    if sxx != 0.0 && syy != 0.0 {
        Some(sxy / (sxx * syy).sqrt())
    } else {
        None
    }
}

/// `int(x)` of a non-negative float (truncation).
pub fn trunc(x: f64) -> usize {
    x as usize
}

/// Python's `round(x)` to an integer (halves to even).
pub fn round_int(x: f64) -> i64 {
    crate::py::round_even(x) as i64
}

/// Python's `round(x, n)` (the nearest float to the decimal rounded to n places).
pub fn round_to(x: f64, n: usize) -> f64 {
    format!("{x:.n$}").parse().unwrap_or(x)
}

/// The distinct values of a column, as Python's set of numbers sees them (1 and 1.0 the same).
pub fn distinct(values: impl Iterator<Item = f64>) -> usize {
    let mut seen: Vec<u64> = values
        .map(|v| {
            if v == 0.0 {
                0f64.to_bits()
            } else {
                v.to_bits()
            }
        })
        .collect();
    seen.sort_unstable();
    seen.dedup();
    seen.len()
}
