//! iy-rs ai build|status and iy-rs tahric (YZ1, YZ2, A4): the meaning index and the other narrations of a hadith, as
//! the Python isnady's `cli.cmd_ai` and `cmd_tahric`.

use clap::Args;
use std::process::ExitCode;

use isnady_core::py::thousands;
use isnady_core::{isnad, semantic, takhrij};

#[derive(Args)]
pub struct AiArgs {
    #[arg(value_parser = ["build", "status"])]
    action: String,
    /// build: number of concepts (default 200)
    #[arg(long, default_value_t = 200)]
    dims: usize,
}

#[derive(Args)]
pub struct TahricArgs {
    book: String,
    number: String,
}

/// A Python float as print() writes it: 12.3, 5.0.
fn py_float(v: &serde_json::Value) -> String {
    match v.as_f64() {
        Some(f) if f.fract() == 0.0 => format!("{f:.1}"),
        Some(f) => format!("{f}"),
        None => v.to_string(),
    }
}

fn n(v: &serde_json::Value) -> String {
    thousands(v.as_i64().unwrap_or(0))
}

pub fn ai_cmd(a: AiArgs) -> ExitCode {
    let conn = match isnady_core::sources::store::connect_rw(None) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(1);
        }
    };
    if a.action == "build" {
        let say = |m: &str| println!("  {m}");
        let meta = match semantic::build(&conn, a.dims, &say) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::from(2);
            }
        };
        println!(
            "Meaning index in {} s: {} hadith, {} words, {} concepts.",
            py_float(&meta["seconds"]),
            n(&meta["documents"]),
            n(&meta["terms"]),
            meta["dims"]
        );
        let found = match takhrij::build(&conn, &say) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::from(2);
            }
        };
        println!(
            "Takhrij in {} s: {} pairs of the same hadith, {} probably the same report.",
            py_float(&found["seconds"]),
            n(&found["same"]),
            n(&found["same_report"])
        );
        isnady_core::cache::clear();
        return ExitCode::SUCCESS;
    }
    let st = semantic::status(&conn);
    if st["built"] != serde_json::json!(true) {
        println!("Meaning index: not built yet. Build it with: iy ai build");
        return ExitCode::SUCCESS;
    }
    println!(
        "Meaning index: {} hadith, {} words, {} concepts, built {} ({}){}",
        n(&st["documents"]),
        n(&st["terms"]),
        st["dims"],
        st["built_at"].as_str().unwrap_or(""),
        st["method"].as_str().unwrap_or(""),
        if st["stale"] == serde_json::json!(true) {
            "; OLDER than the data: iy ai build"
        } else {
            ""
        }
    );
    let (_pairs, same, report) = takhrij::status(&conn);
    println!(
        "Takhrij: {} pairs of the same hadith, {} probably the same report",
        thousands(same),
        thousands(report)
    );
    ExitCode::SUCCESS
}

pub fn tahric_cmd(a: TahricArgs) -> ExitCode {
    let conn = match crate::open() {
        Ok(c) => c,
        Err(code) => return code,
    };
    let Some(hid) = isnad::hadith_id(&conn, &a.book, &a.number) else {
        match isnad::number_range(&conn, &a.book).ok().flatten() {
            Some(rng) => eprintln!(
                "No hadith {} in '{}'. Its numbers run from {} to {}; the nearest is {}.",
                a.number,
                a.book,
                rng.first,
                rng.last,
                isnad::nearest_number(&conn, &a.book, &a.number).unwrap_or_else(|| "None".into())
            ),
            None => eprintln!("No hadith {} in '{}'. See: iy book", a.number, a.book),
        }
        return ExitCode::from(1);
    };
    let found = takhrij::related(&conn, hid, &isnady_core::i18n::ui_language());
    if found.is_empty() {
        println!("No other narration found (or not built yet: iy ai build).");
        return ExitCode::SUCCESS;
    }
    println!("Other narrations of {} {}:", a.book, a.number);
    for r in found {
        let level = if r.kind == "same" {
            "same text"
        } else {
            "probably the same report (same Companion)"
        };
        println!(
            "  {:22} {:>6}   overlap {:.2}   {level}",
            r.book_name, r.number, r.score
        );
    }
    ExitCode::SUCCESS
}
