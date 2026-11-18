"""Deletes the build outputs Cargo keeps but no build uses any more.

Cargo writes a new copy of a crate's library or a test program whenever its code,
version or flags change, and keeps every old copy; a test program that links LLVM is
about 200 MB, so the copies fill the disk. In each deps directory this keeps the newest
copy of each file and any copy rebuilt in the last 12 hours, so two variants in use on
the same day do not rebuild each other, and deletes the rest; it deletes incremental
caches untouched for 12 hours too.
"""

from __future__ import annotations

import os
import re
import shutil
import sys
import time
from pathlib import Path

__all__ = ["main"]

#: Seconds since its last build after which an older copy is deleted.
AGE = 12 * 3600

#: A file Cargo names `name-<16 hex digits>`, with an extension or none.
HASHED = re.compile(r"^(?P<name>.+)-[0-9a-f]{16}(?P<extension>\..*)?$")


def sweep_deps(directory: Path, now: float) -> int:
    """Delete the stale copies in one deps directory and return the bytes freed."""
    groups: dict[tuple[str, str], list[Path]] = {}
    for entry in directory.iterdir():
        match = HASHED.match(entry.name)
        if match and entry.is_file():
            key = (match["name"], match["extension"] or "")
            groups.setdefault(key, []).append(entry)
    freed = 0
    for copies in groups.values():
        copies.sort(key=lambda copy: copy.stat().st_mtime, reverse=True)
        for copy in copies[1:]:
            status = copy.stat()
            if now - status.st_mtime > AGE:
                freed += status.st_size
                copy.unlink()
    return freed


def size(path: Path) -> int:
    """Return the bytes of the files under `path`."""
    return sum(entry.stat().st_size for entry in path.rglob("*") if entry.is_file())


def sweep_incremental(directory: Path, now: float) -> int:
    """Delete the caches of one incremental directory untouched for `AGE` seconds."""
    freed = 0
    for cache in directory.iterdir():
        if cache.is_dir() and now - cache.stat().st_mtime > AGE:
            freed += size(cache)
            shutil.rmtree(cache)
    return freed


def main() -> int:
    """Sweep every deps and incremental directory of the build directory."""
    target = Path(os.environ.get("CARGO_TARGET_DIR", "target"))
    if not target.is_dir():
        return 0
    now = time.time()
    freed = 0
    for directory in target.rglob("*"):
        if directory.name == "deps" and directory.is_dir():
            freed += sweep_deps(directory, now)
        elif directory.name == "incremental" and directory.is_dir():
            freed += sweep_incremental(directory, now)
    print(f"sweep: freed {freed / 1e9:.1f} GB of stale build outputs in {target}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
