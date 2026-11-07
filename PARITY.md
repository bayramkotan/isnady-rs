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
| `iy search --grade …`, `--every-grader`, `--no-sahihayn` | ✅ | the Sunni and the Imami scales, the two Sahihs as sahih, "N found by the words in all" |
| `iy search --chapter BOOK:CHAPTER` | ✅ | by number or the start of the title, the chapters under it too |
| `iy search --recent`, `--clear-recent`, `iy search` with no words | ✅ | |
| `iy search --in narrators`, `--tradition`; `iy search --in scholars` | ✅ | both traditions or one, the closest first: the famous ones by any spelling, a scholar known by the name typed, the name he is known by, close spellings, the same consonants read otherwise (with difflib's measure, letter for letter); `--offset`/`--limit` past the first 400. The scholars by a name or a work, in any spelling |
| `iy search --mode meaning` | ⏳ | |
| `iy chain BOOK`, `iy chain BOOK NUMBER` | ✅ | `--raw`, `--lang`. Every link with its term, the narrator Ibn Hajar's Taqrib or al-Najashi identifies (tabaqa, year of death, the first verdict), tahwil meetings, groups and unnamed narrators of road B, whom the chain reaches, and the chain in words in Turkish and English. About 30 ms where `iy` takes seconds. |
| `iy chain BOOK --chapter CH` | ✅ | the hadith of a chapter with how far each chain was read |
| reading the chains, identifying the narrators (`iy isnads`) | ⏳ | the chain reader (isnady-isnad-15) and the narrator matcher (isnady-narrators-17) stay with the Python isnady for now: `iy-rs chain` shows what it stored, and asks for one `iy chain` after books are imported |
| `iy narrator NAME`, `iy narrator --famous` | ✅ | `--limit`, `--tradition`: Arabic or Latin letters (the consonant skeleton), close spellings, the famous ones by any spelling people type ("Ebubekir", "Omer ibn Hattap", "Hz. Ali"); every verdict, tabaqa, death, the book marks, teachers, students, the compilers and the hadith |
| `iy scholar`, `iy life`, `iy tree`, `iy group` | ⏳ | |
| `iy book`, `iy book KEY`, `iy book KEY CHAPTER` | ✅ | the shelf (`--tradition`), a book's chapters in reading order, a chapter (`--language`, `--limit`, `--width`; the sira and the tabaqat wrapped as Python's textwrap does), the reader's places and markers (`--resume`, `--mark`, `--marks`, `--go`, `--unmark`, `--forget`, in the same state.json). The trees of newly imported books are built by the Python isnady (one `iy book`). |
| `iy quran`, `iy sira`, `iy sahifa` | ⏳ | |
| `iy stats …` | ⏳ | |
| `iy catalog …`, `iy repo …`, `iy import …`, `iy database …`, `iy remove` | ⏳ | the database is imported by the Python isnady |
| `iy export`, `iy import-data`, `iy bookmark`, `iy config`, `iy profile`, `iy learn`, `iy term`, `iy ai`, `iy cache`, `iy log`, `iy reset`, `iy update`, `iy doctor` | ⏳ | |

## The window

| | isnady-rs |
|---|---|
| The window itself: `isnady-rs` (Tauri 2) | ✅ the sidebar of the Python window in its order, its themes (system, light, dark, paper, slate, emerald, midnight, contrast, garden, sky, starry), the interface in English, Turkish and Arabic (right to left) from the same catalogs, Amiri for the Arabic. The pages are drawn by the system's web view, so Arabic renders exactly. |
| The menu bar and the sidebar | ✅ File, Edit, View, Tools, Help with every item, separator, shortcut and letter of the Python window; Back and Forward; the sections in the order chosen with their signs; the Interface and Content buttons; the footer. Working: Open Data Folder, Profiles (switching), Quit, Find, Copy, Back, Forward, the sections, Interface Language, Content Languages, Theme, Reading Text Size, Full Screen, Search Tips, Keyboard Shortcuts, Licences, the links, About. ⏳ (each says so and where it is in the Python window): Data Sources, Export, Import, Preferences, Choose with previews, the Tools menu, Check for Updates, Check Installation, Log, New/Rename/Delete/Export/Import a profile |
| Search | ✅ as the Python page draws it: ⌂, what is searched (Hadith, Narrators of both traditions or one, Scholars), the words, Match, Whole words only, the books (Sunni, Shia, other; ticked), the grades (Sunni and Imami scales, every grader agrees, the two Sahihs), the languages (and those the data repository has more of), the chapter chosen, the count and time, the welcome with its examples and the recent searches, nothing found (and why), 25 more at a time. Each hadith as its card: ☆ bookmark (the same bookmarks.json), In its book, View chain, Also narrated in, Verses it names, the chain right to left with every narrator, his verdict and the words in Turkish and English under each name, the grades, the texts in the content languages (the others folded), the right-click menus. Each narrator and scholar found as his card (the name he is known by and its Arabic, his full name, tradition, rank with its Arabic, tabaqa, death, how often in the chains, close spelling or weaker match with the words matched, Open in Narrators; a scholar's roles and works), the weaker matches under their own heading, more as asked for. ⏳ (each says so): Export, By meaning, the Hadith Scholars page a scholar opens, Language settings |
| Isnad Chains | ✅ a book and a number, previous, next, random; every chain link by link as the Python page draws it: the term, the name as written and in the rijal work, its reading, the critic's verdict with Ibn Hajar's rank (or al-Najashi's category), tabaqa, year of death, tahwil meetings, al-Kulayni's groups, whom it reaches, raised and cut chains, and the chain in words in Turkish and English. The contents picker of a book: ⏳ |
| Books | ✅ the shelf (Sunni, Shia, all; by kind), a book's contents, a chapter to read (hadith with their texts and View chain, rijal entries, the sira's sections), previous and next chapter, where you left each book. Markers, search in a chapter, PDF: ⏳ |
| Narrators | ✅ the famous ones group by group (Sunni, Shia, all), the search as you type, a narrator with his names, every verdict (Ibn Hajar's rank, al-Najashi's category), tabaqa, death, books, teachers and students (open them), the compilers and his hadith (View chain). Genealogy, filters by tabaqa/rank/book: ⏳ |
| Hadith Scholars, Companions, Sira, Written sources, Genealogy, Statistics, Bookmarks, Learn; Settings; Data sources | ⏳ one page at a time; until then the Python window shows them, on the same data |

## The core

| Module (Python) | isnady-rs | |
|---|---|---|
| `isnady.data.paths` | ✅ `paths.rs` | |
| `isnady.core.normalize` | ✅ `normalize.rs` | same rules, `NORMALIZER_VERSION` 1: the search index the Python isnady builds is read as it is |
| `isnady.core.search` | ✅ `search.rs` | the grades and chapters filters too (`grades.rs`, `works::hadith_under`) |
| `isnady.core.grades` | ✅ `grades.rs` | |
| `isnady.core.bookmarks` | ✅ `bookmarks.rs` (is it marked, mark, unmark) | folders: with the Bookmarks page |
| `isnady.core.quran` (marks_in, cites, sura names) | ✅ `quran.rs` | reading the Qur'an: with the Qur'an in Books |
| `isnady.core.takhrij.related` | ✅ `takhrij.rs` | |
| `isnady.core.books` (order, active books) | ✅ `books.rs` | |
| `isnady.config` (settings, state), `isnady.core.profiles` | ✅ `config.rs` (reading, the recent searches) | |
| `isnady.data.db` | ✅ reading schema 8 (`db.rs`) | creating and upgrading stay with the Python isnady |
| `isnady.core.isnad` (the stored chains, reading aids, the chain in words), `tahwil_meet.meeting` | ✅ `isnad.rs` | the parser itself: ⏳ |
| `isnady.core.names` (names in Latin letters), `names_lexicon` | ✅ `names.rs` | the word list itself is read from the Python isnady (`data/tables.json`) |
| `isnady.core.narrators.describe`, `known_aliases`, `links_ready`; `rijal.display_name`; `name_parts` (the name he is known by) | ✅ `narrators.rs` | the matcher: ⏳ |
| `isnady.core.traditions`, `companions` (what a chain needs of them) | ✅ `isnad.rs` | |
| `isnady.core.works`, `titles`, `chapters` (the reader's tree, book and chapter titles in the interface language, the tables of contents of Musnad Ahmad and al-Darimi) | ✅ `works.rs` | building the trees: ⏳ |
| `isnady.core.readmarks` | ✅ `readmarks.rs` | |
| `isnady.core.narrators` (search_persons, describe, relations, hadiths_of), `famous` | ✅ `people.rs` | the famous ones' spellings come folded from the Python isnady (`data/tables.json`) |
| `isnady.core.narrators` (browse, find, find_more, _closeness, _part_closeness) | ✅ `narrators.rs` | `difflib.SequenceMatcher.ratio` ported letter for letter (`difflib.rs`) |
| `isnady.core.name_parts` (parse, present) | ✅ `name_parts.rs` | |
| `isnady.core.scholars` (the register, roles, search, named, as_narrator, short_name, name_view) | ✅ `scholars.rs` | the register is read from the Python isnady (`data/tables.json`); the measures of their work: with the Hadith Scholars page |
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
- **The same word lists.** The names list, the Companions' names, the Ma'sumun, the transmission terms, the
  Turkish of the chain's words and the patterns that cut a name are not copied by hand: `tools/gen_tables.py` writes
  them from the Python isnady into `crates/isnady-core/data/tables.json`, and the Parity workflow checks with
  `--check` that they are still the Python isnady's.
- **The same README.** isnady-rs's README is isnady's README, word for word, under a head of its own
  (`tools/sync_readme.py`).
- **The same places.** The Python isnady is on PyPI and npm. isnady-rs is on crates.io (`cargo install isnady`) and
  in GitHub Releases. Both are published by their workflows for every tag.
