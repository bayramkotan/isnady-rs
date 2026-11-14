<!-- The head of isnady-rs's README; everything below the line is the README of the Python isnady, word for word
     (tools/sync_readme.py writes it — never edit README.md by hand). -->
<h1 align="center">isnady-rs — isnady in Rust</h1>

<p align="center">
  <img src="https://img.shields.io/badge/isnady--rs-v0.7.7-1D4777?style=for-the-badge&logo=rust" alt="version">
  <a href="https://crates.io/crates/isnady"><img src="https://img.shields.io/crates/v/isnady?style=for-the-badge&logo=rust&label=crates.io" alt="crates.io"></a>
  <a href="https://github.com/bayramkotan/isnady-rs/releases/latest"><img src="https://img.shields.io/github/v/release/bayramkotan/isnady-rs?style=for-the-badge&label=release" alt="release"></a>
  <img src="https://img.shields.io/badge/licence-MIT-5B6878?style=for-the-badge" alt="MIT">
</p>

**isnady-rs is isnady.** It is the same program, written again in Rust. It uses the same data folder, the same
database, the same settings and the same version number, and it gives the same answers. That is why this README is
isnady's own, word for word, below the line. The Rust command is `iy-rs` where the Python one is `iy`.

**Version 0.7.7 — what isnady-rs does today: everything the Python isnady does.** Every command of `iy` is a command
of `iy-rs` with the same options and the same output, word for word; every page, menu, dialog, theme and language of the
Python window is in the window `isnady-rs`. The two programs import the same books into the same rows, read the same
chains, identify the same narrators, learn the same meaning index (array for array), export the same files and take the
same ready database. `tools/parity.py`, `parity_db.py`, `parity_export.py`, `parity_ai.py` and `parity_ready.py`
compare them on every push and before every release; [PARITY.md](PARITY.md) lists each part. A search takes about
3 ms, a chain about 30 ms.

**Get it:**

| Where | What |
|---|---|
| [Releases](https://github.com/bayramkotan/isnady-rs/releases/latest) | **The window:** Windows `-setup.exe` (no administrator rights), `.msi`, or the portable `.exe`; Linux the portable `.AppImage` or a `.deb`; macOS a `.dmg` or the `.app` zipped (Apple silicon and Intel). **The command line:** `iy-rs` for each of them. Every file with its sha256 in `SHA256SUMS.txt` |
| [crates.io](https://crates.io/crates/isnady) | `cargo install isnady` gives `iy-rs`; `cargo install isnady-gui` gives the window `isnady-rs` (Linux needs `webkit2gtk-4.1`) |
| The source | `cargo build --release`: `iy-rs` and `isnady-rs` in `target/release/`; `cargo tauri build` in `crates/isnady-gui` makes the installers |

A folder named `isnady-data` beside a portable copy (the AppImage, the `.exe`, the `.app`) keeps every database and
setting there. With no book yet, the window offers **Import all** or isnady-data's **ready database** (`iy-rs database
download`): everything already read, in one download.

---

<h1 align="center">📜 isnady &nbsp;<sub>إسناد</sub></h1>

<p align="center">
  <strong>Hadith search built around the isnad, the chain of transmission</strong><br>
  <sub>Search the Sunni and Shia collections, read every chain narrator by narrator, and see how the scholars of each tradition read it</sub>
</p>

<p align="center">
  <a href="https://pypi.org/project/isnady/">
    <img src="https://img.shields.io/badge/PyPI-v0.7.7-1D4777?style=for-the-badge&logo=pypi&logoColor=white" alt="PyPI">
  </a>
  <img src="https://img.shields.io/pypi/pyversions/isnady?style=for-the-badge&color=A47E24&logo=python&logoColor=white" alt="Python">
  <img src="https://img.shields.io/badge/Platform-Windows%20%7C%20Linux%20%7C%20macOS-5B6878?style=for-the-badge" alt="Platform">
  <a href="https://www.npmjs.com/package/isnady">
    <img src="https://img.shields.io/badge/npm-v0.7.7-14304F?style=for-the-badge&logo=npm" alt="npm">
  </a>
  <a href="https://github.com/bayramkotan/isnady-data">
    <img src="https://img.shields.io/badge/data-isnady--data-2E6B4F?style=for-the-badge&logo=github" alt="isnady-data">
  </a>
  <img src="https://img.shields.io/badge/License-MIT-D6B25E?style=for-the-badge" alt="License">
</p>

<p align="center">
  <a href="#-two-repositories-one-project">Repositories</a> •
  <a href="#-why-isnady">Why</a> •
  <a href="#-install">Install</a> •
  <a href="#-quick-start">Quick Start</a> •
  <a href="#-isnady-data--the-data">isnady-data</a> •
  <a href="#-search">Search</a> •
  <a href="#-chains-of-transmission">Chains</a> •
  <a href="#-sunni-and-shia-two-roads">Sunni and Shia</a> •
  <a href="#-narrators">Narrators</a> •
  <a href="#-books">Books</a> •
  <a href="#-hadith-scholars">Scholars</a> •
  <a href="#-companions-and-the-household">Companions</a> •
  <a href="#-sira">Sira</a> •
  <a href="#-written-sources">Written sources</a> •
  <a href="#-statistics">Statistics</a> •
  <a href="#-learn">Learn</a> •
  <a href="#-search-by-meaning-ai">Meaning (AI)</a> •
  <a href="#-appearance-and-settings">Appearance</a> •
  <a href="#%EF%B8%8F-cli">CLI</a> •
  <a href="#%EF%B8%8F-roadmap">Roadmap</a>
</p>

<p align="center">
  <img src="https://raw.githubusercontent.com/bayramkotan/isnady/main/assets/screenshots/search-arabic.png" alt="Searching النيات finds بالنيات in Sahih al-Bukhari 1, with the chain above the text" width="850">
</p>

> **Pre-alpha (0.7.7).** Search, chains of transmission, narrators, books, scholars, statistics and
> the glossary work today, Sunni and Shia, on data you import in one click. Everything is built in
> the open, one piece at a time.

---

## 🧭 Two repositories, one project

isnady is two repositories that work as one:

| | [**bayramkotan/isnady**](https://github.com/bayramkotan/isnady) — the application | [**bayramkotan/isnady-data**](https://github.com/bayramkotan/isnady-data) — the data |
|---|---|---|
| **What it is** | The program: a desktop window and a command line on one engine | Every book isnady reads — the Qur'an, the hadith, the sira, the lives of the scholars, the rijal — in one format |
| **What is in it** | Search, the chain reader (Sunni and Shia), narrator identification, books, scholars, statistics, the glossary, meaning search | **The Qur'an** with 60 translations in 18 languages · **17 Sunni and 16 Shia hadith collections — 99,746 hadith** — in Arabic and up to eight translations, with their grades · **Ibn Hisham's *Sira*** · **al-Dhahabi's *Siyar* and *Tadhkira*** · **Ibn Hajar's *Taqrib*** (8,824 narrators), **al-Najashi's *Rijal*** (1,269) and **al-Tusi's *Fihrist* and *Rijal*** (910 and 6,429) |
| **How you get it** | [PyPI](https://pypi.org/project/isnady/) (`pip install isnady`), the desktop applications of the [Releases](https://github.com/bayramkotan/isnady/releases/latest), or the one-line installers | Nothing to download by hand: isnady lists its books in **Data sources** and imports them in one click — or clone it and read it offline, or take a whole [ready database](https://github.com/bayramkotan/isnady-data/releases) |
| **Licence** | MIT | each book its own, recorded with it (Help → Licences) |

```
   bayramkotan/isnady-data                         bayramkotan/isnady
   ───────────────────────                         ──────────────────
   catalog.json ─────────────── Data sources ───▶  import ─▶ isnady.db ─▶ search · chains · narrators
   quran/                                            (your data folder)      books · scholars · statistics
   hadith/sunni/  hadith/shia/                                                window  and  command line
   sahifa/sunni|shia/  sira/sunni/  tabaqat/sunni/  rijal/sunni|shia/
   Releases: isnady-db-<date>.zip ── Download the ready database ──▶ isnady.db
```

The application ships without hadith data, so it stays small and the data can grow on its own:
a new book in isnady-data reaches every isnady at once, without a new version of the program.

**And isnady in Rust: [bayramkotan/isnady-rs](https://github.com/bayramkotan/isnady-rs).** The same program,
written again in Rust: `iy-rs` on the command line and the window `isnady-rs`. It uses the same data folder, the same
database and the same settings, and it carries **the same version number** — every release of isnady is a release of
isnady-rs, on the same day. Every command and every page is ported and gives the same answers, word for word;
parity tests compare the two programs on every push (its
[PARITY.md](https://github.com/bayramkotan/isnady-rs/blob/main/PARITY.md)). Get it from
[crates.io](https://crates.io/crates/isnady) (`cargo install isnady` for `iy-rs`, `cargo install isnady-gui` for the
window) or ready to run from its [Releases](https://github.com/bayramkotan/isnady-rs/releases/latest) (see
[Install](#-install)). Its README is this one, word for word.

---

## 🎯 Why isnady

A hadith is two things: the text (*matn*) and the chain of people who passed it on (*isnad*).
Most hadith sites let you search the text and show a grade. isnady is built the other way round:
the chain comes first, because that is where the scholars of hadith did their work.

- **The chain is read, not typed in.** isnady reads the chain from the Arabic text itself: who
  narrated to whom, with which words, and whom it reaches.
- **Each tradition by its own road.** A Sunni book's chains are read the Sunni way and judged by
  the Sunni critics; a Shia book's chains are read the Shia way and judged by the Imami critics.
  The two meet on the same pages and are never measured by each other's scale.
- **Nothing is guessed.** Where the wording cannot be read with confidence, the chain is kept
  whole and marked, never split wrongly. A narrator is named only when the evidence leaves one
  person; otherwise isnady says how many it could be.
- **Every fact has a source.** Every hadith, text and grade records where it came from, under
  which licence.
- **Grades are shown as the scholars gave them.** Each grade keeps its scholar's name and wording;
  grades are ordered categories, never averaged, and isnady never invents its own.
- **Computed once, kept.** Every statistic and model is saved as JSON beside your data and read
  back, never computed again for the same data.
- **Window and command line, same engine.** Everything the window does, the command line does
  too, with the same results.

---

## 📦 Install

### Desktop application

Download isnady for your system from the **[latest release](https://github.com/bayramkotan/isnady/releases/latest)** —
everything included, the AI features too:

| System | File |
|---|---|
| Windows 10/11 | `isnady-<version>-windows-portable.exe` — Windows may say the publisher is unknown (the app is not signed): *More info → Run anyway* |
| Linux | `isnady-<version>-linux-x86_64.AppImage` — make it executable (`chmod +x`) and run it |
| macOS, Apple silicon / Intel | `isnady-<version>-macos-arm64.zip` / `…-macos-x86_64.zip` — unzip and run `isnady.app`; the first time, right-click it and choose *Open* (it is not notarized) |

All three are **portable**: nothing is installed, each runs from wherever it is. Put a folder named
`isnady-data` beside the file and isnady keeps its databases and settings there — isnady on a USB stick.

Put isnady on the desktop and in the applications menu with **Tools → Create Desktop Shortcut** (or
`iy shortcut`): the shortcut starts isnady the way it is installed — the AppImage, the application, or the
`isnady-gui` command of a Python install.

### isnady-rs (the same isnady in Rust)

From the **[isnady-rs releases](https://github.com/bayramkotan/isnady-rs/releases/latest)** — the window `isnady-rs`
and the command line `iy-rs`, with the same data folder as the Python isnady:

| System | The window | The command line |
|---|---|---|
| Windows 10/11 | `isnady-rs-<version>-windows-x86_64-setup.exe` (installs for you, no administrator rights), `…-windows-x86_64.msi` (for all users), or `…-windows-x86_64-portable.exe` (nothing installed) | `iy-rs-<version>-windows-x86_64.zip` |
| Linux | `isnady-rs-<version>-linux-x86_64.AppImage` (portable: `chmod +x` and run it) or `isnady-rs-<version>-linux-amd64.deb` (`sudo apt install ./…deb`) | `iy-rs-<version>-linux-x86_64.tar.gz` |
| macOS, Apple silicon / Intel | `isnady-rs-<version>-macos-arm64.dmg` / `…-macos-x86_64.dmg` (drag it to Applications), or the `.zip` of the app (portable); the first time, right-click it and choose *Open* | `iy-rs-<version>-macos-arm64.tar.gz` / `…-macos-x86_64.tar.gz` |

Or with Rust: `cargo install isnady` (`iy-rs`) and `cargo install isnady-gui` (the window; on Linux it needs
`webkit2gtk-4.1`). The portable ones keep everything in a folder named `isnady-data` beside them, as above. Every file of
a release is listed with its sha256 in `SHA256SUMS.txt`.

### With Python

One line, on any system. Install isnady wherever you like — for all users, for yourself, in a
virtual environment, with pipx: the same line installs it, and later updates every copy where it
is, asking for administrator rights only when a copy needs them. Nothing is ever removed.

```bash
# Linux and macOS
curl -fsSL https://raw.githubusercontent.com/bayramkotan/isnady/main/install.sh | bash
```

```powershell
# Windows (PowerShell)
irm https://raw.githubusercontent.com/bayramkotan/isnady/main/install.ps1 | iex
```

Or with pip:

```bash
pip install isnady
isnady            # or the short name:  iy
```

Run inside a clone of this repository, the installer makes an editable (developer) install of
that clone instead.

The same program answers to several names: **`iy`** to type, **`isnady`** to read, and
`isnady-cli` kept from the first releases. Without arguments it opens the window; with
arguments it is the command line. On Windows, **`isnady-gui`** opens the window without a
console.

<details>
<summary><b>🐧 On Linux, pip may refuse to install</b></summary>
<br>

Most current distributions mark the system Python as *externally managed* (PEP 668), so a
plain `pip install` stops with `error: externally-managed-environment`. Two ways around it:

```bash
# Isolated — recommended, no system packages touched
pipx install isnady

# Into your user site — needs the override flag
pip install isnady --break-system-packages --no-cache-dir -U
```

</details>

### Upgrading, and when something is not right

```bash
iy update
```

`iy update` updates every copy of isnady where it is installed. Your data stays where it is; a
newer isnady upgrades the database in place the first time it opens it, and reads the chains again
when its chain reader has changed (a minute or two, once).

`iy doctor` lists every copy and which one each command really starts — an older copy in the user
folder can start before a newer one elsewhere, so every copy is kept at the same version. The same
report is under **Help → Check Installation** (and the ⚠ button in the sidebar when something is behind), with an
**Update** button that does what `iy update` does: every copy updated where it is, the output shown as it comes; a
copy installed for all users asks for the administrator password in the system's own window (pkexec on Linux, the
administrator prompt on Windows). If an old copy starts even for `iy update`, run `python3 -s -m isnady update`
(Windows: `py -s -m isnady update`).

---

## 🚀 Quick Start

1. **Open isnady** — `iy`, or the desktop application. The first time, with no books yet, it offers to import
   isnady-data at once, in the background.
2. **File → Data Sources.** The repository is already chosen: **isnady-data**. Tick the books and
   the languages you want and press **Import** — or **Import all**. **Languages for every book** adds a language to
   all the books that have it (or takes it out of all of them) in one step. Later, **⟳ Get updates** brings in only
   what is new or changed in the repository.
3. **Import the narrators too** — Ibn Hajar's *Taqrib* for the Sunni books, al-Najashi's *Rijal* and
   al-Tusi's *Fihrist* and *Rijal* for the Shia books — and every name in every chain is matched to its narrator.
4. **Search**, open a chain, open a narrator, read a book.

The same from the command line:

```bash
iy catalog list                                           # the books of isnady-data
iy catalog import sahih_al_bukhari --language ara --language tur
iy catalog import musnad_ahmad_ibn_hanbal
iy catalog import al_kafi_al_kulayni                      # with al-Majlisi's and al-Behbudi's grades
iy catalog import taqrib_al_tahdhib_ibn_hajar_al_asqalani # the narrators of the Sunni chains
iy catalog import rijal_al_najashi_ahmad_ibn_ali_al_najashi  # the narrators of the Shia chains
iy catalog import --all                                   # everything
iy catalog updates                                        # later: what is new or changed in the repository
iy catalog update                                         # …and import only that

iy search النيات              # Arabic, with or without diacritics
iy search "niyetlere göre"     # Turkish, English, any imported language
iy search niyet --grade sahih --grade hasan   # only sahih and hasan hadith
iy chain bukhari 1             # one chain, narrator by narrator
iy chain kafi 5:1740           # a Shia chain, read the Shia way
iy narrator الزهري             # who a narrator is
```

Importing every book of isnady-data takes about a minute and a half: the next books download while one is
written, and the search index, the chains and the narrators are built once, at the end. No network? See
[Without the network](#without-the-network) below.

---

## 📚 isnady-data — the data

**[bayramkotan/isnady-data](https://github.com/bayramkotan/isnady-data)** is isnady's own data
repository and its default: every book isnady reads, kept once in isnady's own format, and every one of them is
listed in **Data sources** and imported from there with one click — or all of them with **Import all**
(`iy catalog import --all`). After that, **⟳ Get updates** (`iy catalog update`) imports only what changed: a book
new to the repository, a book whose files changed (every book's fingerprint is in the catalog), or a language the
repository added to a book that you read in your other books. Nothing else is read again.

| What | Books | Read in isnady |
|---|---|---|
| 📖 **The Qur'an** | The Arabic (Tanzil's text, verbatim) and **60 translations in 18 languages** — Turkish 20 (Diyanet İşleri, Diyanet Vakfı, Elmalılı Hamdi Yazır, Ali Bulaç, Süleyman Ateş, Hasan Basri Çantay …), English 12 (Saheeh International, Abdel Haleem, Mustafa Khattab …), Urdu, Indonesian, Bengali, French, Russian, Tamil, German, Persian, Spanish, Bosnian, Albanian, Azerbaijani, Uzbek, Kazakh, Tatar, Malay | Books → The Qur'an: each verse with the translations chosen, and the hadith that name it |
| 📚 **Hadith** | 17 Sunni and 16 Shia collections — 99,746 hadith (below) | Search, Books, Isnad Chains, Statistics |
| 📜 **Written sources** | 6 Sunni (the Sahifa of Hammam b. Munabbih 138, al-Sahifa al-Sadiqa 253, the sahifa of 'Ali, Abu Bakr's letter on zakat, the letter for 'Amr b. Hazm, writing in the chains) and 4 Shia (the Book of 'Ali, al-Jami'a, Mushaf Fatima as the Shia books relate it, the letters to the Imams) — each a list of where its hadith stand in the collections | Books, as a book of its own; Written sources |
| 🕌 **Sira** | Ibn Hisham, *al-Sira al-Nabawiyya* (from Ibn Ishaq) — 264 chapters, 2,109 sections | Books, chapter by chapter |
| 👥 **Tabaqat** | al-Dhahabi, *Siyar A'lam al-Nubala'* (5,851 lives) and *Tadhkirat al-Huffaz* (1,176 huffaz in 21 generations) | Books; the text beside a narrator's written life; Hadith Scholars → Huffaz |
| ⚖ **Rijal** | Ibn Hajar, *Taqrib al-Tahdhib* (8,824) · al-Najashi, *Rijal* (1,269) · al-Tusi, *al-Fihrist* (910) and *Rijal* (6,429) | Narrators, and every chain's narrators identified |

The hadith collections, Sunni and Shia apart:

| | Sunni | Shia |
|---|---|---|
| **Hadith collections** | **17 books, 76,132 hadith.** Sahih al-Bukhari, Sahih Muslim, Sunan Abi Dawud, Jami' al-Tirmidhi, Sunan al-Nasa'i, Sunan Ibn Majah, the Muwatta — Arabic and up to eight translations (English, Turkish, Urdu, Indonesian, Bengali, French, Russian, Tamil), graded by al-Albani, Shu'ayb al-Arna'ut, Zubair 'Ali Za'i and others · the Forty of al-Nawawi, Forty Hadith Qudsi, the Forty of Shah Waliullah · Musnad Ahmad ibn Hanbal (26,363) and Sunan al-Darimi, fully voweled · Riyad al-Salihin, al-Shama'il al-Muhammadiyya, Bulugh al-Maram, al-Adab al-Mufrad, Mishkat al-Masabih, Arabic and English | **16 books, 23,614 hadith.** al-Kulayni's **al-Kafi** (8 volumes, 14,241 hadith, al-Majlisi's grade on nearly every one, al-Behbudi's selection) · al-Saduq's 'Uyun Akhbar al-Rida, al-Amali, al-Khisal, al-Tawhid, Ma'ani al-Akhbar, Thawab al-A'mal, Sifat al-Shi'a, Fada'il al-Shi'a · al-Mufid's al-Amali · Ibn Qulawayh's Kamil al-Ziyarat · al-Nu'mani's and al-Tusi's Kitab al-Ghayba · al-Ahwazi's al-Mu'min and al-Zuhd · al-Muhsini's Mu'jam al-Ahadith al-Mu'tabara — Arabic and English |
| **Narrators** | Ibn Hajar, *Taqrib al-Tahdhib* — 8,824 narrators: verdict, rank of twelve, tabaqa, death year, book marks | al-Najashi, *Rijal* — 1,269 authors and narrators; al-Tusi, *al-Fihrist* — 910 authors, and *Rijal* — 6,429 companions of the Prophet and the Imams: reliability and creed in the Imami terms |

### How it is laid out

```
isnady-data/                        each KIND in its folder, each ROAD in its own under it
├── catalog.json                    the list isnady reads (isnady-catalog/1): "entries" and the "library"
├── quran/                          the Qur'an, both roads': suras.json, ara/, tur/tur-diyanetisleri.json …
├── hadith/
│   ├── sunni/sahih_al_bukhari/     book.json (numbers, chapters, references, grades) + ara.json, eng.json, tur.json …
│   ├── sunni/musnad_ahmad_ibn_hanbal/ …
│   └── shia/al_kafi_al_kulayni/ …
├── sahifa/
│   ├── sunni/sahifat_hammam_ibn_munabbih.json   a written source: [book, number] of its hadith (isnady-sahifa/1)
│   └── shia/kitab_ali_fi_al_kutub_al_shiiyya.json …
├── sira/
│   └── sunni/al_sira_al_nabawiyya_ibn_hisham.md
├── tabaqat/
│   └── sunni/siyar_alam_al_nubala_al_dhahabi.md, tadhkirat_al_huffaz_al_dhahabi.md
├── rijal/
│   ├── sunni/taqrib_al_tahdhib_ibn_hajar_al_asqalani.md
│   └── shia/rijal_al_najashi_ahmad_ibn_ali_al_najashi.md
├── derived/                        statistics and models already computed, as JSON
└── database.json                   the ready database of the latest Release
```

- **Full names, never a bare one.** Every folder and every id is the book's full title with its
  compiler's name — `musnad_ahmad_ibn_hanbal`, `al_kafi_al_kulayni`,
  `taqrib_al_tahdhib_ibn_hajar_al_asqalani`. isnady imports by these ids, and names its exports and
  saved computations after them.
- **One format for every book** (`isnady-book/1`): a `book.json` with each hadith's number, place,
  chapter, other numberings and every grade as `[scholar, grade]`, exactly as written; and one file
  per language with the chapter titles and the texts. The formats are described in the
  [isnady-data README](https://github.com/bayramkotan/isnady-data#-the-formats).
- **Sunni and Shia apart.** Every book says its tradition, and isnady keeps them on their own roads
  (below): in the repository each kind has a folder for each road, `hadith/sunni/` beside `hadith/shia/`.
- **Data sources is laid out the same way.** One button for each kind — *The Qur'an · Hadith collections · Written
  sources · Sira · Tabaqat · Rijal*, each with how many books it holds — then *Both roads · Sunni · Shia*; under each
  road its folder in the repository (`isnady-data/hadith/sunni/`) and its books. **Every book's card is the same:**
  its tags (kind, road, licence, size), what it is, its **languages** (a hadith collection's translations, the
  Qur'an's translations by language with *Translators…* for the others, *Arabic* for the works written in it), and
  the same four buttons — **Import** (or **Update**), **Remove data**, **Open in Books**, **Export…** (the hadith
  collections; for the other kinds it is shown greyed out, with the reason). `iy catalog list` lists them in the
  same order, each kind and road under its folder.
- **Get updates, not everything again (0.7.4).** isnady remembers each book's fingerprint and languages when it
  imports it (`imported.json` in your data folder). The button at the top of Data sources says how many books
  wait — *⟳ Get updates (3)* — and each of their cards carries a mark: *New*, *Changed* or *New language*. Importing
  them brings the search index, the chains and the narrators up to date once, at the end.
- **In your language.** The cards say what each book is in the interface language — Turkish and Arabic as well as
  English (*"Buhârî (ö. 256) derledi. Arapça ve 8 tercüme …"*).
- **The layout of 0.7.3.** Until 0.7.2 the hadith collections stood at `sunni/<book>/` and `shia/<book>/`, and the
  sira and tabaqat with no road folder. isnady 0.7.3 reads the new places at once; an older isnady finds them after
  **Refresh** in Data sources (or by itself a day later).
- **A written source is a book that copies nothing.** Its file lists `[book, number]` for each of its hadith;
  importing it makes a book of **Books** whose chapters are the collections it lives in (for Hammam: Musnad Ahmad
  138, then its parallels in al-Bukhari, Muslim …), each leaf the hadith itself in its own collection. Search and the
  statistics still count every hadith once. Not imported yet? It waits on the shelf of Books: one click imports it.
- **Your own data stays yours.** What you import from your own files is never written into a
  ready database for isnady-data: it does not leave your computer.

### Without the network

isnady-data works the same from a folder on your computer — a `git clone`, a USB stick, a shared
folder:

```bash
git clone https://github.com/bayramkotan/isnady-data ~/isnady-data
iy repo add ~/isnady-data       # in the window: Data sources → A folder on this computer…
iy catalog import --all
```

Or take **a whole ready database** — every book imported, every chain read, every narrator
identified — in one download instead of importing book by book. The first start offers it beside
**Import all** when isnady has no book yet; later it is **Data folder → Download the ready database…**
in Data sources, or one command:

```bash
iy database download --check   # what it holds: date, books, hadith, narrators, size
iy database download           # download it, check it and use it
iy database use ~/Downloads/isnady-db-<date>.zip   # or a file you already have (Data folder → Use a ready database…)
```

isnady reads what the ready database holds from isnady-data's `database.json`, downloads the zip
of the [isnady-data Releases](https://github.com/bayramkotan/isnady-data/releases) into your data
folder and checks it (its size and sha256 as `database.json` gives them, the zip must hold
`isnady.db`, the schema must be one it reads). It takes the place of your database at the next start
and your previous database is kept beside it (`isnady.<date-time>.bak.db`). Nothing is deleted. A new
ready database is published with every release of isnady, so it carries the newest chain reader.

<p align="center">
  <img src="https://raw.githubusercontent.com/bayramkotan/isnady/main/assets/screenshots/data-sources.png" alt="Settings → Data sources — one button for each kind of book, then the road; the Sunni hadith collections under isnady-data/hadith/sunni/, each card with its languages and the same four buttons: Update, Remove data, Open in Books, Export" width="760">
</p>
<p align="center">
  <img src="https://raw.githubusercontent.com/bayramkotan/isnady/main/assets/screenshots/data-sources-sahifa.png" alt="Data sources → Written sources — the Sunni ones under isnady-data/sahifa/sunni/ and the Shia ones under sahifa/shia/, each with the same card" width="49%">
  <img src="https://raw.githubusercontent.com/bayramkotan/isnady/main/assets/screenshots/data-sources-quran.png" alt="Data sources → The Qur'an — the translations ticked by language, Translators… for the others, and the same four buttons" width="49%">
</p>

### Other repositories

isnady-data is all you need. isnady can also read a few datasets that others publish, as they
publish them, under their own keys (e.g. `bukhari-lk`) so that editions with other numbering never
mix with isnady-data's. Choose one in **Settings → Data sources** (`iy repo use ID`) or add any
repository with an isnady catalog, a Hugging Face dataset, a URL or a folder (`iy repo add …`):

| Repository | Holds | Licence |
|---|---|---|
| [fawazahmed0/hadith-api](https://github.com/fawazahmed0/hadith-api) | 10 collections, Arabic + 8 languages, grades | Unlicense |
| [Thaqalayn](https://github.com/MohammedArab1/ThaqalaynAPI) | 16 Shia collections, Arabic + English | not stated |
| [Open Hadith Data](https://github.com/mhashim6/Open-Hadith-Data) | the nine books, voweled Arabic | ODbL |
| [hadith-json](https://github.com/AhmedBaset/hadith-json) | 17 books, Arabic + English | not stated |
| [LK Hadith Corpus](https://github.com/ShathaTm/LK-Hadith-Corpus) | the six books, isnad and matn marked apart | not stated |
| [meeAtif/hadith_datasets](https://huggingface.co/datasets/meeAtif/hadith_datasets) (Hugging Face) | the six books, Arabic + English | MIT (as declared) |
| [Hadith-Data-Sets](https://github.com/abdelrahmaan/Hadith-Data-Sets) | the nine books, Arabic | not stated |
| [OpenITI](https://github.com/OpenITI) | the Taqrib, al-Najashi's Rijal, al-Tusi's Fihrist and Rijal | not stated |

Every source records a licence **tier** — **A** may be redistributed, **B** under its conditions,
**C** may not — and **Help → Licences** lists them all. Any CSV or JSON table of hadith can be read
through a catalog that names its columns (format `table`); files and links of your own, with or
without authentication (credentials are never stored), are read as well.

---

## 🔎 Search

| | |
|:--|:--|
| **Arabic without diacritics** | `الاعمال` finds `الأَعْمَالُ`. Alef forms, alef maqsura, hamza seats and ta marbuta are treated alike, so you type the way you normally write |
| **Attached prefixes** | `النيات` finds `بالنيات`, because Arabic joins bi-, wa-, fa- and al- to the word. *Whole words only* turns this off |
| **Latin-script text** | Case and accents are ignored: `NIYET`, `niyet` and `nîyet` are one word; `Muʿādh`, `Mu'adh` and `Muadh` too |
| **Match** | All words, any word, the exact phrase — or by meaning (AI, below) |
| **Unknown letters** | `?` stands for one letter you do not know, `*` for any letters: `ي?من` finds `يؤمن`, `مسل*` finds `مسلم`, `مسلمون` … |
| **Misspellings** | A search that finds nothing as typed is tried again with each word one letter away — a letter more, less or different, in as many words as needed: `لا يمن احدكم حتى يحب لاخبه` finds `لا يؤمن أحدكم حتى يحب لأخيه`. The spellings used are named on the result line |
| **Pasted passages** | Punctuation and quotation marks clinging to the words do not matter |
| **Books** | All books, all Sunni, all Shia, or any few ticked one by one |
| **Order** | Book by book in the order of the book lists — Sahih al-Bukhari, Sahih Muslim, Sunan Abi Dawud … then the Shia books — and within a book by its numbering |
| **Language** | One language or several to search in — tick them in the list (Arabic and Turkish, say); the languages you do not read are folded behind an arrow on each hadith, a click opens them. The list holds the languages imported; the languages the same books have in isnady-data (Urdu, Indonesian, Bengali, French, Russian, Tamil) are named at its end — choosing them opens **File → Data Sources** to import them |
| **Active books** | A book can be switched off in **Settings → Books and languages**: it stays imported but is left out of searches, book lists and statistics until switched on |
| **Grades** | Shown under each hadith with the scholar's name, exactly as given, and explained when you point at them |
| **Search by grade** | **Grade** beside Books: only sahih, only hasan, both, or any grades ticked — the Sunni scale (sahih, hasan, da'if, very weak, fabricated, not graded) and the Shia books' Imami scale (sahih, hasan, muwaththaq, da'if) apart, never matched to each other. A hadith is found when one of its graders gave a grade ticked, or — with *Every grader agrees* — only when all did. Al-Bukhari's and Muslim's hadith, which no grader grades, count as sahih by their compiler's condition (said so on each; one tick leaves them out). `iy search niyet --grade sahih --grade hasan`, `--every-grader`, `--no-sahihayn` |
| **Search in a chapter** | **🔎 Search in this chapter** on Books: the Search page with that book and that chapter (and the chapters under it) alone, shown as 📖 beside Books — ✕ searches the whole book again. `iy search العلم --chapter bukhari:3`, `--chapter "bukhari:Knowledge"` |
| **Export the results** | **⤓ Export these results** beside the count: every hadith found (up to 1,000) with the books, languages, grades and chapter chosen, to Word, PDF, Excel, CSV, JSON, EPUB …; the file says how they were found (the words, the books, the grades, the chapter). `iy export search العلم --in-chapter bukhari:3 --grade sahih -f docx` |
| **Highlighting** | On the original, diacritised text, diacritics included |
| **Recent searches** | The last twenty, under the examples on the start page — a click searches again, one link forgets them all; kept with your profile. `iy search --recent`, `iy search --clear-recent` |
| **Home** | ⌂ beside the box (Alt+Home) brings the page back to how it opens: no words, every filter at its default. Esc empties the box |
| **Narrators and scholars** | The same box finds narrators — Sunni and Shia, each marked — and scholars, by Arabic or Latin name |

Results appear at once and fill in as you read; the window never waits for them.

<p align="center">
  <img src="https://raw.githubusercontent.com/bayramkotan/isnady/main/assets/screenshots/search-folded.png" alt="Search in two books with Arabic chosen: the English and Turkish texts folded behind ▸ Other languages, opened with a click" width="850">
</p>
<p align="center">
  <img src="https://raw.githubusercontent.com/bayramkotan/isnady/main/assets/screenshots/book-picker.png" alt="Books — tick all books, all Sunni, all Shia, or any few one by one" width="420">
</p>

---

## 🔗 Chains of transmission

Every Arabic text is read for its chain:

```
Sahih al-Bukhari 1
   1. حدثنا (narrated to us)  الْحُمَيْدِيُّ عَبْدُ اللَّهِ بْنُ الزُّبَيْرِ
   2. حدثنا (narrated to us)  سُفْيَانُ
   3. حدثنا (narrated to us)  يَحْيَى بْنُ سَعِيدٍ الْأَنْصَارِيُّ
   4. أخبرني (informed me)    مُحَمَّدُ بْنُ إِبْرَاهِيمَ التَّيْمِيُّ
   5. سمع (heard)            عَلْقَمَةَ بْنَ وَقَّاصٍ اللَّيْثِيَّ
   6. سمعت (I heard)          عُمَرَ بْنَ الْخَطَّابِ
      -> the Prophet ﷺ
```

In the window, each result shows its chain as a row of names ending at whom it reaches, and
**View chain** opens it as a timeline from the book to the Prophet ﷺ — or, in a Shia book, to the
Imam — with the Arabic text below: the chain in lighter ink, the text of the hadith in full ink.
Under each Arabic name of a card's chain its words are written in the languages the texts are shown in (Turkish
and English so far), and under the Arabic text of Isnad Chains the whole chain is said, link by link as the Arabic
says it ("his father" says whose father and who he is: "Hişâm b. Urve'nin babası Urve b. ez-Zübeyr'den"; a
Companion has "(r.a.)", the Prophet "(s.a.v.)"; the compiler, the "us" of "narrated to us", heads the chain) — "Qutayba b. Sa'id narrated to us, Layth narrated to us, from Yazid b. Abi Habib, …
from the Prophet ﷺ" / "Kuteybe b. Saîd bize rivayet etti, Leys bize rivayet etti, Yezîd b. Ebî Habîb'den, … Hz.
Peygamber ﷺ'den" (`iy chain … --lang tr,en`).
Every link says who the narrator is and what his critic says of him; a click anywhere on him opens him, and pointing
at him says how he was identified (by his name, by the narrators around him, or as the famous one of that name). A
name not identified looks for its narrators on Search; the book at the top opens the hadith in its book; the
Prophet ﷺ at the end opens his life in the Genealogies (on road B, the Imam the chain reaches). The chains drawn
on the cards of Search and Books open the same way, name by name, and every teacher and student named in a life
looks for himself among the narrators.

Choose a book and a number, or let isnady choose: beside the number stands the book's last number ("/ 7563") and
where the hadith stands among the book's ("hadith 36 of 7,467"); ⏮ ‹ › ⏭ go to the first, previous, next and last
hadith, so do the arrow keys and the mouse wheel; typing lists the numbers that begin so, a number the book does not
have offers the nearest, and **Random hadith** picks one among those whose chain was read. `iy chain kafi` prints
the range. **📖 From the contents…** chooses without knowing the number: the book's chapters, then the hadith of the
chapter with the start of their text and how many narrators each chain was read into, a word to look for among them,
and a double click shows the chain (`iy chain bukhari --chapter 3`). A hadith a book gives without a chain says so
above its text.

<p align="center">
  <img src="https://raw.githubusercontent.com/bayramkotan/isnady/main/assets/screenshots/chain.png" alt="Isnad Chains — Sahih al-Bukhari 1: the number with the book's last number and its place, first / previous / next / last, and the chain as a timeline from the book to the Prophet" width="850">
</p>

### 👤 Who each narrator is

Every name in a chain is matched to a narrator of its own tradition's rijal work — the *Taqrib*
for a Sunni book, al-Najashi for a Shia book (then al-Tusi's *Fihrist*, then his *Rijal*, for the men al-Najashi
does not have: a man found in an earlier book is that book's entry). A name is linked only when the evidence leaves one
person: the words of the name, the book marks (a narrator in al-Bukhari must be one Ibn Hajar marks
خ), the order of generations along the chain, the last link before the Prophet being a Companion,
and — in al-Kafi — al-Kulayni's own teachers at the head of his chains. When several narrators
remain — "Sufyan" can be al-Thawri or Ibn 'Uyayna — the chain says so and names how many, rather
than choosing one. In blind, hand-checked samples of the identified names, the last fifty were all
correct. Names are kept exactly as written, with the clarifications the compilers added
(*"— yaʿnī Ibn Muḥammad —"*).

**More narrators known (0.7.6).** Ibn Hajar writes inside a name how it is spelt — "سماك بكسر أوله وتخفيف الميم بن
حرب" — and those words had kept 1,952 of his entries from the chains' plain "سماك بن حرب"; they are now left out of
the name looked up (the entry keeps them). A name the chains give without "بن" — a kunya, a nisba, an ism with its
nisba: "عمرو الناقد", "خالد الحذاء", "القعنبي", "أبي الصباح الكناني" — is found inside the entries' names when no
entry begins with it; "سمعت أنسا" is Anas; al-Saduq's "حدثنا أبي" (and "أبي ره") is his father. With al-Tusi's two
books beside al-Najashi, the Shia chains find al-Hasan b. Mahbub, al-Husayn b. Sa'id, al-Nadr b. Suwayd and the
companions of the Imams al-Najashi left out. On the whole of isnady-data, the narrators identified rose from **76.7%
to 79.4%** of the Sunni links and from **53.8% to 66.8%** of the Shia links (al-Kafi 65% → 75%); the chains
themselves are read as before. The Python isnady and isnady-rs identify the same narrator at every link.

**Fathers and bare names (0.7.7).** "عن أبيه" is now read from the son's own entry far more often: the father is the
entry that carries what the son's nasab says of him — his grandfather (with the letters' dots set aside: "الخصيب" is
"الحصيب"), his kunya anywhere in his entry, the ancestors above him, his tribe, his patron, or the son named in it
("والد عطاء", "أبو المعتمر"): Mu'tamir's father is Sulayman al-Taymi, Ibn Abi Najih's Abu Najih Yasar, 'Ata' b.
al-Sa'ib's al-Sa'ib b. Malik, Ahmad b. Muhammad b. 'Isa's Muhammad b. 'Isa al-Ash'ari. A son left between namesakes —
"سالم عن أبيه", "يعقوب، حدثنا أبي", "ابن نمير، حدثنا أبي" — is the one whose entry gives him a father, taken with that
father when the chains have seen the pair. And the bare names left open most often are read by the narrators around
them, as the books of *tamyiz al-muhmal* read them: "محمد بن جعفر" from Shu'ba is Ghundar, "يحيى بن سعيد" between
Musaddad and Shu'ba al-Qattan and between Malik and 'Amra the Ansari qadi, "إسماعيل" from Ayyub Ibn 'Ulayya and from
Qays Ibn Abi Khalid, "جرير" from Mansur Ibn 'Abd al-Hamid, "عطاء" from Ibn Jurayj Ibn Abi Rabah, "زهير" from Abu
Ishaq Ibn Mu'awiya, "حجاج", "جابر"; "غندر", "أحمد بن يونس" and "عبد بن حميد" are found by those names. On the whole of
isnady-data the narrators identified rose from **79.4% to 83.8%** of the Sunni links (al-Bukhari 84.8% → 89.7%,
Muslim 86.2% → 91.0%, Ahmad's Musnad 70.8% → 76.1%) and from **66.8% to 67.7%** of the Shia links — the same in both
programs, table by table.

<details>
<summary><b>📊 How well it reads today</b></summary>
<br>

| | Chains (roads) | Read whole | Reaching the Prophet / a Ma'sum | Names identified |
|:--|--:|--:|--:|--:|
| Sahih al-Bukhari | 8,085 | 99.7% | 78.3% | 89.7% |
| Sahih Muslim | 12,382 | 100.0% | 70.7% | 91.0% |
| Sunan Abi Dawud | 6,422 | 100.0% | 76.7% | 86.6% |
| al-Kafi (read the Shia way) | 14,436 | 98.8% | 94.6% | 75.9% |

(isnady 0.5.4, chains isnady-isnad-15; the names identified: isnady 0.7.7, narrators isnady-narrators-19, on the whole
of isnady-data. A road of a tahwil (ح) is a chain of its own;
"reaching" counts the chains whose last link is the Prophet ﷺ or a Ma'sum — a chain may also end with a Companion's
own word. Only what the text says is drawn: where a book writes "وعنه" or "بهذا الإسناد", the chain of the hadith
before it is drawn beside the road, marked as taken from there, and kept out of these figures; a text that merely
goes on from the hadith before is given no chain. A road of a tahwil whose meeting the text does not name goes on
as the other road only where the narrators attest it: its last narrator is found narrating from that one in at least
three other hadith, in chains read whole — and the chain page says so.)

In a blind, hand-checked sample of 30 Sunni chains: **no wrong narrator**, 27 complete. The rest are
kept whole, each with its reason: *tahwil* (ح, a second chain joining), two teachers at one link
(*qiran*), a chain that refers to an earlier one (*bi-isnadihi*), or wording that could not be read
with confidence. `iy isnads` gives the figures for every book you imported.

</details>

### 🎓 The words of transmission explain themselves

| Term | Reads | What the critics took it to mean |
|:--:|:--|:--|
| حدثنا | narrated to us | The teacher recited it to a group: heard directly |
| حدثني | narrated to me | Recited to the narrator alone: heard directly |
| أخبرنا | informed us | Often a text read back to the teacher (*ʿarḍ*) |
| أنبأنا | told us | Later often transmission by permission (*ijāza*) |
| سمعت | I heard | Direct hearing, stated explicitly |
| قرأت على | I read to | The narrator read the text back to the teacher |
| عن | from | Does not say how it was received (*ʿanʿana*); connected when the two could have met and the narrator is not known for *tadlīs* |
| أن | that | Treated by most critics like ʿan (*muʾannan*) |
| — | (his teacher) | The compiler names his teacher without a term, as al-Kulayni does: his own transmission from that teacher |

---

## 🕌 Sunni and Shia: two roads

Sunni and Shia meet on the same pages — one search, one Narrators page, one shelf of books — and
every Shia narrator and book carries a gold **Shia** tag. They meet without being mixed: every value
isnady derives is reached along the road of the book's own tradition.

| | Road A — Sunni books | Road B — Shia books |
|---|---|---|
| How a chain opens | a transmission term on every link (حدثنا، أخبرنا، عن …) | al-Kulayni names his teacher without a term (علي بن إبراهيم عن أبيه …) |
| Groups and joined roads | two narrators at one link (qiran) | "عدة من أصحابنا" — al-Kulayni's groups, shown with the teachers he said they stand for; "… جميعاً عن …", two roads joined; "رفعه"; "عن رجل"; "عنه" |
| Where it ends | the Prophet ﷺ | one of the Fourteen Ma'sumun: the Prophet ﷺ or an Imam, most often by his kunya — أبو عبد الله is Ja'far al-Sadiq, أبو جعفر al-Baqir, أبو الحسن al-Kazim or al-Rida; "سألته" is *mudmar* |
| Who identifies its narrators | Ibn Hajar's *Taqrib al-Tahdhib* | only the Shia rijal works: al-Najashi's *Rijal*, then al-Tusi's *Fihrist* and *Rijal* |
| How a narrator is judged | Ibn Hajar's twelve ranks | the Imami critics' own terms — **reliability** (*thiqa*; praised — *jalil*, *wajh*, *'ayn*; weak) and **creed** (Imami, or Waqifi, Fathi, Zaydi, *'ammi* …): an Imami *thiqa*, a praised narrator (*mamduh*), a *thiqa* of another school (*muwaththaq*), a weak one |
| Whose grades | al-Albani, Shu'ayb al-Arna'ut, Zubair 'Ali Za'i … | al-Majlisi (*Mir'at al-'Uqul*), al-Behbudi (*Sahih al-Kafi*) … |

Nothing is mapped onto the other tradition's measure, and a narrator of one tradition is never a
candidate for a link of the other's chains. Read this way, **96.8%** of al-Kafi's 14,239 chains
split into their narrators (9 did when they were read the Sunni way), **92%** reach a Ma'sum, and
three in four of their 64,231 names are identified in the Shia rijal works (3 were before).

<p align="center">
  <img src="https://raw.githubusercontent.com/bayramkotan/isnady/main/assets/screenshots/chain-shia.png" alt="A chain of al-Kafi read the Shia way: al-Kulayni's group of teachers, the narrators judged by al-Najashi, ending at Imam Ja'far al-Sadiq" width="850">
</p>

```bash
iy chain kafi 5:1740                                 # al-Kulayni's group, al-Najashi's verdicts, the Imam it reaches
iy search "زرارة" --in narrators --tradition shia
iy isnads                                            # per book: chains split, and whom they reach
```

---

## 👥 Narrators

The **Narrators** page lists every narrator of the imported rijal works — **All**, **Sunni** or
**Shia** at the top of the list — searchable in Arabic or in Latin letters (`Abu Hurayra`, `Zuhri`,
`Ibn Umar`, `زرارة`). For each narrator:

- the name he is **known by** (al-A'mash, Ibn 'Umar, Zurara), with his full name and every part of
  it laid out — name, lineage, kunya, by-name, nisbas, client of — the part he is known by marked ★;
- **his life**, in English, Turkish or Arabic: for 152 people — the Companions who narrated most, the
  ten promised Paradise, the Mothers of the Believers, the great Successors and their followers, the
  imams of law and of criticism, the compilers of the books, and on road B the companions of the Imams
  and the Imami scholars — a life written for isnady: full name, birth, death, family, teachers,
  students, works and what they are known for, told in a few paragraphs and summarised from the
  classical biographical works named under it (Ibn Sa'd, Ibn 'Abd al-Barr, Ibn Hajar's *al-Isaba*,
  al-Dhahabi's *Siyar*, al-Mizzi … for the Sunni road; al-Najashi, al-Tusi, al-Kashshi … for the Shia
  road). For every other narrator, his life as far as the data says it, in plain sentences: his
  generation, the year of his death with its Common Era year, his rank, the books his hadith are in,
  how often he is in the chains, his teachers and students most often met there;
- what his critic says, in the critic's own words: Ibn Hajar's verdict and its rank for a Sunni
  narrator; al-Najashi's words, reliability and creed for a Shia narrator — the entry as written one
  click away;
- **the entry of the classical work beside the life written for isnady**: for 109 of the written lives, al-Dhahabi's
  *Siyar A'lam al-Nubala'* in its own words — "vol. 2, no. 126" for Abu Hurayra — downloaded once on request (about
  22 MB, from OpenITI) and kept in your data folder; the Prophet's life ﷺ points to Ibn Hisham's *Sira* and to
  al-Tirmidhi's *al-Shama'il* in isnady;
- tabaqa and death year, the books his hadith appear in (Ibn Hajar's marks);
- **whom he narrates from and who narrates from him** in the imported chains, and the compilers who
  narrate from him directly;
- every hadith whose chain includes him, each a click from its chain.

**Who a name in a chain is.** A chain names its men as the narrators named each other — "Ibrahim", "Abu Ishaq",
"al-Hasan" — and isnady reads each name against the Taqrib (or al-Najashi on road B) by the name itself, the
generation, the books, and then by its neighbours: whom the same pair of teacher and student is in the chains where
they are named in full; a client named "mawla of X" by his patron; the well-known men a bare name means in a given
circle (Ibrahim from 'Alqama is al-Nakha'i, Abu Ishaq from al-Bara' is al-Sabi'i, al-Humaydi's Sufyan is Ibn
'Uyayna, Abu Nu'aym's al-Thawri, "Hisham from his father" Hisham b. 'Urwa); and last, a bare name still open is the
famous man of that name when nothing else excludes him ("Anas" Anas b. Malik, "Malik" Malik b. Anas, "Ibn 'Umar"
'Abd Allah b. 'Umar). The unnamed men of the Taqrib ("'Ata' b. Yasar from one of the Companions") are never matched
to a name. On road B the same help reads al-Kulayni's chains: his teachers named short ("'Ali b. Ibrahim" b. Hashim
al-Qummi, "Muhammad b. Yahya" al-'Attar), the nisbas and kunyas the chains use alone (al-Sakuni, al-Nawfali, Abu
Basir, al-Halabi, Ibn Faddal) and "from his father" read from the son's full name ("'Ali b. Ibrahim from his father"
is Ibrahim b. Hashim). About 280,000 links are identified this way — two of every three in the Sunni chains, one in
two in the Shia ones.

**Who is not a narrator.** The Prophet ﷺ named by his kunya or an epithet ("سمعت أبا القاسم ﷺ", "حدثني خليلي",
"الصادق المصدوق") ends the chain; he is never matched to a narrator of that kunya. The man a Companion tells a story
about is not his teacher: "عن أبي هريرة أنّ سعد بن عبادة قال لرسول الله" ends at Abu Hurayra, and so does "عمر،
سمعت هشام بن حكيم يقرأ". A second road written after a link — "… عن عروة، عن عائشة، وعن هشام بن عروة، عن أبيه، عن
عائشة", "وحدثنا …", "قال وأخبرني …", "، وأبي الزناد، عن الأعرج" after a Companion — is drawn as its own chain, joined to
the first where they meet, never read on as one; a road begun again without a sign (a narrator named twice) is cut
there. And two neighbours that cannot both be right — a narrator his own teacher, a teacher four generations after
his student or six before him — are not left so: the less certain of the two is left unidentified.

**The famous ones, however they are written.** ★ **Famous Companions** on the Narrators page and in the Genealogies
lists them group by group — the four Caliphs, the ten promised Paradise, the Mothers of the Believers, the Prophet's
household, those who narrated most and some forty more (`iy narrator --famous`); on road B, the Imami scholars' own
Ashab al-Ijma'. Typed, a famous Companion comes first whichever way he is written — "Ebubekir", "Ebû Bekr",
"Abu Bakr", "Hz. Ömer", "Omer ibn Hattap", "Umar bin Khattab", "Osman", "Uthman", "Ayşe", "Aisha", أبو بكر الصديق —
shown by the name people know him by ("Hz. Ebû Bekir es-Sıddîk (r.a.)": his entry begins with 'Abd Allah b.
'Uthman), and every name found carries its tabaqa and death year, so 'Umar b. al-Khattab the Caliph is not mistaken
for the two later men of that name.

Sunni narrators filter by tabaqa, rank and book; Shia narrators by the Imami assessment.

<p align="center">
  <img src="https://raw.githubusercontent.com/bayramkotan/isnady/main/assets/screenshots/narrators.png" alt="Narrators — 'Abdullah b. 'Umar: Ibn Hajar's verdict, tabaqa, death year, teachers and students in the chains" width="850">
</p>
<p align="center">
  <img src="https://raw.githubusercontent.com/bayramkotan/isnady/main/assets/screenshots/narrators-shia.png" alt="Narrators — Zurara b. A'yan, a Shia narrator: al-Najashi's words and their reading in the Imami terms" width="850">
</p>

---

## 📖 Books

isnady is not only for searching: the **Books** section opens every imported work and reads it in
its own order. A hadith collection reads chapter by chapter, each hadith with its card (chain,
narrators, other narrations, grades) and the languages you choose; Ibn Hajar's *Taqrib* and
al-Najashi's *Rijal* read entry by entry, each narrator a click away. Under **Sira and tabaqat**, on every shelf,
stand Ibn Hisham's *Sira* — the life of the Prophet ﷺ in 264 chapters, each section under its own heading, each
report with its chain as Ibn Ishaq gives it — and al-Dhahabi's *Siyar A'lam al-Nubala'* and *Tadhkirat al-Huffaz*,
life by life, generation by generation: the author's text as the edition gives it, nothing summarised.

<p align="center">
  <img src="https://raw.githubusercontent.com/bayramkotan/isnady/main/assets/screenshots/books-sira.png" alt="Books — Ibn Hisham's Sira, chapter 20 of 264: each section under its own heading, the reports with their chains as Ibn Ishaq and Ibn Hisham give them; on the shelf the Qur'an and, under Sira and tabaqat, al-Dhahabi's Siyar and Tadhkira" width="850">
</p> Go to a chapter by its number ("34 / 57")
or to the first, previous, next or last one; in a hadith collection go to any hadith by its number, with the
book's last number beside it and the same ⏮ ‹ › ⏭ as on Isnad Chains. Find within a chapter, and isnady remembers
where you were. From any search result, **In its
book** opens the hadith where it stands, among the hadith of its chapter. **⤓ PDF** writes the book, or the chapter
being read, as a PDF in the languages you choose — or as Word, EPUB, HTML and the other formats of File → Export
(`iy export book bukhari --chapter 3 -f pdf --language Turkish`).

**The Qur'an** stands first on the shelf. It is downloaded once, on request (about 3 MB), from **isnady-data**: the
Arabic (Tanzil's text, verbatim) with a translation in each language you read — the interface language first, then
the content languages (Turkish: Diyanet İşleri; English: Saheeh International …), shown in that order. A language you
read with no translation yet is offered above the verses in one click. **➕ Translations…** adds or removes
the others: the 60 translations isnady-data keeps in 18 languages come first, marked ★, under the names their readers
know them by (Diyanet İşleri, Elmalılı Hamdi Yazır, Ali Bulaç, Süleyman Ateş, Hasan Basri Çantay …; Saheeh
International, Abdel Haleem, Mustafa Khattab …); any other of some 440 translations in 98 languages is read from the
source isnady-data's are taken from, and so is a text isnady-data cannot give at that moment. The contents list the 114 suras; a sura shows its verses in Arabic with the translations ticked above
it, ⟨ ⟩ go to the previous and next sura, and the search looks through the whole text, in Arabic or in a
translation's words. Under each verse stand **the hadith that name it** — those whose translation marks the verse as
the one it quotes or explains, "(33:33)", "[43:67]", "(Ahzab 33/51)": the translator's mark, each checked against the
number of verses of its sura (2,280 hadith name 1,960 verses), the Sunni and the Shia books each on a line of their
own; a click reads the hadith in its book. And on every hadith card **📖 Verses it names** opens the verse in the
Qur'an. In the terminal: `iy quran`, `iy quran 33:33 --hadith`, `iy quran search sabır`.

Book and chapter titles follow the interface language: the sources give chapter titles in English only, so
isnady carries its own — all 642 chapter titles of the 33 books in Turkish and in Arabic ("Vahyin Başlangıcı",
"كتاب بدء الوحي" for "Revelation"), and the Arabic titles a source gives itself where it has them (al-Adab
al-Mufrad, Bulugh al-Maram, Mishkat, Riyad al-Salihin, al-Shama'il). The book titles too: *Sahîh-i Buhârî*,
*صحيح البخاري*.

Musnad Ahmad and Sunan al-Darimi come as numbered texts without chapters; isnady keeps their tables of contents
itself, two levels deep, from the editions in OpenITI: the Musnad in its 13 parts (the Ten promised Paradise, the
Household of the Prophet ﷺ, the Companions who narrated most, the Companions of Mecca, Medina, Syria and Kufa, the
Ansar, the Tribes) and under them the musnad of each Companion — 1,252 in all, Abu Bakr al-Siddiq's first —; al-Darimi
in its 24 books (Introduction, Purification, Prayer …) and 1,382 chapters (bab). The parts and books are named in
the interface language; the musnad of every Companion in Turkish and English too ("Hz. Ebû Bekir es-Sıddîk (r.a.)
müsnedi", "Musnad of Abu Bakr al-Siddiq (RA)"), and al-Darimi's 1,382 chapters as well ("Temizlik", "Purification"…
"Abdestin fazileti", "The virtue of ablution").

The shelf holds the Sunni and the Shia books together, each under its own heading — *All*, *Sunni*
or *Shia* at the top shows one of them alone — and every Shia book carries a gold *Shia* tag. Books
are not tied to hadith collections: a work is a tree of any depth (volume, book, chapter, section)
whose leaves are hadith, narrators' entries or paragraphs — so commentaries, manuals of fiqh and the
other works of the scholars can be read the same way as they are added.

<p align="center">
  <img src="https://raw.githubusercontent.com/bayramkotan/isnady/main/assets/screenshots/books.png" alt="Books — the Sunni and Shia shelves; Sahih al-Bukhari chapter 2 of 98 and hadith 8 of 7,583, with first / previous / next / last for chapters and hadith; Arabic, English and Turkish, each hadith with its chain" width="850">
</p>

```bash
iy book                      # the books
iy book bukhari              # its chapters
iy book bukhari 2 --language Turkish
iy book taqrib 1
```

---

## 🎓 Hadith Scholars

The scholars whose work is in the imported data, and what the data measures of it: **compilers**
(their collection, the teachers their chains begin with, the Companions they end with), **graders**
(how they grade, and how far they agree with each other on the same hadith — agreement, Cohen's
kappa, and a strictness index: the classical *mutashaddid* / *mutasahil*, measured), and **critics**
of narrators — each with **his life** (the same written lives as on Narrators) — Sunni and Shia on one
page (*All / Sunni / Shia*; Narrators, Books and Scholars open on *Sunni*), each by his own road: al-Kulayni,
al-Saduq, al-Mufid and al-Tusi with their chains counted to a Ma'sum; al-Majlisi and al-Behbudi compared
only with each other on the four Imami grades (sahih, hasan, muwaththaq, da'if); al-Najashi's verdicts in
the Imami terms. On Sunan Abi Dawud, for example, Zubair 'Ali Za'i grades most strictly (−0.21) and
agrees with al-Albani on 70% of the hadith; the page also flags a pair of graders whose 98%
agreement suggests the source did not keep them independent.

A fourth group, **Huffaz**, is al-Dhahabi's *Tadhkirat al-Huffaz* (d. 748 AH): the **1,176 masters of hadith**
he counts, in his **21 generations**, from Abu Bakr al-Siddiq to his own teachers — each with his generation, his
year of death (al-Dhahabi's, read from the entry; else the *Taqrib*'s, marked), al-Mizzi's signs of the six books
he narrates in, and, for the **418** who are also narrators of the *Taqrib*, Ibn Hajar's verdict, the chains he
is in here and a button to his narrator page (tied only when the name, the father, the grandfather and the two
years of death agree). Above them the book at a glance: how many each generation holds and when they died, by
century of the Hijra, beside al-Dhahabi's own count of a generation where he gives one. Only the index is
bundled; al-Dhahabi's entry itself is downloaded once on request (OpenITI, about 3 MB) and opens on the card.

<p align="center">
  <img src="https://raw.githubusercontent.com/bayramkotan/isnady/main/assets/screenshots/scholars.png" alt="Hadith Scholars — al-Albani: his grades on Sunan Abi Dawud and his agreement with the other graders" width="850">
</p>

---

## 🌿 Companions and the Household

Groups of the Companions and of the Prophet's family ﷺ as the hadith name them, each on its own road (Sunni and Shia
apart, as everywhere in isnady). On the Sunni road: **the ten promised Paradise** (al-'Ashara al-Mubashshara: 'Abd
al-Rahman b. 'Awf's list in al-Tirmidhi, Sa'id b. Zayd's in Abu Dawud, and how they differ), **the people of the cloak**
(the hadith of the cloak in Muslim and al-Tirmidhi with al-Ahzab 33:33, what the Sunni scholars read in it, and Zayd
b. Arqam's word on the household in the hadith of the two weighty things) and **the Mothers of the Believers**. On the
Shia road: **the Fourteen Infallibles** — the Prophet ﷺ, Fatima and the twelve Imams, with al-Kafi's hadith on
al-Nisa' 4:59 and the years of al-Mufid's al-Irshad. Each member has his years, his life, a button to him among the
narrators and to his box in the family tree, and what the imported books hold of him: on the Sunni road how many
chain links and hadith he is in (Abu Bakr 54 hadith, 'Ali 518, 'A'isha 5,976), on the Shia road how many chains end at
him (Ja'far al-Sadiq 12,007; al-Baqir 2,969). The hadith that name each group are shown in full, chain and text, from
the books imported here. In the terminal: `iy group`, `iy group ten --lang tr`, `iy group masumin`.

---

## 🕌 Sira

The life of the Prophet ﷺ on one line, from the Year of the Elephant to his death, in its three periods — before the
mission, Mecca, Medina. Thirty-eight events, each a dot on the line and a row in the list: the birth, Halima and the
desert, the first revelation in Hira, the emigration to Abyssinia, the boycott, al-Ta'if, the Night Journey, the pledges
of al-'Aqaba, the Hijra, the change of the qibla, Badr, Uhud, the Trench, al-Hudaybiya, Khaybar, the conquest of Mecca,
Hunayn, Tabuk, the Farewell Pilgrimage and his death. Each event gives its date in the Hijri and the Common Era years,
his age, the place, and **where the reports differ, says so beside it** (the day of his birth, the year of the Trench,
how long the boycott lasted …). Under it: **the verses revealed about it** (each opens the Qur'an at the verse), **the
chapter of Ibn Hisham's Sira that tells it** (read in Books when the Sira is imported) and **the hadith that name it**
— those of the Sunni books with its phrase ("يوم بدر": 204 in the full data), opened in Search. The dates follow Ibn
Ishaq as Ibn Hisham edited him. In the terminal: `iy sira`, `iy sira badr`.

<p align="center">
  <img src="https://raw.githubusercontent.com/bayramkotan/isnady/main/assets/screenshots/sira.png" alt="Sira — the life of the Prophet on one line in three periods; the battle of Badr with its date, place, his age, the verses of al-Anfal and Al 'Imran, Ibn Hisham's chapter and the hadith that say 'the day of Badr'" width="850">
</p>

---

## 📜 Written sources

Most hadith came down by word of mouth, but some were written in the first generations, and the page shows where that
writing lives in the books imported here — the Sunni road and the Shia road apart.

- **Came down whole: the Sahifa of Hammam b. Munabbih.** Hammam wrote what Abu Hurayra told him; Ahmad b. Hanbal
  copied the sahifa into his Musnad under one chain ('Abd al-Razzaq ← Ma'mar ← Hammam ← Abu Hurayra), the first hadith
  with the chain and the rest "And the Messenger of God ﷺ said …". isnady finds those runs in the imported Musnad —
  **138 hadith, the number of Hamidullah's edition of the manuscript** — and shows beside each its parallels in the
  other books that narrate it through Hammam: 84 of them in al-Bukhari, Muslim, Abu Dawud, al-Tirmidhi and al-Adab
  al-Mufrad (128 hadith), matched by wording.
- **Lives in later books:** 'Abdullah b. 'Amr's *al-Sahifa al-Sadiqa* ('Amr b. Shu'ayb from his father from his
  grandfather — 253 hadith), the sahifa of 'Ali, Abu Bakr's letter on zakat, the letter for 'Amr b. Hazm; on the
  Shia road the Book of 'Ali (68), al-Jami'a and what the Shia books relate of Mushaf Fatima — each in the words of
  its own books.
- **Writing in the chains:** the links that say the text came in writing — "he wrote to me", "I found in the book of
  …" ('Abdullah b. Ahmad's "I found in my father's book, in his hand"), "I read in the book of …"; on the Shia road
  the letters to the Imams.

Every list is isnady's reading of the texts — a chain or a wording — and the page says how each was found. In the
terminal: `iy sahifa`, `iy sahifa hammam`, `iy sahifa kitab-ali`.

**Each one is also a book.** isnady-data keeps every written source as its own entry — under `sahifa/sunni/` and
`sahifa/shia/` — and **Data sources → Written sources** lists them the same way, the Sunni road and the Shia road
apart. Imported, it stands on the shelf of **Books** with them (*Sunni written sources · 6*, *Shia written sources ·
4*) and reads chapter by chapter: the Sahifa of Hammam opens with Musnad Ahmad (138), then *Its parallels in Sahih
al-Bukhari* (56), *in Sahih Muslim* (56) … Nothing is copied — each leaf is the hadith in its own collection. From the
terminal: `iy catalog import hammam`, `iy book sahifat_hammam_ibn_munabbih`.

<p align="center">
  <img src="https://raw.githubusercontent.com/bayramkotan/isnady/main/assets/screenshots/books-sahifa.png" alt="Books — the Sahifa of Hammam b. Munabbih as a book: Musnad Ahmad 138, then its parallels in al-Bukhari, Muslim, al-Adab al-Mufrad, al-Tirmidhi and Abu Dawud" width="850">
</p>

<p align="center">
  <img src="https://raw.githubusercontent.com/bayramkotan/isnady/main/assets/screenshots/written-sources.png" alt="Written sources — the Sahifa of Hammam b. Munabbih: 138 hadith in the Musnad, 84 with parallels in al-Bukhari, Muslim and the others" width="850">
</p>

---

## 📊 Statistics

Measured in depth, each figure with what it means in plain words and how sure it is — on two roads:
*All Sunni books* (Ibn Hajar's ranks, chains to the Prophet ﷺ) and *All Shia books* (al-Najashi's Imami
categories, chains to a Ma'sum; al-Majlisi's and al-Behbudi's grades in the four Imami categories —
sahih, hasan, muwaththaq, da'if), never mixed. Every section is measured on both roads. Grades are
ordered categories, so they are never turned into numbers or averaged, and no figure is read
against the science of hadith: the statistics measure what the scholars did, they do not overrule
it. Every result is computed once and saved as JSON in your data folder (`derived/`), and computed again only when
what it was computed from changed — new books or hadith, narrators identified differently, new verdicts or grades,
books switched off — or the analysis itself; a new version of isnady alone does not.

**For readers who never studied statistics:** under every headline an open box, *In plain words*, says what came out
here ("τb 0.35 — a moderate relation, too clear to be chance"; "the busiest tenth of the narrators carry 84% of all
appearances — the pivots (madar) the critics speak of"), and its words — Gini, Kendall's τb, the 95% interval, the
odds ratio, kappa … — open on a click with what they measure, how to read them, what they mean for the science of
hadith, and an example.

**Out of isnady, piece by piece or whole:** every finding has a ⤓ — its chart as a picture (PNG, SVG), a PDF or on
the clipboard, its figures as CSV, Excel, JSON or Markdown; the page's **Export** writes a section or every section
as one PDF or one web page with the charts and their tables, every table as one Excel workbook (a sheet each) or as
CSV files, and the computed results as JSON (`iy export statistics [BOOK]`).

| Section | What it shows |
|---|---|
| **All books** | the first entry of the book list: every section measured on both roads at once, each road under its own heading — never added together |
| **Overview** | the corpus at a glance: books, hadith, chains, narrators, grades |
| **Narrators** | reliability, concentration (the busiest 10% carry 75% of the transmission — the Lorenz curve) and the pillars of transmission |
| **Chains** | length, the weakest link, and the time between narrators (who could not have met) |
| **Grades and chains** | the weakest narrator and the length of a chain against its grade |
| **Books** | the books side by side |
| **Graders** | agreement of every pair (same grade, Cohen's and the ordinal kappa, 95% intervals, Krippendorff's alpha); a **Dawid–Skene model** of each hadith's true grade and each grader's confusion matrix; **strictness**; the **disputed hadith** |
| **The hadith model** | what each scholar's grade follows — ordinal regression with odds ratios and 95% intervals, and the hadith graded higher or lower than their chain |
| **Terms explained** | every measure of the page in everyday words — what it measures, how to read it, what it means for the science of hadith, an example (`iy stats --explain`) |
| **Correlations and tests** | every measure against every other (Spearman's ρ with Benjamini–Hochberg q-values, the partial ρ with the others held equal) and what differs: graders pair by pair (Stuart–Maxwell), books (Kruskal–Wallis, Mann–Whitney with Cliff's δ), whether the model's shape holds (Brant) |

<p align="center">
  <img src="https://raw.githubusercontent.com/bayramkotan/isnady/main/assets/screenshots/statistics.png" alt="Statistics — Narrators: the busiest 10% of narrators carry 75% of the transmission; the Lorenz curve and the pillars" width="850">
</p>
<p align="center">
  <img src="https://raw.githubusercontent.com/bayramkotan/isnady/main/assets/screenshots/statistics-model.png" alt="The hadith model — for al-Albani, a weak or unknown weakest narrator multiplies the odds of a weaker grade by 14; odds ratios with 95% intervals" width="850">
</p>
<p align="center">
  <img src="https://raw.githubusercontent.com/bayramkotan/isnady/main/assets/screenshots/statistics-lab.png" alt="Correlations and tests — each factor against each scholar's grade with the others held equal (partial Spearman's rho), Sunan Abi Dawud" width="850">
</p>

```bash
iy stats                     # what the database holds, per book
iy stats graders --book abudawud
iy stats model --grader albani
iy stats lab --csv ./lab     # every table behind the figures, as CSV
iy stats --explain kendall   # any measure in plain words (or every one: iy stats --explain)
```

---

## 🎓 Learn

The terms of hadith and its sciences, written for isnady with their classical sources: 81 terms —
the grades (sahih, hasan, da'if, mawdu', shadhdh, munkar, mu'allal), the kinds of hadith by their
chain (mutawatir, mursal, munqati', mu'allaq, mudallas …), chain and text (mutaba'a, shahid,
takhrij, madar), the ways of receiving (sama', ijaza, haddathana, 'an), judging narrators (the
twelve ranks of Ibn Hajar, thiqa, saduq, majhul, matruk, mudallis), generations, the parts of an
Arabic name, the Imami grades (sahih, hasan, muwaththaq, da'if), the books, and isnady's own
measures. Each with its Arabic, a one-line and a longer definition in **English, Turkish or Arabic** — in the interface language when the glossary
has it — related terms one click away, and its source. The grades on every result explain themselves
when you point at them. `iy term mursal --lang tr`, `iy term mursal --lang ar`.

Each of the ten categories has its own colour and sign — on the chips above the list (a click shows that
category alone), the dot before each term, the band of the term's card, its "In short" box and the chips of its
related terms; the card reads from the top: the term large with its Arabic, its names in the other languages,
*In short*, *In detail*, the source, the related terms, and more of the same category to read on.

**History and method** (the 📜 button at the top of Learn, and `iy learn`): four articles in English, Turkish and
Arabic, each step with its source — *How the hadith were written down and gathered* (the Companions' written sheets,
'Umar b. 'Abd al-'Aziz's order and al-Zuhri, the books by subject, the Musnads, the two Sahihs and the Sunans, the
science and its commentaries, with a timeline), *Transmission in writing, and its numbers*, *The logic of the
method* (the chain, the five conditions of the sahih, jarh and ta'dil, mutawatir and ahad, why the two Sahihs and
not the Musnad stand first, why the sifting itself is the proof) and *Hadith in numbers*. Their numbers are counted
live from the books you imported, each tradition apart: the chains that say a written text stood between two
narrators (read to the teacher, sent in writing, found in a book, dictated, from his book; "akhbarana" in Muslim and
al-Nasa'i by their own usage), the hadith of Hammam b. Munabbih's Sahifa, the texts that speak of writing, how many
books and chains carry "Whoever lies about me deliberately", and the size of the corpus — books, hadith, chains,
names, narrators by generation, the Companions most often at the end of the chains, the graders' grades.
`iy learn writing --lang tr`.

<p align="center">
  <img src="https://raw.githubusercontent.com/bayramkotan/isnady/main/assets/screenshots/learn.png" alt="Learn — the term mursal in its category's colour: Arabic, In short, In detail, source, related terms" width="850">
</p>

---

## 🌳 Genealogies

Four trees, drawn so they can be zoomed, moved and clicked (**Genealogies** in the sidebar, `iy tree`):

| Tree | What it shows |
|---|---|
| **The prophets** | From Adam to Muhammad ﷺ as the Qur'an, al-Bukhari and the classical historians give it — Ibn Hisham, Ibn Sa'd, al-Tabari, Ibn Kathir: Nuh, Ibrahim and his sons, the prophets of the Children of Israel, the line of Isma'il to 'Adnan and on to the Prophet ﷺ. A link whose generations are not known is drawn dashed; a report the sources differ on has a dashed box and says so |
| **Quraysh and the Prophet's family** | From Ka'b b. Lu'ayy: the Prophet's family ﷺ, the lineages of the four caliphs joining his, the Umayyads, and the Imams of the Shia |
| **A narrator's family** | Read from the narrators' own names: "X b. Y b. Z" is the son of "Y b. Z" when exactly one narrator of that name, of the same tradition and of an earlier or the same generation, is found — over a thousand such links in the Taqrib and al-Najashi (Ibn 'Umar's sons and grandsons, the family of al-Zubayr …). A reading of the names, said so on the page |
| **A narrator's line of learning** | His teachers above him and theirs, his students below him and theirs, the most frequent at each step, counted from the imported chains |

**Where the written trees come from.** Sunni sources, each link marked with how strong its source is: the Qur'an
where it names a relation (Ibrahim's sons Isma'il and Ishaq, Ya'qub …); al-Bukhari's *Sahih* (Manaqib al-Ansar,
the Prophet's lineage ﷺ up to 'Adnan, on which the scholars agree); the sira and history works (Ibn Hisham, Ibn
Sa'd, al-Tabari, Ibn Kathir); the genealogists of Quraysh (Mus'ab al-Zubayri, Ibn Hazm). The names between Adam
and Ibrahim and between Isma'il and 'Adnan are the historians' only — they took them partly from the People of the
Book, and neither the Qur'an nor an authentic hadith gives them; the page says so on those names. The Imams of the
Shia are shown as the line of the Prophet's family ﷺ that the Sunni genealogists record too; that they are Imams
is the Shia's belief, shown as theirs (where the Sunni historians differ — a son of al-Hasan al-'Askari — the box
is dashed and both are given).

Click a name for what the sources say about it. The Prophet ﷺ opens his life, with the sira to read; a prophet
whose name the Qur'an and the hadith give opens the hadith that speak of him — the Sunni books and the Shia books
apart, each a click from its book; a Companion, a caliph or an Imam whose life isnady has opens it, and the Narrators
page links back to his place in the tree of Quraysh. A narrator opens on the Narrators page, which has the links to
his family and his line of learning too. `iy tree prophets`, `iy tree prophets Musa`, `iy tree family "Ibn
Umar"`, `iy tree learning al-Zuhri --depth 2 --width 4`.

<p align="center">
  <img src="https://raw.githubusercontent.com/bayramkotan/isnady/main/assets/screenshots/genealogy.png" alt="Genealogies — the prophets: Ibrahim, his sons Isma'il and Ishaq, Ya'qub and the tribes" width="850">
</p>
<p align="center">
  <img src="https://raw.githubusercontent.com/bayramkotan/isnady/main/assets/screenshots/genealogy-family.png" alt="Genealogies — a narrator's family read from the narrators' names: 'Umar b. al-Khattab, Ibn 'Umar, Salim and their sons" width="850">
</p>

---

## ⭐ Bookmarks

Every hadith card, narrator, scholar, term of Learn, chapter of Books and place in the Genealogies has a ☆: one
click keeps it among your **Bookmarks**, and the star says what it keeps ("Add this narrator to your bookmarks").
Each bookmark shows its kind — Hadith, Narrator, Scholar, Term, Chapter, Genealogy — and a line on what it is (a
narrator's road, tabaqa, death and verdict; which tree); the list filters by kind, and a narrator kept as a
narrator and the same man kept as a place in his family tree are two bookmarks. There they live in folders of your own — nested as deep as you like, one bookmark in as
many folders as you want, dragged onto a folder to file it — each with a note of your own, found by words or by
kind, opened with a click where it belongs. They are kept in `bookmarks.json` in isnady's data folder (yours
alone, never published) and can be exported and imported as a file; a hadith is kept by its book and number, so a
bookmark survives a new import. `iy bookmark add hadith bukhari:1 --folder "Intention/Deeds" --note "…"`,
`iy bookmark list`, `iy bookmark list tree`, `iy bookmark add tree prophets:musa`, `iy bookmark folders`,
`iy bookmark export file.json`.

**Where you left off.** Each book remembers by itself the chapter and the hadith you had come to, and opens there
again ("🔖 You left off at hadith 59" under it on the shelf). Put markers of your own wherever you like and name them
("Prayer reading", "Ramadan"), as many as you want, in any book; the 🔖 Markers menu of the Books page goes to any of
them, or to where you left each book. Each marker in that menu opens to *Go to this marker*, *Rename…* and *Remove
this marker*; each book's place to *Go there* and *Forget where I left this book*; and at the bottom *Remove all
markers…* and *Forget where I left every book…*. They are kept with your profile, and the same from the command
line: `iy book bukhari --resume`, `iy book bukhari 3 --mark "Knowledge"`, `iy book --marks`, `iy book --go knowledge`,
`iy book --unmark knowledge` (`--unmark all`), `iy book --forget bukhari` (`--forget all`).

<p align="center">
  <img src="https://raw.githubusercontent.com/bayramkotan/isnady/main/assets/screenshots/bookmarks.png" alt="Bookmarks — folders on the left, a hadith, a narrator, a scholar and a term with their folders and a note" width="850">
</p>

---

## 🧠 Search by meaning (AI)

Choose **Match → By meaning (AI)** to find hadith that say the same thing in other words or in
another language: `komşu hakları` finds the hadith on the neighbour's rights, a Turkish sentence
finds its Arabic original, and `إنما الأعمال بالنيات` gathers the narrations of the hadith on
intentions from every imported book. Each result shows how close it is in meaning (0–1), which says
nothing about authenticity.

The model is learnt on your computer from the texts you imported — Arabic beside its translations
is a parallel corpus, and cross-lingual latent semantic analysis learns the concepts the languages
share. Nothing is downloaded and no outside model is used. Measured on al-Bukhari and Abu Dawud: for
hadith the model never saw while learning, a Turkish translation finds its Arabic original first 68%
of the time and among the first ten 93% of the time (chance: 0.8%). For an exact quotation the
ordinary word search stays the better tool.

The same step finds the **other narrations of each hadith** across the imported books (takhrij):
every result card says where else it is narrated — *Also narrated in: Sahih al-Bukhari 5070 · 6689 ·
6953 | Sunan Abi Dawud 2201* — and each number opens that hadith. Only the texts are compared, never
the chains. In blind, hand-checked samples 34 of 35 pairs of the first kind and 18 of 20 of the
second were right.

```bash
pip install "isnady[ai]"        # numpy and scipy
iy ai build                     # a few seconds; or Tools → Build AI Indexes
iy search --mode meaning "komşu hakları"
iy tahric bukhari 1             # the other narrations of a hadith
```

---

## 📤 Export and import

**File → Export…** (Ctrl+E) writes anything isnady holds to a file, each in the formats that fit it, with a preview
before it is written; every hadith card has an **Export** link too. **File → Import…** brings it back.

| What | Export as | Import from |
|---|---|---|
| Hadith (by number), search results | JSON, JSON Lines, CSV, TSV, Excel, Markdown, HTML, text, EPUB, Word, PDF, SQLite | — |
| A book or a chapter | the same, and **isnady book** (the format of isnady-data, imported again as a book) | isnady book; an export as JSON, JSON Lines, CSV or TSV |
| Bookmarks | JSON (isnady's own, with the folders), JSON Lines, CSV, TSV, Excel, Markdown, HTML, text, PDF | JSON, JSON Lines, CSV, TSV |
| Narrators | JSON, JSON Lines, CSV, TSV, Excel, Markdown, HTML, text, PDF, SQLite | — |
| Isnad network (who narrated from whom, how often) | GraphML, GEXF (Gephi, yEd), CSV, JSON | — |
| Settings | JSON, CSV, TSV, Markdown | JSON, CSV, TSV |
| Profile | isnady profile, JSON | isnady profile, JSON |
| Everything of mine (backup) | one file: settings, place, bookmarks and the list of the books | the backup |

A hadith keeps its book and number, chapter, texts in the languages chosen, grades and chains — the Arabic as
written, then said in Turkish and English. PDF is drawn by Qt, so the Arabic is joined and set right to left as on
the screen; Excel, Word and EPUB files are written without any extra package.

**Several at once, or all:** books are ticked in a list (All, None, Sunni, Shia) and go into one file, or each into
its own file in a folder; the isnad network is drawn from the books ticked; several profiles each into their own
file; and Import takes several files at once.

**Big exports beside the window:** every export runs in a thread of its own, with a progress bar, the time gone and
the time left, and a Stop button — in the dialog and in the bar at the bottom of the window, so the dialog can be
closed and isnady used meanwhile. Hadith are read and written one after another: every book in every language, with
the chains, as one EPUB (99,567 hadith, 51 MB) takes about a minute and little memory; a stopped export leaves no
half-written file. `iy export` shows the same bar on the terminal.

```bash
iy export --list                                     # what, and in which formats
iy export hadith bukhari:1 bukhari:5-10 -f pdf -o intention.pdf
iy export book bukhari --chapter 1 -f docx --language Arabic --language Turkish
iy export book nawawi -f isnady-book                 # imported again with: iy import-data book …
iy export book bukhari muslim -f xlsx -o two.xlsx    # several books in one file
iy export book sunni -f json --separate -o sunni     # every Sunni book, each in its own file (all, sunni, shia)
iy export network bukhari muslim abudawud -f gexf    # the chains of these books together
iy export narrators -f xlsx --tradition shia
iy export network bukhari -f gexf                    # open in Gephi
iy export search "النيات" -f md
iy export backup -o mine.isnady-backup.zip
iy import-data bookmarks bookmarks.csv --folder Imported
iy import-data book nawawi.isnady-book.zip
iy import-data bookmarks a.json b.csv                # several files at once
```

<p align="center">
  <img src="https://raw.githubusercontent.com/bayramkotan/isnady/main/assets/screenshots/export.png" alt="Export — every book ticked (All, None, Sunni, Shia), all languages with the chains, as EPUB; the bar shows 14%, the book being written, the time gone and the time left, with Stop" width="640">
</p>
<p align="center">
  <img src="https://raw.githubusercontent.com/bayramkotan/isnady/main/assets/screenshots/export-progress.png" alt="The export goes on with the dialog closed: the bar at the bottom of the window, with Stop, while the window is used" width="850">
</p>

---

## 🎨 Appearance and settings

Ten themes, chosen in **Edit → Preferences → Themes** — each shown as a small live isnady window —
or in **View → Theme**. *Sky* (bright sky blue with sunny gold) and *Starry Night* (a gentle dark of lavender and
stars), which "Follow system" switches between, and *Rose Garden* (soft rose and cream) are the friendly ones:
round corners, more room, a sign before each section of the sidebar, and typefaces made for the screen — Nunito
and Lexend for the interface and the translations, Baloo 2 for the titles — while the Arabic stays in Amiri.
Then *Lapis* (lapis and gold of illuminated manuscripts) and *Lapis Night*; *Paper* (warm cream and ink, for long
reading); *Slate*; *Emerald*; *Midnight*; and *High Contrast* (black, white and yellow, for low vision). Every theme has
the same round corners, room and sidebar signs; each keeps its own colours and typefaces.
The Preferences window maximizes and minimizes like any other (and can be resized from its corner), so Data sources
and the long lists have the whole screen when they need it.

Every script has its own reading font, size, colour and line spacing — Arabic, Latin, Cyrillic,
Bengali and Tamil today. The light and the dark theme each keep their own colours, and the interface
font can be changed too. **Edit → Preferences** (Ctrl + ,) applies every change at once; each row
has its own Default button.

**The sidebar in your own order:** Search, Books, Narrators, Isnad Chains, Hadith Scholars, Genealogies,
Statistics, Bookmarks and Learn by default; **Preferences → Sidebar** moves any section up or down (drag it, or
the arrows), Ctrl+1 … Ctrl+9 and the View menu follow, Default puts isnady's order back.
`iy config set view.sidebar books,search,narrators`.

**The languages in your own order:** Arabic always first, then the others as you put them — **Preferences → Books
and languages** (drag a language, or the arrows; Default: A to Z). Every hadith card, the chain said in words and
the language lists follow. `iy config set view.language_order Turkish,English`.

**Fast, with a cache:** whatever costs time to read from the database — the books and their counts, the narrators'
lists and readings, each scholar's numbers, the chains' figures per book, the families read from the names, searches
— is counted once and kept, in memory and in `cache/*.json` in the data folder, until something is imported or
removed. A new start reads the kept files instead of the database, and the pages are drawn when first opened.
**Tools → Clear the Cache** (or `iy cache clear`; `iy cache` shows what is kept) is needed only if something looks
out of date. A long text is laid out once, at the width it is shown at, and the first computation of Statistics runs in
a separate process, so the window stays responsive while it works.

**Interface language** (sidebar, or **View → Interface Language**): menus, pages, buttons, messages and the
statistics are in **English, Türkçe and العربية** — Arabic right to left, the whole window mirrored. The list
already names 58 languages, each in its own name and in the interface language's (Azərbaycan türkcəsi ·
Azerbaycan Türkçesi in Turkish), with how much of it is translated — the languages of the Muslim world (فارسی,
اردو, Bahasa Indonesia, Bahasa Melayu, বাংলা, پښتو, دری, Kurdî, Soomaali, Hausa, Kiswahili, Shqip, Bosanski,
தமிழ், हिन्दी), the Turkic languages (Azərbaycan türkcəsi, Oʻzbekcha, Қазақша, Кыргызча, Türkmençe, ئۇيغۇرچە,
Татарча, Башҡортса, Qırımtatarca), the languages of Europe (Deutsch, Français, Español, Português, Italiano,
Nederlands, Polski, Čeština, Slovenčina, Magyar, Română, Български, Ελληνικά, Русский, Українська, Српски,
Hrvatski, Slovenščina, Македонски, Svenska, Dansk, Norsk, Suomi, Eesti, Latviešu, Lietuvių) and 中文, 日本語,
한국어, ไทย, Tiếng Việt; a text not translated yet is shown in
English, and the translations follow release by release. The window is rebuilt in the new language at once,
nothing to restart. `iy config set view.ui_language tr`. The hadith texts keep their own languages
(**Content languages**), the command line stays in English.

<p align="center">
  <img src="https://raw.githubusercontent.com/bayramkotan/isnady/main/assets/screenshots/preferences.png" alt="Preferences — font, size, line spacing and colour for each script" width="760">
</p>

**Back and forward**, as in a browser: ◀ ▶ under the name in the sidebar, Alt+Left / Alt+Right, the mouse's back
and forward buttons — every narrator, chain, book, scholar, term and tree opened is a step, and pointing at the
button says where it goes; so is every narrator, scholar, hafiz, book, chapter and term picked in a page's own list.

**Right-click** a hadith, a book, a chapter, a narrator, a scholar, a hafiz, a narrator of a chain, a bookmark or a
term for what can be done with it: open it in its book or its chain, the other narrations, where you left a book and
its markers, search in a chapter, put a marker, his family and his line of learning, his entry in the rijal work, the
narrators of a name the chain gives, a note or a folder for a bookmark, add to or remove from the bookmarks, copy (the
reference, the Arabic, a translation, everything, a name) and export.

**A new version starts clean:** what the earlier version kept for speed (`cache/`) is deleted once and computed
again, and the footer says so; the saved statistics (`derived/`) stay while their data and analysis are the same; the settings, bookmarks, profiles, recent searches, the database and the downloaded
works are kept. **Help → Check for Updates** asks PyPI and npm (whichever isnady is installed from — both, ticked,
when both are behind) and shows what is new; `iy update --check`, `iy update --npm`. A page that cannot be built
shows a plain card instead of stopping isnady, the details go to `isnady.log` in the data folder, and an import that
was cut short is named on the Search page at the next start. At every start with no books yet, isnady asks to
import isnady-data — every hadith book and rijal work with one click (**Import all**) — in the background, the
progress in the bar at the bottom, or to download isnady-data's ready database instead (**Download the ready
database**: everything already read, used from the next start); on the command line `iy` says so, and
`iy catalog import --all` or `iy database download` does it.

**Settings → Books and languages** switches imported books on and off (Sunni and Shia apart),
chooses the languages the texts are shown in — the others are folded behind an arrow on each hadith — and puts
them in your own order: Arabic always first, the others dragged or moved with the arrows.

<p align="center">
  <img src="https://raw.githubusercontent.com/bayramkotan/isnady/main/assets/screenshots/books-languages.png" alt="Settings → Books and languages — books on and off, Sunni and Shia apart; the languages shown and their order, Arabic always first, the others folded behind an arrow" width="760">
</p>

```bash
iy config list --prefix text.arabic       # what is set, and what is changed
iy config set text.arabic.size 22
iy config set view.theme paper
iy config reset text.arabic               # back to the defaults
```

**Back to zero.** Settings → Reset has three cards — only the settings, only the books, or everything (books,
settings, bookmarks and the downloaded data) — each listing what is deleted and what is kept, and each asking once
more; `iy reset settings|books|all`.

<p align="center">
  <img src="https://raw.githubusercontent.com/bayramkotan/isnady/main/assets/screenshots/reset.png" alt="Settings → Reset — three cards: only the settings, only the books, everything; each says what is deleted and what is kept" width="760">
</p>

### Your data folder

Everything isnady keeps is in one folder — `iy datadir` shows it, `iy datadir /elsewhere` moves it,
and a folder named `isnady-data` beside a portable application is used instead:

| | |
|---|---|
| `isnady.db` | the database: books, texts, grades, chains, narrators (SQLite) |
| `derived/*.json` | statistics and models, computed once and kept |
| `settings.json` | appearance and other settings, shared by the window and the command line |
| `repos/`, `user_repositories.json` | the catalogs read from the repositories, and the repositories you added |
| `isnady.<date-time>.bak.db` | a database replaced by a ready one, kept |
| `logs/isnady.log`, `logs/crash_*.log` | what isnady did and what went wrong; a hard crash's stack (Help → Log, `iy log`) |

**The log.** Every start writes a banner — version, Python, PySide6, system, data folder, profile, database,
books — and then a line for each thing isnady does: a page drawn, a search and how long it took, an import, an
export, the chains read, the narrators identified, the statistics computed; every error with its traceback.
**Help → Log…** (Ctrl+Shift+L) follows it live, shows only the errors or the lines holding a text, larger or
smaller, copies it with the version at its head for a report, opens its folder, and removes what is older than a
day. A crash Python itself cannot catch leaves `logs/crash_<time>.log`. `iy log`, `iy log --level error`,
`iy log -f`; `iy config set log.level debug` for more.

<p align="center">
  <img src="https://raw.githubusercontent.com/bayramkotan/isnady/main/assets/screenshots/log.png" alt="Help → Log — this run's banner and every step with its time, followed live; only errors, a text, larger or smaller, copy, the folder, what is older than a day removed" width="850">
</p>

---

**Profiles.** Several people can use one isnady, each with their own settings, place in the books and bookmarks:
**File → Profiles** makes, switches, renames, deletes, exports and imports them (one `.isnady-profile.zip` file
carries a profile to another computer). The books and the database are shared. The first profile is the data folder
itself, so nothing changes for anyone who never makes a second one. `iy profile list`, `iy profile new Ayşe`,
`iy profile use Ayşe`, `iy profile export Ayşe ~/`; `ISNADY_PROFILE=Ayşe iy …` for one command.

## ⌨️ CLI

Everything the window does also works without it — on a server, over SSH, or in a script. The command line never
loads Qt, so it starts in a moment and needs no screen. The same engine answers both: a search, a chain, a narrator
or a statistic is the same number in the terminal as in the window.

**Three names, one program.** `iy` is the short name, `isnady` the long one; without arguments either opens the window,
with a command it works in the terminal. `isnady-cli` never opens the window. Every command explains itself:
`iy --help` lists them, `iy COMMAND --help` gives every option of one. Every command that takes a book accepts its
full name (`musnad_ahmad_ibn_hanbal`) or its short key (`ahmad`); `iy book` lists them. Names may be written in Arabic
or in Latin letters, with or without diacritics: `iy narrator "Ebu Hureyre"`, `iy narrator "Abu Hurayra"` and
`iy narrator "أبو هريرة"` find the same man. The interface language (`iy config set view.ui_language tr`) also sets
the language of the answers where they are written for isnady (lives, terms, history); `--lang` changes it for one
command.

**At a glance**

| Command | What it does |
|:--------|:-------------|
| `iy` | Open the window |
| [`iy search`](#-searching) | Search hadith text, narrators or scholars |
| [`iy chain`](#-chains-books-and-other-narrations) · [`iy tahric`](#-chains-books-and-other-narrations) · [`iy book`](#-chains-books-and-other-narrations) · [`iy quran`](#-chains-books-and-other-narrations) | One chain read link by link · the other narrations of a hadith · read a book in its order · the Qur'an and the hadith that name each verse |
| [`iy narrator`](#-narrators-scholars-and-their-lives) · [`iy life`](#-narrators-scholars-and-their-lives) · [`iy scholar`](#-narrators-scholars-and-their-lives) · [`iy group`](#-narrators-scholars-and-their-lives) · [`iy sira`](#-narrators-scholars-and-their-lives) · [`iy sahifa`](#-narrators-scholars-and-their-lives) · [`iy tree`](#-narrators-scholars-and-their-lives) | A narrator · a life · the scholars and the huffaz · the Companions and the Household · the Sira · the written sources · the genealogies |
| [`iy term`](#-the-sciences-of-hadith) · [`iy learn`](#-the-sciences-of-hadith) | The glossary · the history and method of hadith |
| [`iy stats`](#-statistics-1) | Every statistic of the Statistics page, with its plain-words explanation |
| [`iy catalog`](#-data-books-and-sources) · [`iy repo`](#-data-books-and-sources) · [`iy database`](#-data-books-and-sources) · [`iy import`](#-data-books-and-sources) · [`iy sources`](#-data-books-and-sources) · [`iy remove`](#-data-books-and-sources) · [`iy isnads`](#-data-books-and-sources) · [`iy ai`](#-data-books-and-sources) · [`iy formats`](#-data-books-and-sources) | Books and data: import, repositories, a ready database, your own files |
| [`iy bookmark`](#-your-things-bookmarks-profiles-settings-files) · [`iy profile`](#-your-things-bookmarks-profiles-settings-files) · [`iy config`](#-your-things-bookmarks-profiles-settings-files) · [`iy export`](#-your-things-bookmarks-profiles-settings-files) · [`iy import-data`](#-your-things-bookmarks-profiles-settings-files) | Bookmarks, profiles, settings, export and import |
| [`iy update`](#-installation-and-upkeep) · [`iy doctor`](#-installation-and-upkeep) · [`iy shortcut`](#-installation-and-upkeep) · [`iy datadir`](#-installation-and-upkeep) · [`iy cache`](#-installation-and-upkeep) · [`iy log`](#-installation-and-upkeep) · [`iy reset`](#-installation-and-upkeep) · `iy -V` | Updates, installations, shortcuts, the data folder, the cache, the log, starting over |

### 🔎 Searching

`iy search WORDS` searches the texts of every imported book in every language, as the Search page does. Arabic
diacritics and letter forms are ignored (الاعمال finds الأَعْمَالُ), and Latin-letter text ignores case and accents.

```console
$ iy search niyet --book abudawud --limit 1
43 hadith found in 8 ms

Sunan Abu Dawud #472
  grades: Al-Albani: Hasan; Muhammad Muhyi Al-Din Abdul Hamid: Hasan; Shuaib Al Arnaut: Daif; Zubair Ali Zai: Daif
  [Turkish] Ebu Hureyre (r.a.); Resulullah (Sallallahu aleyhi ve Sellem)'in şöyle buyurduğunu rivayet etmiştir: "Bir kimse mescid'e hangi [niyet]le gelirse nasibi ondan ibarettir"
```

```bash
iy search "إنما الأعمال" --mode phrase                 # the words together, in this order
iy search sabır şükür --mode any                       # any of the words
iy search namaz --whole-words --language Turkish       # whole words, Turkish texts only
iy search niyet --grade sahih --grade hasan            # only sahih or hasan hadith
iy search niyet --grade sahih --every-grader           # only where every grader said sahih
iy search niyet --grade sahih --no-sahihayn            # without al-Bukhari and Muslim
iy search ilim --chapter bukhari:3                     # only in Bukhari's chapter 3 (Knowledge) and below it
iy search ilim --chapter "bukhari:Knowledge"           # the same chapter by the start of its title
iy search "Ebu Hureyre" --in narrators                 # narrators of both traditions, by name
iy search Zurara --in narrators --tradition shia       # only al-Najashi's narrators
iy search Albani --in scholars                         # the scholars
iy search patience --mode meaning                      # by meaning, across languages (pip install "isnady[ai]", iy ai build)
iy search --recent                                     # the last 20 searches, as in the window
```

| Option | Meaning |
|:-------|:--------|
| `--mode all\|any\|phrase\|meaning` | every word (default), any word, the words in order, or by meaning |
| `--whole-words` | whole words only |
| `--book KEY` | only this book (repeatable) |
| `--language NAME` | only texts of this language (repeatable) |
| `--grade G` | Sunni: `sahih`, `hasan`, `daif`, `very-weak`, `fabricated`, `ungraded`; Shia, in their own scale: `imami-sahih`, `imami-hasan`, `muwaththaq`, `imami-daif`, `imami-ungraded` (repeatable; the two scales are never mixed) |
| `--every-grader` | with `--grade`: every grade the hadith was given is among those chosen |
| `--no-sahihayn` | with `--grade sahih`: al-Bukhari's and Muslim's hadith are not counted (by default they are, by their compiler's condition) |
| `--chapter BOOK:CH` | only this chapter and the chapters under it (repeatable) |
| `--in hadith\|narrators\|scholars` · `--tradition sunni\|shia` | what to search; with narrators, one road |
| `--limit N` · `--offset N` · `--width N` · `--all-texts` | how many, from where, characters shown, every text and not only those that matched |
| `--recent` · `--clear-recent` | the recent searches, or forget them |

### 🔗 Chains, books and other narrations

`iy chain BOOK NUMBER` reads one chain link by link: the word of transmission and its meaning, who each narrator is
(Ibn Hajar's or al-Najashi's entry, tabaqa, year of death, verdict) and whom the chain reaches, then the whole chain
said in words.

```console
$ iy chain bukhari 1 --lang en
Sahih al-Bukhari 1
   1. حدثنا (narrated to us)  الْحُمَيْدِيُّ عَبْدُ اللَّهِ بْنُ الزُّبَيْرِ
      = عبد الله بن الزبير بن عيسى القرشي الأسدي الحميدي المكي أبو بكر
        tabaqa 10; d. 219 AH; Ibn Hajar al-'Asqalani: ثقة حافظ فقيه أجل أصحاب بن عيينة (rank 2)
   2. حدثنا (narrated to us)  سُفْيَانُ
      = سفيان بن عيينة بن أبي عمران ميمون الهلالي أبو محمد الكوفي ثم المكي
        tabaqa 8; d. 198 AH; Ibn Hajar al-'Asqalani: ثقة حافظ فقيه إمام حجة … (rank 2)
   …
```

```bash
iy chain bukhari                     # the book's numbers, first to last, with an example
iy chain kafi 5:1740                 # al-Kafi and 'Uyun number volume:hadith
iy chain bukhari 1 --raw             # also the chain as written
iy chain bukhari 1 --lang tr,en      # the chain said in Turkish and English
iy chain bukhari --chapter 3         # the hadith of chapter 3 with the start of their text and how far each chain
                                     # was read — to choose one without knowing its number
iy tahric bukhari 1                  # the other narrations of the same hadith in the imported books (takhrij)
```

```console
$ iy chain bukhari --chapter 3
bukhari — 3. Knowledge: 76 hadith
        59  11 narrators  حَدَّثَنَا مُحَمَّدُ بْنُ سِنَانٍ، قَالَ حَدَّثَنَا فُلَيْحٌ، ح …
        60   5 narrators  حَدَّثَنَا أَبُو النُّعْمَانِ، عَارِمُ بْنُ الْفَضْلِ قَالَ …
```

`iy book` reads a book in its own order, as the Books page does, and keeps where you left it.

```bash
iy book                              # the Sunni books (--tradition shia, --tradition all)
iy book bukhari                      # its chapters, numbered in reading order
iy book bukhari 3 --language English --limit 5    # chapter 3, five hadith, in English
iy book bukhari --resume             # on where you left it
iy book bukhari 3 --mark "Knowledge"             # a marker of your own at this chapter
iy book --marks                      # your markers, and where you left each book
iy book --go Knowledge               # open a marker
iy book --unmark Knowledge           # remove it (all: every marker)
iy book --forget bukhari             # forget where you left a book (all: every book)
iy book taqrib 1                     # the rijal works read the same way, entry by entry
```

`iy quran` reads the Qur'an (downloaded once on request) with its translations and the hadith that name each verse.

```bash
iy quran download                    # the Arabic and a translation in each language you read (about 1 MB each, once)
iy quran editions --language Turkish # the translations of a language, ★ those isnady-data keeps (98 languages)
iy quran editions --own              # only isnady-data's (60 translations, 18 languages)
iy quran download tur-elmalilihamdiya eng-abdelhaleem   # more translations
iy quran editions --installed        # those downloaded ; iy quran remove KEY
iy quran                             # the 114 suras
iy quran 1                           # a sura
iy quran 2:255-257 --edition tur-diyanetisleri         # verses, in one translation
iy quran 33:33 --hadith              # with the hadith that name each verse, Sunni and Shia books apart
iy quran search sabır                # search the whole text, in Arabic or a translation's words
```

### 👥 Narrators, scholars and their lives

```console
$ iy narrator "Ibn Umar"
Abd Allah b. Umar (RA)  —  عبد الله بن عمر بن الخطاب العدوي أبو عبد الرحمن
  also: أبو عبد الرحمن, بن عمر
  tabaqa 1: Companions
  death: 73 AH  (مات سنة ثلاث وسبعين في آخرها أو أول التي تليها)
  Ibn Hajar al-'Asqalani, Taqrib al-Tahdhib: … rank 1: Companion
  books: ع all six books
  identified in 6172 chain links of the imported collections
```

```bash
iy narrator الزهري                   # Arabic, with or without diacritics
iy narrator "Ebu Bekir" --limit 1    # with his teachers, students and hadith
iy narrator --famous                 # the famous Companions by group (the four Caliphs, the Ten, the Mothers of the
                                     # Believers, the household, those who narrated most)
iy narrator --famous --tradition shia    # road B: the Ashab al-Ijma'
iy life "Abu Hurayra" --lang tr      # a life: written for isnady, or read from the rijal work and the chains
iy life Zuhri --source               # also his entries in al-Dhahabi's Siyar and Tadhkirat al-Huffaz (downloaded once)
iy life "Zurara" --tradition shia    # only among one road's narrators
iy scholar                           # the scholars whose work is imported: compilers, graders, critics
iy scholar albani                    # one scholar measured: grades, agreement, kappa, strictness
iy scholar --tradition shia          # the Shia scholars, by their own road
iy scholar --huffaz                  # al-Dhahabi's Tadhkirat al-Huffaz: 21 generations, deaths by century
iy scholar --huffaz --tabaqa 3       # one generation
iy scholar --huffaz zuhri --source   # one hafiz: generation, death, al-Mizzi's signs, his Taqrib entry, his entry
iy group                             # the groups: ten, kisa, mothers (Sunni), masumin (Shia)
iy group ten --lang tr               # the ten promised Paradise: the hadith, what each road reads, each member
iy group masumin                     # the Fourteen Infallibles: how many chains of the Shia books end at each
iy sira                              # the life of the Prophet: 38 events in three periods, with their dates
iy sira badr                         # one event: date, place, age, verses, Ibn Hisham's chapter, the hadith naming it
iy sahifa                            # the written sources, Sunni and Shia apart, with how many hadith each
iy sahifa hammam                     # Hammam's sahifa in the Musnad (138), each hadith with its parallels
iy tree prophets                     # the prophets' lineage (--hadith: the hadith that speak of one)
iy tree quraysh "Abu Bakr"           # the tree of Quraysh, with someone found in it
iy tree family "Ibn Umar"            # a narrator's family among the narrators
iy tree learning Zuhri --depth 2 --width 4    # his teachers above and his students below
```

### 🎓 The sciences of hadith

```console
$ iy term mursal --lang tr
mürsel  مرسل   [Senedine göre hadis türleri]
  Bir tâbiînin, sahâbîyi zikretmeden "Peygamber şöyle dedi" demesi.
  …
  source: Ibn Hajar, Nukhbat al-Fikar; Ibn al-Salah, Muqaddima
```

```bash
iy term                              # every term, by category
iy term --search tadlis              # look in names and definitions
iy learn                             # the articles of History and method
iy learn writing --lang tr           # tadwin, writing, method, numbers — the numbers counted live from your books
```

### 📊 Statistics

`iy stats` gives every statistic of the Statistics page; what was computed is saved and read again until the data
changes.

```console
$ iy stats corpus
All Sunni books: 75,953 hadith, 88,941 chains, 304,753 of 405,054 names identified (75.2%), 5,999 narrators

Concentration: busiest 1% carry 47.7%, busiest 10% 84.3%; 69 of 5,999 narrators carry half; Gini 0.884
  → in plain words: very uneven: a few carry most of it — the pivots (madar) of the corpus  (iy stats --explain gini)
```

```bash
iy stats narrators                   # reliability, concentration, the pillars
iy stats chains                      # length, weakest link, time gaps
iy stats grades                      # the weakest narrator and the length against the grade
iy stats books                       # the books compared
iy stats graders --book abudawud     # agreement, kappa, strictness, the disputed hadith
iy stats model --grader albani       # what a scholar's grade follows (ordinal regression)
iy stats lab                         # correlations and tests
iy stats corpus --book @shia         # the Shia books; @all both roads, one after the other
iy stats chains --csv ~/stats        # every table behind it as CSV files
iy stats --explain kappa             # a measure in plain words (no term: all of them)
iy stats corpus --recompute          # compute again
```

### 📚 Data, books and sources

```bash
iy catalog list                      # the books of the repository, each kind and road under its folder
iy catalog import sahih_muslim --language tur --language eng   # one book, in these languages
iy catalog import --all              # everything: the Qur'an, the hadith, the sira, the tabaqat, the rijal
iy catalog updates                   # what is new or changed in the repository since you imported
iy catalog update                    # import only that (new books, changed books, new languages)
iy catalog import quran              # the Qur'an (the Arabic and a translation); more: iy quran download KEY
iy catalog import sira               # Ibn Hisham's Sira; siyar, tadhkira: al-Dhahabi's two works
iy catalog import hammam             # a written source as a book: hammam, sadiqa, ali, kitab-ali, jamia, fatima …
iy book sahifat_hammam_ibn_munabbih  # its chapters: Musnad Ahmad 138, then its parallels in each book
iy book sira                         # its 264 chapters; iy book sira 12: read chapter 12; iy book siyar 3
iy catalog languages add tr en       # one language for every book (Arabic always stays)
iy catalog remove sahih_muslim       # remove what was imported of it
iy catalog add "My notes" isnady ~/notes/book.json --license CC-BY-4.0   # a book of your own in the list
iy repo                              # the data repositories (isnady-data by default)
iy repo add ~/isnady-data --name "My copy"      # a GitHub or Hugging Face repository, a URL or a folder
iy repo use lk-hadith-corpus         # read the books from another repository
iy database download                 # isnady-data's ready database, downloaded and checked (yours is kept beside it)
iy database use isnady-db-2026-10-12.zip        # a ready database from a file (yours is kept beside it)
iy import isnady ~/my_book/book.json --license CC-BY-4.0   # import one file of your own
iy import fawazahmed0 https://…/editions.json --book bukhari --language Turkish   # from an index of editions
iy formats                           # the formats that can be imported
iy sources                           # what is imported, with its licence and tier
iy remove KEY                        # a source and everything imported from it
iy isnads --rebuild                  # read every chain again and report per book
iy ai build                          # learn the meaning index (pip install "isnady[ai]"); iy ai status
```

Credentials for a private source are asked when it is imported and never stored: `--user` (the password is asked,
or `--password-env`), `--token-env`, `--api-key-env`, `--api-key-header`, `--api-key-param`.

### ⭐ Your things: bookmarks, profiles, settings, files

```bash
iy bookmark list                     # every bookmark (list hadith: only one kind)
iy bookmark add hadith bukhari:1 --folder Prayer --note "the first hadith"
iy bookmark add narrator taqrib:3467 --folder Companions
iy bookmark add chapter bukhari:3 ; iy bookmark add term mursal ; iy bookmark add tree prophets:musa
iy bookmark folders                  # the folders, Prayer/Friday inside Prayer
iy bookmark remove ID
iy bookmark export ~/marks.json ; iy bookmark import ~/marks.json
iy profile list                      # several people on one isnady, each with their own settings and bookmarks
iy profile new Ayşe --copy default ; iy profile use Ayşe ; iy profile rename Ayşe Aişe
iy profile export Ayşe ~/ ; iy profile import ~/Ayşe.isnady-profile.zip ; iy profile delete Ayşe
ISNADY_PROFILE=Ayşe iy search sabır  # one command in another profile
iy config list --prefix text.arabic  # the settings, as in Edit → Preferences
iy config set view.theme sky         # sky, starry, garden, light, dark, paper, slate, emerald, midnight, contrast, system
iy config set text.arabic.size 22 ; iy config reset text.arabic.size
iy export hadith bukhari:1 bukhari:5-10 -f pdf -o ~/hadith.pdf
iy export book bukhari --chapter 3 -f docx --language Turkish
iy export search niyet --grade sahih --in-chapter bukhari:3 -f xlsx    # the results as they were found
iy export narrators --tradition sunni -f csv ; iy export network bukhari -f graphml
iy export bookmarks -f md ; iy export backup -o ~/isnady-backup.zip ; iy export statistics -f xlsx
iy export --list                     # what can be exported and in which formats
iy import-data book ~/book.isnady.json ; iy import-data bookmarks ~/marks.json --folder Imported
iy import-data settings ~/settings.json --replace ; iy import-data backup ~/isnady-backup.zip
```

### 🔧 Installation and upkeep

```bash
iy -V                                # the version
iy update --check                    # the installed and the latest version on PyPI and npm, and what is new
iy update                            # update every copy (system, user, venv, pipx, clone); --yes: do not ask
iy update --npm                      # the npm package too
iy doctor                            # every isnady on this computer and which one each command starts
iy doctor --fix                      # update the copies that are behind
iy shortcut                          # a desktop and applications-menu shortcut (--no-desktop, --no-menu)
iy datadir                           # where the data folder is; iy datadir /data/isnady moves it (--as-is, --default)
iy cache info ; iy cache clear       # the counts kept for speed
iy log --level error                 # the log: errors only; -f follows it; --crash the newest crash report
iy log --grep import -n 200 ; iy log --before 2026-10-01 ; iy log --clear
iy reset settings                    # back to zero: settings, books or all (--yes: do not ask)
```

---

## 📸 More screenshots

<p align="center">
  <img src="https://raw.githubusercontent.com/bayramkotan/isnady/main/assets/screenshots/search-grades.png" alt="Search — Sunan Abi Dawud with each scholar's grade and the chain above the text" width="850">
</p>
<p align="center">
  <img src="https://raw.githubusercontent.com/bayramkotan/isnady/main/assets/screenshots/chain-dark.png" alt="Dark theme — a seven-narrator chain from Sunan Abi Dawud" width="850">
</p>
<p align="center">
  <img src="https://raw.githubusercontent.com/bayramkotan/isnady/main/assets/screenshots/chains-overview.png" alt="Chains overview — per-book statistics and what was kept whole" width="850">
</p>
<p align="center">
  <img src="https://raw.githubusercontent.com/bayramkotan/isnady/main/assets/screenshots/books-taqrib.png" alt="Books — Ibn Hajar's Taqrib read letter by letter, each entry a click from its narrator" width="850">
</p>

---

## 🗺️ Roadmap

- **Shia narrators, further** — al-Tusi's *Rijal* and *Fihrist* and al-Kashshi's reports; names
  resolved by teacher and student; a Sunni–Shia comparison.
- **More in isnady-data** — al-Tusi's *Tahdhib al-Ahkam* and *al-Istibsar*, al-Saduq's *Man la
  yahduruhu al-faqih*; al-Hakim's *Mustadrak*, al-Bayhaqi's *Sunan*, the *Mu'jam*s.
- **Narrators** — the other half of the names identified through teachers and students (al-Mizzi's
  *Tahdhib al-Kamal*), the verdicts of other critics beside Ibn Hajar's, and whether each link could
  have met the next; fuller, richer lives of narrators and scholars.
- **Settings and export** — more of isnady set from one place; export as ODS, TEI-XML and BibTeX/RIS, of the
  scholars and the glossary, and a preview of an import before it comes in.
- **Every interface language in full** — the 21 listed after English, Türkçe and العربية; the command line
  and the glossary in them too.
- **isnady.net** — the same engine on the web.

---

## 🔁 Versions and releases

One version number on GitHub, PyPI and npm, always. A release builds the Python package and the
Windows, Linux and macOS applications first; only when every build succeeds is anything published —
PyPI, then the [GitHub Release](https://github.com/bayramkotan/isnady/releases) with every file and
its checksum. What each version brought is in the
[CHANGELOG](https://github.com/bayramkotan/isnady/blob/main/CHANGELOG.md). The README on npm is made
from this one. The data has its own rhythm: a book added to
[isnady-data](https://github.com/bayramkotan/isnady-data) needs no new version of isnady.

## 🏗️ Build from Source

```bash
git clone https://github.com/bayramkotan/isnady.git
cd isnady
pip install -e .
iy
```

Python 3.10 or newer and PySide6. The command line alone needs no PySide6. The desktop application
of your own system: `pip install ".[ai]" pyinstaller`, then
`python packaging/build_app.py linux|windows|macos VERSION`.

The data repository is built by two tools in this one: `tools/build_data_repo.py --mirror DIR`
converts every source into isnady-data, and `tools/publish_database.py DIR` prepares a ready
database for its Releases.

## 📝 License

MIT for the application code. Hadith data comes from separate sources, each under its own licence,
recorded with the data and listed under **Help → Licences**.

The Arabic and reading typeface is [Amiri](https://github.com/aliftype/amiri) by Khaled Hosny,
bundled under the SIL Open Font License 1.1 (`src/isnady/assets/fonts/OFL.txt`). The friendly themes' typefaces —
[Nunito](https://github.com/googlefonts/nunito), [Lexend](https://github.com/googlefonts/lexend),
[Baloo 2](https://github.com/EkType/Baloo2) — and [Noto Naskh Arabic](https://github.com/notofonts/arabic) are
bundled under the same licence (`OFL-Nunito.txt`, `OFL-Lexend.txt`, `OFL-Baloo2.txt`, `OFL-NotoNaskhArabic.txt`),
as fixed weights cut from their variable fonts.
