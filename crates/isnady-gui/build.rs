// The Rust that built the window, for Help → About isnady (the Python window shows its Python there).
fn main() {
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".into());
    let version = std::process::Command::new(rustc)
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|v| v.split_whitespace().nth(1).map(String::from))
        .unwrap_or_default();
    println!("cargo:rustc-env=ISNADY_RUSTC={version}");
    tauri_build::build()
}
