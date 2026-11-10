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

**Version 0.7.4 — what `iy-rs` does today:** `iy-rs search` (all, any, phrase, whole words, books, languages,
grades, chapters, paging, wildcards, near spellings, the recent searches; the narrators of both traditions and the
scholars by name), `iy-rs chain` (every link, its narrator and his
grade, whom the chain reaches, the chain in Turkish and English), `iy-rs book` (the shelf, a book's chapters, a
chapter, your places and markers in each book), `iy-rs narrator` (by any spelling, his verdicts, teachers, students
and hadith), `iy-rs life` (the life of a Companion, a narrator or a scholar), `iy-rs isnads` (the chains of
every book read and their narrators identified, as the Python isnady does it), `iy-rs scholar` (the scholars and what the data measures of their work, al-Dhahabi's huffaz),
`iy-rs group` (the Companions and the Household), `iy-rs sira` (the life of the Prophet ﷺ as a timeline),
`iy-rs tree` (the genealogies), `iy-rs bookmark`, `iy-rs term` and `iy-rs learn` (the glossary, History and method),
`iy-rs sahifa` (the written sources), `iy-rs quran` (the Qur'an with its translations and the hadith that name each
verse), `iy-rs stats` (every statistic, the graders, the hadith model, the laboratory, in plain words too), `iy-rs config` and `iy-rs profile` (the settings and the profiles), `iy-rs cache`, `iy-rs log`, `iy-rs reset
settings`, `iy-rs shortcut`, `iy-rs sources`, `iy-rs datadir` (show it, or move it) and `iy-rs -V`.
A search takes about 60 ms, a chain about 30 ms. **The window, `isnady-rs`,** has begun: the menus and the sidebar of the Python window, its Search page as it
is (filters, cards, chains, bookmarks, the narrators and the scholars), its Narrators page as it is (the list, the
filters, the name card, the life, the critics, teachers and students), its Isnad Chains page as it is (the numbers of a
book, its contents, the overview, every chain node by node), Books (the shelf, the contents, a chapter, markers),
Hadith Scholars (with the huffaz), Companions and the Household, Sira, Written sources, Genealogies, Bookmarks and
Learn, Statistics, the Qur'an in Books, Edit → Preferences and Help → Log, in English, Turkish or Arabic, in every theme of the Python window; the other pages come
one at a time. Each of these prints what `iy` prints, word for word; `tools/parity.py` checks it on
every push and before every release. Everything else below is still done by the Python isnady, on the same data. The
ported parts are listed in [PARITY.md](PARITY.md), and more are ported with every release.

**Get it:**

| Where | How |
|---|---|
| [crates.io](https://crates.io/crates/isnady) | `cargo install isnady`, which gives `iy-rs` |
| [Releases](https://github.com/bayramkotan/isnady-rs/releases/latest) | `iy-rs` for Windows, Linux and macOS (Apple silicon and Intel), ready to run: unpack it and run `iy-rs` |
| The source | `cargo build --release`, which puts `iy-rs` and the window `isnady-rs` in `target/release/`; or `cargo install --path crates/isnady-gui` for the window alone (Linux needs `webkit2gtk-4.1`) |

`iy-rs catalog import --all` (or Import all in the window's Data sources) imports all of isnady-data itself — every
hadith book, the written sources, the rijal works, the sira, the tabaqat and the Qur'an — then builds the search index,
the books' chapters, the chains and their narrators: the same rows the Python isnady writes. The other repositories'
formats (fawazahmed0, tables in CSV or JSON) are read too.

---

