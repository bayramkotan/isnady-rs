<h1 align="center">isnady-rs</h1>

<p align="center"><b>isnady in Rust</b> — the same program as <a href="https://github.com/bayramkotan/isnady">isnady</a>,
written again in Rust: the same data, the same rules, the same answers, the same version number.</p>

<p align="center">
  <img src="https://img.shields.io/badge/isnady--rs-v0.7.4-1D4777?style=for-the-badge&logo=rust" alt="version">
  <img src="https://img.shields.io/badge/parity-search%20%C2%B7%20sources%20%C2%B7%20datadir-2E7D32?style=for-the-badge" alt="parity">
  <img src="https://img.shields.io/badge/licence-MIT-5B6878?style=for-the-badge" alt="MIT">
</p>

> **Version 0.7.4 — pre-alpha, a beginning.** The Python isnady is the complete one: the window, the importers, the
> chain reader, the narrators, the statistics. isnady-rs takes it over piece by piece. Every piece is ported **word
> for word**, and a test proves it gives the Python answer. [PARITY.md](PARITY.md) lists what is done.

## Why two

- **One isnady.** isnady-rs reads and writes what the Python isnady does: the data folder, the database
  (`isnady.db`), the settings, the recent searches. Import your books with isnady, search them with either.
- **The same number.** isnady 0.7.4 and isnady-rs 0.7.4 are the same program. Each release of one is a release of
  the other, and both repositories carry the tag.
- **The same answers.** `tools/parity.py` runs both programs on the same data and compares what they print, word for
  word. The **Parity** workflow does it on every push and before every release. A release whose answers differ is
  not built.
- **Faster.** A search starts and ends in about 60 ms; the Python `iy` takes about 900 ms, most of it starting Python.

## Install

From [Releases](https://github.com/bayramkotan/isnady-rs/releases/latest): `iy-rs` for Windows, Linux and macOS
(Apple silicon and Intel). Unpack it anywhere and run it. Or build it:

```bash
cargo install --git https://github.com/bayramkotan/isnady-rs isnady-cli     # gives iy-rs
```

The books are imported by the Python isnady (`pip install isnady`, then `iy catalog import --all` or **File → Data
sources**). isnady-rs finds the same data folder by itself.

## Use

```bash
iy-rs search النيات                          # Arabic, with or without diacritics
iy-rs search "إنما الأعمال بالنيات" --mode phrase
iy-rs search niyet --language Turkish --limit 5
iy-rs search صلاة زكاة --mode any --book bukhari
iy-rs search "مسل*"                          # a wildcard: any letters; ? one letter
iy-rs sources                                # what is imported
iy-rs datadir                                # where the data is
iy-rs -V                                     # isnady 0.7.4 (Rust)
```

Every command prints exactly what the Python `iy` prints for it. A command or option that is not ported yet says so
and names the `iy` command that does it.

## How it is made

```
isnady-rs/
├── crates/isnady-core/      the core: paths, normalize, books, search, config, db — a module for each module of
│                            the Python isnady, with the same rules
├── crates/isnady-cli/       iy-rs, the command line
├── tools/parity.py          the same answer: both programs, the same cases, compared word for word
├── tools/bump_version.py    the same number: python tools/bump_version.py X.Y.Z
├── tools/release_check.py   the tag, Cargo.toml and CHANGELOG agree
├── PARITY.md                what is ported, what is not yet
└── .github/workflows/       CI (build and test on Windows, Linux, macOS), Parity, Release
```

**Next:** the chain reader and the narrator matcher (`iy chain`), then the books (`iy book`, `iy quran`, `iy sira`,
`iy sahifa`), the narrators and the scholars. Then the window, with Tauri: the core in Rust, the pages drawn by the
system's web view, so Arabic, right-to-left text and the Amiri font render exactly as they do now.

```bash
cargo build --release
cargo test
python tools/parity.py --python iy        # with the Python isnady of the same version on PATH
```

## Licence

MIT, as isnady. The data keeps its own licences, recorded in
[isnady-data](https://github.com/bayramkotan/isnady-data).
