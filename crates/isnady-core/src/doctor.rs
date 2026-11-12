//! iy doctor, iy update, Help → Check Installation / Check for Updates (A5; `isnady/doctor.py`, `core/updates.py`).
//!
//! The Python isnady looks at every copy pip installed; isnady-rs is installed by cargo (or downloaded from the
//! Releases), so here the copies are the ones cargo recorded (`.crates2.json`) and the ones on PATH. Each copy is
//! updated where it is: from crates.io (`cargo install isnady --locked --force`), or from its clone (`git pull` and
//! `cargo install --path …`). What is new comes from isnady-rs's CHANGELOG on GitHub. Nothing is run without a yes.

use serde_json::Value;
use std::path::{Path, PathBuf};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
const CRATES_API: &str = "https://crates.io/api/v1/crates/";
const CHANGELOG: &str = "https://raw.githubusercontent.com/bayramkotan/isnady-rs/main/CHANGELOG.md";
/// (package on crates.io, its program)
pub const PACKAGES: [(&str, &str); 2] = [("isnady", "iy-rs"), ("isnady-gui", "isnady-rs")];

pub fn vkey(v: &str) -> Vec<u64> {
    v.split(|c: char| !c.is_ascii_digit())
        .filter(|p| !p.is_empty())
        .take(4)
        .filter_map(|p| p.parse().ok())
        .collect()
}

/// One copy cargo installed (`Install`).
#[derive(Debug, Clone)]
pub struct Install {
    pub package: String,
    pub version: String,
    /// "crates.io", "clone" (a folder) or "git"
    pub kind: String,
    /// the clone's folder (kind "clone"), or the registry
    pub source: String,
    pub bins: Vec<String>,
}

/// One program on PATH (`Command`).
#[derive(Debug, Clone)]
pub struct Command {
    pub name: String,
    pub path: PathBuf,
    pub version: Option<String>,
    pub first: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Report {
    pub running_version: String,
    pub running_file: String,
    pub latest: Option<String>,
    pub installs: Vec<Install>,
    pub commands: Vec<Command>,
    pub problems: Vec<String>,
    pub fixes: Vec<Vec<String>>,
}

fn cargo_home() -> PathBuf {
    std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| crate::paths::expand_user("~/.cargo"))
}

/// The copies cargo installed, as `.crates2.json` records them.
pub fn find_installs() -> Vec<Install> {
    let Ok(text) = std::fs::read_to_string(cargo_home().join(".crates2.json")) else {
        return vec![];
    };
    let Ok(data) = serde_json::from_str::<Value>(&text) else {
        return vec![];
    };
    let mut out = vec![];
    for (key, info) in data["installs"].as_object().into_iter().flatten() {
        // "isnady 0.7.4 (registry+https://github.com/rust-lang/crates.io-index)", "isnady 0.7.4 (path+file:///…)"
        let mut parts = key.splitn(3, ' ');
        let (Some(package), Some(version), Some(source)) =
            (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        if !PACKAGES.iter().any(|p| p.0 == package) {
            continue;
        }
        let source = source
            .trim_start_matches('(')
            .trim_end_matches(')')
            .to_string();
        let (kind, source) = if let Some(url) = source.strip_prefix("path+") {
            let path = url::Url::parse(url)
                .ok()
                .and_then(|u| u.to_file_path().ok());
            (
                "clone",
                path.map(|p| p.display().to_string())
                    .unwrap_or_else(|| url.to_string()),
            )
        } else if source.starts_with("git+") {
            ("git", source.trim_start_matches("git+").to_string())
        } else {
            ("crates.io", source)
        };
        let bins = info["bins"]
            .as_array()
            .map(|b| {
                b.iter()
                    .filter_map(|x| x.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        out.push(Install {
            package: package.into(),
            version: version.into(),
            kind: kind.into(),
            source,
            bins,
        });
    }
    out.sort_by(|a, b| {
        (a.package.as_str(), a.kind.as_str()).cmp(&(b.package.as_str(), b.kind.as_str()))
    });
    out
}

fn exe_names(name: &str) -> Vec<String> {
    if cfg!(windows) {
        vec![format!("{name}.exe")]
    } else {
        vec![name.to_string()]
    }
}

/// What `iy-rs version` says of itself ("isnady 0.7.4 (Rust)" → "0.7.4").
fn version_of(path: &Path) -> Option<String> {
    let out = std::process::Command::new(path)
        .arg("version")
        .stdin(std::process::Stdio::null())
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    text.split_whitespace().nth(1).map(String::from)
}

/// Every iy-rs and isnady-rs on PATH, in PATH's order: the first of each name is the one that starts.
pub fn find_commands() -> Vec<Command> {
    let mut out: Vec<Command> = vec![];
    let dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    for (_package, bin) in PACKAGES {
        let mut first = true;
        let mut seen: Vec<PathBuf> = vec![];
        for d in &dirs {
            for n in exe_names(bin) {
                let p = d.join(&n);
                if p.is_file() {
                    let real = std::fs::canonicalize(&p).unwrap_or(p.clone());
                    if seen.contains(&real) {
                        continue;
                    }
                    seen.push(real);
                    // the window cannot say its version on the command line: the copy cargo recorded beside it says it
                    let version = if bin == "iy-rs" { version_of(&p) } else { None };
                    out.push(Command {
                        name: bin.into(),
                        path: p,
                        version,
                        first,
                    });
                    first = false;
                }
            }
        }
    }
    out
}

/// The newest version on crates.io, or None when it could not be reached.
pub fn latest_on_crates(package: &str) -> Option<String> {
    let bytes = crate::sources::read_bytes(&format!("{CRATES_API}{package}"), Some(5)).ok()?;
    let v: Value = serde_json::from_slice(&bytes).ok()?;
    v["crate"]["max_stable_version"]
        .as_str()
        .or_else(|| v["crate"]["max_version"].as_str())
        .map(String::from)
}

/// The command that updates a copy where it is.
pub fn upgrade_argv(i: &Install) -> Vec<String> {
    let mut argv: Vec<String> = vec!["cargo".into(), "install".into()];
    match i.kind.as_str() {
        "clone" => argv.extend(["--path".into(), i.source.clone()]),
        "git" => argv.extend([
            "--git".into(),
            i.source.split('#').next().unwrap_or(&i.source).into(),
            i.package.clone(),
        ]),
        _ => argv.push(i.package.clone()),
    }
    argv.extend(["--locked".into(), "--force".into()]);
    argv
}

/// The clone a copy was installed from, to bring up to date first (`git -C … pull`): the repository's root.
fn clone_root(i: &Install) -> Option<PathBuf> {
    let mut p = PathBuf::from(&i.source);
    loop {
        if p.join(".git").exists() {
            return Some(p);
        }
        if !p.pop() {
            return None;
        }
    }
}

/// Every copy, which one each command starts, and what is out of date (`doctor.diagnose`).
pub fn diagnose(check_crates: bool) -> Report {
    let running_file = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let latest = if check_crates {
        latest_on_crates("isnady")
    } else {
        None
    };
    let installs = find_installs();
    let commands = find_commands();
    let mut problems = vec![];
    let mut fixes: Vec<Vec<String>> = vec![];
    if let Some(l) = &latest {
        for i in &installs {
            if vkey(&i.version) < vkey(l) {
                problems.push(format!(
                    "{} {} ({}) is older than {l} on crates.io.",
                    i.package, i.version, i.kind
                ));
                if i.kind == "clone" {
                    if let Some(root) = clone_root(i) {
                        let pull = vec![
                            "git".into(),
                            "-C".into(),
                            root.display().to_string(),
                            "pull".into(),
                            "--ff-only".into(),
                        ];
                        if !fixes.contains(&pull) {
                            fixes.push(pull);
                        }
                    }
                }
                fixes.push(upgrade_argv(i));
            }
        }
    }
    for c in &commands {
        if c.first {
            if let Some(v) = &c.version {
                if vkey(v) < vkey(VERSION) {
                    problems.push(format!(
                        "{} on PATH starts {v} ({}), older than this {VERSION}: an older copy comes first on PATH.",
                        c.name,
                        c.path.display()
                    ));
                }
            }
        }
    }
    let names: Vec<&str> = commands.iter().map(|c| c.name.as_str()).collect();
    if !names.contains(&"iy-rs") {
        problems.push("iy-rs is not on PATH: add cargo's bin folder to PATH, or install it: cargo install isnady --locked".into());
    }
    Report {
        running_version: VERSION.into(),
        running_file,
        latest,
        installs,
        commands,
        problems,
        fixes,
    }
}

fn shown(argv: &[String]) -> String {
    argv.iter()
        .map(|a| {
            if a.chars().any(|c| " <>|&".contains(c)) {
                format!("\"{a}\"")
            } else {
                a.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The report as `iy doctor` prints it and the window shows it (`doctor.as_text`).
pub fn as_text(r: &Report, lang: &str) -> String {
    let t = |s: &str, v: &[(&str, String)]| crate::i18n::t(lang, s, v);
    let mut lines = vec![t(
        "isnady {version} running from {file}",
        &[
            ("version", format!("{} (Rust)", r.running_version)),
            ("file", r.running_file.clone()),
        ],
    )];
    if let Some(l) = &r.latest {
        lines.push(t("Latest on crates.io: {latest}", &[("latest", l.clone())]));
    }
    lines.push(format!("\n{}", t("Installations:", &[])));
    for i in &r.installs {
        let place = if i.kind == "clone" {
            i.source.clone()
        } else {
            format!("{} ({})", i.package, i.kind)
        };
        lines.push(format!("  {:10} {:9} {place}", i.version, i.kind));
    }
    if r.installs.is_empty() {
        lines.push(format!(
            "  {}",
            t(
                "none found (downloaded from the Releases, or built in place)",
                &[]
            )
        ));
    }
    lines.push(format!(
        "\n{}",
        t(
            "Commands on PATH (the first of each name is the one that starts):",
            &[]
        )
    ));
    for c in &r.commands {
        lines.push(format!(
            "  {} {}  -> {}",
            if c.first { '*' } else { ' ' },
            c.path.display(),
            c.version.as_deref().unwrap_or("?")
        ));
    }
    lines.push(String::new());
    if r.problems.is_empty() {
        lines.push(t("No problems found.", &[]));
    } else {
        lines.push(t("Problems:", &[]));
        lines.extend(r.problems.iter().map(|p| format!("  - {p}")));
        if !r.fixes.is_empty() {
            lines.push(format!(
                "\n{}",
                t(
                    "To update every copy where it is (iy-rs update runs these):",
                    &[]
                )
            ));
            lines.extend(r.fixes.iter().map(|f| format!("  {}", shown(f))));
        }
    }
    lines.join("\n")
}

/// (version, the CHANGELOG's text) of the versions after `since`, newest first; [] when not reachable (`whats_new`).
pub fn whats_new(since: &str, limit: usize) -> Vec<(String, String)> {
    let Ok(bytes) = crate::sources::read_bytes(CHANGELOG, Some(5)) else {
        return vec![];
    };
    let text = String::from_utf8_lossy(&bytes).to_string();
    let mut out = vec![];
    let heads: Vec<(usize, String)> = text
        .match_indices("\n## [")
        .filter_map(|(at, _)| {
            let rest = &text[at + 5..];
            let v = rest.split(']').next()?;
            (vkey(v).len() == 3).then(|| (at, v.to_string()))
        })
        .collect();
    for (at, v) in &heads {
        if vkey(v) <= vkey(since) {
            continue;
        }
        let start = text[*at + 1..]
            .find('\n')
            .map(|e| at + 1 + e + 1)
            .unwrap_or(*at);
        let end = text[start..]
            .find("\n## [")
            .map(|e| start + e)
            .unwrap_or(text.len());
        let body = crate::pre!(r"\n[ \t]+")
            .sub(text[start..end].trim(), " ")
            .replace("**", "")
            .replace('`', "");
        out.push((v.clone(), body));
    }
    out.truncate(limit);
    out
}

/// Run each command, asking first unless `yes` (`cli._run_plan`); (command, exit code) of each.
pub fn run_plan(
    plan: &[Vec<String>],
    yes: bool,
    ask: &dyn Fn(&str) -> bool,
    say: &dyn Fn(&str),
) -> Vec<(String, i32)> {
    let mut out = vec![];
    for argv in plan {
        let text = shown(argv);
        if !yes && !ask(&text) {
            say("  skipped");
            continue;
        }
        say(&format!("$ {text}"));
        let code = std::process::Command::new(&argv[0])
            .args(&argv[1..])
            .status()
            .map(|s| s.code().unwrap_or(1))
            .unwrap_or(127);
        if code != 0 {
            say(&format!("  ended with code {code}"));
        }
        out.push((text, code));
    }
    out
}

/// On Windows a running program cannot be written over: this copy steps aside (renamed, which Windows allows) so
/// cargo can put the new one in its place; the old one is removed at the next update.
pub fn step_aside() {
    if !cfg!(windows) {
        return;
    }
    let Ok(me) = std::env::current_exe() else {
        return;
    };
    let old = me.with_extension("old.exe");
    let _ = std::fs::remove_file(&old);
    let _ = std::fs::rename(&me, &old);
}

// The other isnady (TEK PROJE): the Python isnady has its own copies (pip's); each program's report and update take
// in the other's, so `iy-rs doctor` and `iy doctor` say the same and `iy-rs update` brings both up to date.
pub const OTHER: &str = "iy";
pub const OTHER_NAME: &str = "isnady (Python)";
const NESTED: &str = "ISNADY_DOCTOR_NESTED";

/// The other program on PATH, or None (and None when this report is itself the other's).
pub fn other_command() -> Option<PathBuf> {
    if std::env::var_os(NESTED).is_some() {
        return None;
    }
    let dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    let names: Vec<String> = if cfg!(windows) {
        vec![
            format!("{OTHER}.exe"),
            format!("{OTHER}.cmd"),
            format!("{OTHER}.bat"),
        ]
    } else {
        vec![OTHER.to_string()]
    };
    dirs.iter()
        .flat_map(|d| names.iter().map(move |n| d.join(n)))
        .find(|p| p.is_file())
}

/// Run the other program's doctor or update (it does not call this one back); (exit code, what it printed). With
/// `capture` false its output goes straight to the terminal, and it may ask.
pub fn other_run(args: &[&str], capture: bool) -> (i32, String) {
    let Some(exe) = other_command() else {
        return (0, String::new());
    };
    let mut cmd = std::process::Command::new(exe);
    cmd.args(args)
        .env(NESTED, "1")
        .env("PYTHONIOENCODING", "utf-8");
    if capture {
        cmd.stdin(std::process::Stdio::null());
        match cmd.output() {
            Ok(o) => (
                o.status.code().unwrap_or(1),
                String::from_utf8_lossy(&o.stdout).trim_end().to_string(),
            ),
            Err(e) => (1, e.to_string()),
        }
    } else {
        match cmd.status() {
            Ok(s) => (s.code().unwrap_or(1), String::new()),
            Err(e) => (1, e.to_string()),
        }
    }
}

/// What the other isnady says of its copies, under a heading of its own; "" when it is not on this computer.
pub fn other_text(offline: bool) -> String {
    let Some(exe) = other_command() else {
        return String::new();
    };
    let args: Vec<&str> = if offline {
        vec!["doctor", "--offline"]
    } else {
        vec!["doctor"]
    };
    let (_code, out) = other_run(&args, true);
    if out.is_empty() {
        String::new()
    } else {
        format!("\n\n— {OTHER_NAME}: {} —\n{out}", exe.display())
    }
}
