"""The chain reader and the narrator matcher of iy-rs against the Python isnady's, row for row (RS17).

The database of the data folder is copied twice; `iy isnads --rebuild` reads every chain and identifies every narrator
in one copy, `iy-rs isnads --rebuild` in the other; then the chains, their links (with the narrator identified, the
confidence and the number of candidates), the roads joined at a tahwil and the stamps must be the same, row for row.

    python tools/parity_db.py --python iy --rust target/release/iy-rs
"""

import argparse
import hashlib
import os
import shlex
import shutil
import sqlite3
import subprocess
import sys
import tempfile
from pathlib import Path

QUERIES = {
    "isnads": "SELECT id, hadith_id, ordinal, raw_text, source_id, derived_by, reaches_prophet, problem, ends_with, raised "
              "FROM isnads ORDER BY id",
    "isnad_links": "SELECT isnad_id, position, raw_name, transmission, person_id, match_confidence, candidates "
                   "FROM isnad_links ORDER BY isnad_id, position",
    "isnad_meetings": "SELECT * FROM isnad_meetings ORDER BY isnad_id",
    "core_meta": "SELECT key, value FROM core_meta WHERE key IN ('isnad_parser', 'isnads_ready', 'narrator_matcher') "
                 "ORDER BY key",
}


def digest(db: Path) -> dict:
    conn = sqlite3.connect(db)
    out = {}
    for name, sql in QUERIES.items():
        h, n = hashlib.md5(), 0
        try:
            for row in conn.execute(sql):
                h.update(repr(row).encode())
                n += 1
        except sqlite3.OperationalError:
            pass
        out[name] = (n, h.hexdigest())
    conn.close()
    return out


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--python", default="iy")
    ap.add_argument("--rust", default="target/release/iy-rs")
    args = ap.parse_args()
    data = Path(os.environ.get("ISNADY_DATA_DIR") or "")
    if not (data / "isnady.db").exists():
        print(f"No database in {data} (set ISNADY_DATA_DIR)", file=sys.stderr)
        return 2
    results = {}
    with tempfile.TemporaryDirectory() as tmp:
        for side, cmd in (("python", args.python), ("rust", args.rust)):
            folder = Path(tmp) / side
            folder.mkdir()
            shutil.copy(data / "isnady.db", folder / "isnady.db")
            env = dict(os.environ, ISNADY_DATA_DIR=str(folder))
            run = subprocess.run(shlex.split(cmd) + ["isnads", "--rebuild"], env=env, capture_output=True, text=True)
            if run.returncode:
                print(f"{side}: {run.stderr}", file=sys.stderr)
                return 1
            results[side] = digest(folder / "isnady.db")
    bad = 0
    for name in QUERIES:
        a, b = results["python"][name], results["rust"][name]
        same = a == b
        bad += not same
        print(f"{'✓' if same else '✗'} {name}: {a[0]} rows" + ("" if same else f" (iy-rs: {b[0]})"))
    print("the same database" if not bad else f"{bad} table(s) differ")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
