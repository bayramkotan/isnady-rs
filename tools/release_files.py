"""Gather what one system's release build made under the names of the GitHub Release (Release workflow):

    python tools/release_files.py linux-x86_64|windows-x86_64|macos-arm64|macos-x86_64 OUT

after `cargo build --release -p isnady` (iy-rs) and `cargo tauri build` in crates/isnady-gui (the window):

    iy-rs-V-<system>.tar.gz / .zip          iy-rs and isnady-rs with README, PARITY and LICENSE: unpack and run
    isnady-rs-V-linux-x86_64.AppImage       the window, portable: make it executable and run it
    isnady-rs-V-linux-amd64.deb             the window, installed (sudo apt install ./…deb)
    isnady-rs-V-windows-x86_64-setup.exe    the window's installer (for this user, no administrator rights)
    isnady-rs-V-windows-x86_64.msi          the same as an .msi (for all users, or for deployment)
    isnady-rs-V-windows-x86_64-portable.exe the window, portable: one file, nothing installed
    isnady-rs-V-macos-<arch>.dmg            the window: open it and drag isnady-rs to Applications
    isnady-rs-V-macos-<arch>.zip            isnady-rs.app zipped, portable

A folder named isnady-data beside a portable copy (the AppImage, the .exe, the .app) keeps every database and
setting there, as in the Python isnady's portable applications.
"""

import shutil
import subprocess
import sys
import tarfile
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RELEASE = ROOT / "target" / "release"
BUNDLE = RELEASE / "bundle"
DOCS = ("README.md", "PARITY.md", "LICENSE")


def version() -> str:
    for line in (ROOT / "Cargo.toml").read_text(encoding="utf-8").splitlines():
        if line.startswith("version = "):
            return line.split('"')[1]
    raise SystemExit("no version in Cargo.toml")


def one(pattern: str) -> Path:
    """The one file a bundler wrote: none or several is an error, not a guess."""
    found = sorted(BUNDLE.glob(pattern))
    if len(found) != 1:
        raise SystemExit(f"expected one {pattern} in {BUNDLE}, found {[p.name for p in found]}")
    return found[0]


def put(src: Path, out: Path, name: str) -> None:
    target = out / name
    shutil.copy2(src, target)
    print(f"{name}  ({target.stat().st_size / 1e6:.1f} MB)")


def cli_archive(system: str, v: str, out: Path) -> None:
    windows = system.startswith("windows")
    ext = ".exe" if windows else ""
    folder = f"iy-rs-{v}-{system}"
    files = [(RELEASE / f"iy-rs{ext}", f"iy-rs{ext}"), (RELEASE / f"isnady-rs{ext}", f"isnady-rs{ext}")]
    files += [(ROOT / d, d) for d in DOCS]
    if windows:
        target = out / f"{folder}.zip"
        with zipfile.ZipFile(target, "w", zipfile.ZIP_DEFLATED) as z:
            for src, name in files:
                z.write(src, f"{folder}/{name}")
    else:
        target = out / f"{folder}.tar.gz"
        with tarfile.open(target, "w:gz") as t:
            for src, name in files:
                info = t.gettarinfo(str(src), f"{folder}/{name}")
                info.mode = 0o755 if name in ("iy-rs", "isnady-rs") else 0o644
                with open(src, "rb") as f:
                    t.addfile(info, f)
    print(f"{target.name}  ({target.stat().st_size / 1e6:.1f} MB)")


def main() -> int:
    if len(sys.argv) != 3:
        print(__doc__)
        return 2
    system, out = sys.argv[1], Path(sys.argv[2])
    out.mkdir(parents=True, exist_ok=True)
    v = version()
    cli_archive(system, v, out)
    base = f"isnady-rs-{v}"
    if system == "linux-x86_64":
        image = one("appimage/*.AppImage")
        put(image, out, f"{base}-linux-x86_64.AppImage")
        (out / f"{base}-linux-x86_64.AppImage").chmod(0o755)
        put(one("deb/*.deb"), out, f"{base}-linux-amd64.deb")
    elif system == "windows-x86_64":
        put(one("nsis/*-setup.exe"), out, f"{base}-windows-x86_64-setup.exe")
        put(one("msi/*.msi"), out, f"{base}-windows-x86_64.msi")
        put(RELEASE / "isnady-rs.exe", out, f"{base}-windows-x86_64-portable.exe")
    elif system.startswith("macos-"):
        arch = system.split("-", 1)[1]
        put(one("dmg/*.dmg"), out, f"{base}-macos-{arch}.dmg")
        app = one("macos/*.app")
        target = out / f"{base}-macos-{arch}.zip"
        # ditto keeps what a .app needs (its permissions and links) the way Finder's own zipping does
        subprocess.run(["ditto", "-c", "-k", "--sequesterRsrc", "--keepParent", str(app), str(target)], check=True)
        print(f"{target.name}  ({target.stat().st_size / 1e6:.1f} MB)")
    else:
        print(f"unknown system {system}")
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
