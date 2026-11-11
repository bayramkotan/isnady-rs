"""Parity of the exports and imports: the Rust iy-rs and the Python iy write the same files, and bring the same in.

Every case below is exported by both programs from the same data folder and the two files compared: the text
formats byte for byte, the zipped ones (xlsx, docx, epub, isnady book, backup, profile) entry by entry, the SQLite
ones table by table. Only the moment of the export is left out. Then the files written are imported by both, each
into its own copy of the profile, and what they say and what they keep compared.

    python tools/parity_export.py --data-dir /tmp/isnady-test
    python tools/parity_export.py --python "python -m isnady" --rust target/release/iy-rs.exe --data-dir DIR

Exit code 0 when every case is the same, 1 otherwise.
"""

import argparse
import io
import os
import re
import shlex
import shutil
import sqlite3
import subprocess
import sys
import tempfile
import zipfile

DOC = ["json", "jsonl", "csv", "tsv", "xlsx", "md", "html", "txt", "epub", "docx", "sqlite"]
TABLE = ["json", "jsonl", "csv", "tsv", "xlsx", "md", "html", "txt"]

# (name, arguments, formats)
CASES = [
    ("hadith", ["hadith", "nawawi:1", "riyad:1-3", "sifat-shia:1"], DOC),
    ("hadith_plain", ["hadith", "riyad:1-2", "--no-chains", "--language", "Arabic", "--language", "Turkish"],
     ["json", "csv", "md", "html", "txt"]),
    ("chapter", ["book", "riyad", "--chapter", "1"], ["isnady-book"] + DOC),
    ("book", ["book", "nawawi40"], ["isnady-book", "json", "csv", "html", "epub", "docx", "sqlite"]),
    ("books", ["book", "nawawi40", "shamail"], ["json", "csv", "md"]),
    ("books_apart", ["book", "nawawi40", "shamail", "--separate"], ["json", "isnady-book"]),
    ("search", ["search", "النيات"], DOC),
    ("search_grade", ["search", "الصلاة", "--grade", "sahih", "--book", "riyad", "--in-chapter", "riyad:2"],
     ["json", "csv"]),
    ("bookmarks", ["bookmarks"], TABLE),
    ("narrators", ["narrators", "الزهري"], TABLE + ["sqlite"]),
    ("narrators_shia", ["narrators", "زرارة", "--tradition", "shia"], ["json", "csv"]),
    ("network", ["network", "nawawi40"], ["graphml", "gexf", "csv", "json"]),
    ("network_two", ["network", "nawawi40", "shamail", "--tradition", "sunni"], ["json"]),
    ("settings", ["settings"], ["json", "csv", "tsv", "md"]),
    ("profile", ["profile"], ["isnady-profile", "json"]),
    ("backup", ["backup"], ["backup"]),
    ("statistics", ["statistics", "nawawi40"], ["json"]),
    # what cannot be done says so, in the same words
    ("no_book", ["book", "nosuchbook"], ["json"]),
    ("no_ref", ["hadith", "riyad"], ["json"]),
    ("no_hadith", ["hadith", "riyad:999999"], ["json"]),
    ("no_range", ["hadith", "riyad:a-b"], ["json"]),
    ("nothing_found", ["search", "zzzqqq"], ["json"]),
    ("wrong_format", ["settings"], ["epub"]),
    ("no_profile", ["profile", "nosuchprofile"], ["json"]),
]

# kind, export name, format, more arguments
IMPORTS = [
    ("bookmarks", "bookmarks", "json", []),
    ("bookmarks", "bookmarks", "csv", ["--folder", "Brought/In"]),
    ("bookmarks", "bookmarks", "jsonl", ["--replace"]),
    ("bookmarks", "bookmarks", "tsv", []),
    ("settings", "settings", "json", []),
    ("settings", "settings", "csv", ["--replace"]),
    ("settings", "settings", "tsv", []),
    ("profile", "profile", "isnady-profile", ["--name", "Brought"]),
    ("profile", "profile", "json", []),
    ("backup", "backup", "backup", []),
    ("book", "chapter", "isnady-book", []),
    ("book", "chapter", "json", []),
    ("book", "hadith", "jsonl", []),
    ("book", "hadith_plain", "csv", []),
    ("book", "chapter", "tsv", []),
    ("bookmarks", "settings", "json", []),
    ("settings", "bookmarks", "md", []),
    ("profile", "settings", "json", []),
    ("backup", "profile", "isnady-profile", ["--format", "backup"]),
]

EXT = {"isnady-book": ".isnady-book.zip", "isnady-profile": ".isnady-profile", "backup": ".isnady-backup.zip",
       "sqlite": ".sqlite"}
STAMP = re.compile(rb"\d{4}-\d\d-\d\d(?:[T ]\d\d:\d\d(?::\d\d)?)?")
IDS = re.compile(rb"urn:uuid:[0-9a-f-]+")
FROM = re.compile(rb'"from_file": [^\n]+')       # the statistics: computed now or read from the saved results


def plain(data: bytes) -> bytes:
    return FROM.sub(b'"from_file": N', IDS.sub(b"urn:uuid:N", STAMP.sub(b"DATE", data)))


def tables(path: str) -> dict:
    conn = sqlite3.connect(path)
    out = {}
    for (name, sql) in conn.execute("SELECT name, sql FROM sqlite_master WHERE type = 'table' ORDER BY name"):
        out[name] = (sql, [tuple(plain(str(v).encode()) for v in r) for r in conn.execute(f'SELECT * FROM "{name}"')])
    conn.close()
    return out


def content(path: str):
    """What a file holds, in a shape that compares: bytes, or the entries of a zip, or the tables of a database."""
    if os.path.isdir(path):
        return {n: content(os.path.join(path, n)) for n in sorted(os.listdir(path))}
    with open(path, "rb") as f:
        data = f.read()
    if data[:4] == b"PK\x03\x04":
        with zipfile.ZipFile(io.BytesIO(data)) as z:
            return [(i.filename, i.compress_type, plain(z.read(i))) for i in z.infolist()]
    if data[:15] == b"SQLite format 3":
        return tables(path)
    return plain(data)


def where(a, b, at="") -> str:
    """The first place two contents differ, said shortly."""
    if type(a) is not type(b):
        return f"{at}: {type(a).__name__} / {type(b).__name__}"
    if isinstance(a, dict):
        for k in sorted(set(a) | set(b)):
            if k not in a or k not in b:
                return f"{at}/{k}: only in {'Rust' if k not in a else 'Python'}"
            if a[k] != b[k]:
                return where(a[k], b[k], f"{at}/{k}")
    if isinstance(a, (list, tuple)):
        if len(a) != len(b):
            return f"{at}: {len(a)} / {len(b)} items; {[x[0] if isinstance(x, tuple) else x for x in a][:12]} / " \
                   f"{[x[0] if isinstance(x, tuple) else x for x in b][:12]}"
        for n, (x, y) in enumerate(zip(a, b)):
            if x != y:
                return where(x, y, f"{at}[{x[0] if isinstance(x, tuple) and isinstance(x[0], str) else n}]")
    if isinstance(a, bytes):
        n = next((i for i, (x, y) in enumerate(zip(a, b)) if x != y), min(len(a), len(b)))
        return f"{at} at byte {n}:\n    Python: {a[max(0, n - 60):n + 80]!r}\n    Rust:   {b[max(0, n - 60):n + 80]!r}"
    return f"{at}: {a!r} / {b!r}"


def run(cmd, args, env, cwd):
    p = subprocess.run(cmd + args, capture_output=True, text=True, encoding="utf-8", errors="replace", env=env, cwd=cwd)
    return p.returncode, p.stdout, p.stderr


def profile_state(folder: str) -> dict:
    """What an import left behind: the settings, bookmarks and profiles of a data folder, and its books."""
    out = {}
    for base, _dirs, files in os.walk(folder):
        for n in files:
            full = os.path.join(base, n)
            rel = os.path.relpath(full, folder).replace(os.sep, "/")
            if n.endswith(".json") and not rel.startswith(("repos/", "cache/", "derived/", "logs/")):
                with open(full, "rb") as f:
                    out[rel] = re.sub(rb'"(added|created|id|parent)": ?[^,}\n]+', b'"\\1": N',
                                      re.sub(rb'"[0-9a-f]{8,32}"', b'"ID"', plain(f.read())))
    db = os.path.join(folder, "isnady.db")
    if os.path.exists(db):
        conn = sqlite3.connect(db)
        for t in ("sources", "collections", "editions", "hadiths", "texts", "grades", "sections", "section_titles"):
            out["db:" + t] = conn.execute(f"SELECT COUNT(*) FROM {t}").fetchone()[0]
        out["db:books"] = conn.execute("SELECT key, name, tradition FROM collections ORDER BY key").fetchall()
        out["db:sources"] = conn.execute("SELECT key, name, format, license, origin FROM sources ORDER BY key").fetchall()
        conn.close()
    return out


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--python", default="iy", help='the Python isnady\'s command (default "iy")')
    ap.add_argument("--rust", default=os.path.join("target", "release", "iy-rs" + (".exe" if os.name == "nt" else "")))
    ap.add_argument("--data-dir", default=os.environ.get("ISNADY_DATA_DIR"),
                    help="the data folder both export from (default: ISNADY_DATA_DIR)")
    ap.add_argument("--only", help="only the cases whose name starts so")
    ap.add_argument("--keep", help="keep the files written in this folder")
    a = ap.parse_args()
    if not a.data_dir:
        ap.error("which data folder? --data-dir DIR (or ISNADY_DATA_DIR)")
    env = dict(os.environ, PYTHONIOENCODING="utf-8", PYTHONUTF8="1", ISNADY_DATA_DIR=os.path.abspath(a.data_dir))
    py, rs = shlex.split(a.python, posix=os.name != "nt"), [os.path.abspath(a.rust)]
    work = a.keep or tempfile.mkdtemp(prefix="isnady-export-")
    out = {"py": os.path.join(work, "py"), "rs": os.path.join(work, "rs")}
    for d in out.values():
        os.makedirs(d, exist_ok=True)
    # something to export: a few bookmarks in folders, a setting changed
    for args in (["bookmark", "add", "hadith", "riyad:1", "--folder", "Prayer/Friday", "--note", "the first"],
                 ["bookmark", "add", "hadith", "nawawi:1"], ["bookmark", "add", "narrator", "sunni:1", "--folder", "People"]):
        run(py, args, env, work)
    same = total = 0
    for name, args, formats in CASES:
        if a.only and not name.startswith(a.only):
            continue
        for fmt in formats:
            total += 1
            file = name + "_" + fmt.replace("-", "_") + EXT.get(fmt, "." + fmt)
            if name == "books_apart":
                file = name + "_" + fmt.replace("-", "_")
            got = {}
            for who, cmd in (("py", py), ("rs", rs)):
                target = os.path.join(out[who], file)
                code, said, err = run(cmd, ["export"] + args + ["--format", fmt, "--output", target], env, out[who])
                said = said.replace(out[who], "DIR")
                err = "\n".join(line for line in err.replace(out[who], "DIR").splitlines() if not line.startswith("  "))
                got[who] = (code, said, err, content(target) if os.path.exists(target) else None)
            ok = got["py"] == got["rs"]
            print(f"{'✓' if ok else '✗'} iy export {' '.join(args)} --format {fmt}")
            if ok:
                same += 1
                continue
            for n, what in enumerate(("exit code", "output", "errors")):
                if got["py"][n] != got["rs"][n]:
                    print(f"    {what}: Python {got['py'][n]!r}\n    {' ' * len(what)}  Rust   {got['rs'][n]!r}")
            if got["py"][3] != got["rs"][3]:
                print("   ", where(got["py"][3], got["rs"][3]))
    for kind, source, fmt, more in IMPORTS:
        if a.only and not ("import_" + kind).startswith(a.only):
            continue
        total += 1
        file = source + "_" + fmt.replace("-", "_") + EXT.get(fmt, "." + fmt)
        got = {}
        for who, cmd in (("py", py), ("rs", rs)):
            # each into its own data folder, a copy of the profile; a book into an empty database
            folder = tempfile.mkdtemp(prefix="isnady-import-")
            for n in os.listdir(a.data_dir):
                full = os.path.join(a.data_dir, n)
                if n.endswith(".json") and os.path.isfile(full):
                    shutil.copy(full, folder)
                elif n == "profiles" and os.path.isdir(full):
                    shutil.copytree(full, os.path.join(folder, n))
            code, said, err = run(cmd, ["import-data", kind, os.path.join(out["py"], file)] + more,
                                  dict(env, ISNADY_DATA_DIR=folder), folder)
            err = "\n".join(line for line in err.replace(out["py"], "DIR").splitlines() if not line.startswith("  "))
            got[who] = (code, said.replace(folder, "DATA"), err.replace(folder, "DATA"), profile_state(folder))
            shutil.rmtree(folder, ignore_errors=True)
        ok = got["py"] == got["rs"]
        print(f"{'✓' if ok else '✗'} iy import-data {kind} {file} {' '.join(more)}")
        if ok:
            same += 1
            continue
        for n, what in enumerate(("exit code", "output", "errors")):
            if got["py"][n] != got["rs"][n]:
                print(f"    {what}: Python {got['py'][n]!r}\n    {' ' * len(what)}  Rust   {got['rs'][n]!r}")
        if got["py"][3] != got["rs"][3]:
            print("   ", where(got["py"][3], got["rs"][3])[:1500])
    if not a.keep:
        shutil.rmtree(work, ignore_errors=True)
    print(f"\n{same} of {total} the same")
    return 0 if same == total else 1


if __name__ == "__main__":
    sys.exit(main())
