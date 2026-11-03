"""Set isnady-rs's version — always the number the Python isnady has (Bayram, 2026-11-03: "sürüm numaraları birebir
aynı olsun"):

    python tools/bump_version.py 0.7.5

Writes Cargo.toml (the workspace's version), Cargo.lock (the two crates of the workspace, so `cargo build --locked`
agrees) and the version in README.md and PARITY.md; CHANGELOG.md must already have the section of that version.
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def main() -> int:
    if len(sys.argv) != 2 or not re.fullmatch(r"\d+\.\d+\.\d+", sys.argv[1]):
        print(__doc__)
        return 2
    new = sys.argv[1]
    cargo = ROOT / "Cargo.toml"
    text = cargo.read_text(encoding="utf-8")
    old = re.search(r'^\[workspace\.package\]\s*\nversion = "([^"]+)"', text, re.M).group(1)
    cargo.write_text(re.sub(r'(^\[workspace\.package\]\s*\nversion = ")[^"]+(")', rf"\g<1>{new}\g<2>", text,
                            flags=re.M), encoding="utf-8", newline="\n")
    lock = ROOT / "Cargo.lock"
    if lock.exists():
        t = lock.read_text(encoding="utf-8")
        for crate in ("isnady-core", "isnady-cli"):
            t = re.sub(rf'(name = "{crate}"\nversion = ")[^"]+(")', rf"\g<1>{new}\g<2>", t)
        lock.write_text(t, encoding="utf-8", newline="\n")
    for name in ("README.md", "PARITY.md"):        # every mention of the version: it is the current one
        path = ROOT / name
        path.write_text(path.read_text(encoding="utf-8").replace(old, new), encoding="utf-8", newline="\n")
    has = f"## [{new}]" in (ROOT / "CHANGELOG.md").read_text(encoding="utf-8")
    print(f"{old} → {new}: Cargo.toml, Cargo.lock, README, PARITY; CHANGELOG {'already has' if has else 'NEEDS'} [{new}]")
    return 0 if has else 1


if __name__ == "__main__":
    sys.exit(main())
