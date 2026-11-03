<!-- The head of isnady-rs's README; everything below the line is the README of the Python isnady, word for word
     (tools/sync_readme.py writes it — never edit README.md by hand). -->
<h1 align="center">isnady-rs — isnady in Rust</h1>

<p align="center">
  <img src="https://img.shields.io/badge/isnady--rs-v0.7.4-1D4777?style=for-the-badge&logo=rust" alt="version">
  <a href="https://crates.io/crates/isnady"><img src="https://img.shields.io/crates/v/isnady?style=for-the-badge&logo=rust&label=crates.io" alt="crates.io"></a>
  <a href="https://github.com/bayramkotan/isnady-rs/releases/latest"><img src="https://img.shields.io/github/v/release/bayramkotan/isnady-rs?style=for-the-badge&label=release" alt="release"></a>
  <img src="https://img.shields.io/badge/licence-MIT-5B6878?style=for-the-badge" alt="MIT">
</p>

**isnady-rs is isnady.** It is the same program, written again in Rust. It uses the same data folder, the same
database, the same settings and the same version number, and it gives the same answers. That is why this README is
isnady's own, word for word, below the line. The Rust command is `iy-rs` where the Python one is `iy`.

**Version 0.7.4 — what `iy-rs` does today:** `iy-rs search` (all of it: all, any, phrase, whole words, books,
languages, paging, wildcards, near spellings, the recent searches), `iy-rs chain` (every link, its narrator and his
grade, whom the chain reaches, the chain in Turkish and English), `iy-rs sources`, `iy-rs datadir` and `iy-rs -V`.
A search takes about 60 ms, a chain about 30 ms. Each of these prints what `iy` prints, word for word; `tools/parity.py` checks it on
every push and before every release. Everything else below is still done by the Python isnady, on the same data. The
ported parts are listed in [PARITY.md](PARITY.md), and more are ported with every release.

**Get it:**

| Where | How |
|---|---|
| [crates.io](https://crates.io/crates/isnady) | `cargo install isnady`, which gives `iy-rs` |
| [Releases](https://github.com/bayramkotan/isnady-rs/releases/latest) | `iy-rs` for Windows, Linux and macOS (Apple silicon and Intel), ready to run: unpack it and run `iy-rs` |
| The source | `cargo build --release`, which puts the program in `target/release/iy-rs` |

The books are imported by the Python isnady (`pip install isnady`, then `iy catalog import --all`). `iy-rs` finds the
same data folder by itself.

---

