# Changelog

isnady-rs carries the same version number as the Python isnady (github.com/bayramkotan/isnady): each release of one
is a release of the other. The text of each GitHub Release is taken from its section here.

## [Unreleased]

- **PDF, pictures, Excel and the classical works (A5, the rest).** isnady's own PDF writer (`pdf.rs`): Amiri
  embedded (SIL OFL, in `data/fonts`), the Arabic shaped by rustybuzz and set in the bidirectional order, lines
  broken by width, tables with their columns fitted, the text searchable and copyable, pictures placed as JPEG. With
  it `iy-rs export … --format pdf` writes every kind the Python isnady writes as PDF (hadith, a book or chapter,
  search results, bookmarks, narrators), and in the window File → Export… and Books' ⤓ PDF. Statistics: every
  finding's ⤓ as PDF, a picture (PNG) and a vector picture (SVG), its figures as Excel; the section or every section
  as PDF, the section as one picture, every table as Excel (a sheet each) — each finding drawn as the page draws it.
  `iy-rs life … --source` and `iy-rs scholar --huffaz … --source` download the classical work once (from isnady-data,
  else the OpenITI corpus) into the same `sources/<work>.db` as the Python isnady's (the same rows), and the
  window's lives download it on a click. `tools/parity_export.py`: 114 cases (the PDFs written by both).

- **Doctor, update and the profiles in the window (A5, the first part).** `iy-rs doctor [--fix --yes --offline]` and
  `iy-rs update [--check --yes]`: every copy cargo installed (from crates.io, a clone or git, as `.crates2.json`
  records them) and every iy-rs and isnady-rs on PATH with the one that starts, the newest on crates.io and what is
  new in it (isnady-rs's CHANGELOG), and each copy brought up to date where it is; nothing runs without a yes. On
  Windows the running program steps aside so cargo can write the new one. In the window: Help → Check for Updates…
  and Check Installation… with Update (the commands' output shown), and File → Profiles → New, Rename, Delete,
  Export and Import, as the Python window has them.

- **Search by meaning, `ai` and `tahric` (A4).** `iy-rs search --mode meaning`, `iy-rs ai build|status` and
  `iy-rs tahric BOOK NUMBER`, as the Python isnady's; core/semantic.py and core/takhrij.py are in Rust (semantic.rs,
  takhrij.rs). The index is the Python isnady's own files (ai/lsa.npz, lsa.vocab, lsa.json), numpy's format read and
  written here: built by either program, read by both. On the Python isnady's index iy-rs finds the same hadith with
  the same scores in the same order; built by iy-rs, the takhrij pairs are the same rows (258,216 on the whole of
  isnady-data), the vocabulary, idf and TF-IDF rows the same, and the concepts the same subspace up to a small angle
  (scipy's ARPACK is not in Rust; a randomized subspace iteration finds it), so the same hadith come first. In the
  window: Match → By meaning (AI) with the closeness on each card, Build the meaning index now, Tools → Build AI
  Indexes (Meaning, Takhrij) in the background, the other narrations of each hadith on its card.
  `tools/parity_ai.py` checks all of it and runs in the Parity workflow (which now installs `isnady[ai]`).
- 4 more parity cases (245 in all).

- **Export and import (A3; Bayram: "Diğer tüm problemleri (eşitlemeleri) hallet").** `iy-rs export` and `iy-rs
  import-data`, with every argument of the Python isnady's: hadith by number, a book or a chapter (several books,
  all, sunni, shia; each in its own file with `--separate`), search results with the grades, books and chapter
  chosen, bookmarks, narrators, the isnad network, settings, a profile, a backup and the statistics, as JSON, JSON
  Lines, CSV, TSV, Excel, Markdown, HTML, text, EPUB, Word, SQLite, isnady book, GraphML and GEXF; a book, bookmarks,
  settings, a profile and a backup brought back. core/formats.py and core/transfer.py are in Rust (formats.rs,
  transfer.rs): the Excel, Word and EPUB files are written without a library, as the Python isnady writes them. In
  the window: File → Export… (Ctrl+E) with its preview, progress and Stop (the export goes on when the dialog is
  closed, its progress at the bottom of the window, then Show in folder), File → Import… (Ctrl+Shift+E), Export
  these results on the Search page, Export on a hadith's card and in its menu, ⤓ in Books (the book or the chapter
  being read), Export / import in Bookmarks, ⤴ Export… on a card of Data sources. PDF is the one format still to
  come (with the Statistics' PDF). `tools/parity_export.py` exports 111 cases with both programs and compares the
  files (the text ones byte for byte, the zipped ones entry by entry, the SQLite ones table by table), then imports
  them with both and compares what is said and kept; it runs in the Parity workflow.
- 4 more parity cases (241 in all).

- **Importing, the rest (A2b, A2c; Bayram: "ya lütfen hepsini çöz").** Every format the Python isnady reads is read
  here too: the rijal works (`taqrib`: Ibn Hajar's ranks, tabaqat, years of death with his rule, book marks and
  cross-references; `najashi`: the Shia scale, core/shia_rijal.py), the sira and the tabaqat (`openiti-work`, with
  the entries the written lives quote kept in sources/siyar.db and tadhkira.db), the Qur'an (`iy-rs quran download`,
  `iy-rs quran editions`, the Arabic and the translations in sources/quran.db, isnady-data's first, then
  fawazahmed0/quran-api's), and the other repositories' formats (`fawazahmed0`; `table`: CSV with Python's csv rules,
  or JSON records, in parts or volumes). core/rijal.py is in Rust (rijal.rs): the matcher reads the years of death
  again when the rule changed instead of asking for the Python isnady. In the window: the Qur'an's card and
  Translators… in Data sources, ➕ Translations…, Download {language} and Download the Qur'an in the reader.
  Imported by both programs into empty folders, the whole of isnady-data in Arabic gives the same database row for
  row (49 sources; 371,319 of 524,826 links identified) and the same Qur'an, Siyar and Tadhkira databases; the
  parity workflow imports the rijal, the Tadhkira, the Qur'an and two books of other repositories with both.
- 6 more parity cases (237 in all).

- **Importing (A2a; Bayram: "ok devam et").** isnady-rs imports the books of isnady-data itself, as the Python
  isnady does: the importers of the formats `isnady` (every hadith book, one file per language) and `isnady-sahifa`
  (the written sources) and core/imports.py — then the search index, the chains and the narrators, once.
  `iy-rs catalog import ID | --all`, `catalog update`, `catalog languages add`, `iy-rs import FORMAT LOCATION` with
  every option of `iy import` (the credentials: basic, bearer, an API key; asked, never stored), and in the window
  Preferences → Data sources: Import, Update, Import all, Get updates, Add to every book, in the background. The
  books after the one being written are downloaded meanwhile; an import cut short is said at the next start. A
  database that is not there yet is made, an older one upgraded, at the first open, as `db.connect` does. Imported
  by both programs into empty folders, the database is the same, table for table and row for row, and so is
  imported.json (the full isnady-data in Arabic: 43 sources, 99,263 texts; `tools/parity_db.py --import`, a step of
  the parity workflow). The rijal works, the sira and the tabaqat, the Qur'an and the other repositories' formats
  come next, and say so. New dependency: rpassword (a password asked without showing it).
- 5 more parity cases (231 in all).

- **The search index and the books' trees (RS17c, A0 + A1; Bayram: "tamam da eşitle artık").** isnady-rs builds
  them itself, as the Python isnady does: search.ensure_index and rebuild_index (the normalised texts, the FTS5
  trigram index, the word list, the stamp — the texts normalised on every core) and works.ensure_collection_works
  with the tables of contents of core/chapters.py. `iy-rs search` indexes what is not indexed yet, `iy-rs book`
  and the Books page build the trees of the books imported last; the window indexes, reads the chains and
  identifies the narrators after a new version, in that order. Built again from nothing by both programs on the full
  isnady-data, the index (199,271 texts, 327,227 words) and the trees (48 works, 127,796 nodes) are the same rows.
- **Tools → Rebuild Search Index, Read Chains Again, Identify Narrators Again** work, in the background, saying what
  they do at the foot of the window. The links to the data sources on the Sahifa page and in the languages of Search
  open Preferences → Data sources.
- `tools/parity_db.py` builds the search index and the trees again in both copies and compares them too.

- **The chain reader and the narrator matcher (RS17; Bayram: "önce tam yakala").** isnady-rs reads the chains from
  the Arabic texts and identifies their narrators itself, as the Python isnady does: core/isnad.py (isnady-isnad-15:
  both roads, the tahwil, the second roads, the stories after a chain, the books that cite without chains, "بهذا
  الإسناد" drawn beside the road) and core/narrators.py (isnady-narrators-17: the name, the book's mark, the
  generations, the relatives, the teacher and the student, the client, the bare name, the famous one, the impossible
  pairs) with core/tahwil_meet.py, line for line. Every chain of the full isnady-data read again by both programs is
  the same row in the database — 114,047 chains, 525,733 links, the narrator, the confidence and the candidates of
  each, the 345 roads joined at a tahwil — and the narrators are identified in half the time. The patterns are the
  Python isnady's own, read as Python's `re` reads them (`reader/pyre.rs`: `\w`, `\s`, `$`, positions in
  characters), written to `data/isnad_parser.json` by tools/gen_tables.py. `iy-rs isnads [--rebuild]` and
  `iy-rs chain` read what is not read yet; the window does it once after a new version, in the background, saying
  so at its foot (search_page._CatchUp). The books are read on every core.
- `tools/parity_db.py` (and a step of the parity workflow): the chains and narrators read by both programs must be
  the same rows; 3 more parity cases (226 in all).

- **Data sources, as the Python window has them (RS16a; Bayram: "geri kalanları da hallet").** From
  gui/sources_page.py, in Preferences → Data sources and File → Data Sources…: the repository chosen (isnady-data by
  default, the datasets others publish, those you add by address or folder) with where its catalog came from; its
  books by kind (the Qur'an, hadith, written sources, sira, tabaqat, rijal) and road, each with its tags, what it is,
  its languages and what of it is imported; Remove data, Open in Books; what is new or changed in the repository; one
  language taken out of every book; your own sources; the data folder and a ready database. Preferences → Reset now
  deletes the books and everything too. Importing says it comes with the chain reader (RS17).
- **`iy-rs repo`, `iy-rs catalog list | updates | languages | remove | add | delete`, `iy-rs formats`, `iy-rs database
  [use FILE]`, `iy-rs remove KEY`, `iy-rs reset books | all`** word for word as `iy`, with the same files
  (repos/*.json, user_repositories.json, user_sources.json, imported.json) and the same database afterwards. New in
  the core: `sources/` — the repositories (`core/repositories.py`), the catalog and its status (`core/catalog.py`),
  reading a file or a URL (`data/fetch.py`), the database written to, created and upgraded as `db.connect` does it,
  the ready database (`data/ready.py`), back to zero (`core/reset.py`). The schema, its upgrades, the import formats
  and the catalogs shipped with isnady are written from the Python isnady by `tools/gen_tables.py`
  (`data/sources.json`). New dependencies: ureq (HTTPS, with the system's certificates) and url.
- 16 more parity cases (223 in all).

- **Statistics, as the Python window has it (RS12; Bayram: "tamam devam et. geri kalanları bitir").** From
  gui/statistics_page.py and gui/stats_charts.py: the books chosen above (All books, the Sunni road, the Shia road, one
  book), the nine sections on the left — Overview, Narrators, Chains, Grades and chains, The hadith model, Correlations
  and tests, Books, Graders, Terms explained — each a few findings with the sentence that says what the numbers show,
  its plain words, its chart (tiles, stacked bars, columns, the Lorenz curve, people ranked, interval plots, heat maps,
  calibration), the figures on hover, How to read it and Save as CSV; Recompute; Export (a section or every section as
  a web page, every table as CSV files, the results as JSON; PDF, PNG and Excel say they come later); a narrator or a
  hadith opens where it belongs. The computation runs in a thread of its own, its steps under the head.
- **`iy-rs stats [corpus | narrators | chains | grades | books | model | lab | graders] [--book KEY | @shia | @all]
  [--grader] [--csv DIR] [--recompute] [--limit] [--explain [TERM]]`** word for word as `iy stats`, and plain `iy-rs
  stats` (what the database holds). New in the core: `stats/` — the corpus (`stats_corpus.py`), the graders with their
  Dawid–Skene model (`stats_graders.py`), the ordinal hadith model (`stats_models.py`), the laboratory (`stats_lab.py`),
  the plain words (`stats_words.py`), Python's Mersenne Twister (the same bootstrap samples), its compensated `sum`
  and its own `lgamma`: the same data gives the same figures, to the last digit. The results are kept where the
  Python isnady keeps them (derived/*.json, the same fingerprint, the same file byte for byte): either program reads
  what the other computed. serde_json now reads floats exactly (`float_roundtrip`). New dependency: md5 (the name of
  the results kept for a set of active books, as Python names it).
- 19 more parity cases (207 in all).

- **Written sources, the Qur'an, Preferences and profiles, and the small commands (Bayram: "ok. geri kalanları da
  bitir").** RS9, RS10, RS14 and RS18 at once, each compared with the Python window and with `iy`, word for word.
- **Written sources as the Python window has it (RS9).** From gui/sahifa_page.py: the Sunni road and the Shia road
  apart, the sources of each by layer (came down whole, lives in later books, writing in the chains) with how many
  hadith the imported books hold of each; a source with its road, its layer, what it is, how isnady finds it, Read it as
  a book in Books (or Import it, with Data sources); in the books imported here, book by book; its hadith as Search
  draws them, twenty at a time — Hammam's each with its parallels through his chain. The count runs once per change
  of the database in a thread of its own, a little after the window opens: the window is never held, and the page
  fills in by itself. **`iy-rs sahifa [ID] [--tradition] [--limit] [--lang] [--width]`** word for word as `iy sahifa`.
  New in the core: `sahifa.rs`; gen_tables writes data/sahifa.json.
- **The Qur'an in Books (RS10).** From gui/quran_view.py: the Qur'an first on the shelf, the suras as the contents, a
  sura with its title, verses, Meccan or Medinan and how many hadith name its verses; ‹ Previous sura, Next sura ›; the
  translations downloaded to tick (in the reader's languages first), the languages with no translation yet offered;
  every verse in Arabic and the translations chosen, and under it the hadith that name it — the Sunni books and the
  Shia books each on a line of its own, each opening in its book; a search through the texts shown. The verses a
  hadith names (Search, Books, the right-click menu) and the verses of an event of the Sira now open it, at the verse.
  Downloading the Qur'an and more translations comes with the data sources (RS16; it says so). **`iy-rs quran [SURA |
  S:A | S:A-B | search WORDS | editions | remove KEY] [--hadith] [--edition] [--lang] [--limit] [--width]`** word for
  word as `iy quran`. New in the core: the suras, editions, verses, the search and the translations shown
  (`quran.rs`).
- **Preferences (RS14).** From gui/preferences.py, Edit → Preferences (Ctrl+,): Themes (every theme drawn as a small
  isnady window), Reading text (each script's font, size, line spacing and colour, with a sample), Colours (the light
  and the dark theme each their own), Interface (font, size, theme), Sidebar (dragged or moved with the arrows),
  Books and languages (books switched on and off, the languages of the texts in their order, fold or leave out the
  others), Reset (the settings; the books go with the data sources). Every change is saved at once and the window
  redraws. The user's own colours of the Lapis pair now apply, as in the Python window. **`iy-rs config
  list|get|set|reset`**, **`iy-rs profile list|new|use|rename|delete|export|import`** and **`iy-rs datadir PATH
  [--as-is] [--default]`** as `iy`, on the same files (settings validated as `config.validate` does them).
- **The small ones (RS18).** Tools → Clear the Cache, Tools → Create Desktop Shortcut… (the window `isnady-rs`, with
  its icon, beside the Python one's), Help → Log… (the log in its own window: the files, followed live, a level, a
  text, larger or smaller, copied, its folder, what is older removed, cleared). The two programs now write one log
  (logs/isnady.log, the same line, the same rotation): every `iy-rs` command and every start of the window. **`iy-rs
  cache info|clear`**, **`iy-rs log`** (`-n`, `--level`, `--grep`, `-f`, `--crash`, `--files`, `--path`,
  `--before`, `--clear`), **`iy-rs reset settings`** and **`iy-rs shortcut`** as `iy`. Like `iy`, `iy-rs` says so
  when the database has no book yet.
- 24 more parity cases (188 in all).

- **Four pages more at once (Bayram: "Devam et. Önce ne kaldı onları listele sonra yukarıdaki gibi birçoğunu çöz").**
  Sira, Genealogies, Bookmarks and Learn, each built again from its Python page and compared with the Python window
  side by side; their commands print what `iy` prints, word for word.
- **Sira as the Python window has it (RS8).** From gui/sira_page.py: the life of the Prophet ﷺ on one line — the three
  periods as bands, every event a dot (stacked in the same year), the one chosen named above the line —, the events by
  period in the list, an event (its period, date, time of year, place, his age; the text; where the reports differ;
  the verses revealed about it; Read it in Ibn Hisham's Sira, when imported; the hadith that say its phrase, opening
  Search with the words in that order), ‹ › to the event before and after, Back and Forward. The verses open the
  Qur'an reader, which comes with RS10 (it says so). **`iy-rs sira [EVENT] [--lang] [--width]`** word for word as
  `iy sira`. New in the core: `sira.rs`; gen_tables writes data/sira.json.
- **Genealogies as the Python window has it (RS11).** From gui/genealogy_page.py: the prophets from Adam to Muhammad ﷺ,
  Quraysh and the Prophet's family, a narrator's family read from the narrators' names, a narrator's line of learning
  (teachers above, students below) — drawn as trees to drag, zoom (the wheel, − +, Whole tree) and click; dashed lines
  where generations are not known, dashed boxes where the reports differ; the box chosen: its names, kind, whose son,
  the children shown, how many generations from the root, the notes, his life (written for isnady), the hadith that
  name a prophet (the Sunni and the Shia books apart, each opening in its book), Open in Narrators, His family, His line
  of learning, ☆ the narrator and ☆ this place in the tree; find in a tree or a narrator, ★ the famous Companions. The
  links that waited for this page now open it: the Narrators page (his family, his line of learning, his place in the
  tree of Quraysh, and its right-click menu), the end of a chain on Isnad Chains and on Search, a member on Companions.
  **`iy-rs tree prophets|quraysh|family|learning [NAME] [--depth] [--width] [--hadith]`** word for word as `iy tree`.
  New in the core: `genealogy.rs` (the written trees, find, the families read from the names — kept while the
  database is unchanged —, the lines of learning, the hadith that name a prophet); gen_tables writes
  data/genealogy.json.
- **Bookmarks as the Python window has it (RS13).** From gui/bookmarks_page.py: the folders as a tree (All bookmarks,
  Not in a folder, folders inside folders, with their counts), a bookmark dragged onto a folder (or many, chosen with
  Ctrl), every kind with its colour and sign, the detail line (a narrator's road, generation, death and verdict; a
  genealogy's tree), the subtitle, its folders and its note; Open, Folders (ticks), Note, Remove; the right-click
  menus of a bookmark and of a folder (new folder inside, rename, move into, delete); a bookmark opens where it
  belongs (a hadith in its book, a narrator, a scholar, a term, a chapter, a place in a genealogy). Export and import
  go with File → Export / Import, which come later (they say so). **`iy-rs bookmark list|folders|add|remove|export|
  import`** as `iy bookmark`, on the same bookmarks.json. New in the core: the whole `bookmarks.Store` (folders,
  paths, filing, merging) and the references of every kind. The ☆ of every kind now says what it adds (it said "this
  hadith" on all of them).
- **Learn as the Python window has it (RS13).** From gui/learn_page.py: the glossary in colour (each category its
  colour and sign: the chips, the list, the band of a term's card, its short definition, its related terms), in
  English, Türkçe or العربية, a search, a category alone; a term (its names, In short, In detail, the source, ☆, the
  related terms, more of its category); History and method: the four articles with their blocks of numbers counted
  live from this database (the corpus, the traces of writing, the mutawatir hadith, the timeline), each counted once
  in a thread of its own. **`iy-rs term`** and **`iy-rs learn`** word for word as `iy term` and `iy learn`. New in the
  core: `learn.rs`; gen_tables writes data/learn.json. The wordings are looked for as plain words between word
  boundaries (the same matches as Python's regular expressions, in about a second instead of minutes).
- **Fixed: Isnad Chains.** The Companions page (RS7) named some of its classes as Isnad Chains names its own
  (`cp-row`, `cp-title` …): the rows of Isnad Chains took the Companions' height and line. The Companions' are now
  `cmp-…`.
- 32 more parity cases (164 in all).

- **Three pages at once (Bayram: "bir sonraki evreye geç ama biraz çok yap. tek tek gitmeyelim").** Books, Hadith
  Scholars and Companions and the Household, each built again from its Python page and compared with the Python window
  side by side.
- **Books as the Python window has it (RS-WIN3c).** From gui/books_page.py: the shelf (Sunni, Shia, all; by kind), a
  book's contents as a tree, a chapter read in its own order — the hadith as Search draws them (every text, nothing
  folded), the sira's sections, the lives of the tabaqat, the rijal entries, the cross-references — four at first and
  more as you read; ⏮ ‹ › ⏭ through the chapters and their place typed in; the hadith number field (hadith_nav) going
  to a hadith of the book; where you left each book (the same state.json); 🔖 Markers (put one where you are reading,
  rename, remove, remove all, go to one, where you left each book, forget it); Search in this chapter; the right-click
  menus of a book and of a chapter. A hadith, a person or a chapter opened from Search, Isnad Chains or Narrators opens
  here at its place, and each is a place in Back and Forward. ⏳ (each says so): the Qur'an reader, the PDF, the
  waiting and import rows of a book not imported. **Fixed (Python too):** the Books tree never set the Arabic font on
  an Arabic chapter title (a doubled backslash in books_page.py).
- **Hadith Scholars as the Python window has it (RS5).** From gui/scholars_page.py: All / Sunni / Shia / Huffaz; the
  scholars by role (compilers, graders, critics) with their names in the interface language, the Arabic and the dates;
  a scholar: the name card with ☆, his tradition's measure, his works, Open his books, Open him as a narrator, his life
  (written for isnady, or read from his rijal entry), what a compiler, a grader and a critic is; his collection (hadith,
  chains read, split, reaching the Prophet or a Ma'sum, identified; his teachers and the Companions — on road B whom his
  chains reach — as bars that open the narrator); his grades (the groups, Sunni or Imami, his most frequent wordings,
  the agreement with every other grader of his tradition with Cohen's kappa and the warning when it is too high,
  strictness by order only); his verdicts on narrators (Ibn Hajar's twelve ranks, or the Imami terms and the creeds).
  The Huffaz: al-Dhahabi's Tadhkirat al-Huffaz, 1,176 masters in 21 generations — the list by generation with a name
  to look for and a generation to choose, the book at a glance (each generation's count and years, by century), a
  hafiz (his number, generation, death and whose year it is, al-Mizzi's signs, his Taqrib entry with its verdict and
  chain links, his entry in the Tadhkira to read). The right-click menus, Back and Forward; a scholar found by Search
  opens here.
- **`iy-rs scholar [NAME] [--tradition] [--huffaz] [--tabaqa N]`**, word for word as `iy scholar` (`--source`, the
  Tadhkira's text, stays with the Python isnady). New in the core: `scholars::{present, grader_names, compiler_stats,
  grader_stats, critic_stats, written_life}`, `huffaz.rs` (index, people, reading, generations, by_century, find),
  `cache.rs` (a count kept while the database is unchanged, as core/cache.py), `grades::isnad_only`, `i18n::dates`;
  gen_tables writes data/huffaz.json.
- **Companions and the Household as the Python window has it (RS7).** From gui/companions_page.py: Sunni / Shia; the
  ten promised Paradise, the people of the cloak, the Mothers of the Believers, the Fourteen Infallibles on the Shia
  road; a group: what the sources say of it, its members two to a row (his death, his chain links and hadith — on the
  Shia road the chains that end at him and those that end at an Imam not named —, the first lines of his life, Life
  and narrations, In the family tree), the hadith that name it as Search draws them.
- **`iy-rs group [ID] [--tradition] [--lang] [--width]`**, word for word as `iy group`. New in the core: `sahaba.rs`;
  gen_tables writes data/sahaba.json (the groups, the boxes of the tree of Quraysh they need).
- 26 more parity cases (132 in all). The texts' `{x:.1f}` and `{s:+.2f}` are now filled in the window as Python fills
  them.

- **The Isnad Chains page as the Python window has it (RS-WIN3b).** Built again from gui/isnad_page.py,
  chain_widgets.py (ChainNode), hadith_nav.py and chapter_picker.py and compared with the Python window side by side,
  hadith by hadith (one chain, four chains joined by tahwil, a tahwil meeting, a chain from the hadith before, a text
  going on from the one before, a book without chains, al-Kafi with al-Kulayni's group and a raised chain, Turkish).
  The book list with the Sunni and Shia collections apart under their headings, 📖 From the contents… (the chapters as
  a tree, opened at the hadith shown; the hadith of a chapter with the start of their text and how far each chain was
  read; a word looked for among them; a double click or Show chain), Random hadith; the number field with the book's
  last number beside it, the numbers that begin with what is typed, the arrow keys and the mouse wheel stepping through
  the book, Show chain, ⏮ ‹ › ⏭ and "hadith 93 of 7,583 in this book" (hadith_nav, also for Books in RS-WIN3c); a
  number the book does not have, said with how the book writes its numbers, its range and Show … instead. The
  overview: per book the chains read, split into narrators, reaching the Prophet (a Ma'sum on road B), narrators per
  chain, identified in the Taqrib or al-Najashi, and why the rest were kept whole; a book opens its first chain. A
  hadith: what the page says first, the count of its chains, each chain as a timeline — the compiler's book (opening the
  hadith in Books), each link with its term and what the critics took it to mean, the name as written and its reading,
  the narrator in the rijal work with his verdict, rank, tabaqa and death (opening him on Narrators) or why he is not
  identified (a name several narrators bear opens them on Search), the tahwil meetings, al-Kulayni's groups, whom it
  reaches (the Genealogies: not here yet) — then the Arabic with the chain in lighter ink and the chain in words in
  Turkish and English. Opened from Search, Narrators or Books, the hadith is a place in Back and Forward; a name looked
  for opens Search on Narrators, as in the Python window.
- **`iy-rs isnads`**: the report of every book as `iy isnads` prints it, word for word (reading the chains again,
  `--rebuild`, stays with the Python isnady). New in the core: `isnad::{hadith_id, numbers_starting, position,
  describe, arabic_text, split_text, book_stats, known_by_book, identified_by_book, term_why}`; gen_tables also writes
  what each transmission term meant to the critics and the reasons a chain was kept whole. 4 more parity cases (106 in
  all).
- **Fixed:** Search showed no hadith after RS-WIN3a (a function of the Narrators page had the name of the one that
  draws a hadith's card). The Python window's Isnad Chains page opened with no book chosen and its buttons off
  (`fill_book_combo` took the line between the Sunni and the Shia books); it now opens at the first book, and isnady-rs
  does the same.

- **The Narrators page as the Python window has it (RS-WIN3a).** Built again from gui/narrators_page.py, name_card.py
  and life_card.py and compared with the Python window side by side. Left: All / Sunni / Shia, the overview of each
  road (how many narrators from which works, how many names of its chains are identified), the search as you type with
  the famous ones by any spelling first, ★ Famous Companions (Famous narrators on road B) group by group, the filters
  under their titles — generation, Ibn Hajar's rank or the Imami assessment, the six books by Ibn Hajar's marks —
  coloured when chosen, Clear filters, the list 200 at a time and the next ones at its end, its right-click menu (Open,
  his family, his line of learning, his entry in the rijal work, the bookmark, Copy). Right: the name card (the name he
  is known by and its Arabic, his full name, how he is known, the parts of his name and what they mean, ☆), the
  Genealogies links, his life — written for isnady, or read from the rijal work and the chains — with its facts, its
  teachers and students looked for among the narrators, the classical entry in its author's words when the work is
  downloaded, the sources; what the critics say (the entry as the critic wrote it, every verdict with its rank and the
  glossary on the hover, tabaqa, death, how each road judges, Open his entry in the book, Ibn Hajar's marks opening
  their books), teachers and students in the imported chains with the compilers, and the hadith he is in, each opening
  its chain. A narrator opened from Search or a chain is shown in his own tradition. A drop-down shows ten items at a
  time, as Qt does.
- **`iy-rs life NAME`** (`--lang`, `--tradition`, `--limit`, `--source`), word for word as `iy life`. New in the core:
  `lives.rs` (the written lives, carried in data/lives.json by gen_tables; the lives read from the data; the classical
  entries; al-Dhahabi's huffaz), `narrators::browse_by` (the page's filters), `overview`, `name_view`,
  `py::fill_with`. 10 more parity cases (102 in all).

- **Search → Narrators and Scholars, as the Python window has them (RS-WIN2b, S3).** The drop-down's Narrators finds
  the narrators of both traditions, or of one (Tradition), by any spelling of any part of the name — Arabic, English,
  Turkish — the closest first, as `narrators.find` orders them: a famous Companion by any spelling ("Ebubekir"), a
  scholar known by the name typed ("Bukhari"), the name he is known by, the query in his name, then only in the words
  about him; close spellings after the exact ones; the same consonants read otherwise ("Bayram" and al-Barmaki) under
  "Weaker matches". Each as his card: the name he is known by and its Arabic, his full name, Sunni or Shia (with the
  work), Ibn Hajar's rank or al-Najashi's category in its colour with the Arabic on the hover, tabaqa, death, how
  often in the chains, the words matched; a click opens him on the Narrators page in his tradition; right-click: Open
  in Narrators, Copy. Scholars finds the scholars of hadith by a name or a work in any spelling, close spellings too:
  his card gives his names, dates, works and the roles the imported data gives him (compiler, grader, critic);
  right-click: Open in Hadith Scholars, the bookmark star, Copy. The count, the close and the weaker ones and the time
  beside them; 25 more at a time, the matches after the first 400 read when asked for. The web view's own menu (Back,
  Reload, Inspect Element) no longer appears where the Python window has no menu.
- **`iy-rs search --in narrators [--tradition sunni|shia]` and `iy-rs search --in scholars`**, word for word as
  `iy search --in …` prints them. New in the core: `name_parts.rs` (the parts of a name and the one he is known by),
  `scholars.rs` (the register from the Python isnady, roles, search, named, as_narrator), `narrators::browse`, `find`,
  `find_more`, `difflib.rs` (Python's SequenceMatcher ratio, letter for letter); the scholars' register and the name
  parts' places in data/tables.json, the people cards' tables in data/window.json. 16 more parity cases (92 in all).

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
