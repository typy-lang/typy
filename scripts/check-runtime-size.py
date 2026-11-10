#!/usr/bin/env python3
"""Checks the two size budgets of NFR-02 on the runtime that a program links.

For each target, a hello world is built with `--release-small` and linked with its runtime
archive, garbage-collected sections and a link map, as `typy build` links it. The runtime's
size is the code and data the map places in the executable from the runtime's own object;
libc and compiler_builtins are not part of it. The core is that size less the large-integer
sections (`typy_runtime::int` and the exported `typy_rt_int_*` functions); cycle collection
does not exist yet, so nothing else is taken off. Code a program does not use is never
linked, so the runtime may grow with the language while programs stay small.
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
LINKERS = {
    "x86_64-unknown-linux-gnu": ["cc"],
    "x86_64-unknown-linux-musl": ["musl-gcc", "-static"],
}
LARGE_INTEGER = re.compile(r"12typy_runtime3int|typy_rt_int_")
SECTION = re.compile(r"^ (\.(?:text|rodata|data|bss)\S*)(?:\s+0x[0-9a-f]+\s+0x([0-9a-f]+)\s+(\S+))?$")
CONTINUATION = re.compile(r"^\s+0x[0-9a-f]+\s+0x([0-9a-f]+)\s+(\S+)$")


def run(command: list[str], cwd: Path) -> None:
    """Run a command that must succeed."""
    subprocess.run(command, cwd=cwd, check=True, capture_output=True, text=True)


def linked_sizes(link_map: Path) -> tuple[int, int]:
    """Return the runtime's linked code and data, and the large-integer part of it."""
    whole = large = 0
    name = None
    # The map lists the discarded sections first; only what follows was linked.
    text = link_map.read_text()
    placed = text[text.index("Linker script and memory map"):]
    for line in placed.splitlines():
        size = owner = None
        if match := SECTION.match(line):
            name, size, owner = match.groups()
        elif name and (match := CONTINUATION.match(line)):
            size, owner = match.groups()
        else:
            name = None if line.strip() == "" else name
            continue
        if size is None or "(typy_runtime-" not in owner:
            continue
        whole += int(size, 16)
        if LARGE_INTEGER.search(name):
            large += int(size, 16)
        name = None
    return whole, large


def measure(compiler: Path, archive: Path, linker: list[str]) -> tuple[int, int]:
    """Build and link a hello world against `archive` and measure its runtime."""
    with tempfile.TemporaryDirectory() as scratch:
        work = Path(scratch)
        (work / "hello.py").write_text('print("Hello, world!")\n')
        run([str(compiler), "build", "hello.py", "--release-small", "--emit=obj", "-o", "hello.o"], work)
        run([*linker, "-o", "hello", "hello.o", str(archive), "-lm", "-Wl,--gc-sections",
             "-Wl,-Map,hello.map"], work)
        return linked_sizes(work / "hello.map")


def main() -> int:
    """Measure each target and fail when a budget is exceeded."""
    target = Path(os.environ.get("CARGO_TARGET_DIR", "target")).resolve()
    compiler = target / "debug" / "typy"
    if not compiler.exists():
        print(f"check-runtime-size: {compiler} is missing; run make build", file=sys.stderr)
        return 1
    status = 0
    for triple, linker in LINKERS.items():
        archive = target / "lib/typy" / triple / "libtypy_runtime.a"
        if not archive.exists():
            print(f"check-runtime-size: {archive} is missing; run make runtime", file=sys.stderr)
            return 1
        whole, large = measure(compiler, archive, linker)
        core = whole - large
        print(f"{triple}: runtime linked into hello world {whole} bytes of {WHOLE_LIMIT}, "
              f"core {core} of {CORE_LIMIT}")
        if whole >= WHOLE_LIMIT or core >= CORE_LIMIT:
            print(f"check-runtime-size: {triple} is over the budget of NFR-02", file=sys.stderr)
            status = 1
    return status


if __name__ == "__main__":
    sys.exit(main())
