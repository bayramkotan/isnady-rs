# Changelog

isnady-rs carries the same version number as the Python isnady (github.com/bayramkotan/isnady): each release of one
is a release of the other. The text of each GitHub Release is taken from its section here.

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
