#!/usr/bin/env python3
"""Measures the share of a fixed corpus of `mypy --strict` modules that TyPy compiles unchanged
and runs with CPython 3.15's output (NFR-24), as the corpus report describes.

Two corpora are fetched once, at pinned commits, into the cache volume: the algorithm scripts of
TheAlgorithms/Python and the run tests of mypyc, a compiler of typed Python. `--select` chooses
the corpus, the modules that import only the standard library, pass `mypy --strict` and run to
the same output twice under CPython 3.15, and writes their names to tests/corpus/modules.txt; it
is run when a pin changes, and needs the mypy of `make bench-peers`. A module of the list counts
when TyPy builds it unchanged and the program prints what CPython prints and exits as CPython
exits. `--check` fails when a share falls below the one the report holds, as the release workflow
runs it, and `--write` puts the figures into the report unless a share fell (NFR-24).
"""

from __future__ import annotations

import argparse
import ast
import collections
import concurrent.futures
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from collections.abc import Callable
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CACHE = Path(os.environ.get("TYPY_CORPUS", "/var/cache/typy/corpus"))
TOOLS = Path(os.environ.get("BENCH_TOOLS", "/var/cache/typy/bench"))
TARGET = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target"))
REPORT = ROOT / "docs" / "testing" / "corpus-report.md"
MODULES = ROOT / "tests" / "corpus" / "modules.txt"
MYPY = TOOLS / "python-3.14" / "bin" / "python3"
PYTHON = "python3.15"
TIMEOUT = 20
BUILD_TIMEOUT = 300
# Programs are checked, built and run on half the processors at once: only their output is
# compared, never their time, and the rest leaves each run room to finish within its limit.
WORKERS = max(1, (os.cpu_count() or 2) // 2)


@dataclass(frozen=True)
class Source:
    """A corpus: where it comes from, the commit it is pinned to, and the part of it read."""

    name: str
    url: str
    commit: str
    part: str


SOURCES = [
    Source(
        "TheAlgorithms/Python",
        "https://github.com/TheAlgorithms/Python",
        "30507bbe41397065728167fb09aa820f9fce8dc7",
        "",
    ),
    Source(
        "mypyc run tests",
        "https://github.com/python/mypy",
        "1572a18848d181f5cd91877aa3ab4e1644b14f20",
        "mypyc/test-data",
    ),
]


@dataclass
class Tally:
    """What became of a corpus's modules, and why each left out or failing one did."""

    found: int = 0
    selected: int = 0
    built: int = 0
    passed: int = 0
    reasons: collections.Counter[str] = field(
        default_factory=lambda: collections.Counter[str]()
    )
    names: list[str] = field(default_factory=list)


type Ran = subprocess.CompletedProcess[str] | None


def run(command: list[str], directory: Path, timeout: int = TIMEOUT) -> Ran:
    """Runs a command with no input, or returns `None` when it outlives `timeout` seconds."""
    try:
        return subprocess.run(
            command,
            cwd=directory,
            stdin=subprocess.DEVNULL,
            capture_output=True,
            text=True,
            errors="replace",
            timeout=timeout,
            check=False,
        )
    except subprocess.TimeoutExpired:
        return None


def parallel[T, R](function: Callable[[T], R], items: list[T]) -> list[R]:
    """Returns `function` of each item, in the items' order, computed `WORKERS` at a time."""
    with concurrent.futures.ThreadPoolExecutor(WORKERS) as pool:
        return list(pool.map(function, items))


def fetch(source: Source) -> Path:
    """Returns the checkout of a corpus at its commit, fetching it the first time."""
    directory = CACHE / source.commit[:12]
    marker = directory / ".typy-fetched"
    if marker.exists():
        return directory / source.part
    shutil.rmtree(directory, ignore_errors=True)
    directory.mkdir(parents=True)
    git = ["git", "-C", str(directory)]
    subprocess.run([*git, "init", "-q"], check=True)
    subprocess.run([*git, "remote", "add", "origin", source.url], check=True)
    if source.part:
        # Only the part read is checked out, with the blobs it needs.
        subprocess.run([*git, "config", "remote.origin.promisor", "true"], check=True)
        subprocess.run(
            [*git, "config", "remote.origin.partialclonefilter", "blob:none"], check=True
        )
        subprocess.run([*git, "sparse-checkout", "set", source.part], check=True)
        subprocess.run(
            [*git, "fetch", "-q", "--depth", "1", "--filter=blob:none", "origin", source.commit],
            check=True,
        )
    else:
        subprocess.run([*git, "fetch", "-q", "--depth", "1", "origin", source.commit], check=True)
    subprocess.run([*git, "checkout", "-q", "FETCH_HEAD"], check=True)
    marker.write_text(source.commit)
    return directory / source.part


def standard_only(tree: ast.Module) -> bool:
    """Returns true when every import names a module of the standard library."""
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            names = [alias.name for alias in node.names]
        elif isinstance(node, ast.ImportFrom):
            if node.level:
                return False
            names = [node.module or ""]
        else:
            continue
        if any(name.split(".")[0] not in sys.stdlib_module_names for name in names):
            return False
    return True


def has_main(tree: ast.Module) -> bool:
    """Returns true when the module runs something under `if __name__ == "__main__":`."""
    for node in tree.body:
        if isinstance(node, ast.If) and "__main__" in ast.unparse(node.test):
            return True
    return False


def algorithms(directory: Path, work: Path) -> list[tuple[str, Path]]:
    """Returns each algorithm script that may join the corpus, as a program of its own."""
    found: list[tuple[str, Path]] = []
    for path in sorted(directory.rglob("*.py")):
        relative = path.relative_to(directory)
        if relative.parts[0].startswith(".") or relative.parts[0] == "scripts":
            continue
        if path.name.startswith("test_") or path.name.endswith("_test.py"):
            continue
        if path.name in {"__init__.py", "conftest.py"}:
            continue
        program = work / "algorithms" / relative
        program.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(path, program)
        found.append((str(relative), program))
    return found


CASE = re.compile(r"^\[case (\w+)\]\s*$", re.MULTILINE)
SECTION = re.compile(r"^\[[a-z]", re.MULTILINE)


def run_tests(directory: Path, work: Path) -> list[tuple[str, Path]]:
    """Returns each run test of mypyc whose main module stands alone, with a driver that calls
    its test functions in order, as mypyc's harness does."""
    found: list[tuple[str, Path]] = []
    for path in sorted(directory.glob("run-*.test")):
        text = path.read_text()
        for match in CASE.finditer(text):
            rest = text[match.end():]
            following = SECTION.search(rest)
            body = rest[: following.start()] if following else rest
            sections = text[match.end() + len(body):]
            nested = CASE.search(sections)
            if "[file " in sections[: nested.start() if nested else len(sections)]:
                continue
            try:
                tree = ast.parse(body)
            except SyntaxError:
                continue
            tests = [
                node.name
                for node in tree.body
                if isinstance(node, ast.FunctionDef) and node.name.startswith("test_")
            ]
            if not tests:
                continue
            calls = "".join(f"    {name}()\n" for name in tests)
            driver = f"\n\nif __name__ == \"__main__\":\n{calls}    print(\"done\")\n"
            program = work / "mypyc" / f"{path.stem}-{match.group(1)}.py"
            program.parent.mkdir(parents=True, exist_ok=True)
            program.write_text(body.rstrip() + driver)
            found.append((f"{path.name}::{match.group(1)}", program))
    return found


def strict(programs: list[Path], cache: Path) -> set[Path]:
    """Returns the programs that pass `mypy --strict` for Python 3.15, each checked alone, its
    verdict kept by the hash of its text."""
    verdicts_path = cache / "mypy-verdicts.json"
    verdicts: dict[str, bool] = (
        json.loads(verdicts_path.read_text()) if verdicts_path.exists() else {}
    )
    digests = [hashlib.sha256(program.read_bytes()).hexdigest() for program in programs]
    unchecked = [
        (digest, program)
        for digest, program in zip(digests, programs, strict=True)
        if digest not in verdicts
    ]

    def check(item: tuple[str, Path]) -> bool:
        digest, program = item
        # Each check has a cache of its own, as two mypy processes may not share one.
        checked = run(
            [
                str(MYPY), "-m", "mypy", "--strict", "--python-version", "3.15",
                "--no-error-summary", "--cache-dir", str(cache / "mypy-cache" / digest[:2]),
                program.name,
            ],
            program.parent,
            timeout=120,
        )
        return checked is not None and checked.returncode == 0

    for (digest, _), verdict in zip(unchecked, parallel(check, unchecked), strict=True):
        verdicts[digest] = verdict
    verdicts_path.write_text(json.dumps(verdicts))
    return {
        program for digest, program in zip(digests, programs, strict=True) if verdicts[digest]
    }


def select(programs: list[tuple[str, Path]], cache: Path) -> Tally:
    """Chooses the corpus among `programs`, keeping the names of those chosen and the reason
    each other one is left out."""
    tally = Tally(found=len(programs))
    eligible: list[tuple[str, Path]] = []
    for name, program in programs:
        try:
            tree = ast.parse(program.read_text())
        except (SyntaxError, UnicodeDecodeError):
            tally.reasons["not Python 3.15"] += 1
            continue
        if not standard_only(tree):
            tally.reasons["imports a module outside the standard library"] += 1
        elif not has_main(tree):
            tally.reasons["runs nothing as a program"] += 1
        else:
            eligible.append((name, program))
    passing = strict([program for _, program in eligible], cache)
    typed = [(name, program) for name, program in eligible if program in passing]
    tally.reasons["fails mypy --strict"] += len(eligible) - len(typed)

    def twice(program: Path) -> tuple[Ran, Ran]:
        return run([PYTHON, program.name], program.parent), run(
            [PYTHON, program.name], program.parent
        )

    for (name, _), (first, second) in zip(
        typed, parallel(twice, [program for _, program in typed]), strict=True
    ):
        if first is None or first.returncode != 0:
            tally.reasons["fails or runs too long under CPython"] += 1
        elif second is None or (second.stdout, second.returncode) != (
            first.stdout,
            first.returncode,
        ):
            tally.reasons["prints differently each run under CPython"] += 1
        else:
            tally.names.append(name)
    tally.selected = len(tally.names)
    return tally


def outcome(program: Path, typy: Path) -> str:
    """Builds and runs one module of the corpus, and returns "same" when it runs as CPython runs
    it, or why it does not."""
    expected = run([PYTHON, program.name], program.parent)
    binary = program.with_suffix(".bin")
    built = run([str(typy), "build", program.name, "-o", binary.name], program.parent, BUILD_TIMEOUT)
    if built is None or built.returncode != 0:
        code = re.search(r"error\[(E\d{4})\]", built.stdout + built.stderr) if built else None
        return f"refused: {code.group(1) if code else 'build failed or timed out'}"
    ran = run([f"./{binary.name}"], program.parent)
    if ran is None:
        return "differs: runs too long"
    if expected is None or (ran.stdout, ran.returncode) != (expected.stdout, expected.returncode):
        return "differs: output or exit status"
    return "same"


def measure(programs: list[tuple[str, Path]], typy: Path) -> Tally:
    """Counts the modules of the corpus that TyPy builds and runs as CPython runs them."""
    tally = Tally(found=len(programs), selected=len(programs))
    outcomes = parallel(lambda program: outcome(program, typy), [path for _, path in programs])
    for result in outcomes:
        if not result.startswith("refused"):
            tally.built += 1
        if result == "same":
            tally.passed += 1
        else:
            tally.reasons[result] += 1
    return tally


def share(tally: Tally) -> float:
    """Returns the share of the corpus TyPy builds and runs alike, in percent."""
    return 100.0 * tally.passed / tally.selected if tally.selected else 0.0


def recorded() -> dict[str, float]:
    """Returns each corpus's share as the report holds it."""
    shares: dict[str, float] = {}
    row = re.compile(r"^\| (.+?) \| [0-9a-f]{7} \| \d+ \| \d+ \| \d+ \| ([\d.]+) % \|$")
    for line in REPORT.read_text().splitlines():
        match = row.match(line)
        if match:
            shares[match.group(1)] = float(match.group(2))
    return shares


def write_list(selections: dict[Source, Tally]) -> None:
    """Writes the names of the corpus, each corpus's under a heading of its name."""
    lines = [
        "# The corpus of NFR-24: the modules scripts/corpus.py --select chose from each corpus",
        "# at its pinned commit, under a heading of the corpus's name.",
    ]
    for source, tally in selections.items():
        lines += [f"[{source.name}]", *tally.names]
    MODULES.parent.mkdir(parents=True, exist_ok=True)
    MODULES.write_text("\n".join(lines) + "\n")


def read_list() -> dict[str, list[str]]:
    """Returns the names of the corpus by the name of their corpus."""
    names: dict[str, list[str]] = {}
    current: list[str] = []
    for line in MODULES.read_text().splitlines():
        if not line or line.startswith("#"):
            continue
        if line.startswith("[") and line.endswith("]"):
            current = names.setdefault(line[1:-1], [])
        else:
            current.append(line)
    return names


def reasons(tallies: dict[Source, Tally], heading: str) -> list[str]:
    """Returns the table of the reasons each corpus's modules were counted out."""
    lines = ["", f"| Corpus | {heading} | Modules |", "| --- | --- | --- |"]
    for source, tally in tallies.items():
        for reason, count in sorted(tally.reasons.items(), key=lambda item: (-item[1], item[0])):
            lines.append(f"| {source.name} | {reason} | {count} |")
    return lines


def selection_table(selections: dict[Source, Tally]) -> str:
    """Returns how the corpus was chosen, as the report's tables."""
    lines = [
        "| Corpus | Commit | Modules read | Chosen |",
        "| --- | --- | --- | --- |",
    ]
    for source, tally in selections.items():
        lines.append(
            f"| {source.name} | {source.commit[:7]} | {tally.found} | {tally.selected} |"
        )
    return "\n".join(lines + reasons(selections, "Left out because"))


def results_table(results: dict[Source, Tally]) -> str:
    """Returns the results as the report's tables."""
    lines = [
        "| Corpus | Commit | Modules | Built | Same output | Share |",
        "| --- | --- | --- | --- | --- | --- |",
    ]
    for source, tally in results.items():
        lines.append(
            f"| {source.name} | {source.commit[:7]} | {tally.selected} | {tally.built} | "
            f"{tally.passed} | {share(tally):.1f} % |"
        )
    return "\n".join(lines + reasons(results, "Outcome"))


def replace_between(marker: str, content: str) -> None:
    """Writes `content` into the report between `<!-- marker -->` and its end marker."""
    text = REPORT.read_text()
    opening = f"<!-- {marker} -->"
    start = text.index(opening) + len(opening)
    end = text.index(f"<!-- end of {marker} -->")
    REPORT.write_text(f"{text[:start]}\n\n{content}\n\n{text[end:]}")


def main() -> int:
    """Chooses or measures the corpus; writes the report or checks it, as the arguments ask."""
    parser = argparse.ArgumentParser(description=(__doc__ or "").split("\n\n")[0])
    parser.add_argument(
        "--select", action="store_true", help="choose the corpus and write its list"
    )
    parser.add_argument("--write", action="store_true", help="write the figures into the report")
    parser.add_argument(
        "--check", action="store_true", help="fail when a share falls below the report's"
    )
    arguments = parser.parse_args()
    typy = TARGET / "release" / "typy"
    measuring = arguments.write or arguments.check
    if measuring and not typy.exists():
        print(f"corpus: {typy} is missing; run cargo build --release -p typy-cli", file=sys.stderr)
        return 1
    CACHE.mkdir(parents=True, exist_ok=True)
    selections: dict[Source, Tally] = {}
    results: dict[Source, Tally] = {}
    with tempfile.TemporaryDirectory() as scratch:
        work = Path(scratch)
        for source, gather in zip(SOURCES, [algorithms, run_tests], strict=True):
            programs = gather(fetch(source), work / source.commit[:12])
            if arguments.select:
                selections[source] = select(programs, CACHE)
        if arguments.select:
            write_list(selections)
            replace_between("corpus selection", selection_table(selections))
        if not MODULES.exists():
            print(f"corpus: {MODULES} is missing; run make corpus-select", file=sys.stderr)
            return 1
        listed = read_list()
        for source, gather in zip(SOURCES, [algorithms, run_tests], strict=True):
            if not measuring:
                break
            paths = dict(gather(fetch(source), work / source.commit[:12]))
            chosen = [(name, paths[name]) for name in listed.get(source.name, [])]
            results[source] = tally = measure(chosen, typy)
            print(
                f"{source.name}: {tally.passed} of {tally.selected} ({share(tally):.1f} %), "
                f"built {tally.built}",
                flush=True,
            )
    # The shares are written to one decimal place, so a share within 0.05 of the report's has
    # not fallen.
    floor = recorded()
    fallen = [
        source.name
        for source, tally in results.items()
        if share(tally) + 0.05 < floor.get(source.name, 0.0)
    ]
    if fallen:
        names = ", ".join(fallen)
        print(f"corpus: the share fell below the report's for {names}", file=sys.stderr)
        return 1
    if arguments.write:
        replace_between("corpus results", results_table(results))
    return 0


if __name__ == "__main__":
    sys.exit(main())
