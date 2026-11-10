#!/usr/bin/env python3.15
"""Writes the runtime's table of format() results from every oracle version (LEX-07).

Each case formats one value with one format specification under CPython 3.12, 3.13, 3.14
and 3.15, and keeps the text, or the exception's type and message, when all four agree, so
the runtime's formatter is tested against every version a program may be judged by. With
--check, it fails when the committed table differs from the one it would write.
"""

from __future__ import annotations

import json
import struct
import subprocess
import sys
from pathlib import Path

OUTPUT = (
    Path(__file__).resolve().parent.parent
    / "crates/typy-runtime/src/format/tests/cases.rs"
)
ORACLES = ("3.12", "3.13", "3.14", "3.15")

# Runs under each oracle and prints [kind, repr(value), spec, [outcome, text]] for each case.
CASES = r'''
import json, sys
ints = [0, 1, -1, 7, 42, -42, 255, 1234, -1234, 1234567, -9876543210, 10**20, -(10**25), 65,
        0x110000]
floats = [0.0, -0.0, 1.0, -1.0, 0.5, 1.5, 2.5, 0.125, 3.14159, -2.71828, 1234.5678, 1e16,
          1e17, 123456789.0, 1e-5, 0.0001, 1.23e-10, 9.995, 0.05, 1e300, 5e-324,
          1.7976931348623157e308, float("inf"), float("-inf"), float("nan"), 100.0, 99.99999,
          0.1, 2.0**60, 12345.0]
strs = ["", "a", "abc", "h\u00e9llo", "\u65e5\u672c"]
specs = ["", "d", "5", "<5", ">5", "^5", "^6", "=8", "*^9", "05", "08", "+", "-", " ", "+5",
         "#x", "#X", "#o", "#b", "x", "X", "o", "b", "c", ",", "_", ",d", "_x", "_b", "010,",
         "08,", "0=10,", "+010,", "e", "E", ".2e", ".0e", "f", "F", ".2f", ".0f", "#.0f", "g",
         "G", ".3g", ".10g", "#g", "#.3g", "%", ".1%", ".2", ".3", ".0", "n", "z.1f", "z", "s",
         ".3s", "10.2s", ">10s", "x<6", "\u00e9^7", ",.2f", "015,.3f", "+.3e", " .2f", "12",
         ",x", ",b", ",s", "_s", "#s", "+s", "=5s", ".2d", "zd", "abc", "5.", ".f", "#c", "+c",
         "q", ",_", "_,", "30f", ".30f", ".17g", ".20e", "r"]
def run(value, spec):
    try:
        return ["ok", format(value, spec)]
    except Exception as error:
        return [type(error).__name__, str(error)]
out = []
for kind, values in (("int", ints), ("bool", [True, False]), ("float", floats), ("str", strs)):
    for value in values:
        for spec in specs:
            out.append([kind, repr(value), spec, run(value, spec)])
json.dump(out, sys.stdout)
'''


def rust_string(text: str) -> str:
    """Return a Rust string literal of `text`, ASCII only."""
    escaped = []
    for character in text:
        if character in '"\\':
            escaped.append("\\" + character)
        elif 32 <= ord(character) < 127:
            escaped.append(character)
        else:
            escaped.append("\\u{%x}" % ord(character))
    return '"' + "".join(escaped) + '"'


def rust_value(kind: str, value: str) -> str:
    """Return the Rust `Value` of a case's value."""
    if kind == "str":
        return "Value::Str(%s)" % rust_string(eval(value))  # noqa: S307 - our own repr output
    if kind == "bool":
        return "Value::Bool(%s)" % ("true" if value == "True" else "false")
    if kind == "int":
        return "Value::Int(%s)" % rust_string(value)
    number = float(value)
    bits = "%016x" % struct.unpack("<Q", struct.pack("<d", number))[0]
    return "Value::Float(0x%s)" % "_".join(bits[index : index + 4] for index in range(0, 16, 4))


def table() -> str:
    """Return the Rust source of the cases every oracle agrees on."""
    results = [
        json.loads(subprocess.run([f"python{version}", "-c", CASES], check=True,
                                  capture_output=True, text=True).stdout)
        for version in ORACLES
    ]
    lines = []
    for index, case in enumerate(results[0]):
        if any(result[index] != case for result in results[1:]):
            continue
        kind, value, spec, (outcome, text) = case
        lines.append(
            "    (%s, %s, %s, %s),"
            % (rust_value(kind, value), rust_string(spec), rust_string(outcome), rust_string(text))
        )
    return (
        "//! The results CPython 3.12, 3.13, 3.14 and 3.15 all give for `format(value, spec)`,\n"
        "//! written by `scripts/generate-format-cases.py`: the text, or the exception's type and\n"
        "//! message.\n\n"
        "use super::Value;\n\n"
        "/// Each case: the value, the specification, `ok` or the exception's type, and the text\n"
        "/// or the message.\n"
        "#[rustfmt::skip]\n"
        "pub(super) const CASES: &[(Value, &str, &str, &str)] = &[\n"
        + "\n".join(lines)
        + "\n];\n"
    )


def main() -> int:
    source = table()
    if "--check" in sys.argv[1:]:
        if OUTPUT.read_text() != source:
            print(f"{OUTPUT} differs from what CPython gives; run make format-cases")
            return 1
        return 0
    OUTPUT.write_text(source)
    return 0


if __name__ == "__main__":
    sys.exit(main())
