#!/usr/bin/env python3.15
"""Writes the runtime's Unicode tables from the unicodedata of the newest oracle (OI-15).

Each property and case mapping is read off CPython 3.15 itself, one code point at a time, so
the runtime gives the results CPython gives. With --check, it fails when the committed tables
differ from the ones it would write.
"""

from __future__ import annotations

import sys
import unicodedata
from pathlib import Path
from typing import Callable

UNICODE = Path(__file__).resolve().parent.parent / "crates/typy-runtime/src/string/unicode"
OUTPUT = UNICODE / "tables.rs"
CASE_OUTPUT = UNICODE / "case_tables.rs"
SIGMA = "\u03a3"
FINAL_SIGMA = "\u03c2"
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


def boundaries(test: Callable[[str], bool]) -> list[int]:
    """Return the code points where `test` of the one-character string changes, from false."""
    found: list[int] = []
    previous = False
    for point in range(LAST_CODE_POINT + 1):
        value = test(chr(point))
        if value != previous:
            found.append(point)
            previous = value
    return found


def case_ignorable(character: str) -> bool:
    """Return whether `str.lower` skips the character when it looks for a cased neighbour of a
    capital sigma: with it on either side, the sigma still ends a word."""
    before = ("A" + character + SIGMA).lower()[-1] == FINAL_SIGMA
    after = ("A" + SIGMA + character).lower()[1] == FINAL_SIGMA
    return before and after


def cased(character: str) -> bool:
    """Return whether the character is cased: `str.title` lowercases the letter after it."""
    return (character + "a").title()[-1] == "a"


def is_titlecase(character: str) -> bool:
    """Return whether the character is titlecase, category Lt."""
    return unicodedata.category(character) == "Lt"


def continues_identifier(character: str) -> bool:
    """Return whether the character may follow the first of an identifier."""
    return ("a" + character).isidentifier()


PROPERTIES = [
    ("ALPHA", "`str.isalpha`", str.isalpha),
    ("DECIMAL", "`str.isdecimal`", str.isdecimal),
    ("DIGIT", "`str.isdigit`", str.isdigit),
    ("NUMERIC", "`str.isnumeric`", str.isnumeric),
    ("SPACE", "`str.isspace`", str.isspace),
    ("UPPER", "an uppercase code point, as `str.isupper` tests it", str.isupper),
    ("LOWER", "a lowercase code point, as `str.islower` tests it", str.islower),
    ("TITLE", "a titlecase code point, category Lt", is_titlecase),
    ("IDENTIFIER_START", "a code point that may start an identifier", str.isidentifier),
    ("IDENTIFIER_CONTINUE", "a code point that may continue an identifier", continues_identifier),
    ("CASED", "a cased code point", cased),
    ("CASE_IGNORABLE", "a case-ignorable code point", case_ignorable),
]

MAPPINGS = [
    ("UPPER", "`str.upper`", str.upper),
    ("LOWER", "`str.lower`, a capital sigma aside", str.lower),
    ("TITLE", "`str.title` of one code point", str.title),
    ("FOLD", "`str.casefold`", str.casefold),
]


Run = tuple[int, int, int, int]
Multiple = tuple[int, list[int]]


def case_runs(mapping: Callable[[str], str]) -> tuple[list[Run], list[Multiple]]:
    """Return the one-to-one mappings as runs, each a first and last code point, a stride of one
    or two and the difference added, and the mappings to more than one code point."""
    runs: list[Run] = []
    multiple: list[Multiple] = []
    for point in range(LAST_CODE_POINT + 1):
        mapped = mapping(chr(point))
        if mapped == chr(point):
            continue
        if len(mapped) > 1:
            multiple.append((point, [ord(each) for each in mapped]))
            continue
        delta = ord(mapped) - point
        if runs:
            first, last, stride, previous = runs[-1]
            step = point - last
            if previous == delta and (stride == step or (first == last and step in (1, 2))):
                runs[-1] = (first, point, step, delta)
                continue
        runs.append((point, point, 1, delta))
    return runs, multiple


def render_case() -> str:
    """Return the text of the case tables module."""
    parts = [
        "//! Unicode properties and case mappings for the methods of `str`, written by",
        "//! scripts/generate-unicode-tables.py from CPython"
        f" {sys.version_info.major}.{sys.version_info.minor},",
        f"//! Unicode {unicodedata.unidata_version} (OI-15). Do not edit;"
        " run `make unicode-tables`.",
        "",
    ]
    for name, meaning, test in PROPERTIES:
        parts.append(
            array(
                name,
                "u32",
                boundaries(test),
                f"Code points where being {meaning} changes; U+0000 is not."
                if not meaning.startswith("`")
                else f"Code points where {meaning} of one code point changes; U+0000 gives false.",
            )
        )
        parts.append("")
    for name, meaning, mapping in MAPPINGS:
        runs, multiple = case_runs(mapping)
        parts.append(f"/// Runs of {meaning}: first, last, stride and the difference added.")
        parts.append(f"pub(super) const {name}_RUNS: [(u32, u32, u32, i32); {len(runs)}] = [")
        for first, last, stride, delta in runs:
            parts.append(f"    (0x{first:04x}, 0x{last:04x}, {stride}, {delta}),")
        parts.append("];")
        parts.append("")
        parts.append(
            f"/// The code points {meaning} maps to more than one code point, padded with 0."
        )
        parts.append(f"pub(super) const {name}_MULTIPLE: [(u32, [u32; 3]); {len(multiple)}] = [")
        for point, mapped in multiple:
            padded = mapped + [0] * (3 - len(mapped))
            parts.append(f"    (0x{point:04x}, [{', '.join(f'0x{each:04x}' for each in padded)}]),")
        parts.append("];")
        parts.append("")
    return "\n".join(parts)


def main() -> int:
    """Write the tables, or with --check compare them with the committed files."""
    outputs = [(OUTPUT, render()), (CASE_OUTPUT, render_case())]
    if "--check" in sys.argv[1:]:
        stale = [
            path
            for path, text in outputs
            if not path.exists() or path.read_text(encoding="ascii") != text
        ]
        for path in stale:
            print(f"{path} is out of date; run make unicode-tables", file=sys.stderr)
        return 1 if stale else 0
    for path, text in outputs:
        path.write_text(text, encoding="ascii")
    return 0


if __name__ == "__main__":
    sys.exit(main())
