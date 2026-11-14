<!-- The head of isnady-rs's README; everything below the line is the README of the Python isnady, word for word
     (tools/sync_readme.py writes it — never edit README.md by hand). -->
<h1 align="center">isnady-rs — isnady in Rust</h1>

<p align="center">
  <img src="https://img.shields.io/badge/isnady--rs-v0.7.8-1D4777?style=for-the-badge&logo=rust" alt="version">
  <a href="https://crates.io/crates/isnady"><img src="https://img.shields.io/crates/v/isnady?style=for-the-badge&logo=rust&label=crates.io" alt="crates.io"></a>
  <a href="https://github.com/bayramkotan/isnady-rs/releases/latest"><img src="https://img.shields.io/github/v/release/bayramkotan/isnady-rs?style=for-the-badge&label=release" alt="release"></a>
  <img src="https://img.shields.io/badge/licence-MIT-5B6878?style=for-the-badge" alt="MIT">
</p>

**isnady-rs is isnady.** It is the same program, written again in Rust. It uses the same data folder, the same
database, the same settings and the same version number, and it gives the same answers. That is why this README is
isnady's own, word for word, below the line. The Rust command is `iy-rs` where the Python one is `iy`.

**Version 0.7.8 — what isnady-rs does today: everything the Python isnady does.** Every command of `iy` is a command
of `iy-rs` with the same options and the same output, word for word; every page, menu, dialog, theme and language of the
Python window is in the window `isnady-rs`. The two programs import the same books into the same rows, read the same
chains, identify the same narrators, learn the same meaning index (array for array), export the same files and take the
same ready database. `tools/parity.py`, `parity_db.py`, `parity_export.py`, `parity_ai.py` and `parity_ready.py`
compare them on every push and before every release; [PARITY.md](PARITY.md) lists each part. A search takes about
3 ms, a chain about 30 ms.

**Get it:**

| Where | What |
|---|---|
| [Releases](https://github.com/bayramkotan/isnady-rs/releases/latest) | **The window:** Windows `-setup.exe` (no administrator rights), `.msi`, or the portable `.exe`; Linux the portable `.AppImage` or a `.deb`; macOS a `.dmg` or the `.app` zipped (Apple silicon and Intel). **The command line:** `iy-rs` for each of them. Every file with its sha256 in `SHA256SUMS.txt` |
| [crates.io](https://crates.io/crates/isnady) | `cargo install isnady` gives `iy-rs`; `cargo install isnady-gui` gives the window `isnady-rs` (Linux needs `webkit2gtk-4.1`) |
| The source | `cargo build --release`: `iy-rs` and `isnady-rs` in `target/release/`; `cargo tauri build` in `crates/isnady-gui` makes the installers |

A folder named `isnady-data` beside a portable copy (the AppImage, the `.exe`, the `.app`) keeps every database and
setting there. With no book yet, the window offers **Import all** or isnady-data's **ready database** (`iy-rs database
download`): everything already read, in one download.

---

