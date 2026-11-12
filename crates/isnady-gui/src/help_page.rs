//! Help → Check for Updates… and Check Installation… (gui/update_dialog.py, gui/install_dialog.py; A5): the copies of
//! isnady-rs, the newest on crates.io and what is new, and Update, which brings every copy up to date where it is.

use serde_json::{json, Value};
use std::sync::Mutex;

use isnady_core::{doctor, i18n};

struct Run {
    running: bool,
    output: String,
    failed: usize,
    total: usize,
}

static RUN: Mutex<Run> = Mutex::new(Run {
    running: false,
    output: String::new(),
    failed: 0,
    total: 0,
});

/// The report and what the dialogs say of it.
#[tauri::command]
pub async fn help_check(updates: bool) -> Value {
    let lang = i18n::ui_language();
    let report = doctor::diagnose(true);
    let mut out = json!({
        "text": doctor::as_text(&report, &lang) + &doctor::other_text(false),
        "problems": report.problems.len(),
        "fixes": report.fixes.len(),
        "version": report.running_version,
        "latest": report.latest,
        "commands": report.commands.iter().any(|c| c.name == "iy-rs"),
    });
    if updates {
        let behind = report
            .latest
            .as_deref()
            .is_some_and(|l| doctor::vkey(l) > doctor::vkey(doctor::VERSION));
        out["behind"] = json!(behind);
        out["news"] = json!(if behind {
            doctor::whats_new(doctor::VERSION, 6)
        } else {
            vec![]
        });
    }
    out
}

/// Update: each command in its turn, in the background, its output kept for the dialog.
#[tauri::command]
pub fn help_update() -> Result<(), String> {
    {
        let mut r = RUN.lock().map_err(|e| e.to_string())?;
        if r.running {
            return Err("An update is running already.".into());
        }
        *r = Run {
            running: true,
            output: String::new(),
            failed: 0,
            total: 0,
        };
    }
    std::thread::spawn(|| {
        let plan = doctor::diagnose(true).fixes;
        doctor::step_aside();
        let say = |m: &str| {
            if let Ok(mut r) = RUN.lock() {
                r.output.push_str(m);
                r.output.push('\n');
            }
        };
        let mut failed = 0;
        for argv in &plan {
            say(&format!("$ {}", argv.join(" ")));
            match std::process::Command::new(&argv[0])
                .args(&argv[1..])
                .stdin(std::process::Stdio::null())
                .output()
            {
                Ok(o) => {
                    say(String::from_utf8_lossy(&o.stdout).trim_end());
                    say(String::from_utf8_lossy(&o.stderr).trim_end());
                    if !o.status.success() {
                        failed += 1;
                        say(&format!(
                            "  ended with code {}",
                            o.status.code().unwrap_or(1)
                        ));
                    }
                }
                Err(e) => {
                    failed += 1;
                    say(&format!("  {e}"));
                }
            }
        }
        if let Some(exe) = doctor::other_command() {
            // the Python isnady too, where it is installed (TEK PROJE)
            say(&format!("$ {} update --yes", exe.display()));
            let (code, out) = doctor::other_run(&["update", "--yes"], true);
            say(&out);
            if code != 0 {
                failed += 1;
            }
        }
        isnady_core::log::info(
            "update",
            &format!("{} command(s), {failed} failed", plan.len()),
        );
        if let Ok(mut r) = RUN.lock() {
            r.running = false;
            r.failed = failed;
            r.total = plan.len();
        }
    });
    Ok(())
}

#[tauri::command]
pub fn help_update_status() -> Value {
    match RUN.lock() {
        Ok(r) => {
            json!({"running": r.running, "output": r.output, "failed": r.failed, "total": r.total})
        }
        Err(_) => json!({"running": false}),
    }
}
