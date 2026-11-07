#!/usr/bin/env python3
"""Checks the two size budgets of NFR-02 on each staged runtime archive.

The runtime is the archive member compiled from typy-runtime; libc and compiler_builtins are
not part of it. Its size is the code and data `size` reports for that member, unwind tables
included. The core is that size less the symbols of the large-integer module
(`typy_runtime::int::` and the exported `typy_rt_int_*` functions); cycle collection does not
exist yet, so nothing else is taken off.
"""

from __future__ import annotations

import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path

WHOLE_LIMIT = 48 * 1024
CORE_LIMIT = 32 * 1024
TRIPLES = ("x86_64-unknown-linux-gnu", "x86_64-unknown-linux-musl")
LARGE_INTEGER = re.compile(r"typy_runtime::int::|^typy_rt_int_")


def run(*command: str, cwd: Path) -> str:
    """Return the standard output of a command that must succeed."""
    return subprocess.run(command, cwd=cwd, check=True, capture_output=True, text=True).stdout


def measure(archive: Path) -> tuple[int, int]:
    """Return the runtime member's code and data size, and the large-integer part of it."""
    with tempfile.TemporaryDirectory() as scratch:
        work = Path(scratch)
        member = next(name for name in run("ar", "t", str(archive), cwd=work).split() if name.startswith("typy_runtime-"))
        run("ar", "x", str(archive), member, cwd=work)
        text, data = run("size", member, cwd=work).splitlines()[1].split()[:2]
        large = 0
        for line in run("nm", "-S", "-C", member, cwd=work).splitlines():
            parts = line.split(None, 3)
            if len(parts) == 4 and parts[2] in "TtRrDd" and LARGE_INTEGER.search(parts[3]):
                large += int(parts[1], 16)
        return int(text) + int(data), large


def main() -> int:
    """Measure each archive and fail when a budget is exceeded."""
    target = Path(os.environ.get("CARGO_TARGET_DIR", "target"))
    status = 0
    for triple in TRIPLES:
        archive = target / "lib/typy" / triple / "libtypy_runtime.a"
        if not archive.exists():
            print(f"check-runtime-size: {archive} is missing; run make runtime", file=sys.stderr)
            return 1
        whole, large = measure(archive)
        core = whole - large
        print(f"{triple}: runtime {whole} bytes of {WHOLE_LIMIT}, core {core} of {CORE_LIMIT}")
        if whole >= WHOLE_LIMIT or core >= CORE_LIMIT:
            print(f"check-runtime-size: {triple} is over the budget of NFR-02", file=sys.stderr)
            status = 1
    return status


if __name__ == "__main__":
    sys.exit(main())
