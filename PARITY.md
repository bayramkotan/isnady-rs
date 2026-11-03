# Parity — what isnady-rs does of the Python isnady

isnady-rs and [isnady](https://github.com/bayramkotan/isnady) are one program written twice. They share the data
folder, the database, the settings, the recent searches and the version number. Each release of isnady is a release
of isnady-rs with the same number. This table says what the Rust one does so far. Whatever it does not do yet, the
Python `iy` does on the same data.

**Version 0.7.4** · ✅ the same as the Python isnady (checked by `tools/parity.py`, word for word) · ⏳ not yet: use `iy`

## Command line

| Command (Python `iy …`) | isnady-rs (`iy-rs …`) | Notes |
|---|---|---|
| `iy -V`, `iy version` | ✅ | prints `isnady 0.7.4 (Rust)` |
| `iy datadir` (show) | ✅ | `ISNADY_DATA_DIR`, a portable `isnady-data` folder, `location.json`, the platform default. Same order as Python. |
| `iy datadir PATH` (move) | ⏳ | |
| `iy sources` | ✅ | |
| `iy search WORDS` | ✅ | all · any · phrase · `--whole-words` · `--book` · `--language` · `--limit` · `--offset` · `--width` · `--all-texts` · wildcards (`?` `*`) · near spellings · the recent searches list |
| `iy search --grade …`, `--every-grader`, `--no-sahihayn` | ⏳ | |
| `iy search --chapter BOOK:CHAPTER` | ⏳ | |
| `iy search --mode meaning`, `--in narrators/scholars`, `--recent`, `--clear-recent` | ⏳ | |
| `iy chain`, `iy isnads` | ⏳ | the chain reader (isnady-isnad-15) and the narrator matcher (isnady-narrators-17) are next |
| `iy narrator`, `iy scholar`, `iy life`, `iy tree`, `iy group` | ⏳ | |
| `iy book`, `iy quran`, `iy sira`, `iy sahifa` | ⏳ | |
| `iy stats …` | ⏳ | |
| `iy catalog …`, `iy repo …`, `iy import …`, `iy database …`, `iy remove` | ⏳ | the database is imported by the Python isnady |
| `iy export`, `iy import-data`, `iy bookmark`, `iy config`, `iy profile`, `iy learn`, `iy term`, `iy ai`, `iy cache`, `iy log`, `iy reset`, `iy update`, `iy doctor` | ⏳ | |

## The window

| | isnady-rs |
|---|---|
| Desktop window (Search, Books, Narrators, Isnad Chains, …) | ⏳ planned with Tauri: the core in Rust, the pages drawn by the system's web view, so Arabic, right-to-left text and the Amiri font render exactly |

## The core

| Module (Python) | isnady-rs | |
|---|---|---|
| `isnady.data.paths` | ✅ `paths.rs` | |
| `isnady.core.normalize` | ✅ `normalize.rs` | same rules, `NORMALIZER_VERSION` 1: the search index the Python isnady builds is read as it is |
| `isnady.core.search` | ✅ `search.rs` | except the grades and chapters filters |
| `isnady.core.books` (order, active books) | ✅ `books.rs` | |
| `isnady.config` (settings, state), `isnady.core.profiles` | ✅ `config.rs` (reading, the recent searches) | |
| `isnady.data.db` | ✅ reading schema 8 (`db.rs`) | creating and upgrading stay with the Python isnady |
| everything else | ⏳ | |

## How parity is kept

- **The same number.** `tools/bump_version.py X.Y.Z` here and `tools/bump_version.py X.Y.Z` in isnady, in the same
  release. The tag `vX.Y.Z` is pushed to both repositories.
- **The same answer.** `tools/parity.py` runs every case with both programs on the same data and compares what they
  print. Only the time a search took is left out. The **Parity** workflow does it on every push and before every
  release. It imports a few books with the Python isnady of the same tag, then compares. A release is not built when
  they differ.
- **One rule, two places.** A rule changed in one isnady is changed in the other in the same release. Where Python
  left a choice open, both now make it the same way (0.7.4: spellings of equal frequency are ordered by the word,
  in both).
- **The same README.** isnady-rs's README is isnady's README, word for word, under a head of its own
  (`tools/sync_readme.py`).
- **The same places.** The Python isnady is on PyPI and npm. isnady-rs is on crates.io (`cargo install isnady`) and
  in GitHub Releases. Both are published by their workflows for every tag.
