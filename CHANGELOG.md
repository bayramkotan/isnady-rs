# Changelog

isnady-rs carries the same version number as the Python isnady (github.com/bayramkotan/isnady): each release of one
is a release of the other. The text of each GitHub Release is taken from its section here.

## [Unreleased]

- **`iy-rs chain` (RS2), word for word as `iy chain`.** `iy-rs chain BOOK` gives the book's numbers; `iy-rs chain BOOK
  NUMBER` shows every chain of the hadith: each link with its term, the narrator Ibn Hajar's Taqrib or al-Najashi
  identifies (tabaqa, year of death, the first verdict, how sure the match is), tahwil meetings, al-Kulayni's groups
  and unnamed narrators, whom the chain reaches, and the chain in words in Turkish and English (`--lang`), with
  `--raw`. It takes about 30 ms. Reading the chains and identifying the narrators stay with the Python isnady: after
  books are imported, one `iy chain` does it, then `iy-rs chain` shows what it stored.
- **The word lists come from the Python isnady.** `tools/gen_tables.py` writes the names list, the Companions' names,
  the Ma'sumun, the terms, the Turkish of the chain's words and the patterns into `crates/isnady-core/data/tables.json`;
  the Parity workflow checks they are still the Python isnady's.
- **Parity: 35 cases**, 17 of them chains, on road A (Ibn Hajar's Taqrib) and road B (al-Najashi, Sifat al-Shi'a);
  the Parity workflow imports those two rijal works and a Shia book too.

## [0.7.4] — 2026-11-03

- **The first isnady-rs (Bayram, 2026-11-03: "isnady-rs ile olur. Ama birebir aynı olması lazım … sürüm numaraları
  birebir aynı olsun").** A Rust workspace: `isnady-core`, the core, and `isnady-cli`, the command line `iy-rs`. It
  reads the data folder, the database (schema 8) and the settings of the Python isnady 0.7.4.
- **Search, word for word as the Python one.** The same normalisation, the same search index (FTS5 trigram) and the
  same order of the books. all, any and phrase, whole words, books, languages, paging and width, wildcards (`?` `*`)
  and near spellings. The recent searches go into the same list as the app. `iy-rs search النيات` takes about 60 ms
  where the Python `iy` takes about 900 ms, most of it starting Python.
- `iy-rs sources`, `iy-rs datadir`, `iy-rs -V`.
- **Parity, checked.** `tools/parity.py` runs 18 cases with both programs and compares what they print. The
  **Parity** workflow runs it on every push and before every release, on books imported by the Python isnady of the
  same tag. PARITY.md lists what is ported and what is not yet.
- Releases: `iy-rs` for Windows, Linux and macOS (Apple silicon and Intel) on GitHub Releases, built by the
  **Release** workflow for every tag `vX.Y.Z`.
- **crates.io:** `cargo install isnady` gives `iy-rs`, as `pip install isnady` gives `iy`. The command-line crate is
  now `isnady` and the core is `isnady-core`. The **crates.io** workflow publishes both after every Release.
- **The README is isnady's own, word for word,** under a short head that says what `iy-rs` does today
  (`tools/sync_readme.py`; `tools/bump_version.py` runs it).
