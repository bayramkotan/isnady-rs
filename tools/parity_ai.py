"""Parity of the meaning index and takhrij (YZ1, YZ2, A4): the Rust iy-rs and the Python iy.

1. The Python isnady builds the index (`iy ai build`) in a copy of the data folder; then both programs read it:
   `iy ai status`, searches by meaning and `iy tahric` must print the same, word for word.
2. iy-rs builds it in another copy: the takhrij pairs must be the same rows, and the index the same files — every
   array of lsa.npz, lsa.vocab and lsa.json (apart from the time): both programs find the concepts by the same
   subspace iteration from the same start (LSA3). The Python isnady reads iy-rs's index and finds the same hadith.

    python tools/parity_ai.py --data-dir DIR [--python "python -m isnady"] [--rust target/release/iy-rs]

Needs numpy (pip install "isnady[ai]"). Exit code 0 when all holds, 1 otherwise.
"""

import argparse
import os
import re
import shlex
import shutil
import sqlite3
import subprocess
import sys
import tempfile

QUERIES = [["niyet"], ["komşu", "hakları"], ["intentions"], ["الصلاة"], ["patience", "in", "hardship"],
           ["sabır", "--limit", "5"], ["oruç", "--grade", "sahih"], ["الصبر", "--book", "riyad"], ["zzqqx"],
           ["anne", "baba", "--limit", "3", "--offset", "4"]]
TIME = re.compile(r" in \d+ ms")


def run(cmd, args, env):
    p = subprocess.run(cmd + args, capture_output=True, text=True, encoding="utf-8", errors="replace", env=env)
    return p.returncode, TIME.sub(" in N ms", p.stdout)


def copy(src: str) -> str:
    dst = tempfile.mkdtemp(prefix="isnady-ai-")
    for n in os.listdir(src):
        s = os.path.join(src, n)
        if n in ("ai", "cache", "logs"):
            continue
        (shutil.copytree if os.path.isdir(s) else shutil.copy2)(s, os.path.join(dst, n))
    return dst


def hadith_of(folder: str, n: int) -> list[list[str]]:
    conn = sqlite3.connect(os.path.join(folder, "isnady.db"))
    # hadith of several books: every 50th hadith that has another narration
    rows = conn.execute("""SELECT c.key, h.number FROM (SELECT DISTINCT hadith_a AS id FROM hadith_relations) r
                           JOIN hadiths h ON h.id = r.id JOIN collections c ON c.id = h.collection_id
                           ORDER BY r.id""").fetchall()[::50][:n]
    conn.close()
    return [[k, str(num)] for k, num in rows]


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--python", default="iy")
    ap.add_argument("--rust", default=os.path.join("target", "release", "iy-rs" + (".exe" if os.name == "nt" else "")))
    ap.add_argument("--data-dir", default=os.environ.get("ISNADY_DATA_DIR"))
    a = ap.parse_args()
    if not a.data_dir:
        ap.error("which data folder? --data-dir DIR (or ISNADY_DATA_DIR)")
    import numpy as np

    py, rs = shlex.split(a.python, posix=os.name != "nt"), [os.path.abspath(a.rust)]
    base = dict(os.environ, PYTHONIOENCODING="utf-8", PYTHONUTF8="1")
    ok = True

    def check(name: str, good: bool, more: str = "") -> None:
        nonlocal ok
        ok &= good
        print(f"{'✓' if good else '✗'} {name}{('   ' + more) if more else ''}")

    pyd, rsd = copy(a.data_dir), copy(a.data_dir)
    try:
        envp, envr = dict(base, ISNADY_DATA_DIR=pyd), dict(base, ISNADY_DATA_DIR=rsd)
        for args in (["ai", "status"], ["search", "--mode", "meaning", "niyet"]):
            check("before the build: iy " + " ".join(args), run(py, args, envp) == run(rs, args, envp))
        code, _ = run(py, ["ai", "build"], envp)
        check("iy ai build (Python)", code == 0)
        # 1. both read the Python isnady's index
        cases = [["ai", "status"]] + [["search", "--mode", "meaning"] + q + ([] if "--limit" in q else ["--limit", "15"])
                                      for q in QUERIES]
        cases += [["tahric"] + h for h in hadith_of(pyd, 12)] + [["tahric", "nosuchbook", "1"]]
        for args in cases:
            p, r = run(py, args, envp), run(rs, args, envp)
            check("iy " + " ".join(args), p == r)
            if p != r:
                pl, rl = p[1].splitlines(), r[1].splitlines()
                at = next((i for i, (x, y) in enumerate(zip(pl, rl)) if x != y), min(len(pl), len(rl)))
                print(f"    Python {pl[at:at + 2]}\n    Rust   {rl[at:at + 2]}")
        # 2. iy-rs builds its own
        code, _ = run(rs, ["ai", "build"], envr)
        check("iy-rs ai build", code == 0)
        q = "SELECT hadith_a, hadith_b, kind, score, method FROM hadith_relations ORDER BY 1, 2"
        rp = sqlite3.connect(os.path.join(pyd, "isnady.db")).execute(q).fetchall()
        rr = sqlite3.connect(os.path.join(rsd, "isnady.db")).execute(q).fetchall()
        check("takhrij: the same pairs", rp == rr, f"{len(rp):,} / {len(rr):,}")
        P, R = np.load(os.path.join(pyd, "ai", "lsa.npz")), np.load(os.path.join(rsd, "ai", "lsa.npz"))
        vocab = [open(os.path.join(d, "ai", "lsa.vocab"), encoding="utf-8").read() for d in (pyd, rsd)]
        check("the vocabulary", vocab[0] == vocab[1])
        for k in P.files:
            same = P[k].shape == R[k].shape and np.array_equal(P[k], R[k])
            check(f"lsa.npz {k}", same, "" if same else f"{int(np.sum(P[k] != R[k])) if P[k].shape == R[k].shape else 'other shape'} values differ")
        import json
        metas = [json.loads(open(os.path.join(d, "ai", "lsa.json"), encoding="utf-8").read()) for d in (pyd, rsd)]
        for m in metas:
            m.pop("seconds", None)
            m.pop("built_at", None)
        check("lsa.json (apart from the time)", metas[0] == metas[1])
        shared = total = 0
        for words in QUERIES[:8]:
            args = ["search", "--mode", "meaning"] + words + ["--limit", "10"]
            found = [set(re.findall(r"^(.+ #\S+)   meaning", run(py, args, env)[1], re.M)) for env in (envp, envr)]
            shared += len(found[0] & found[1])
            total += len(found[0] | found[1])
        check("the Python isnady reads iy-rs's index; the same hadith found", shared == total,
              f"{shared} of {total} in the first ten")
    finally:
        shutil.rmtree(pyd, ignore_errors=True)
        shutil.rmtree(rsd, ignore_errors=True)
    print("\nall the same" if ok else "\nsome differ")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
