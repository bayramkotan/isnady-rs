"""The ready database (DB-READY) in both programs: `iy database download` and `iy-rs database download` must say the
same and leave the same database behind.

    python tools/parity_ready.py --python iy --rust target/release/iy-rs [--data-dir DIR]

A ready database is made here from the data folder's isnady.db (zipped as tools/publish_database.py zips it, with its
database.json), and a few wrong ones beside it: a sha256 that does not match, a newer schema, a size that does not
match, a database.json that is not there. Each program then downloads them (ISNADY_DATABASE_INFO points to them) into
an empty data folder of its own; what each prints, its exit code, the databases left in the folder and the books and
hadith of the one it uses are compared; then a search in it. The date-time of a backup, the folder's path and "N ms"
are the only things read as equal.
"""

import argparse
import hashlib
import json
import os
import re
import shlex
import shutil
import sqlite3
import subprocess
import sys
import tempfile
import zipfile

CASES = [
    # (database.json, the arguments)
    ("good", ["database", "download", "--check"]),
    ("good", ["database", "download"]),
    ("good", ["database", "download"]),        # again: the first one is kept as a backup
    ("good", ["database"]),
    ("bad-sha", ["database", "download"]),
    ("bad-size", ["database", "download"]),
    ("newer", ["database", "download"]),
    ("newer", ["database", "download", "--check"]),
    ("missing", ["database", "download"]),
    ("no-url", ["database", "download"]),
    ("good", ["search", "النيات", "--limit", "1"]),
]


def make_ready(data_dir: str, work: str) -> dict:
    """The data folder's database as a ready one, and database.json files describing it rightly and wrongly."""
    src = os.path.join(data_dir, "isnady.db")
    copy = os.path.join(work, "isnady.db")
    conn = sqlite3.connect(src)
    conn.execute("VACUUM INTO ?", (copy,))
    conn.close()
    conn = sqlite3.connect(copy)
    one = lambda sql: conn.execute(sql).fetchone()[0]  # noqa: E731
    info = {"date": "2026-11-12", "schema": one("PRAGMA user_version"), "books": one("SELECT COUNT(*) FROM collections"),
            "hadith": one("SELECT COUNT(*) FROM hadiths"), "texts": one("SELECT COUNT(*) FROM texts"),
            "narrators": one("SELECT COUNT(*) FROM persons"), "grades": one("SELECT COUNT(*) FROM grades"),
            "sources": [k for (k,) in conn.execute("SELECT key FROM sources ORDER BY key")]}
    conn.close()
    os.makedirs(os.path.join(work, "release"))
    zipped = os.path.join(work, "release", "isnady-db-2026-11-12.zip")
    with zipfile.ZipFile(zipped, "w", zipfile.ZIP_DEFLATED) as z:
        z.write(copy, "isnady.db")
    os.remove(copy)
    data = open(zipped, "rb").read()
    info.update(file="isnady-db-2026-11-12.zip", size=len(data), sha256=hashlib.sha256(data).hexdigest(),
                url="release/isnady-db-2026-11-12.zip")
    variants = {"good": info, "bad-sha": dict(info, sha256="0" * 64), "bad-size": dict(info, size=info["size"] + 1),
                "newer": dict(info, schema=info["schema"] + 90), "no-url": {k: v for k, v in info.items() if k != "url"}}
    where = {}
    for name, body in variants.items():
        path = os.path.join(work, f"{name}.json")
        with open(path, "w", encoding="utf-8") as f:
            json.dump(body, f, indent=1)
        where[name] = path
    where["missing"] = os.path.join(work, "missing.json")
    return where


def state(folder: str) -> dict:
    # the databases and what waits or was left of a download (a cache, a log, the settings are each program's own)
    out = {"files": sorted(re.sub(r"\d{8}-\d{6}", "STAMP", n) for n in os.listdir(folder)
                           if n.startswith(("isnady.", "isnady-db")) and not n.endswith(("-wal", "-shm")))}
    db = os.path.join(folder, "isnady.db")
    if os.path.exists(db):
        conn = sqlite3.connect(db)
        out["books"] = conn.execute("SELECT COUNT(*) FROM collections").fetchone()[0]
        out["hadith"] = conn.execute("SELECT COUNT(*) FROM hadiths").fetchone()[0]
        conn.close()
    return out


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--python", default="iy")
    ap.add_argument("--rust", default=os.path.join("target", "release", "iy-rs" + (".exe" if os.name == "nt" else "")))
    ap.add_argument("--data-dir", default=os.environ.get("ISNADY_DATA_DIR"))
    a = ap.parse_args()
    if not a.data_dir:
        ap.error("which data folder? --data-dir DIR (or ISNADY_DATA_DIR)")
    work = tempfile.mkdtemp(prefix="isnady-ready-")
    where = make_ready(a.data_dir, work)
    programs = {"py": shlex.split(a.python, posix=os.name != "nt"), "rs": [os.path.abspath(a.rust)]}
    folders = {who: tempfile.mkdtemp(prefix=f"isnady-ready-{who}-") for who in programs}
    same = 0
    for info, args in CASES:
        got = {}
        for who, cmd in programs.items():
            env = dict(os.environ, PYTHONIOENCODING="utf-8", PYTHONUTF8="1", ISNADY_DATA_DIR=folders[who],
                       ISNADY_DATABASE_INFO=where[info])
            p = subprocess.run(cmd + args, capture_output=True, text=True, encoding="utf-8", errors="replace", env=env)
            norm = lambda t: re.sub(r"\d+ ms\b", "N ms", re.sub(r"\d{8}-\d{6}", "STAMP",  # noqa: B023, E731
                                                                  t.replace(folders[who], "DATA")))
            got[who] = (p.returncode, norm(p.stdout), norm(p.stderr), state(folders[who]))
        ok = got["py"] == got["rs"]
        same += ok
        print(f"{'✓' if ok else '✗'} [{info}] iy {' '.join(args)}")
        if not ok:
            for n, what in enumerate(("exit code", "output", "errors", "folder")):
                if got["py"][n] != got["rs"][n]:
                    print(f"    {what}: Python {got['py'][n]!r}\n    {' ' * len(what)}  Rust   {got['rs'][n]!r}")
    for f in [work, *folders.values()]:
        shutil.rmtree(f, ignore_errors=True)
    print(f"\n{same} of {len(CASES)} the same")
    return 0 if same == len(CASES) else 1


if __name__ == "__main__":
    sys.exit(main())
