#!/usr/bin/env python3.15
"""Writes the names of CPython's standard library modules for the module loader (LIB-08).

The table is the union of `sys.stdlib_module_names` of CPython 3.12, 3.13, 3.14 and 3.15, so
a module any oracle version ships counts as standard library: the loader leaves an import
of one it cannot find to the checker, which says whether TyPy provides it. With --check, it
fails when the committed table differs from the one it would write.
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

OUTPUT = Path(__file__).resolve().parent.parent / "crates/typy-source/src/stdlib_names.rs"
ORACLES = ("3.12", "3.13", "3.14", "3.15")
NAMES = "import sys; print('\\n'.join(sorted(sys.stdlib_module_names)))"


def table() -> str:
    names: set[str] = set()
    for version in ORACLES:
        output = subprocess.run(
            [f"python{version}", "-I", "-c", NAMES], check=True, capture_output=True, text=True
        ).stdout
        names.update(output.split())
    lines = "\n".join(f'    "{name}",' for name in sorted(names))
    return (
        "//! The names of CPython's standard library modules, the union of\n"
        "//! `sys.stdlib_module_names` of CPython 3.12 to 3.15, written by\n"
        "//! `scripts/generate-stdlib-names.py` (LIB-08).\n\n"
        "/// Every top-level standard library module name, sorted.\n"
        "#[rustfmt::skip]\n"
        f"pub(crate) const STDLIB_NAMES: [&str; {len(names)}] = [\n"
        + lines
        + "\n];\n"
    )


def main() -> int:
    source = table()
    if "--check" in sys.argv[1:]:
        if OUTPUT.read_text() != source:
            print(f"{OUTPUT} differs from what CPython gives; run make stdlib-names")
            return 1
        return 0
    OUTPUT.write_text(source)
    return 0


if __name__ == "__main__":
    sys.exit(main())
