"""isnady-rs's README is isnady's README, word for word, under a short head of its own (tools/readme_head.md):

    python tools/sync_readme.py                       # from ../isnady/README.md (the two repositories side by side)
    python tools/sync_readme.py PATH-OR-URL           # from another copy
    python tools/sync_readme.py --check               # say whether README.md is in step (exit 1 if not)

Run it in every release, after isnady's own README is written.
"""

import sys
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEFAULT = ROOT.parent / "isnady" / "README.md"
URL = "https://raw.githubusercontent.com/bayramkotan/isnady/main/README.md"


def source(where: str | None) -> str:
    if where and where.startswith("http"):
        with urllib.request.urlopen(where, timeout=60) as r:
            return r.read().decode("utf-8")
    path = Path(where) if where else DEFAULT
    if not path.exists():
        print(f"{path} not found; from {URL}")
        return source(URL)
    return path.read_text(encoding="utf-8")


def main() -> int:
    args = [a for a in sys.argv[1:] if a != "--check"]
    body = source(args[0] if args else None)
    text = (ROOT / "tools" / "readme_head.md").read_text(encoding="utf-8") + body
    target = ROOT / "README.md"
    if "--check" in sys.argv:
        same = target.read_text(encoding="utf-8") == text
        print("README.md is in step with isnady's README" if same else "README.md is NOT in step: python tools/sync_readme.py")
        return 0 if same else 1
    target.write_text(text, encoding="utf-8", newline="\n")
    print(f"wrote README.md ({len(text):,} characters): the head and isnady's README")
    return 0


if __name__ == "__main__":
    sys.exit(main())
