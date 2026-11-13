"""Set isnady-rs's version — always the number the Python isnady has (Bayram, 2026-11-03: "sürüm numaraları birebir
aynı olsun"):

    python tools/bump_version.py 0.7.5

Writes Cargo.toml (the workspace's version), Cargo.lock (the two crates of the workspace, so `cargo build --locked`
agrees), the version in PARITY.md and in the README's head, then the README again (tools/sync_readme.py, from
../isnady/README.md); CHANGELOG.md must already have the section of that version.
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
    text = re.sub(r'(^\[workspace\.package\]\s*\nversion = ")[^"]+(")', rf"\g<1>{new}\g<2>", text, flags=re.M)
    text = re.sub(r'(isnady-core = \{ path = "crates/isnady-core", version = "=)[^"]+(")', rf"\g<1>{new}\g<2>", text)
    cargo.write_text(text, encoding="utf-8", newline="\n")
    lock = ROOT / "Cargo.lock"
    if lock.exists():
        t = lock.read_text(encoding="utf-8")
        for crate in ("isnady-core", "isnady", "isnady-gui"):
            t = re.sub(rf'(name = "{crate}"\nversion = ")[^"]+(")', rf"\g<1>{new}\g<2>", t)
        lock.write_text(t, encoding="utf-8", newline="\n")
    # the places that state the current version in PARITY.md and in the README's own head — "Version X", "isnady X
    # (Rust)", the badge's "-vX-" — and no others: a row that says when something came ("DB-READY, 0.7.5") keeps its
    # number (0.7.6; before, every mention was changed). Then the README is written again from the head and isnady's.
    o = re.escape(old)
    for name in ("PARITY.md", "tools/readme_head.md"):
        path = ROOT / name
        text = path.read_text(encoding="utf-8")
        text = re.sub(rf"(Version ){o}(?!\d)", rf"\g<1>{new}", text)
        text = re.sub(rf"(isnady ){o}( \(Rust\))", rf"\g<1>{new}\g<2>", text)
        text = re.sub(rf"(-v){o}(-)", rf"\g<1>{new}\g<2>", text)
        path.write_text(text, encoding="utf-8", newline="\n")
    import subprocess

    subprocess.run([sys.executable, str(ROOT / "tools" / "sync_readme.py")], check=False)
    has = f"## [{new}]" in (ROOT / "CHANGELOG.md").read_text(encoding="utf-8")
    print(f"{old} → {new}: Cargo.toml, Cargo.lock, README, PARITY; CHANGELOG {'already has' if has else 'NEEDS'} [{new}]")
    return 0 if has else 1


if __name__ == "__main__":
    sys.exit(main())
