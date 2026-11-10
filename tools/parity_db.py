"""The chain reader, the narrator matcher, the search index and the books' trees of iy-rs against the Python
isnady's, row for row (RS17, RS17c).

The database of the data folder is copied twice; `iy isnads --rebuild` reads every chain and identifies every narrator
in one copy, `iy-rs isnads --rebuild` in the other. Then, in each copy, the search index and the trees of the hadith
collections are thrown away and built again: `search` builds the index (search.ensure_index), `book` the trees
(works.ensure_collection_works). The chains, their links (with the narrator identified, the confidence and the
number of candidates), the roads joined at a tahwil, the normalised texts, the word list, the trees and the stamps
must be the same, row for row.

With --import (A2), each program starts from an empty data folder of its own and imports the same sources of the
repository (iy catalog import ID); what both print and every table of both databases (all but the times of the
import) must be the same, and so must imported.json.

    python tools/parity_db.py --python iy --rust target/release/iy-rs
    python tools/parity_db.py --import al_arbain_al_nawawiyya --import sahifat_hammam_ibn_munabbih
"""

import argparse
import hashlib
import json
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
    "text_norm": "SELECT text_id, norm FROM text_norm ORDER BY text_id",
    "words": "SELECT word, n FROM words ORDER BY word",
    "works": "SELECT id, key, kind, title, title_ar, author, source_id, collection_id, stamp FROM works ORDER BY id",
    "work_nodes": "SELECT id, work_id, parent_id, ordinal, kind, label, title, hadith_id, person_id, text "
                  "FROM work_nodes ORDER BY id",
    "core_meta": "SELECT key, value FROM core_meta WHERE key IN ('isnad_parser', 'isnads_ready', 'narrator_matcher', "
                 "'index_ready', 'normalizer_version') ORDER BY key",
}

FORGET = """
DROP TRIGGER IF EXISTS text_norm_ai; DROP TRIGGER IF EXISTS text_norm_ad;
DROP TABLE IF EXISTS texts_fts; DROP TABLE IF EXISTS text_norm; DROP TABLE IF EXISTS words;
DELETE FROM core_meta WHERE key = 'index_ready';
DELETE FROM work_nodes WHERE work_id IN (SELECT id FROM works WHERE collection_id IS NOT NULL);
DELETE FROM works WHERE collection_id IS NOT NULL;
"""


def forget_index_and_trees(db: Path) -> None:
    conn = sqlite3.connect(db)
    conn.executescript(FORGET)
    conn.commit()
    conn.close()


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


def every_table(db: Path) -> dict:
    """Every table but the full-text index's own, each row but its import time (imported_at)."""
    conn = sqlite3.connect(db)
    out = {}
    names = [r[0] for r in conn.execute("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE "
                                        "'sqlite_%' AND name NOT LIKE 'texts_fts%' ORDER BY name")]
    for name in names:
        cols = [r[1] for r in conn.execute(f"PRAGMA table_info({name})") if r[1] != "imported_at"]
        h, n = hashlib.md5(), 0
        for row in conn.execute(f"SELECT {', '.join(cols)} FROM {name} ORDER BY {', '.join(cols)}"):
            h.update(repr(row).encode())
            n += 1
        out[name] = (n, h.hexdigest())
    conn.close()
    return out


def imports(args) -> int:
    """Both programs import the same sources into empty folders of their own."""
    said, tables, state = {}, {}, {}
    with tempfile.TemporaryDirectory() as tmp:
        for side, cmd in (("python", args.python), ("rust", args.rust)):
            folder = Path(tmp) / side
            folder.mkdir()
            env = dict(os.environ, ISNADY_DATA_DIR=str(folder), PYTHONIOENCODING="utf-8", PYTHONUTF8="1")
            out = []
            for source in args.imports:
                # "REPO/ID": a source of another repository (iy catalog import ID --repo REPO)
                repo, _, entry = source.rpartition("/")
                run = subprocess.run(shlex.split(cmd) + ["catalog", "import", entry] + (["--repo", repo] if repo else []),
                                     env=env, capture_output=True, text=True)
                out.append(run.stdout + run.stderr + f"(exit {run.returncode})\n")
            said[side] = "".join(out)
            tables[side] = every_table(folder / "isnady.db")
            # the databases beside it: the Qur'an, the entries the written lives quote
            for extra in sorted((folder / "sources").glob("*.db")) if (folder / "sources").is_dir() else []:
                for name, value in every_table(extra).items():
                    tables[side][f"sources/{extra.name}:{name}"] = value
            try:
                state[side] = json.loads((folder / "imported.json").read_text(encoding="utf-8"))
            except (OSError, ValueError):
                state[side] = None
    bad = 0
    same = said["python"] == said["rust"]
    bad += not same
    print(f"{'✓' if same else '✗'} what both print")
    if not same:
        import difflib

        sys.stdout.writelines(difflib.unified_diff(said["python"].splitlines(True), said["rust"].splitlines(True),
                                                   "iy", "iy-rs"))
    for name in sorted(set(tables["python"]) | set(tables["rust"])):
        a, b = tables["python"].get(name), tables["rust"].get(name)
        if a != b:
            bad += 1
            print(f"✗ {name}: {a and a[0]} rows (iy-rs: {b and b[0]})")
    same = state["python"] == state["rust"]
    bad += not same
    print(f"{'✓' if same else '✗'} imported.json")
    print(f"✓ {len(tables['python'])} tables" if not bad else f"{bad} difference(s)")
    print("the same database" if not bad else "not the same")
    return 1 if bad else 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--python", default="iy")
    ap.add_argument("--rust", default="target/release/iy-rs")
    ap.add_argument("--import", dest="imports", action="append", metavar="ID",
                    help="import this source of the repository (REPO/ID: of another repository) into empty folders "
                         "instead (repeatable)")
    args = ap.parse_args()
    if args.imports:
        return imports(args)
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
            forget_index_and_trees(folder / "isnady.db")
            for again in (["search", "صلى"], ["book"]):
                run = subprocess.run(shlex.split(cmd) + again, env=env, capture_output=True, text=True)
                if run.returncode:
                    print(f"{side} {again[0]}: {run.stderr}", file=sys.stderr)
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
