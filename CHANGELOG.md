# Changelog

isnady-rs carries the same version number as the Python isnady (github.com/bayramkotan/isnady): each release of one
is a release of the other. The text of each GitHub Release is taken from its section here.

## [Unreleased]

- **The Search page as the Python window has it (RS-WIN2; Bayram: "iy ile iy-rs birbirinin aynısı olacak! Temalardan
  ayarlardan ... arayüzden ... hepsi aynı olacak").** The page is built again from gui/search_page.py, widgets.py and
  chain_widgets.py, piece by piece, and compared with the Python window side by side: ⌂ back to the start (Alt+Home);
  what is searched (Hadith, Narrators, Scholars — the last two say they come next); the words with their clear
  button; Match (all words, any word, exact phrase, by meaning); Whole words only; the books, ticked by tradition
  (Sunni, Shia, other) with All, Sunni, Shia, None; the grades in the Sunni and the Imami scales with "every grader
  agrees" and "count al-Bukhari's and Muslim's hadith as sahih"; the languages, and those the data repository has
  more of; a chapter chosen; the count and time with the grades' and the chapter's share; Export these results. The
  welcome: the verse, the examples, the recent searches and Clear. Nothing found says why (other spellings, the
  grades, the chapter). Each hadith as its card: the book in the interface's language, ☆ (the same bookmarks.json as
  Python), Export, In its book, View chain; Also narrated in (takhrij) and Verses it names; the chain right to left,
  the compiler at its head, each narrator with his verdict, tabaqa and death on the hover and the words in Turkish and
  English under him, whom it reaches; the grades (with the glossary's words), "In the Sahihayn: sahih"; the texts in
  the content languages with the matches gilded, the others folded; the right-click menus of a card, a text and a
  name. 25 more at a time; the cards come a few at a time so the window never stops.
- **The look of every theme from the Python isnady.** data/window.json now carries every token of every theme
  (colours, typefaces, roundness, room), the reader's typefaces of each script, the rank colours, the compilers,
  whom a chain reaches in Arabic and the catalog's language names; the window draws corners and room as
  theme.stylesheet scales them, sizes in points as Qt does, the interface's typeface as Qt takes it, each text's line
  height as a share of its typeface's own, and Qt-like tooltips. The typefaces of the friendlier themes (Nunito,
  Lexend, Baloo 2, Noto Naskh Arabic) are bundled as Python bundles them (SIL OFL).
- **`iy-rs search` filters (GR-FLT, SRCH-SEC1, S4):** `--grade` (sahih, hasan, daif, very-weak, fabricated, ungraded;
  imami-sahih, imami-hasan, muwaththaq, imami-daif, imami-ungraded), `--every-grader`, `--no-sahihayn`, `--chapter
  BOOK:CHAPTER`, `--recent`, `--clear-recent`, `iy-rs search` with no words; `--in` and `--tradition` taken. New in the
  core: `grades.rs`, `bookmarks.rs`, `quran.rs`, `takhrij.rs`, `works::hadith_under`, `books::more_languages`,
  `search::recent_items`/`clear_recent`, `config::get`. 12 more parity cases.

- **The frame of the window, as the Python window has it (RS-WIN1; Bayram, 2026-11-04: "iy-rs de neden iy deki gibi
  menu yok? … iy ile iy-rs birbirinin aynısı olacak! Temalardan ayarlardan ... arayüzden, prosedürlerden … hepsi aynı
  olacak").** The menu bar: File, Edit, View, Tools and Help with every item of the Python window, in its order, with
  its separators, its shortcuts and its letters (Alt+D opens Dosya in Turkish). What isnady-rs already does works
  there: Open Data Folder, Profiles (switching), Quit; Find, Copy, Back, Forward; every section (Ctrl+1 … Ctrl+9),
  Interface Language (English and the translated languages first, the others in their own submenu, each with its
  name in the interface language and how much of it is translated), Content Languages (ticked in place, applied when
  the menu closes), Theme (Follow system = Sky by day, Starry Night by night, and the ten themes), Reading Text Size
  (Ctrl++, Ctrl+−, Ctrl+0), Full Screen (F11); Search Tips, Keyboard Shortcuts, Licences, isnady on GitHub, Report an
  Issue, About isnady. What it does not do yet (Data Sources, Export, Import, Preferences, the Tools, Check for
  Updates, Check Installation, Log, making or renaming profiles) says so and names its place in the Python window.
  The sidebar: Back and Forward (◀ ▶, Alt+Left, Alt+Right, the mouse's own buttons; sections, chains, books,
  chapters and narrators), the sections in the order chosen (view.sidebar) with their signs, the Interface and
  Content buttons with their menus, the footer "N hadith in M books / version X". Every choice is written to the
  same settings.json in the same form as `config.set` writes it (byte for byte), so the Python window opens with
  it. The sections, themes, languages, shortcuts and search tips come from the Python isnady through
  `tools/gen_tables.py` into `data/window.json`, read from its source without Qt; `--check` keeps them its own. New
  in the core: `config::set`, `state_forget`, `all_profiles`, `switch_profile`, `i18n::languages`.

- **The narrators (RS4; Bayram, 2026-11-04: "bir sonraki problemi çöz (Rust için)").** `iy-rs narrator` as `iy
  narrator`, word for word: a name in Arabic or Latin letters (the consonant skeleton, "Zuhri" = الزهري), close
  spellings ("Buhaari", "zuberyyyr", زبيير), the famous ones by any spelling people type ("Ebubekir", "Omer ibn
  Hattap", "Hz. Ali") first, `--famous` group by group, `--tradition`, `--limit`; each with his other names, tabaqa,
  death, every verdict (Ibn Hajar's rank, al-Najashi's category and school), the book marks, and for one narrator his
  teachers, students, the compilers who narrate from him and his hadith. New in the core: `people.rs`. The window's
  **Narrators** page: the famous ones group by group, the search as you type, a narrator with everything above, his
  teachers and students to open, his hadith to open as chains. Parity: 64 cases (14 of them narrators); 284 lookups
  on the test database, the same as Python's.

- **The books (RS3; Bayram, 2026-11-03: "bir sonraki aşamayı yap rust ile").** `iy-rs book` as `iy book`, word for
  word: the shelf (`--tradition`), a book's chapters in reading order, a chapter with its hadith in the languages asked
  for, the sira's and the tabaqat's sections wrapped as Python's textwrap wraps them, and the reader's places and
  markers (`--resume`, `--mark`, `--marks`, `--go`, `--unmark`, `--forget`) in the same state.json. `iy-rs chain BOOK
  --chapter CH` lists a chapter's hadith with how far each chain was read. New in the core: `works.rs` (with the book
  and chapter titles and the tables of contents of Musnad Ahmad and al-Darimi, from the Python isnady through
  `tools/gen_tables.py` into `data/books.json`), `readmarks.rs`, and `py::fill` (Python's textwrap). The window's
  **Books** page: the shelf, a book's contents, a chapter to read with View chain, previous and next, where you left
  each book. Parity: 50 cases (15 of them books); every book of the test database with three chapters each, the
  same as Python's.

- **The window: `isnady-rs` (RS6 begins; Bayram, 2026-11-03: "GUI ne zaman olacak?" → "başla").** A new crate,
  `isnady-gui`, with Tauri 2: the pages are drawn by the system's web view (WebView2 on Windows, WebKitGTK on Linux,
  WebKit on macOS), so Arabic, right to left and with its vowels, renders exactly, in Amiri. The sidebar of the Python
  window in its order; **Search** and **Isnad Chains** work, word for word with the same core as `iy-rs`; the other
  pages say they come one at a time and that the Python window has them meanwhile. The interface follows the settings
  of the Python isnady: its language (English, Turkish, Arabic right to left — the same catalogs, written by
  `tools/gen_tables.py`), its theme (all eleven) and the languages of the texts.
- `tools/gen_tables.py` also writes the interface catalogs (`crates/isnady-core/data/i18n/<code>.json`) and checks
  them; `tools/bump_version.py` bumps `isnady-gui` too. The workflows install WebKitGTK on Linux; a Release carries
  `isnady-rs` beside `iy-rs`.

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
