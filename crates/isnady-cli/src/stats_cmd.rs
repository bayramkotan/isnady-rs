//! iy-rs stats (`cli.cmd_stats` and its parts): what the database holds; the corpus, the graders, the hadith model
//! and the laboratory, printed as the Python iy prints them; --csv writes every table behind them; --explain says
//! every measure in plain words.

use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;

use isnady_core::stats::Value;
use isnady_core::stats::{self, corpus, graders, lab, models, words};
use isnady_core::{books, i18n, paths, py, scholars};

#[derive(clap::Args)]
pub struct StatsArgs {
    /// corpus: everything below; narrators: reliability, concentration, pillars; chains: length, weakest link, time
    /// gaps; grades: weakest narrator and length against the grade; books: the books compared; model: what each
    /// scholar's grade follows (ordinal regression) and the hadith graded higher or lower than their chain; lab:
    /// correlations (Spearman, partial, BH) and tests (Stuart–Maxwell, Kruskal–Wallis, Mann–Whitney); graders:
    /// agreement, model, strictness, disputed hadith
    #[arg(value_parser = ["corpus", "narrators", "chains", "grades", "books", "model", "lab", "graders"])]
    topic: Option<String>,
    /// with 'model': only the scholars whose name contains this
    #[arg(long)]
    grader: Option<String>,
    /// also write every table behind the statistics as CSV files into DIR
    #[arg(long, value_name = "DIR")]
    csv: Option<String>,
    /// collection key, e.g. abudawud; @shia for all Shia books; @all for both roads, one after the other (default:
    /// all Sunni books; for graders, the book with most graders)
    #[arg(long)]
    book: Option<String>,
    /// compute again instead of reading the saved results
    #[arg(long)]
    recompute: bool,
    /// rows to list (disputed hadith, strongest relations)
    #[arg(long, default_value_t = 10)]
    limit: usize,
    /// every measure in plain words (what it measures, how to read it, what it means for hadith science, an
    /// example); or one, e.g. gini, kendall, kappa
    #[arg(long, value_name = "TERM", num_args = 0..=1, default_missing_value = "all")]
    explain: Option<String>,
}

fn progress(m: &str) {
    eprintln!("  … {m}");
}

fn s(v: &Value) -> &str {
    v.as_str().unwrap_or("")
}

fn n(v: &Value) -> f64 {
    v.as_f64().unwrap_or(f64::NAN)
}

fn i(v: &Value) -> i64 {
    v.as_i64().unwrap_or(0)
}

fn th(v: &Value) -> String {
    py::thousands(i(v))
}

/// What Python prints for a value with `str()` / an f-string without a format.
fn plain(v: &Value) -> String {
    match v {
        Value::Null => "None".into(),
        Value::Bool(b) => if *b { "True" } else { "False" }.into(),
        Value::Number(x) => {
            if let Some(k) = x.as_i64() {
                k.to_string()
            } else {
                py::float_repr(x.as_f64().unwrap_or(0.0))
            }
        }
        Value::String(t) => t.clone(),
        other => other.to_string(),
    }
}

fn cut(text: &str, k: usize) -> String {
    text.chars().take(k).collect()
}

pub fn stats_cmd(a: StatsArgs) -> ExitCode {
    if let Some(which) = &a.explain {
        return explain(which);
    }
    let conn = match crate::open() {
        Ok(c) => c,
        Err(code) => return code,
    };
    let out = match a.topic.as_deref() {
        Some("graders") => graders_cmd(&conn, &a),
        Some("model") => model_cmd(&conn, &a),
        Some("lab") => lab_cmd(&conn, &a),
        Some(t) => {
            let topic = if t == "corpus" { None } else { Some(t) };
            corpus_cmd(&conn, &a, topic, a.book.as_deref(), a.csv.clone())
        }
        None => summary(&conn),
    };
    match out {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(1)
        }
    }
}

fn explain(which: &str) -> ExitCode {
    let chosen: Vec<words::Term> = if which.is_empty() || which == "all" {
        words::all_terms()
    } else {
        words::find(which).into_iter().collect()
    };
    if chosen.is_empty() {
        let known: Vec<String> = words::all_terms().into_iter().map(|t| t.name).collect();
        eprintln!("No measure '{which}'. Known: {}", known.join(", "));
        return ExitCode::from(1);
    }
    for t in chosen {
        println!(
            "\n{}\n  What it measures: {}\n  How to read it:   {}\n  Hadith science:   {}\n  Example:          {}",
            t.name, t.what, t.read, t.hadith, t.example
        );
    }
    ExitCode::SUCCESS
}

/// iy-rs stats: what the database holds, book by book.
fn summary(conn: &isnady_core::db::Connection) -> Result<ExitCode, String> {
    let e = |x: isnady_core::db::Error| x.to_string();
    let order = books::order_sql(conn, "c.key").map_err(e)?;
    let mut stmt = conn
        .prepare(&format!(
            "SELECT c.id, c.name,
                  (SELECT COUNT(*) FROM hadiths h WHERE h.collection_id = c.id) AS hadiths,
                  (SELECT COUNT(DISTINCT g.hadith_id) FROM grades g JOIN hadiths h ON h.id = g.hadith_id
                   WHERE h.collection_id = c.id) AS graded,
                  (SELECT COUNT(*) FROM grades g JOIN hadiths h ON h.id = g.hadith_id
                   WHERE h.collection_id = c.id) AS grades
           FROM collections c ORDER BY {order}"
        ))
        .map_err(e)?;
    let rows: Vec<(i64, String, i64, i64, i64)> = stmt
        .query_map([], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })
        .map_err(e)?
        .collect::<Result<_, _>>()
        .map_err(e)?;
    println!("Database: {}", paths::db_path().display());
    for (id, name, hadiths, graded, grades) in rows {
        println!("  {name}: {hadiths} hadith, {graded} graded ({grades} grades)");
        let mut es = conn
            .prepare("SELECT key, language, text_count FROM editions WHERE collection_id = ? ORDER BY language, key")
            .map_err(e)?;
        let eds: Vec<(String, String, i64)> = es
            .query_map([id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .map_err(e)?
            .collect::<Result<_, _>>()
            .map_err(e)?;
        for (key, language, count) in eds {
            println!("      {key:22} {language:11} {count:>6} texts");
        }
    }
    let t: (i64, i64, i64, i64, i64) = conn
        .query_row(
            "SELECT (SELECT COUNT(*) FROM hadiths), (SELECT COUNT(*) FROM texts), (SELECT COUNT(*) FROM grades),
             (SELECT COUNT(*) FROM persons), (SELECT COUNT(*) FROM isnads)",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .map_err(e)?;
    println!(
        "Total: {} hadith, {} texts, {} grades, {} persons, {} isnads",
        t.0, t.1, t.2, t.3, t.4
    );
    Ok(ExitCode::SUCCESS)
}

fn short_of(g: &str) -> String {
    scholars::all()
        .iter()
        .find(|s| s.grader_names.iter().any(|x| x == g))
        .map(|s| {
            s.short
                .clone()
                .filter(|x| !x.is_empty())
                .unwrap_or_else(|| s.name.clone())
        })
        .unwrap_or_else(|| g.to_string())
}

fn source(r: &Value) -> String {
    match r.get("from_file").and_then(Value::as_str) {
        Some(p) if !p.is_empty() => format!("  (from {p})"),
        _ => "  (computed and saved)".to_string(),
    }
}

fn graders_cmd(conn: &isnady_core::db::Connection, a: &StatsArgs) -> Result<ExitCode, String> {
    let books = graders::books_with_grades(conn).map_err(|e| e.to_string())?;
    if books.is_empty() {
        println!("No grades imported.");
        return Ok(ExitCode::SUCCESS);
    }
    let key = a
        .book
        .clone()
        .filter(|b| !b.is_empty())
        .unwrap_or_else(|| books[0].0.clone());
    let r =
        graders::cached(conn, &key, &progress, a.recompute, false).map_err(|e| e.to_string())?;
    let labels: Vec<String> = match r.get("labels").and_then(Value::as_array) {
        Some(l) if !l.is_empty() => l.iter().map(|x| s(x).to_string()).collect(),
        _ => graders::labels().iter().map(|x| x.to_string()).collect(),
    };
    let mut order: Vec<usize> = match r.get("categories").and_then(Value::as_array) {
        Some(c) if !c.is_empty() => c.iter().map(|x| i(x) as usize).collect(),
        _ => (0..graders::K).collect(),
    };
    order.reverse();
    let names = |v: &Value| -> Vec<String> {
        v.as_array()
            .into_iter()
            .flatten()
            .map(|x| s(x).to_string())
            .collect()
    };
    let list = names(&r["graders"]);
    println!("{key}: {} graders measured{}", list.len(), source(&r));
    if r.get("tradition").and_then(Value::as_str) == Some("shia") {
        println!("  road B: the four Imami categories (sahih, hasan, muwaththaq, da'if), never the Sunni grade groups");
    }
    if let Some(few) = r.get("too_few").and_then(Value::as_object) {
        for (g, k) in few {
            println!(
                "  {}: {} grade(s) of this book — named, not measured",
                short_of(g),
                plain(k)
            );
        }
    }
    for x in r
        .get("selections")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let gi = i(&x["grade"]) as usize;
        let grade = &labels[gi];
        println!(
            "  {}: a selection — all {} hadith he names are {grade}",
            short_of(s(&x["grader"])),
            th(&x["count"])
        );
        for (o, y) in x["others"].as_object().into_iter().flatten() {
            let dist: Vec<String> = order
                .iter()
                .map(|k| format!("{} {}", labels[*k], th(&y["distribution"][*k])))
                .collect();
            let common = i(&y["common"]);
            let same = if common != 0 {
                i(&y["distribution"][gi]) as f64 / common as f64
            } else {
                0.0
            };
            println!(
                "    {} on those {}: {}  ({:.1}% {grade})",
                short_of(o),
                th(&y["common"]),
                dist.join(" · "),
                100.0 * same
            );
        }
    }
    if list.len() < 2 {
        println!("  fewer than two graders who give more than one grade: no agreement or model");
        return Ok(ExitCode::SUCCESS);
    }
    let al = &r["alpha"];
    println!(
        "Krippendorff's alpha (ordinal) {:.3}  95% {:.3}–{:.3}  over {} hadith",
        n(&al["value"]),
        n(&al["ci"][0]),
        n(&al["ci"][1]),
        th(&al["units"])
    );
    println!(
        "  → in plain words: {}  (iy stats --explain alpha)",
        words::alpha(n(&al["value"]))
    );
    for ov in r
        .get("one_voice")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        println!(
            "  ! {} and {} agree {:.1}%: one voice in the model",
            short_of(s(&ov["kept"])),
            short_of(s(&ov["left_out"])),
            100.0 * n(&ov["agree"])
        );
    }
    println!("Pairs: same grade · Cohen's kappa · ordinal kappa (95% intervals)");
    for (k, p) in r["pairs"].as_object().into_iter().flatten() {
        let (ga, gb) = k.split_once('|').unwrap_or((k, ""));
        let ci = &p["ci"];
        println!(
            "  {} – {} (n {}): {:.1}% [{:.1}–{:.1}] · κ {:.2} [{:.2}–{:.2}] · κw {:.2} [{:.2}–{:.2}]",
            short_of(ga),
            short_of(gb),
            th(&p["common"]),
            100.0 * n(&p["agree"]),
            100.0 * n(&ci["agree"][0]),
            100.0 * n(&ci["agree"][1]),
            n(&p["kappa"]),
            n(&ci["kappa"][0]),
            n(&ci["kappa"][1]),
            n(&p["wkappa"]),
            n(&ci["wkappa"][0]),
            n(&ci["wkappa"][1])
        );
    }
    println!(
        "Strictness from the model: P(below the true grade) − P(above it); negative = stricter"
    );
    let mut strict: Vec<(&String, &Value)> = r["model"]["strictness"]
        .as_object()
        .into_iter()
        .flatten()
        .collect();
    strict.sort_by(|x, y| {
        n(&x.1["value"])
            .partial_cmp(&n(&y.1["value"]))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for (g, st) in strict {
        let (lo, hi) = (n(&st["ci"][0]), n(&st["ci"][1]));
        let word = if hi < 0.0 {
            "stricter (mutashaddid)"
        } else if lo > 0.0 {
            "more lenient (mutasahil)"
        } else {
            "neither, within chance"
        };
        println!(
            "  {:<40} {:+.3}  [{lo:+.3}, {hi:+.3}]  {word}",
            short_of(g),
            n(&st["value"])
        );
    }
    let c = &r["certainty"];
    println!(
        "The model is sure (≥95%) of {} of {} hadith, unsure (<60%) of {}; {} disputed",
        th(&c["high"]),
        th(&c["total"]),
        th(&c["low"]),
        th(&r["disputed_total"])
    );
    for d in r["disputed"].as_array().into_iter().flatten().take(a.limit) {
        let grades: Vec<String> = list
            .iter()
            .map(|g| {
                let v = &d["grades"][g.as_str()];
                let shown = if v.is_null() {
                    "not graded".to_string()
                } else {
                    labels[i(v) as usize].clone()
                };
                format!("{}: {shown}", short_of(g))
            })
            .collect();
        println!(
            "  #{}: model {} ({:.0}%) — {}",
            plain(&d["number"]),
            labels[i(&d["consensus"]) as usize],
            100.0 * n(&d["certainty"]),
            grades.join(", ")
        );
    }
    Ok(ExitCode::SUCCESS)
}

fn pct(x: i64, total: i64) -> String {
    if total != 0 {
        format!("{:.1}%", 100.0 * x as f64 / total as f64)
    } else {
        "—".into()
    }
}

/// Path(folder) as Python prints it: no "./", no trailing slash, no doubled slash.
fn py_path(p: &str) -> String {
    let mut out = PathBuf::new();
    for c in Path::new(p).components() {
        if c != Component::CurDir {
            out.push(c.as_os_str());
        }
    }
    let shown = out.display().to_string();
    if shown.is_empty() {
        ".".into()
    } else {
        shown
    }
}

fn corpus_cmd(
    conn: &isnady_core::db::Connection,
    a: &StatsArgs,
    topic: Option<&str>,
    book: Option<&str>,
    csv: Option<String>,
) -> Result<ExitCode, String> {
    if book == Some(stats::ALL_SCOPE) {
        let mut code = ExitCode::SUCCESS;
        for (k, road) in [None, Some(stats::SHIA_SCOPE)].into_iter().enumerate() {
            if k > 0 {
                println!(
                    "\n{}\nThe Shia road, measured apart on its own scale (never together with the Sunni books)\n",
                    "=".repeat(72)
                );
            }
            let sub = csv.as_ref().map(|c| {
                Path::new(c)
                    .join(if road.is_none() { "sunni" } else { "shia" })
                    .display()
                    .to_string()
            });
            let one = corpus_cmd(conn, a, topic, road, sub)?;
            if one != ExitCode::SUCCESS {
                code = one;
            }
        }
        return Ok(code);
    }
    let book = book.filter(|b| !b.is_empty());
    let r = corpus::cached(conn, book, &progress, a.recompute, false).map_err(|e| e.to_string())?;
    if r.get("empty").and_then(Value::as_bool) == Some(true) {
        println!("Nothing to measure: import a hadith collection first.");
        return Ok(ExitCode::SUCCESS);
    }
    let groups: Vec<String> = r["groups"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|g| s(&g[0]).to_string())
        .collect();
    let o = &r["overview"];
    let shia = r.get("tradition").and_then(Value::as_str) == Some("shia");
    let scope = match book {
        None => "All Sunni books".to_string(),
        Some(stats::SHIA_SCOPE) => "All Shia books".to_string(),
        Some(b) => b.to_string(),
    };
    println!(
        "{scope}: {} hadith, {} chains, {} of {} names identified ({}), {} narrators{}",
        th(&o["hadith"]),
        th(&o["chains"]),
        th(&o["identified"]),
        th(&o["links"]),
        pct(i(&o["identified"]), i(&o["links"])),
        th(&o["narrators"]),
        source(&r)
    );
    let shares = |row: &Value| -> String {
        let vals: Vec<i64> = row.as_array().into_iter().flatten().map(i).collect();
        let total: i64 = vals.iter().sum();
        groups
            .iter()
            .zip(&vals)
            .map(|(g, v)| format!("{g} {}", pct(*v, total)))
            .collect::<Vec<_>>()
            .join(" · ")
    };
    if topic.is_none() || topic == Some("narrators") {
        let nr = &r["narrators"];
        println!(
            "\nReliability ({})",
            if shia {
                "al-Najashi's words in the four Imami categories"
            } else {
                "Ibn Hajar's ranks in six groups"
            }
        );
        for (label, row) in [
            (
                if shia {
                    "all narrators of al-Najashi"
                } else {
                    "all narrators of the Taqrib"
                },
                &nr["rank_all"],
            ),
            ("narrators in these chains", &nr["rank_people"]),
            ("every name in these chains", &nr["rank_links"]),
        ] {
            println!("  {label:30} {}", shares(row));
        }
        let c = &nr["concentration"];
        if !c["gini"].is_null() {
            println!(
                "\nConcentration: busiest 1% carry {:.1}%, busiest 10% {:.1}%; {} of {} narrators carry half; Gini {:.3}",
                100.0 * n(&c["top1"]),
                100.0 * n(&c["top10"]),
                th(&c["half_by"]),
                th(&c["people"]),
                n(&c["gini"])
            );
            println!(
                "  → in plain words: {} — the pivots (madar) of the corpus  (iy stats --explain gini)",
                words::gini(n(&c["gini"]))
            );
        }
        println!("\nThe pillars (times in the chains · hadith · rank)");
        for p in nr["pillars"].as_array().into_iter().flatten().take(a.limit) {
            let rank = match p["rank"].as_i64() {
                Some(k) if k != 0 => k.to_string(),
                _ => "—".into(),
            };
            println!(
                "  {:>6} {:>6}  rank {rank:>2}  {}",
                th(&p["links"]),
                th(&p["hadith"]),
                plain(&p["name"])
            );
        }
    }
    if topic.is_none() || topic == Some("chains") {
        let ch = &r["chains"];
        let lengths: Vec<String> = ch["lengths"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|kv| format!("{}: {}", plain(&kv[0]), th(&kv[1])))
            .collect();
        println!("\nChain length (names: chains): {}", lengths.join(", "));
        println!("Weakest link: {}", shares(&ch["weakest"]));
        if i(&ch["order_ok"]) + i(&ch["order_bad"]) != 0 {
            println!(
                "Generations in order: {} pairs right, {} the student earlier than his teacher",
                th(&ch["order_ok"]),
                th(&ch["order_bad"])
            );
        }
        println!("Pairs too far apart in time (student's death − teacher's), most frequent:");
        for sp in ch["gap_suspects"]
            .as_array()
            .into_iter()
            .flatten()
            .take(a.limit)
        {
            println!(
                "  {:+5} years ×{:<4} {}  ←  {}",
                i(&sp["gap"]),
                i(&sp["times"]),
                plain(&sp["student"]["name"]),
                plain(&sp["teacher"]["name"])
            );
        }
    }
    if topic.is_none() || topic == Some("grades") {
        println!("\nWeakest narrator against the grade (Kendall's tau-b, 95% interval, gamma); then chain length");
        for g in r["grades"].as_array().into_iter().flatten() {
            let (w, ln) = (&g["weakest"], &g["length"]);
            if w.is_null() || w["tau"].is_null() {
                continue;
            }
            let ci = if w["tau_ci"].is_array() {
                format!("[{:.3}, {:.3}]", n(&w["tau_ci"][0]), n(&w["tau_ci"][1]))
            } else {
                String::new()
            };
            let lt = if !ln.is_null() && !ln["tau"].is_null() {
                format!("{:.3}", n(&ln["tau"]))
            } else {
                "—".into()
            };
            println!(
                "  {:<34} {:<22} n {:>5}  tau-b {:.3} {ci}  gamma {:.3}  · length tau-b {lt}",
                cut(s(&g["grader"]), 34),
                cut(s(&g["book"]), 22),
                th(&w["n"]),
                n(&w["tau"]),
                n(&w["gamma"])
            );
            let sure = if w["tau_ci"].is_array() {
                words::sure(w["tau_ci"][0].as_f64(), w["tau_ci"][1].as_f64())
            } else {
                String::new()
            };
            println!(
                "    → in plain words: {}, {}  (iy stats --explain kendall)",
                words::strength(w["tau"].as_f64()),
                if sure.is_empty() {
                    "no interval".to_string()
                } else {
                    sure
                }
            );
        }
    }
    if topic.is_none() || topic == Some("books") {
        println!(
            "\nBooks: chains · median length · identified · {} · narrators",
            if shia {
                "reach a Ma'sum"
            } else {
                "reach the Prophet"
            }
        );
        for b in r["books"]["rows"].as_array().into_iter().flatten() {
            let median = match b["median_length"].as_i64() {
                Some(k) if k != 0 => k.to_string(),
                _ => "—".into(),
            };
            println!(
                "  {:<32} {:>7}  {median:>3}  {:5.1}%  {:5.1}%  {:>6}",
                cut(s(&b["name"]), 32),
                th(&b["chains"]),
                100.0 * n(&b["identified"]),
                100.0 * n(&b["reaching"]),
                th(&b["narrators"])
            );
        }
    }
    if let Some(dir) = csv {
        write_corpus_csv(&r, &dir).map_err(|e| e.to_string())?;
    }
    Ok(ExitCode::SUCCESS)
}

/// A cell as Python's csv module writes it.
pub fn cell(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        other => plain(other),
    }
}

/// One CSV file as Python writes it (UTF-8 with BOM, \r\n, quoted only where needed).
pub fn write_csv(path: &Path, header: &[String], rows: &[Vec<Value>]) -> std::io::Result<()> {
    let quote = |t: &str| -> String {
        if t.contains(',') || t.contains('"') || t.contains('\n') || t.contains('\r') {
            format!("\"{}\"", t.replace('"', "\"\""))
        } else {
            t.to_string()
        }
    };
    let mut out = String::from("\u{feff}");
    out.push_str(
        &header
            .iter()
            .map(|h| quote(h))
            .collect::<Vec<_>>()
            .join(","),
    );
    out.push_str("\r\n");
    for r in rows {
        out.push_str(
            &r.iter()
                .map(|c| quote(&cell(c)))
                .collect::<Vec<_>>()
                .join(","),
        );
        out.push_str("\r\n");
    }
    std::fs::write(path, out)
}

fn strs(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| x.to_string()).collect()
}

fn arr(v: &Value) -> Vec<Value> {
    v.as_array().cloned().unwrap_or_default()
}

fn write_corpus_csv(r: &Value, folder: &str) -> std::io::Result<()> {
    let out = Path::new(folder);
    std::fs::create_dir_all(out)?;
    let groups: Vec<String> = arr(&r["groups"])
        .iter()
        .map(|g| s(&g[0]).to_string())
        .collect();
    let with = |first: &[&str]| -> Vec<String> {
        let mut h = strs(first);
        h.extend(groups.iter().cloned());
        h
    };
    let lead = |label: Value, rest: &Value| -> Vec<Value> {
        let mut row = vec![label];
        row.extend(arr(rest));
        row
    };
    let (nr, ch) = (&r["narrators"], &r["chains"]);
    let w = |name: &str, header: Vec<String>, rows: Vec<Vec<Value>>| {
        write_csv(&out.join(format!("{name}.csv")), &header, &rows)
    };
    w(
        "reliability",
        with(&["series"]),
        vec![
            lead("taqrib".into(), &nr["rank_all"]),
            lead("narrators in chains".into(), &nr["rank_people"]),
            lead("names in chains".into(), &nr["rank_links"]),
        ],
    )?;
    w(
        "pillars",
        strs(&["id", "name", "rank", "tabaqa", "death", "times", "hadith"]),
        arr(&nr["pillars"])
            .iter()
            .map(|p| {
                ["id", "name", "rank", "tabaqa", "death", "links", "hadith"]
                    .iter()
                    .map(|k| p[*k].clone())
                    .collect()
            })
            .collect(),
    )?;
    w(
        "lorenz",
        strs(&["share of narrators", "share of names"]),
        arr(&nr["concentration"]["curve"]).iter().map(arr).collect(),
    )?;
    w(
        "tabaqa",
        with(&["tabaqa"]),
        arr(&nr["tabaqa_groups"])
            .iter()
            .enumerate()
            .map(|(k, row)| lead(Value::from(k as i64 + 1), row))
            .collect(),
    )?;
    w(
        "chain_length",
        strs(&["names", "chains"]),
        arr(&ch["lengths"]).iter().map(arr).collect(),
    )?;
    w(
        "weakest_link",
        with(&["series"]),
        vec![
            lead("chains".into(), &ch["weakest"]),
            lead("fully identified".into(), &ch["weakest_full"]),
        ],
    )?;
    w(
        "time_gaps",
        strs(&["years (bin)", "pairs"]),
        arr(&ch["gaps"]).iter().map(arr).collect(),
    )?;
    w(
        "time_suspects",
        strs(&["student", "teacher", "years", "times"]),
        arr(&ch["gap_suspects"])
            .iter()
            .map(|x| {
                vec![
                    x["student"]["name"].clone(),
                    x["teacher"]["name"].clone(),
                    x["gap"].clone(),
                    x["times"].clone(),
                ]
            })
            .collect(),
    )?;
    let mut rows = vec![];
    for g in arr(&r["grades"]) {
        for kind in ["weakest", "length"] {
            let a = &g[kind];
            if !a.is_null() && !a["tau"].is_null() {
                let mut row = vec![
                    g["grader"].clone(),
                    g["book"].clone(),
                    kind.into(),
                    a["n"].clone(),
                    a["tau"].clone(),
                ];
                if a["tau_ci"].is_array() {
                    row.extend(arr(&a["tau_ci"]));
                } else {
                    row.extend([Value::from(""), Value::from("")]);
                }
                row.push(a["gamma"].clone());
                rows.push(row);
            }
        }
    }
    w(
        "grades_association",
        strs(&[
            "grader", "book", "against", "n", "tau_b", "low", "high", "gamma",
        ]),
        rows,
    )?;
    w(
        "books",
        with(&[
            "book",
            "chains",
            "median length",
            "identified",
            "reach the Prophet",
            "narrators",
        ]),
        arr(&r["books"]["rows"])
            .iter()
            .map(|b| {
                let mut row: Vec<Value> = [
                    "name",
                    "chains",
                    "median_length",
                    "identified",
                    "reaching",
                    "narrators",
                ]
                .iter()
                .map(|k| b[*k].clone())
                .collect();
                row.extend(arr(&b["groups"]));
                row
            })
            .collect(),
    )?;
    println!("\nCSV tables written to {}", py_path(folder));
    Ok(())
}

/// `format(p, '.2g')`.
fn g2(x: f64) -> String {
    py::fmt_g(x, 2)
}

fn inf_or(v: &Value) -> f64 {
    v.as_f64().unwrap_or(f64::INFINITY)
}

fn model_cmd(conn: &isnady_core::db::Connection, a: &StatsArgs) -> Result<ExitCode, String> {
    let book = a.book.as_deref().filter(|b| !b.is_empty());
    let r = models::cached(conn, book, &progress, a.recompute, false).map_err(|e| e.to_string())?;
    let list = arr(&r["models"]);
    if list.is_empty() {
        println!("No model: it needs a graded book whose chains are split into names (at least 100 graded hadith).");
        return Ok(ExitCode::SUCCESS);
    }
    for m in &list {
        if let Some(g) = &a.grader {
            if !s(&m["grader"]).to_lowercase().contains(&g.to_lowercase()) {
                continue;
            }
        }
        let labels: Vec<String> = arr(&m["labels"]).iter().map(|x| s(x).to_string()).collect();
        let mut rev = labels.clone();
        rev.reverse();
        println!(
            "\n{} — {}: {} hadith, grades {}",
            s(&m["grader"]),
            s(&m["book_name"]),
            th(&m["n"]),
            rev.join(" < ")
        );
        let base_label = m
            .get("base_label")
            .and_then(Value::as_str)
            .unwrap_or(&labels[0])
            .to_string();
        println!(
            "  fit: McFadden R² {:.3} · right {:.1}% (always “{base_label}”: {:.1}%) · Somers' D {:.3} · LR χ² {:.1} on {} df",
            n(&m["mcfadden"]),
            100.0 * n(&m["accuracy"]),
            100.0 * n(&m["base_rate"]),
            n(&m["somers_d"]),
            n(&m["lr_all"]["chi2"]),
            plain(&m["lr_all"]["df"])
        );
        println!(
            "  → in plain words: the chain explains {} of the grades and gives {} of the hadith; the rest is what the scholar weighed beyond one chain (iy stats --explain r2)",
            words::r2(n(&m["mcfadden"])),
            words::somers(n(&m["somers_d"]))
        );
        println!("  odds ratio of a WEAKER grade (95% interval), p");
        for c in arr(&m["coefficients"]) {
            println!(
                "    {:<34} {:7.2} [{:.2}–{:.2}]  p {}",
                s(&c["name"]),
                inf_or(&c["or"]),
                inf_or(&c["or_ci"][0]),
                inf_or(&c["or_ci"][1]),
                g2(n(&c["p"]))
            );
        }
        println!("  factors (likelihood-ratio test, Holm-adjusted p)");
        let mut tests = arr(&m["tests"]);
        tests.sort_by(|x, y| {
            n(&y["chi2"])
                .partial_cmp(&n(&x["chi2"]))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        for t in tests {
            println!(
                "    {:<34} χ² {:7.1}  df {}  p {}",
                s(&t["label"]),
                n(&t["chi2"]),
                plain(&t["df"]),
                g2(n(&t["p_holm"]))
            );
        }
        for (side, word) in [("graded_higher", "higher"), ("graded_lower", "lower")] {
            println!(
                "  graded {word} than the chain foretells: {}; the least expected:",
                th(&m[format!("{side}_total").as_str()])
            );
            for d in arr(&m[side]).iter().take(a.limit) {
                println!(
                    "    hadith {:>6}  {:>24}  model: {:<24} p {:.3}  weakest {}",
                    plain(&d["number"]),
                    s(&d["grade"]),
                    s(&d["likely"]),
                    n(&d["p"]),
                    plain(&d["factors"]["weakest"])
                );
            }
        }
    }
    let shared = arr(&r["shared"]);
    if !shared.is_empty() {
        println!("\nSurprises two scholars or more share: {}", shared.len());
        for x in shared.iter().take(a.limit) {
            let gs: Vec<String> = arr(&x["graders"])
                .iter()
                .map(|g| format!("{}: {}", plain(&g[0]), plain(&g[1])))
                .collect();
            println!(
                "  {} {} {}: {}",
                if s(&x["side"]) == "graded_higher" {
                    "▲"
                } else {
                    "▼"
                },
                plain(&x["book"]),
                plain(&x["number"]),
                gs.join("; ")
            );
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// `_p`: "< 0.001", "0.042", or nothing.
fn pv(p: Option<f64>) -> String {
    match p {
        None => String::new(),
        Some(p) if p < 0.001 => "< 0.001".into(),
        Some(p) => format!("{p:.3}"),
    }
}

fn lab_cmd(conn: &isnady_core::db::Connection, a: &StatsArgs) -> Result<ExitCode, String> {
    let book = a.book.as_deref().filter(|b| !b.is_empty());
    let r = lab::cached(conn, book, &progress, a.recompute, false).map_err(|e| e.to_string())?;
    for m in arr(&r["hadith"]) {
        println!(
            "\n{} — {} hadith with a chain split into names (Spearman's rho, BH q)",
            s(&m["book"]),
            th(&m["hadith"])
        );
        println!("  the strongest relations:");
        for (x, y, rho, k, _p, q) in lab::pairs_table(&m, "cells").into_iter().take(a.limit) {
            println!(
                "    {x:<30} {y:<34} {rho:+.2}  n {:>5}  q {}",
                py::thousands(k),
                pv(q)
            );
        }
        println!("  each factor against each grade, the other factors held equal (partial rho):");
        for (outcome, factor, plain_rho, adj, _n, q) in lab::adjusted_table(&m) {
            if let (Some(adj), Some(q)) = (adj, q) {
                if q < lab::ALPHA {
                    println!(
                        "    {outcome:<42} {factor:<24} {:+.2} → {adj:+.2}  q {}",
                        plain_rho.unwrap_or(f64::NAN),
                        pv(Some(q))
                    );
                }
            }
        }
    }
    let gl = arr(&r["graders"]);
    if !gl.is_empty() {
        println!(
            "\nDo two scholars grade alike overall? (Stuart–Maxwell, same hadith, Holm-adjusted)"
        );
        for t in &gl {
            println!(
                "  {} / {}: {} hadith; the first grades higher on {}, lower on {}; χ² {:.1} on {} df, p {}",
                s(&t["a"]),
                s(&t["b"]),
                th(&t["n"]),
                th(&t["a_stronger"]),
                th(&t["a_weaker"]),
                n(&t["chi2"]),
                plain(&t["df"]),
                pv(t["p_holm"].as_f64())
            );
        }
    }
    let br = arr(&r["brant"]);
    if !br.is_empty() {
        println!("\nDoes the hadith model's shape hold? (Brant: the same weight at every step of the grades)");
        for b in &br {
            let o = &b["overall"];
            let used = arr(&b["thresholds"]).len() as i64;
            let of = i(&b["of"]);
            println!(
                "  {} — {}: χ² {:.1} on {} df, p {}{}",
                s(&b["grader"]),
                s(&b["book"]),
                n(&o["chi2"]),
                plain(&o["df"]),
                pv(o["p_holm"].as_f64()),
                if used == of {
                    String::new()
                } else {
                    format!(" ({used} of {of} steps estimable)")
                }
            );
            for fct in arr(&b["factors"]) {
                if n(&fct["p_holm"]) < lab::ALPHA {
                    let steps: Vec<String> = arr(&fct["slopes"])
                        .iter()
                        .map(|x| format!("{:.2}", n(x).exp()))
                        .collect();
                    println!(
                        "    {:<34} odds ratio at each step {}  p {}",
                        s(&fct["name"]),
                        steps.join(" → "),
                        pv(fct["p_holm"].as_f64())
                    );
                }
            }
        }
    }
    if let Some(nr) = r.get("narrators").filter(|v| v.is_object()) {
        println!("\nNarrators — {} in the chains", th(&nr["people"]));
        for (x, y, rho, k, _p, q) in lab::pairs_table(nr, "cells").into_iter().take(a.limit) {
            println!(
                "    {x:<22} {y:<22} {rho:+.2}  n {:>5}  q {}",
                py::thousands(k),
                pv(q)
            );
        }
    }
    let bk = arr(&r["books"]);
    if !bk.is_empty() {
        println!("\nThe books compared (Kruskal–Wallis; pairs: Mann–Whitney, Cliff's delta, Holm)");
        for c in &bk {
            let kw = &c["kruskal"];
            if kw.is_object() {
                println!(
                    "  {}: H {:.1} on {} df, p {}, ε² {:.3}",
                    s(&c["measure"]),
                    n(&kw["H"]),
                    plain(&kw["df"]),
                    pv(kw["p"].as_f64()),
                    n(&kw["epsilon2"])
                );
            }
            for p in arr(&c["pairs"]) {
                println!(
                    "    {} / {}: delta {:+.2}, p {}",
                    s(&p["a"]),
                    s(&p["b"]),
                    n(&p["delta"]),
                    pv(p["p_holm"].as_f64())
                );
            }
        }
    }
    if let Some(dir) = &a.csv {
        let files = write_lab_csv(&r, dir).map_err(|e| e.to_string())?;
        println!("\n{} CSV tables written to {dir}", files.len());
    }
    Ok(ExitCode::SUCCESS)
}

fn opt(x: Option<f64>) -> Value {
    x.map(stats::f).unwrap_or(Value::Null)
}

/// Every table of the laboratory as CSV (`stats_lab.write_csv`); the files written.
pub fn write_lab_csv(r: &Value, folder: &str) -> std::io::Result<Vec<String>> {
    let out = Path::new(folder);
    std::fs::create_dir_all(out)?;
    let mut written = vec![];
    let mut w = |name: &str, header: Vec<String>, rows: Vec<Vec<Value>>| -> std::io::Result<()> {
        let p = out.join(format!("{name}.csv"));
        write_csv(&p, &header, &rows)?;
        written.push(p.display().to_string());
        Ok(())
    };
    let head = ["a", "b", "spearman_rho", "n", "p", "q_bh"];
    let pair_rows = |m: &Value, which: &str, lead: Option<&Value>| -> Vec<Vec<Value>> {
        lab::pairs_table(m, which)
            .into_iter()
            .map(|(x, y, rho, k, p, q)| {
                let mut row = vec![];
                if let Some(l) = lead {
                    row.push(l.clone());
                }
                row.extend([
                    Value::from(x),
                    Value::from(y),
                    stats::f(rho),
                    Value::from(k),
                    opt(p),
                    opt(q),
                ]);
                row
            })
            .collect()
    };
    for m in arr(&r["hadith"]) {
        let name = isnady_core::works::full_name(s(&m["key"]));
        let mut h = strs(&["book"]);
        h.extend(strs(&head));
        w(
            &format!("lab_{name}_hadith"),
            h.clone(),
            pair_rows(&m, "cells", Some(&m["book"])),
        )?;
        w(
            &format!("lab_{name}_hadith_partial"),
            h,
            pair_rows(&m, "partial", Some(&m["book"])),
        )?;
        w(
            &format!("lab_{name}_adjusted"),
            strs(&["outcome", "factor", "rho", "rho_others_equal", "n", "q_bh"]),
            lab::adjusted_table(&m)
                .into_iter()
                .map(|(o, fct, p, a, k, q)| {
                    vec![
                        Value::from(o),
                        Value::from(fct),
                        opt(p),
                        opt(a),
                        k.map(Value::from).unwrap_or(Value::Null),
                        opt(q),
                    ]
                })
                .collect(),
        )?;
    }
    let br = arr(&r["brant"]);
    if !br.is_empty() {
        let mut rows = vec![];
        for b in &br {
            let o = &b["overall"];
            rows.push(vec![
                b["book"].clone(),
                b["grader"].clone(),
                "all factors".into(),
                o["chi2"].clone(),
                o["df"].clone(),
                o["p"].clone(),
                o["p_holm"].clone(),
                "".into(),
            ]);
            for fct in arr(&b["factors"]) {
                let slopes: Vec<String> = arr(&fct["slopes"])
                    .iter()
                    .map(|x| format!("{:.4}", n(x)))
                    .collect();
                rows.push(vec![
                    b["book"].clone(),
                    b["grader"].clone(),
                    fct["name"].clone(),
                    fct["chi2"].clone(),
                    fct["df"].clone(),
                    fct["p"].clone(),
                    fct["p_holm"].clone(),
                    slopes.join(" ").into(),
                ]);
            }
        }
        w(
            "lab_model_brant",
            strs(&[
                "book",
                "scholar",
                "factor",
                "chi2",
                "df",
                "p",
                "p_holm",
                "slope_at_each_threshold",
            ]),
            rows,
        )?;
    }
    if let Some(nr) = r.get("narrators").filter(|v| v.is_object()) {
        w("lab_narrators", strs(&head), pair_rows(nr, "cells", None))?;
        w(
            "lab_narrators_partial",
            strs(&head),
            pair_rows(nr, "partial", None),
        )?;
    }
    let gl = arr(&r["graders"]);
    if !gl.is_empty() {
        w(
            "lab_scholars_stuart_maxwell",
            strs(&[
                "book",
                "a",
                "b",
                "hadith",
                "a_stronger",
                "a_weaker",
                "chi2",
                "df",
                "p",
                "p_holm",
            ]),
            gl.iter()
                .map(|t| {
                    [
                        "book",
                        "a",
                        "b",
                        "n",
                        "a_stronger",
                        "a_weaker",
                        "chi2",
                        "df",
                        "p",
                        "p_holm",
                    ]
                    .iter()
                    .map(|k| t[*k].clone())
                    .collect()
                })
                .collect(),
        )?;
    }
    let bk = arr(&r["books"]);
    if !bk.is_empty() {
        let mut rows = vec![];
        for c in &bk {
            let kw = &c["kruskal"];
            let get = |k: &str| {
                if kw.is_object() {
                    kw[k].clone()
                } else {
                    Value::Null
                }
            };
            rows.push(vec![
                c["measure"].clone(),
                "all".into(),
                "".into(),
                get("H"),
                get("df"),
                get("p"),
                "".into(),
                get("epsilon2"),
            ]);
            for p in arr(&c["pairs"]) {
                rows.push(vec![
                    c["measure"].clone(),
                    p["a"].clone(),
                    p["b"].clone(),
                    p["U"].clone(),
                    "".into(),
                    p["p"].clone(),
                    p["p_holm"].clone(),
                    p["delta"].clone(),
                ]);
            }
        }
        w(
            "lab_books",
            strs(&[
                "measure",
                "a",
                "b",
                "H_or_U",
                "df",
                "p",
                "p_holm",
                "epsilon2_or_cliffs_delta",
            ]),
            rows,
        )?;
    }
    let _ = i18n::ui_language;
    Ok(written)
}
