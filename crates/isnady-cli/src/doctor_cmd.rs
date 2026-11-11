//! iy-rs doctor and iy-rs update (A5): every copy of isnady-rs, which one each command starts, what is out of date,
//! and bringing each copy up to date where it is — as the Python isnady's `cli.cmd_doctor` and `cmd_update`.

use clap::Args;
use std::io::Write;
use std::process::ExitCode;

use isnady_core::doctor;

#[derive(Args)]
pub struct DoctorArgs {
    /// Update the copies that are behind, where they are
    #[arg(long)]
    fix: bool,
    /// With --fix: do not ask
    #[arg(long)]
    yes: bool,
    /// Do not ask crates.io for the latest version
    #[arg(long)]
    offline: bool,
}

#[derive(Args)]
pub struct UpdateArgs {
    /// Do not ask before running cargo
    #[arg(long)]
    yes: bool,
    /// Only look: the installed and the latest version on crates.io, and what is new
    #[arg(long)]
    check: bool,
}

fn ask(command: &str) -> bool {
    print!("Run this?\n  {command}\n[y/N] ");
    let _ = std::io::stdout().flush();
    let mut answer = String::new();
    let _ = std::io::stdin().read_line(&mut answer);
    ["y", "yes", "e", "evet"].contains(&answer.trim().to_lowercase().as_str())
}

fn run(plan: &[Vec<String>], yes: bool) {
    doctor::step_aside();
    doctor::run_plan(plan, yes, &ask, &|m| println!("{m}"));
}

pub fn doctor_cmd(a: DoctorArgs) -> ExitCode {
    let lang = isnady_core::i18n::ui_language();
    let report = doctor::diagnose(!a.offline);
    println!("{}", doctor::as_text(&report, &lang));
    if !a.fix || report.fixes.is_empty() {
        return ExitCode::from(if report.problems.is_empty() { 0 } else { 1 });
    }
    println!();
    run(&report.fixes, a.yes);
    let after = doctor::diagnose(!a.offline);
    println!("\nAfter updating:\n{}", doctor::as_text(&after, &lang));
    ExitCode::from(if after.problems.is_empty() { 0 } else { 1 })
}

pub fn update_cmd(a: UpdateArgs) -> ExitCode {
    let latest = doctor::latest_on_crates("isnady");
    let behind = latest
        .as_deref()
        .is_some_and(|l| doctor::vkey(l) > doctor::vkey(doctor::VERSION));
    println!(
        "crates.io: isnady {}{}",
        doctor::VERSION,
        match &latest {
            Some(l) if behind => format!(" → {l} available"),
            Some(_) => " (up to date)".to_string(),
            None => " (crates.io could not be reached)".to_string(),
        }
    );
    if behind {
        for (version, text) in doctor::whats_new(doctor::VERSION, 6) {
            println!("\nWhat is new in {version}:\n{text}");
        }
    }
    if a.check {
        return ExitCode::SUCCESS;
    }
    let report = doctor::diagnose(true);
    if report.fixes.is_empty() {
        println!(
            "isnady {} is up to date{}",
            report.running_version,
            match &report.latest {
                Some(l) => format!(" (latest on crates.io: {l})."),
                None => " (crates.io could not be reached).".to_string(),
            }
        );
        return ExitCode::SUCCESS;
    }
    run(&report.fixes, a.yes);
    let after = doctor::diagnose(true);
    if after.problems.is_empty() {
        println!("\nEvery copy is up to date.");
        ExitCode::SUCCESS
    } else {
        println!(
            "\n{}",
            doctor::as_text(&after, &isnady_core::i18n::ui_language())
        );
        ExitCode::from(1)
    }
}
