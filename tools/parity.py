"""Parity: the Rust iy-rs and the Python iy give the same answer — word for word.

Runs every case below with both programs on the same data folder and compares what they print; only the time a
search took ("in 61 ms") and the number `iy chain BOOK` picks at random are left out of the comparison. Run it before every release (the release blocks do):

    python tools/parity.py                              # the Python iy on PATH, target/release/iy-rs
    python tools/parity.py --python "python -m isnady" --rust target/release/iy-rs.exe
    python tools/parity.py --data-dir /tmp/isnady-test  # another data folder (ISNADY_DATA_DIR)

Exit code 0 when every case is the same, 1 otherwise (the differences are printed, the first lines of each).
"""

import argparse
import os
import re
import shlex
import subprocess
import sys

CASES = [
    ["version"],
    ["datadir"],
    ["sources"],
    ["search", "النيات"],
    ["search", "إنما الأعمال بالنيات", "--mode", "phrase"],
    ["search", "الصلاة", "--limit", "5", "--offset", "3"],
    ["search", "الصلاة", "--book", "bukhari", "--limit", "3"],
    ["search", "صلاة", "زكاة", "--mode", "any", "--limit", "4"],
    ["search", "قال", "النبي", "--whole-words", "--limit", "3"],
    ["search", "niyet", "--language", "Turkish", "--limit", "3"],
    ["search", "intentions", "--limit", "3", "--all-texts"],
    ["search", "komşu", "--limit", "3"],
    ["search", "مسل*", "--limit", "3"],
    ["search", "ي?من", "--limit", "3"],
    ["search", "الصلاه", "--limit", "2", "--width", "80"],
    ["search", "النياتت", "--limit", "2"],                      # not found as typed: its near spellings
    ["search", "«قَالَ: لا يُؤْمِنُ»", "--mode", "phrase", "--limit", "2"],
    ["search", "zzzqqq"],
    # the search by grade and in a chapter (RS-WIN2, GR-FLT, SRCH-SEC1); the recent searches (S4)
    ["search", "الصلاة", "--grade", "sahih", "--limit", "3"],
    ["search", "صلاة", "--grade", "daif", "--grade", "hasan", "--every-grader", "--limit", "3"],
    ["search", "الصلاة", "--grade", "sahih", "--no-sahihayn", "--limit", "2"],
    ["search", "الصلاة", "--grade", "sahih", "--book", "bukhari", "--limit", "2"],
    ["search", "الله", "--grade", "imami-sahih", "--grade", "muwaththaq", "--limit", "2"],
    ["search", "niyet", "--grade", "ungraded", "--limit", "2"],
    ["search", "النيات", "--grade", "fabricated"],
    ["search", "الصلاة", "--chapter", "riyad:3", "--limit", "3"],
    ["search", "قال", "--chapter", "bukhari:1", "--grade", "sahih", "--limit", "2"],
    ["search", "الصلاة", "--chapter", "riyad:nosuch"],
    ["search", "--recent"],
    ["search"],
    # the narrators and the scholars by name (S3, RS-WIN2b): both traditions, the closest first; close spellings,
    # the same consonants read otherwise, the famous ones by any spelling, the scholars by a name or a work
    ["search", "--in", "narrators", "Abu Hurayra"],
    ["search", "--in", "narrators", "الزهري"],
    ["search", "--in", "narrators", "Ibn Umar", "--limit", "5"],
    ["search", "--in", "narrators", "Âişe"],
    ["search", "--in", "narrators", "Bayram"],
    ["search", "--in", "narrators", "zuberyyyr"],
    ["search", "--in", "narrators", "Ebubekir", "--limit", "3"],
    ["search", "--in", "narrators", "Bukhari", "--limit", "3"],
    ["search", "--in", "narrators", "Zurara", "--tradition", "shia", "--limit", "4"],
    ["search", "--in", "narrators", "Muhammad", "--limit", "3", "--offset", "398"],
    ["search", "--in", "narrators", "qqqzzz"],
    ["search", "--in", "scholars", "Buhârî"],
    ["search", "--in", "scholars", "ابن حجر"],
    ["search", "--in", "scholars", "Sünen"],
    ["search", "--in", "scholars", "Albnai"],
    ["search", "--in", "scholars", "qqqq"],
    # the lives (BIO2, RS-WIN3a): written for isnady, or read from the data; in English, Turkish, Arabic
    ["life", "Abu Hurayra"],
    ["life", "Ebu Hureyre", "--lang", "tr"],
    ["life", "muhammad"],
    ["life", "أبو بكر", "--lang", "ar"],
    ["life", "Bayram"],
    ["life", "Bayram", "--lang", "tr"],
    ["life", "Abu Bakr", "--limit", "3"],
    ["life", "Zurara", "--tradition", "shia", "--lang", "tr"],
    ["life", "Aisha", "--limit", "2"],
    ["life", "qqqzzz"],
    # the chains (RS2): read by the Python isnady, shown by both; road A with Ibn Hajar's Taqrib, road B with al-Najashi
    # the report of every book (RS-WIN3b: the overview of the Isnad Chains page)
    ["isnads"],
    ["chain", "bukhari", "1260"],
    ["chain", "muslim", "133"],
    ["chain", "kafi", "1:4"],
    ["chain", "nawawi"],
    ["chain", "nawawi", "1"],
    ["chain", "nawawi", "2", "--raw"],
    ["chain", "nawawi", "10", "--lang", "tr"],
    ["chain", "nawawi", "15", "--lang", "en,tr,ar"],
    ["chain", "riyad", "1"],
    ["chain", "riyad", "12"],
    ["chain", "riyad", "100"],
    ["chain", "riyad", "500", "--raw"],
    ["chain", "shamail", "1"],
    ["chain", "shamail", "7"],
    ["chain", "shamail", "40", "--lang", "en"],
    ["chain", "sifat-shia", "1"],
    ["chain", "sifat-shia", "5", "--raw"],
    ["chain", "sifat-shia", "20"],
    ["chain", "nawawi", "999"],                        # a number the book does not have
    ["chain", "nosuch", "1"],
    # the books (RS3): the shelf, a book's chapters, a chapter; the hadith of a chapter to choose a chain from
    ["book"],
    ["book", "--tradition", "all"],
    ["book", "--tradition", "shia"],
    ["book", "riyad"],
    ["book", "riyad", "3"],
    ["book", "riyad", "12", "--limit", "3", "--width", "60"],
    ["book", "shamail", "2", "--language", "English"],
    ["book", "nawawi", "1"],
    ["book", "sifat-shia", "1", "--limit", "5"],
    ["book", "taqrib", "2", "--limit", "4"],
    ["book", "riyad", "999"],
    ["book", "nosuch"],
    ["chain", "riyad", "--chapter", "1"],
    ["chain", "shamail", "--chapter", "3"],
    ["chain", "riyad", "--chapter", "nosuch"],
    # the narrators (RS4): by Arabic or Latin letters, close spellings, the famous ones by any spelling
    ["narrator", "--famous"],
    ["narrator", "--famous", "--tradition", "shia"],
    ["narrator", "--famous", "--tradition", "all"],
    ["narrator", "الزهري"],
    ["narrator", "ابو هريرة", "--limit", "1"],
    ["narrator", "Ebu Bekir"],
    ["narrator", "Omer ibn Hattap"],
    ["narrator", "Zuhri", "--limit", "3"],
    ["narrator", "Buhaari"],
    ["narrator", "Aysha", "--limit", "1"],
    ["narrator", "Ibn Abbas", "--limit", "1"],
    ["narrator", "زرارة", "--tradition", "shia"],
    ["narrator", "zzzqqq"],
    ["narrator"],
    # the hadith scholars (RS5): the list, one scholar with what the data measures of him; al-Dhahabi's huffaz
    ["scholar"],
    ["scholar", "--tradition", "all"],
    ["scholar", "--tradition", "shia"],
    ["scholar", "bukhari"],
    ["scholar", "albani"],
    ["scholar", "kulayni"],
    ["scholar", "majlisi"],
    ["scholar", "najashi"],
    ["scholar", "ibn", "hajar"],
    ["scholar", "behbudi"],
    ["scholar", "nobody"],
    ["scholar", "--huffaz"],
    ["scholar", "--huffaz", "--tabaqa", "3"],
    ["scholar", "--huffaz", "zuhri"],
    ["scholar", "--huffaz", "abu", "hurayra"],
    ["scholar", "--huffaz", "الشافعي"],
    ["scholar", "--huffaz", "yahya", "--tabaqa", "7"],
    ["scholar", "--huffaz", "nothingx"],
    # the Companions and the Household (RS7): the groups of each road, one group with its members
    ["group"],
    ["group", "--tradition", "shia"],
    ["group", "ten"],
    ["group", "kisa", "--width", "60"],
    ["group", "mothers", "--lang", "tr"],
    ["group", "masumin"],
    ["group", "masumin", "--lang", "ar"],
    ["group", "nope"],
    # the Sira as a timeline (RS8)
    ["sira"],
    ["sira", "--lang", "tr"],
    ["sira", "badr"],
    ["sira", "elephant"],
    ["sira", "hijra", "--lang", "tr"],
    ["sira", "revelation", "--lang", "ar"],
    ["sira", "uhud", "--width", "60"],
    ["sira", "nope"],
    # the genealogies (RS11)
    ["tree", "prophets"],
    ["tree", "quraysh"],
    ["tree", "prophets", "Musa"],
    ["tree", "prophets", "Musa", "--hadith"],
    ["tree", "prophets", "isa", "--hadith"],
    ["tree", "quraysh", "هاشم"],
    ["tree", "prophets", "xyz"],
    ["tree", "family"],
    ["tree", "family", "Ibn Umar"],
    ["tree", "family", "عبد الله بن عمر"],
    ["tree", "learning", "Zuhri"],
    ["tree", "learning", "مالك بن أنس", "--depth", "1", "--width", "3"],
    # the glossary and History and method (RS13)
    ["term"],
    ["term", "sahih"],
    ["term", "mursal", "--lang", "tr"],
    ["term", "tadlis", "--lang", "ar"],
    ["term", "--search", "kunya"],
    ["term", "xyzq"],
    ["learn"],
    ["learn", "tadwin"],
    ["learn", "writing", "--lang", "tr"],
    ["learn", "method", "--lang", "ar"],
    ["learn", "numbers"],
    ["learn", "nope"],
    # RS9 written sources, RS10 the Qur'an, RS14 settings, RS18 the cache and the log
    ["sahifa"],
    ["sahifa", "--tradition", "shia", "--lang", "tr"],
    ["sahifa", "hammam", "--limit", "15"],
    ["sahifa", "sadiqa", "--lang", "tr"],
    ["sahifa", "jamia"],
    ["sahifa", "chains-shia", "--limit", "5", "--lang", "ar"],
    ["sahifa", "chains-sunni"],
    ["sahifa", "nope"],
    ["quran"],
    ["quran", "--lang", "tr"],
    ["quran", "2:255"],
    ["quran", "33:28-34", "--hadith"],
    ["quran", "24:30-31", "--hadith", "--lang", "tr"],
    ["quran", "editions"],
    ["quran", "search", "mercy", "--limit", "5"],
    ["quran", "search", "الرحمن", "الرحيم"],
    ["quran", "115"],
    ["config", "list"],
    ["config", "get", "text.arabic.size"],
    ["config", "get", "nope"],
    ["config", "set", "view.theme", "blue"],
    ["profile", "list"],
    ["cache"],
    ["log", "--path"],
    # the statistics (RS12): the corpus, the graders, the hadith model, the laboratory, both roads apart
    ["stats"],
    ["stats", "corpus"],
    ["stats", "narrators", "--limit", "3"],
    ["stats", "chains", "--book", "kafi"],
    ["stats", "grades", "--book", "@shia"],
    ["stats", "books"],
    ["stats", "corpus", "--book", "@all", "--limit", "2"],
    ["stats", "graders"],
    ["stats", "graders", "--book", "kafi"],
    ["stats", "graders", "--book", "abudawud", "--limit", "3"],
    ["stats", "model", "--book", "tirmidhi", "--limit", "2"],
    ["stats", "model", "--book", "@shia"],
    ["stats", "model", "--grader", "albani", "--limit", "1"],
    ["stats", "lab", "--book", "abudawud", "--limit", "4"],
    ["stats", "lab", "--book", "@shia", "--limit", "3"],
    ["stats", "lab", "--limit", "3"],
    ["stats", "--explain"],
    ["stats", "--explain", "kappa"],
    ["stats", "--explain", "nosuch"],
    # the data sources (RS16): the formats, the repositories, the catalog with what is imported, what changed
    ["formats"],
    ["repo", "list"],
    ["repo", "show", "fawazahmed0", "--refresh"],
    ["repo", "show", "nosuch"],
    ["repo", "remove", "isnady"],
    ["catalog", "list"],
    ["catalog", "list", "--repo", "thaqalayn"],
    ["catalog", "list", "--repo", "nosuch"],
    ["catalog", "updates"],
    ["catalog", "languages"],
    ["catalog", "languages", "add"],
    ["catalog", "remove", "nosuch"],
    ["catalog", "delete", "nosuch"],
    ["catalog", "add", "T", "nosuchformat", "x.json"],
    ["catalog", "import", "nosuch"],
    ["catalog", "update"],
    ["import", "nosuchformat", "x.json"],
    ["import", "isnady", "/nonexistent/book.json"],
    ["import", "isnady-sahifa", "/nonexistent/sahifa.json"],
    ["import", "taqrib", "/nonexistent/taqrib.md"],
    ["import", "najashi", "/nonexistent/najashi.md"],
    ["import", "openiti-work", "/nonexistent/work.md"],
    ["import", "fawazahmed0", "/nonexistent/editions.json"],
    ["import", "table", "/nonexistent/x.csv"],
    ["quran", "remove", "nosuch-edition"],
    ["database"],
    # export and import-data (EX1, A3): what there is; the files themselves are compared by tools/parity_export.py
    ["export", "--list"],
    ["export"],
    ["export", "book", "nosuchbook"],
    ["import-data", "bookmarks", "/nonexistent/bookmarks.json"],
    ["remove", "nosuch"],
    # the chain reader and the narrator matcher (RS17): every chain read again, every narrator identified again —
    # the report, and (tools/parity_db.py) the same rows in the database
    ["isnads"],
    ["isnads", "--rebuild"],
    ["chain", "bukhari", "1"],
]

TIME = re.compile(r" in \d+ ms")
EXAMPLE = re.compile(r"(Example: iy chain \S+) \S+")     # a number chosen at random
# the statistics say whether they were computed now or read from the saved results: whichever program ran first
SAVED = re.compile(r"  \((?:from .*|computed and saved)\)$", re.M)


def run(cmd: list[str], args: list[str], env: dict) -> tuple[int, str]:
    p = subprocess.run(cmd + args, capture_output=True, text=True, encoding="utf-8", errors="replace", env=env)
    return p.returncode, SAVED.sub("  (saved)", EXAMPLE.sub(r"\1 N", TIME.sub(" in N ms", p.stdout)))


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--python", default="iy", help='the Python isnady\'s command (default "iy")')
    ap.add_argument("--rust", default=os.path.join("target", "release", "iy-rs" + (".exe" if os.name == "nt" else "")))
    ap.add_argument("--data-dir", help="the data folder for both (ISNADY_DATA_DIR)")
    a = ap.parse_args()
    env = dict(os.environ, PYTHONIOENCODING="utf-8", PYTHONUTF8="1")
    if a.data_dir:
        env["ISNADY_DATA_DIR"] = a.data_dir
    py, rs = shlex.split(a.python, posix=os.name != "nt"), [a.rust]
    same = 0
    for case in CASES:
        pc, pout = run(py, case, env)
        rc, rout = run(rs, case, env)
        if case == ["version"]:                      # "isnady 0.7.4" and "isnady 0.7.4 (Rust)": the same number
            pv, rv = pout.split()[1:2], rout.split()[1:2]
            ok = pv == rv
        else:
            ok = pout == rout and pc == rc
        print(f"{'✓' if ok else '✗'} iy {' '.join(case)}")
        if ok:
            same += 1
            continue
        pl, rl = pout.splitlines(), rout.splitlines()
        for i in range(max(len(pl), len(rl))):
            x = pl[i] if i < len(pl) else "<none>"
            y = rl[i] if i < len(rl) else "<none>"
            if x != y:
                print(f"    line {i + 1}\n      python: {x[:200]}\n      rust:   {y[:200]}")
                break
        if pc != rc:
            print(f"    exit code: python {pc}, rust {rc}")
    print(f"\n{same} of {len(CASES)} the same")
    return 0 if same == len(CASES) else 1


if __name__ == "__main__":
    sys.exit(main())
