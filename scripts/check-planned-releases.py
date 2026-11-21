#!/usr/bin/env python3
"""Fails when a diagnostic names a release for a requirement that the SRS plans for another one.

A message such as "planned for TyPy 1.0 (GEN-16)", or a call such as
`planned("...", "1.0", RuleId::GEN_16)`, must name the release the requirement
has in the SRS, so moving a requirement to another release cannot leave a stale
message behind.
"""

from __future__ import annotations

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
SRS = ROOT / "docs" / "requirements" / "srs.md"
ROW = re.compile(r"^\| ([A-Z]+-\d+) \| (.*)$")
RELEASE = re.compile(r"^\d+\.\d+$")
MESSAGE = re.compile(r"planned for TyPy (\d+\.\d+)[^\"(]*\(([A-Z]+-\d+)\)")
CALL = re.compile(r"\"(\d+\.\d+)\",\s*RuleId::([A-Z]+)_(\d+)\b")


def releases() -> dict[str, str]:
    """Returns the release of each requirement of the SRS."""
    found = {}
    for line in SRS.read_text(encoding="utf-8").splitlines():
        match = ROW.match(line)
        if not match:
            continue
        cells = [cell.strip(" |") for cell in match.group(2).split(" | ")]
        for cell in cells[1:]:
            if RELEASE.match(cell):
                found[match.group(1)] = cell
                break
    return found


def sources() -> list[pathlib.Path]:
    """Returns the compiler's source files, leaving out tests."""
    return sorted(
        path
        for path in (ROOT / "crates").glob("*/src/**/*.rs")
        if "tests" not in path.parts and not path.name.endswith("tests.rs")
    )


def main() -> int:
    planned = releases()
    problems = []
    for path in sources():
        # A string continued with a backslash reads as one line.
        text = re.sub(r"\\\n\s*", "", path.read_text(encoding="utf-8"))
        named = [(m.group(1), m.group(2)) for m in MESSAGE.finditer(text)]
        named += [(m.group(1), f"{m.group(2)}-{m.group(3)}") for m in CALL.finditer(text)]
        for release, requirement in named:
            wanted = planned.get(requirement)
            if wanted != release:
                where = path.relative_to(ROOT)
                problems.append(f"{where}: {requirement} named for {release}, SRS: {wanted}")
    for problem in problems:
        print(f"check-planned-releases: {problem}", file=sys.stderr)
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
