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
    # the chains (RS2): read by the Python isnady, shown by both; road A with Ibn Hajar's Taqrib, road B with al-Najashi
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
]

TIME = re.compile(r" in \d+ ms")
EXAMPLE = re.compile(r"(Example: iy chain \S+) \S+")     # a number chosen at random


def run(cmd: list[str], args: list[str], env: dict) -> tuple[int, str]:
    p = subprocess.run(cmd + args, capture_output=True, text=True, encoding="utf-8", errors="replace", env=env)
    return p.returncode, EXAMPLE.sub(r"\1 N", TIME.sub(" in N ms", p.stdout))


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
