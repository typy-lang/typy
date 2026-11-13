#!/usr/bin/env python3.15
"""Writes the runtime's table of glibc's error texts, which an `OSError` shows (ADR-0019).

The oracle runs on glibc, so a program built for musl shows glibc's words too, as the
oracle does. The table holds `os.strerror(n)` for each number from 1 to the last one glibc
names. With --check, it fails when the committed table differs from the one it would write.
"""

from __future__ import annotations

import errno
import os
import sys
from pathlib import Path

OUTPUT = Path(__file__).resolve().parent.parent / "crates/typy-runtime/src/text_file/errno_text.rs"


def table() -> str:
    last = max(errno.errorcode)
    lines = "\n".join(f'    "{os.strerror(number)}",' for number in range(1, last + 1))
    return (
        "//! The texts glibc's `strerror` gives each error number, written by\n"
        "//! `scripts/generate-errno-texts.py` (ADR-0019).\n\n"
        "/// The text of error number `n` is entry `n - 1`.\n"
        "#[rustfmt::skip]\n"
        f"pub(crate) const ERRNO_TEXTS: [&str; {last}] = [\n"
        + lines
        + "\n];\n"
    )


def main() -> int:
    source = table()
    if "--check" in sys.argv[1:]:
        if OUTPUT.read_text() != source:
            print(f"{OUTPUT} differs from what glibc gives; run make errno-texts")
            return 1
        return 0
    OUTPUT.write_text(source)
    return 0


if __name__ == "__main__":
    sys.exit(main())
