//! Desktop and menu shortcuts for the window (Tools → Create Desktop Shortcut, `iy-rs shortcut`): the Python isnady's
//! `core/shortcut.py`, for the Rust window `isnady-rs`. The shortcut has its own name (isnady-rs) beside the Python
//! one's, and its icon is written beside it from the program itself; the Desktop folder is asked of the system (it is
//! "Masaüstü" on a Turkish Linux, and may be redirected to OneDrive on Windows).

use std::path::{Path, PathBuf};
use std::process::Command;

pub const APP_ID: &str = "isnady-rs";
const PNG: &[u8] = include_bytes!("../data/icons/isnady-rs.png");
const ICO: &[u8] = include_bytes!("../data/icons/isnady-rs.ico");
const ICNS: &[u8] = include_bytes!("../data/icons/isnady-rs.icns");

fn home() -> PathBuf {
    std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn which(name: &str) -> Option<PathBuf> {
    let exe = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    };
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|d| d.join(&exe))
            .find(|p| p.is_file())
    })
}

/// (program, how) — what a shortcut must run to start the window as installed here.
pub fn launch_command() -> (PathBuf, &'static str) {
    if let Ok(img) = std::env::var("APPIMAGE") {
        if !img.is_empty() {
            return (PathBuf::from(img), "the AppImage");
        }
    }
    let exe_name = if cfg!(windows) {
        "isnady-rs.exe"
    } else {
        "isnady-rs"
    };
    if let Ok(me) = std::env::current_exe() {
        if me.file_name().map(|n| n == exe_name).unwrap_or(false) {
            return (me, "this program");
        }
        if let Some(beside) = me
            .parent()
            .map(|d| d.join(exe_name))
            .filter(|p| p.is_file())
        {
            return (beside, "the isnady-rs command");
        }
    }
    match which("isnady-rs") {
        Some(p) => (p, "the isnady-rs command"),
        None => (PathBuf::from("isnady-rs"), "the isnady-rs command"),
    }
}

/// The desktop folder as the system names it (`shortcut.desktop_dir`).
pub fn desktop_dir() -> PathBuf {
    if cfg!(windows) {
        if let Ok(out) = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "[Environment]::GetFolderPath('Desktop')",
            ])
            .output()
        {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if out.status.success() && !s.is_empty() {
                return PathBuf::from(s);
            }
        }
        return home().join("Desktop");
    }
    if !cfg!(target_os = "macos") {
        if let Ok(out) = Command::new("xdg-user-dir").arg("DESKTOP").output() {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if out.status.success() && !s.is_empty() && Path::new(&s) != home() {
                return PathBuf::from(s);
            }
        }
        let config = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home().join(".config"));
        if let Ok(text) = std::fs::read_to_string(config.join("user-dirs.dirs")) {
            for line in text.lines() {
                if let Some(v) = line.strip_prefix("XDG_DESKTOP_DIR=") {
                    let v = v
                        .trim()
                        .trim_matches('"')
                        .replace("$HOME", &home().to_string_lossy());
                    if !v.is_empty() && Path::new(&v) != home() {
                        return PathBuf::from(v);
                    }
                }
            }
        }
    }
    home().join("Desktop")
}

fn quote(arg: &str) -> String {
    if arg.chars().any(|c| " \"'\\".contains(c)) {
        format!("\"{arg}\"")
    } else {
        arg.to_string()
    }
}

/// Create the shortcuts; [(what, path)] of what was written, or the reason it could not be.
pub fn create(desktop: bool, menu: bool) -> Result<Vec<(String, PathBuf)>, String> {
    let (program, how) = launch_command();
    if cfg!(target_os = "macos") {
        macos(&program, desktop, menu)
    } else if cfg!(windows) {
        windows(&program, desktop, menu)
    } else {
        linux(&program, how, desktop, menu)
    }
}

fn linux(
    program: &Path,
    how: &str,
    desktop: bool,
    menu: bool,
) -> Result<Vec<(String, PathBuf)>, String> {
    let icon_dir = home().join(".local/share/icons/hicolor/512x512/apps");
    std::fs::create_dir_all(&icon_dir).map_err(|e| e.to_string())?;
    let icon = icon_dir.join(format!("{APP_ID}.png"));
    std::fs::write(&icon, PNG).map_err(|e| e.to_string())?;
    let entry = format!(
        "[Desktop Entry]\nType=Application\nName=isnady (Rust)\nGenericName=Hadith search and chains of transmission\n\
         Comment=Search hadith, follow their chains of transmission, read the books\nExec={}\nIcon={}\n\
         Terminal=false\nCategories=Education;Literature;\nKeywords=hadith;isnad;rijal;Arabic;\nStartupWMClass={APP_ID}\n\
         X-isnady-Launches={how}\n",
        quote(&program.to_string_lossy()),
        icon.display()
    );
    let mut targets = vec![];
    if menu {
        targets.push((
            "menu",
            home().join(format!(".local/share/applications/{APP_ID}.desktop")),
        ));
    }
    if desktop {
        targets.push(("desktop", desktop_dir().join(format!("{APP_ID}.desktop"))));
    }
    let mut made = vec![];
    for (what, path) in targets {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&path, &entry).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755));
        }
        if what == "desktop" {
            let _ = Command::new("gio")
                .args(["set", &path.to_string_lossy(), "metadata::trusted", "true"])
                .output();
        }
        made.push((what.to_string(), path));
    }
    if menu {
        let _ = Command::new("update-desktop-database")
            .arg(home().join(".local/share/applications"))
            .output();
    }
    Ok(made)
}

fn windows(program: &Path, desktop: bool, menu: bool) -> Result<Vec<(String, PathBuf)>, String> {
    let appdata = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join("AppData").join("Roaming"));
    let icon = appdata.join("isnady").join(format!("{APP_ID}.ico"));
    if let Some(parent) = icon.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&icon, ICO);
    let mut targets = vec![];
    if desktop {
        targets.push(("desktop", desktop_dir().join(format!("{APP_ID}.lnk"))));
    }
    if menu {
        targets.push((
            "menu",
            appdata
                .join("Microsoft/Windows/Start Menu/Programs")
                .join(format!("{APP_ID}.lnk")),
        ));
    }
    let mut made = vec![];
    for (what, path) in targets {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let ps = "$s = (New-Object -ComObject WScript.Shell).CreateShortcut($env:ISNADY_LNK); \
                  $s.TargetPath = $env:ISNADY_TARGET; $s.Arguments = ''; \
                  $s.WorkingDirectory = $env:USERPROFILE; $s.Description = 'isnady - hadith search'; \
                  if ($env:ISNADY_ICON) { $s.IconLocation = $env:ISNADY_ICON + ',0' }; $s.Save()";
        let out = Command::new("powershell")
            .args(["-NoProfile", "-Command", ps])
            .env("ISNADY_LNK", &path)
            .env("ISNADY_TARGET", program)
            .env("ISNADY_ICON", &icon)
            .output()
            .map_err(|e| e.to_string())?;
        if !out.status.success() || !path.exists() {
            let why = String::from_utf8_lossy(&out.stderr).trim().to_string();
            return Err(format!(
                "Windows could not write {}: {}",
                path.display(),
                if why.is_empty() {
                    "no reason given".into()
                } else {
                    why
                }
            ));
        }
        made.push((what.to_string(), path));
    }
    Ok(made)
}

fn macos(program: &Path, desktop: bool, menu: bool) -> Result<Vec<(String, PathBuf)>, String> {
    let mut made = vec![];
    let app = home().join("Applications").join("isnady-rs.app");
    let macos_dir = app.join("Contents").join("MacOS");
    let res = app.join("Contents").join("Resources");
    std::fs::create_dir_all(&macos_dir).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&res).map_err(|e| e.to_string())?;
    let launcher = macos_dir.join("isnady-rs");
    std::fs::write(
        &launcher,
        format!(
            "#!/bin/sh\nexec {} \"$@\"\n",
            quote(&program.to_string_lossy())
        ),
    )
    .map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&launcher, std::fs::Permissions::from_mode(0o755));
    }
    let _ = std::fs::write(res.join("isnady-rs.icns"), ICNS);
    std::fs::write(
        app.join("Contents").join("Info.plist"),
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \
         \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\"><dict>\n\
         <key>CFBundleName</key><string>isnady</string>\n<key>CFBundleDisplayName</key><string>isnady (Rust)</string>\n\
         <key>CFBundleIdentifier</key><string>com.bayramkotan.isnady-rs.launcher</string>\n\
         <key>CFBundleExecutable</key><string>isnady-rs</string>\n<key>CFBundleIconFile</key><string>isnady-rs</string>\n\
         <key>CFBundlePackageType</key><string>APPL</string>\n</dict></plist>\n",
    )
    .map_err(|e| e.to_string())?;
    if menu {
        made.push(("menu".to_string(), app.clone()));
    }
    if desktop {
        let link = desktop_dir().join("isnady-rs.app");
        if link
            .symlink_metadata()
            .map(|m| m.file_type().is_symlink())
            .unwrap_or(false)
        {
            let _ = std::fs::remove_file(&link);
        }
        #[cfg(unix)]
        if !link.exists() {
            std::os::unix::fs::symlink(&app, &link).map_err(|e| e.to_string())?;
        }
        made.push(("desktop".to_string(), link));
    }
    Ok(made)
}
