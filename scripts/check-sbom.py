#!/usr/bin/env python3
"""Fails when the bill of materials in a release archive misses a file, a crate or a library
of the archive, or lists a file with another SHA-256 (NFR-25)."""

from __future__ import annotations

import hashlib
import importlib.util
import json
import pathlib
import sys
import tarfile

HERE = pathlib.Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("write_sbom", HERE / "write-sbom.py")
assert spec is not None and spec.loader is not None
write_sbom = importlib.util.module_from_spec(spec)
spec.loader.exec_module(write_sbom)


def main() -> int:
    archive = pathlib.Path(sys.argv[1])
    problems = []
    with tarfile.open(archive) as tar:
        members = [member for member in tar.getmembers() if member.isfile()]
        root = members[0].name.split("/", 1)[0]
        held = {}
        bill = None
        for member in members:
            relative = member.name.split("/", 1)[1]
            data = tar.extractfile(member).read()
            if relative == write_sbom.BILL:
                bill = json.loads(data)
            else:
                held[relative] = hashlib.sha256(data).hexdigest()
    if bill is None:
        print(f"check-sbom: {archive} holds no {write_sbom.BILL}", file=sys.stderr)
        return 1
    listed_files = {
        entry["name"]: entry["hashes"][0]["content"]
        for entry in bill["components"]
        if entry["type"] == "file"
    }
    for relative, digest in sorted(held.items()):
        if listed_files.get(relative) != digest:
            problems.append(f"the file {root}/{relative} is missing or has another SHA-256")
    for relative in sorted(set(listed_files) - set(held)):
        problems.append(f"the bill lists {relative}, which the archive does not hold")
    listed = {(entry["name"], entry["version"]) for entry in bill["components"] if entry["type"] == "library"}
    linked = write_sbom.crates("typy-cli") | write_sbom.crates("typy-runtime")
    for name, version, _, _ in sorted(linked):
        if (name, version) not in listed:
            problems.append(f"the crate {name} {version} is missing")
    for library in ("llvm", "rust-std"):
        if not any(name == library for name, _ in listed):
            problems.append(f"the library {library} is missing")
    for problem in problems:
        print(f"check-sbom: {problem}", file=sys.stderr)
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
