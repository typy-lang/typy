#!/usr/bin/env python3.15
"""Writes the runtime's Unicode tables from the unicodedata of the newest oracle (OI-15).

With --check, it fails when the committed tables differ from the ones it would write.
"""

from __future__ import annotations

import sys
import unicodedata
from pathlib import Path

OUTPUT = Path(__file__).resolve().parent.parent / "crates/typy-runtime/src/string/unicode/tables.rs"
LAST_CODE_POINT = 0x10FFFF
PER_LINE = 8


def printable_boundaries() -> list[int]:
    """Return the code points where str.isprintable changes, starting from not printable at 0."""
    boundaries: list[int] = []
    previous = False
    for point in range(LAST_CODE_POINT + 1):
        printable = chr(point).isprintable()
        if printable != previous:
            boundaries.append(point)
            previous = printable
    return boundaries


def decimal_zeros() -> list[int]:
    """Return the code point of each decimal digit zero; each starts a run of digits 0 to 9."""
    zeros = [
        point
        for point in range(LAST_CODE_POINT + 1)
        if unicodedata.decimal(chr(point), None) == 0
    ]
    for zero in zeros:
        for value in range(10):
            if unicodedata.decimal(chr(zero + value), None) != value:
                sys.exit(f"decimal digits from U+{zero:04X} do not run from 0 to 9")
    count = sum(
        1 for point in range(LAST_CODE_POINT + 1) if unicodedata.decimal(chr(point), None) is not None
    )
    if count != 10 * len(zeros):
        sys.exit("a decimal digit lies outside the runs that start at a zero")
    return zeros


def array(name: str, element: str, values: list[int], comment: str) -> str:
    """Return a Rust constant array of `values`, eight to a line."""
    lines = [f"/// {comment}", f"pub(super) const {name}: [{element}; {len(values)}] = ["]
    for start in range(0, len(values), PER_LINE):
        chunk = values[start : start + PER_LINE]
        lines.append("    " + " ".join(f"0x{value:04x}," for value in chunk))
    lines.append("];")
    return "\n".join(lines)


def render() -> str:
    """Return the text of the tables module."""
    boundaries = printable_boundaries()
    basic = [point for point in boundaries if point <= 0xFFFF]
    supplementary = [point for point in boundaries if point > 0xFFFF]
    parts = [
        "//! Unicode tables for `repr`, `int()` and `float()`, written by",
        "//! scripts/generate-unicode-tables.py from the unicodedata of CPython"
        f" {sys.version_info.major}.{sys.version_info.minor},",
        f"//! Unicode {unicodedata.unidata_version} (OI-15). Do not edit; run `make unicode-tables`.",
        "",
        array(
            "PRINTABLE_BASIC",
            "u16",
            basic,
            "Code points up to U+FFFF where `str.isprintable` changes; U+0000 is not printable.",
        ),
        "",
        array(
            "PRINTABLE_SUPPLEMENTARY",
            "u32",
            supplementary,
            "Code points above U+FFFF where `str.isprintable` changes, continuing the list above.",
        ),
        "",
        array(
            "DECIMAL_ZEROS",
            "u32",
            decimal_zeros(),
            "Each decimal digit zero; the digits 1 to 9 follow it.",
        ),
        "",
    ]
    return "\n".join(parts)


def main() -> int:
    """Write the tables, or with --check compare them with the committed file."""
    text = render()
    if "--check" in sys.argv[1:]:
        if OUTPUT.read_text(encoding="ascii") != text:
            print(f"{OUTPUT} is out of date; run make unicode-tables", file=sys.stderr)
            return 1
        return 0
    OUTPUT.write_text(text, encoding="ascii")
    return 0


if __name__ == "__main__":
    sys.exit(main())
