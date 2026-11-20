#!/usr/bin/env python3
"""Writes the software bill of materials of a release archive's directory (NFR-25).

The bill is CycloneDX 1.6 JSON. It lists each file the archive holds with its
SHA-256, each crate compiled into the compiler or the runtime with its licence,
LLVM, which the compiler holds, and the Rust standard library. Its timestamp is
the commit's, so the same commit gives the same bill (NFR-06).
"""

from __future__ import annotations

import hashlib
import json
import os
import pathlib
import re
import subprocess
import sys

BILL = "sbom.cdx.json"
SPEC = re.compile(r"^(\S+) v(\S+)(?: \((.*)\))?$")


def run(*command: str) -> str:
    """Returns what `command` prints, or stops the script when it fails."""
    return subprocess.run(command, capture_output=True, text=True, check=True).stdout


def crates(package: str) -> set[tuple[str, str, str, bool]]:
    """Returns each crate `package` links, with its version, its licence, and whether it is
    one of TyPy's own; procedural macros run while compiling and are left out."""
    listed = run(
        "cargo", "tree", "-p", package, "-e", "normal,no-proc-macro", "--prefix", "none",
        "--format", "{p}|{l}", "--locked", "--offline",
    )
    found = set()
    for line in listed.splitlines():
        spec, licence = line.removesuffix(" (*)").split("|", 1)
        match = SPEC.match(spec)
        if match is None:
            sys.exit(f"write-sbom: cannot read the crate {spec!r}")
        name, version, source = match.groups()
        found.add((name, version, licence, source is not None and "/" in source))
    return found


def component(name: str, version: str, licence: str, purl: str | None) -> dict[str, object]:
    """Returns the CycloneDX component of a library."""
    entry: dict[str, object] = {
        "type": "library",
        "bom-ref": f"{name}@{version}",
        "name": name,
        "version": version,
        "licenses": [{"expression": licence}],
    }
    if purl is not None:
        entry["purl"] = purl
    return entry


def libraries() -> list[dict[str, object]]:
    """Returns the crates of the compiler and the runtime, LLVM and the Rust standard
    library."""
    entries = []
    for name, version, licence, own in sorted(crates("typy-cli") | crates("typy-runtime")):
        purl = None if own else f"pkg:cargo/{name}@{version}"
        entries.append(component(name, version, licence, purl))
    llvm_config = pathlib.Path(os.environ["LLVM_SYS_231_PREFIX"]) / "bin" / "llvm-config"
    llvm = run(str(llvm_config), "--version").strip()
    entries.append(
        component("llvm", llvm, "Apache-2.0 WITH LLVM-exception", f"pkg:generic/llvm@{llvm}")
    )
    rust = run("rustc", "-V").split()[1]
    entries.append(
        component("rust-std", rust, "MIT OR Apache-2.0", f"pkg:generic/rust-std@{rust}")
    )
    return entries


def files(stage: pathlib.Path) -> list[dict[str, object]]:
    """Returns each file under `stage` but the bill itself, with its SHA-256."""
    entries = []
    for path in sorted(stage.rglob("*")):
        relative = path.relative_to(stage).as_posix()
        if not path.is_file() or relative == BILL:
            continue
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        entries.append(
            {
                "type": "file",
                "bom-ref": f"file:{relative}",
                "name": relative,
                "hashes": [{"alg": "SHA-256", "content": digest}],
            }
        )
    return entries


def main() -> None:
    stage, version = pathlib.Path(sys.argv[1]), sys.argv[2]
    timestamp = run("git", "log", "-1", "--format=%cI").strip()
    bill = {
        "bomFormat": "CycloneDX",
        "specVersion": "1.6",
        "version": 1,
        "metadata": {
            "timestamp": timestamp,
            "component": {
                "type": "application",
                "bom-ref": f"typy@{version}",
                "name": "typy",
                "version": version,
                "licenses": [{"expression": "MIT OR Apache-2.0"}],
            },
        },
        "components": libraries() + files(stage),
    }
    text = json.dumps(bill, indent=2, ensure_ascii=True) + "\n"
    (stage / BILL).write_text(text, encoding="ascii")


if __name__ == "__main__":
    main()
