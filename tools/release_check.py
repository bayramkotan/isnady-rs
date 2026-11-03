"""Check a release before it is tagged, and write its GitHub Release text:

    python tools/release_check.py v0.7.4 [--notes notes.md]

The tag must be the version of Cargo.toml (and so of the Python isnady), CHANGELOG.md must have its section, and
PARITY.md must exist. With --notes, the CHANGELOG section is written to that file. Used by the Release workflow.
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def main() -> int:
    args = sys.argv[1:]
    if not args or not re.fullmatch(r"v\d+\.\d+\.\d+", args[0]):
        print(__doc__)
        return 2
    version = args[0][1:]
    cargo = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
    have = re.search(r'^\[workspace\.package\]\s*\nversion = "([^"]+)"', cargo, re.M).group(1)
    ok = True
    if have != version:
        print(f"✗ Cargo.toml says {have}, the tag {version}")
        ok = False
    else:
        print(f"Cargo.toml is {version}")
    log = (ROOT / "CHANGELOG.md").read_text(encoding="utf-8")
    m = re.search(rf"^## \[{re.escape(version)}\][^\n]*\n(.*?)(?=^## \[|\Z)", log, re.M | re.S)
    if not m:
        print(f"✗ CHANGELOG.md has no section for {version}")
        ok = False
    else:
        print(f"CHANGELOG.md has the {version} section")
    if not (ROOT / "PARITY.md").exists():
        print("✗ PARITY.md is missing")
        ok = False
    if ok and "--notes" in args:
        out = Path(args[args.index("--notes") + 1])
        out.write_text(m.group(1).strip() + "\n\nThe same version as the Python isnady: "
                       f"https://github.com/bayramkotan/isnady/releases/tag/v{version}\n", encoding="utf-8")
        print(f"Release notes written to {out}")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
