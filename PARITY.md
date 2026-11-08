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
| `iy isnads` | ✅ | the report of every book as `iy isnads` prints it: chains read, split into narrators, reaching the Prophet (on road B a Ma'sum), links, why the rest were kept whole, then how many narrators of the chains are identified |
| reading the chains, identifying the narrators (`iy isnads --rebuild`) | ⏳ | the chain reader (isnady-isnad-15) and the narrator matcher (isnady-narrators-17) stay with the Python isnady for now: `iy-rs chain` and `iy-rs isnads` show what it stored, and ask for one `iy isnads` after books are imported |
| `iy narrator NAME`, `iy narrator --famous` | ✅ | `--limit`, `--tradition`: Arabic or Latin letters (the consonant skeleton), close spellings, the famous ones by any spelling people type ("Ebubekir", "Omer ibn Hattap", "Hz. Ali"); every verdict, tabaqa, death, the book marks, teachers, students, the compilers and the hadith |
| `iy life NAME` (`--lang`, `--tradition`, `--limit`, `--source`) | ✅ | the lives written for isnady (the same text, carried in data/lives.json) and every other narrator's life read from the data; `--source` prints the classical entry once the Python isnady has downloaded the work (downloading: ⏳) |
| `iy scholar [NAME]`, `--tradition`, `--huffaz`, `--tabaqa` | ✅ | the scholars in the data by role; one scholar with his books, grades and verdicts measured (agreement, Cohen's kappa, strictness by order; the Imami terms on road B); al-Dhahabi's huffaz by generation, by name, one hafiz with his Taqrib entry. `--source` (the Tadhkira's text): ⏳ |
| `iy group [ID]` | ✅ | `--tradition`, `--lang`, `--width`: the ten, the people of the cloak, the Mothers of the Believers, the Fourteen Infallibles, with what the imported books hold of each member |
| `iy tree prophets, quraysh, family, learning [NAME]` | ✅ | `--depth`, `--width`, `--hadith`: the written trees, a box found with its line and notes, the hadith that name a prophet (the two roads apart), a narrator's family read from the names, his line of learning |
| `iy book`, `iy book KEY`, `iy book KEY CHAPTER` | ✅ | the shelf (`--tradition`), a book's chapters in reading order, a chapter (`--language`, `--limit`, `--width`; the sira and the tabaqat wrapped as Python's textwrap does), the reader's places and markers (`--resume`, `--mark`, `--marks`, `--go`, `--unmark`, `--forget`, in the same state.json). The trees of newly imported books are built by the Python isnady (one `iy book`). |
| `iy sira [EVENT]` | ✅ | `--lang`, `--width`: the timeline by period; an event with its date, place, age, verses, Ibn Hisham's chapter and the hadith that say its phrase |
| `iy quran`, `iy sahifa` | ⏳ | |
| `iy stats …` | ⏳ | |
| `iy catalog …`, `iy repo …`, `iy import …`, `iy database …`, `iy remove` | ⏳ | the database is imported by the Python isnady |
| `iy bookmark list, folders, add, remove, export, import` | ✅ | `--folder`, `--title`, `--note`; the same bookmarks.json |
| `iy term [NAME]`, `iy learn [ARTICLE]` | ✅ | `--search`, `--lang`, `--width`; the articles' numbers counted live |
| `iy export`, `iy import-data`, `iy config`, `iy profile`, `iy ai`, `iy cache`, `iy log`, `iy reset`, `iy update`, `iy doctor` | ⏳ | |

## The window

| | isnady-rs |
|---|---|
| The window itself: `isnady-rs` (Tauri 2) | ✅ the sidebar of the Python window in its order, its themes (system, light, dark, paper, slate, emerald, midnight, contrast, garden, sky, starry), the interface in English, Turkish and Arabic (right to left) from the same catalogs, Amiri for the Arabic. The pages are drawn by the system's web view, so Arabic renders exactly. |
| The menu bar and the sidebar | ✅ File, Edit, View, Tools, Help with every item, separator, shortcut and letter of the Python window; Back and Forward; the sections in the order chosen with their signs; the Interface and Content buttons; the footer. Working: Open Data Folder, Profiles (switching), Quit, Find, Copy, Back, Forward, the sections, Interface Language, Content Languages, Theme, Reading Text Size, Full Screen, Search Tips, Keyboard Shortcuts, Licences, the links, About. ⏳ (each says so and where it is in the Python window): Data Sources, Export, Import, Preferences, Choose with previews, the Tools menu, Check for Updates, Check Installation, Log, New/Rename/Delete/Export/Import a profile |
| Search | ✅ as the Python page draws it: ⌂, what is searched (Hadith, Narrators of both traditions or one, Scholars), the words, Match, Whole words only, the books (Sunni, Shia, other; ticked), the grades (Sunni and Imami scales, every grader agrees, the two Sahihs), the languages (and those the data repository has more of), the chapter chosen, the count and time, the welcome with its examples and the recent searches, nothing found (and why), 25 more at a time. Each hadith as its card: ☆ bookmark (the same bookmarks.json), In its book, View chain, Also narrated in, Verses it names, the chain right to left with every narrator, his verdict and the words in Turkish and English under each name, the grades, the texts in the content languages (the others folded), the right-click menus. Each narrator and scholar found as his card (the name he is known by and its Arabic, his full name, tradition, rank with its Arabic, tabaqa, death, how often in the chains, close spelling or weaker match with the words matched, Open in Narrators; a scholar's roles and works), the weaker matches under their own heading, more as asked for. ⏳ (each says so): Export, By meaning, Language settings |
| Isnad Chains | ✅ as the Python page draws it: the book list (Sunni and Shia apart under their headings), 📖 From the contents… (the chapters as a tree, the hadith of a chapter with the start of their text and how far each chain was read, a word looked for among them), Random hadith; the number with the book's last number, the numbers that begin with what is typed, the arrow keys and the mouse wheel, Show chain, ⏮ ‹ › ⏭ and the hadith's place in the book; a number the book does not have (how it writes its numbers, its range, the nearest); the overview (per book: chains read, split, reaching the Prophet or a Ma'sum, narrators per chain, identified, and why the rest were kept whole; a book opens its first chain); a hadith: what the page says first about it, each chain as a timeline from the compiler's book (opening it in Books) to whom it reaches, node by node (the term and its meaning, the name as written and its reading, the narrator in the rijal work with his verdict, rank, tabaqa and death — opening him on Narrators — or why he is not identified, a name to look for on Search), tahwil meetings, al-Kulayni's groups, raised and cut chains, the Arabic with the chain in lighter ink, the chain in words in Turkish and English. The Prophet ﷺ or an Imam at its end opens the Genealogies |
| Books | ✅ as the Python page draws it: the shelf (Sunni, Shia, all; by kind), a book's contents as a tree, a chapter read in its own order (the hadith as Search draws them, the sira's sections, the lives of the tabaqat, the rijal entries, cross-references; more as you read), ⏮ ‹ › ⏭ through the chapters, the hadith number, where you left each book, 🔖 Markers, Search in this chapter, the right-click menus; a hadith, a person or a chapter opened at its place from the other pages. ⏳ (each says so): the Qur'an reader, PDF |
| Hadith Scholars | ✅ as the Python page draws it: All / Sunni / Shia / Huffaz, the scholars by role; a scholar's name card, life, books, his collection, grades and agreement, verdicts, as bars; al-Dhahabi's huffaz by generation, the book at a glance, a hafiz with his Taqrib entry and his entry in the Tadhkira |
| Companions and the Household | ✅ as the Python page draws it: Sunni / Shia, the groups, a group's members two to a row (In the family tree opens the Genealogies) and the hadith that name it |
| Sira | ✅ as the Python page draws it: the timeline, the events by period, an event with its chips, text, verses, Ibn Hisham's chapter and the hadith of its phrase, ‹ ›. ⏳ (it says so): the Qur'an reader a verse opens |
| Genealogies | ✅ as the Python page draws it: the prophets, Quraysh, a narrator's family, his line of learning — dragged, zoomed, clicked; the box's names, line, notes, life, the hadith that name a prophet, ☆; find, ★ the famous Companions |
| Bookmarks | ✅ as the Python page draws it: the folders, the bookmarks with their kind, detail, folders and notes, dragging onto a folder, every menu, Open where it belongs. ⏳ (each says so): Export / import (File → Export, Import) |
| Learn | ✅ as the Python page draws it: the glossary in colour in three languages, the chips, the list, a term's card; History and method with its live numbers |
| Narrators | ✅ as the Python page draws it: All / Sunni / Shia, the overview of each road, the search as you type (the famous ones by any spelling first), ★ the famous ones group by group, the filters (generation, Ibn Hajar's rank or the Imami assessment, the six books by Ibn Hajar's marks) with Clear filters, the list 200 at a time and the next ones at its end, its right-click menu; a narrator: the name card (the name he is known by, the parts of his name, ☆), his life (written, or read from the data; the classical entry when downloaded), what the critics say (the entry as written, every verdict with the glossary on the hover, tabaqa, death, Open his entry in the book, Ibn Hajar's marks opening their books), teachers and students, the compilers, the hadith (each opens its chain). ⏳ (it says so): downloading a classical work |
| Written sources, Statistics; Settings; Data sources | ⏳ one page at a time; until then the Python window shows them, on the same data |

## The core

| Module (Python) | isnady-rs | |
|---|---|---|
| `isnady.data.paths` | ✅ `paths.rs` | |
| `isnady.core.normalize` | ✅ `normalize.rs` | same rules, `NORMALIZER_VERSION` 1: the search index the Python isnady builds is read as it is |
| `isnady.core.search` | ✅ `search.rs` | the grades and chapters filters too (`grades.rs`, `works::hadith_under`) |
| `isnady.core.grades` | ✅ `grades.rs` | |
| `isnady.core.bookmarks` | ✅ `bookmarks.rs` | the whole store: folders, paths, filing, merging, the references of every kind |
| `isnady.core.sira` | ✅ `sira.rs` | the events from the Python isnady (`data/sira.json`) |
| `isnady.core.genealogy` | ✅ `genealogy.rs` | the written trees and the words for the prophets from the Python isnady (`data/genealogy.json`) |
| `isnady.core.learn`, `learn_history` | ✅ `learn.rs` | the terms and articles from the Python isnady (`data/learn.json`) |
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
| `isnady.core.lives` (written, view, read, life, find), `source_entries` (reading an entry), `huffaz.of_key` | ✅ `lives.rs` | the lives come from the Python isnady (`data/lives.json`); downloading a work: ⏳ |
| `isnady.core.narrators.overview`, `browse`'s filters | ✅ `narrators.rs` | |
| `isnady.core.scholars` (the register, roles, search, named, as_narrator, short_name, name_view, present, compiler_stats, grader_stats, critic_stats) | ✅ `scholars.rs` | the register is read from the Python isnady (`data/tables.json`); kept while the database is unchanged (`cache.rs`, as core/cache.py in memory) |
| `isnady.core.huffaz` | ✅ `huffaz.rs` | the index from the Python isnady (`data/huffaz.json`) |
| `isnady.core.sahaba` | ✅ `sahaba.rs` | the groups from the Python isnady (`data/sahaba.json`) |
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
