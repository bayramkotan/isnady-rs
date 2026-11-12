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
    let other = doctor::other_command();
    let mut other_code = 0;
    println!("{}", doctor::as_text(&report, &lang));
    let fix_other = |code: &mut i32| {
        if let Some(exe) = &other {
            println!("\n— {}: {} —", doctor::OTHER_NAME, exe.display());
            let mut args = vec!["doctor", "--fix"];
            if a.yes {
                args.push("--yes");
            }
            if a.offline {
                args.push("--offline");
            }
            *code = doctor::other_run(&args, false).0;
        }
    };
    if let (Some(exe), false) = (&other, a.fix) {
        // the other isnady's copies too (TEK PROJE)
        let args: Vec<&str> = if a.offline {
            vec!["doctor", "--offline"]
        } else {
            vec!["doctor"]
        };
        let (code, out) = doctor::other_run(&args, true);
        other_code = code;
        println!("\n— {}: {} —\n{out}", doctor::OTHER_NAME, exe.display());
    }
    if !a.fix || report.fixes.is_empty() {
        if a.fix {
            fix_other(&mut other_code);
        }
        return ExitCode::from(if report.problems.is_empty() && other_code == 0 {
            0
        } else {
            1
        });
    }
    println!();
    run(&report.fixes, a.yes);
    let after = doctor::diagnose(!a.offline);
    println!("\nAfter updating:\n{}", doctor::as_text(&after, &lang));
    fix_other(&mut other_code);
    ExitCode::from(if after.problems.is_empty() && other_code == 0 {
        0
    } else {
        1
    })
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
        if let Some(exe) = doctor::other_command() {
            // the other isnady too (TEK PROJE)
            let (_code, out) = doctor::other_run(&["update", "--check"], true);
            println!("\n— {}: {} —\n{out}", doctor::OTHER_NAME, exe.display());
        }
        return ExitCode::SUCCESS;
    }
    let mut other_code = 0;
    if let Some(exe) = doctor::other_command() {
        // the other isnady first: it is not this program, so nothing stands in its way
        println!("— {}: {} —", doctor::OTHER_NAME, exe.display());
        let args: Vec<&str> = if a.yes {
            vec!["update", "--yes"]
        } else {
            vec!["update"]
        };
        other_code = doctor::other_run(&args, false).0;
        println!();
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
        return ExitCode::from(if other_code == 0 { 0 } else { 1 });
    }
    run(&report.fixes, a.yes);
    let after = doctor::diagnose(true);
    if after.problems.is_empty() && other_code == 0 {
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
