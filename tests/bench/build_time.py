#!/usr/bin/env python3
"""Times the release-built compiler on generated projects against the budgets of NFR-04,
NFR-05 and NFR-14, and fails when one is exceeded.

Each project is written when the benchmark starts, so no generated file is committed: a
10,000-line project in one file, the same in 200 modules that import each other, and 50 and
200 modules for the scaling check. Each figure is the best of three runs, which is the least
disturbed by other work on the machine; memory is the peak resident size of each build.
"""

from __future__ import annotations

import os
import subprocess
import sys
import tempfile
import time
from pathlib import Path

RUNS = 3
DEBUG_BUDGET = 2.0  # NFR-04, seconds
RELEASE_BUDGET = 20.0  # NFR-05, seconds
MEMORY_BUDGET = 1024  # NFR-14, MB
# Checking four times as many modules may take this many times as long; linear growth is
# about 4, and cost that grows with the square of the modules is 16.
SCALING_LIMIT = 6.0


def group(index: int) -> str:
    """Returns about 23 lines of typical code: a class, a function, a loop and collections."""
    return f'''
class Item{index}:
    def __init__(self, value: int) -> None:
        self.value = value
        self.name = "item{index}"

    def doubled(self) -> int:
        return self.value * 2


def work{index}(limit: int) -> int:
    total = 0
    values: list[int] = []
    for n in range(limit):
        if n % 3 == 0:
            values.append(n * {index})
        elif n % 5 == 1:
            total += n
        else:
            total -= 1
    table = {{"a": total, "b": len(values)}}
    return Item{index}(total).doubled() + table["a"] + sum(values)
'''


def one_file(directory: Path, lines: int) -> None:
    """Writes a project of one file of at least `lines` lines."""
    text = ""
    count = 0
    while text.count("\n") < lines:
        text += group(count)
        count += 1
    calls = " + ".join(f"work{index}(10)" for index in range(0, count, 50))
    (directory / "app.py").write_text(f"{text}\nprint({calls})\n")


def modules(directory: Path, count: int, groups: int) -> None:
    """Writes a project of `count` modules, each importing the one before it."""
    for index in range(count):
        text = f"import mod{index - 1}\n" if index else ""
        text += "".join(group(index * groups + part) for part in range(groups))
        if index:
            text += f"\n\ndef chain{index}() -> int:\n    return mod{index - 1}.work{(index - 1) * groups}(3)\n"
        (directory / f"mod{index}.py").write_text(text)
    last = count - 1
    call = f"mod{last}.chain{last}()" if last else "mod0.work0(3)"
    (directory / "app.py").write_text(f"import mod{last}\nprint({call})\n")


def measure(compiler: str, directory: Path, arguments: list[str]) -> tuple[float, int]:
    """Returns the best time of `RUNS` runs of the compiler, and its largest peak memory in MB."""
    best = float("inf")
    peak = 0
    for _ in range(RUNS):
        start = time.perf_counter()
        process = subprocess.Popen(
            [compiler, *arguments],
            cwd=directory,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
        )
        _, status, usage = os.wait4(process.pid, 0)
        took = time.perf_counter() - start
        if os.waitstatus_to_exitcode(status) != 0:
            error = process.stderr.read().decode() if process.stderr else ""
            sys.exit(f"typy {' '.join(arguments)} failed in {directory}:\n{error}")
        best = min(best, took)
        peak = max(peak, usage.ru_maxrss // 1024)
    return best, peak


def main() -> int:
    compiler = os.environ.get("TYPY", f"{os.environ['CARGO_TARGET_DIR']}/release/typy")
    failures: list[str] = []
    with tempfile.TemporaryDirectory(prefix="typy-bench-") as base:
        projects = {
            "10,000 lines in one file": Path(base, "one"),
            "10,000 lines in 200 modules": Path(base, "many"),
            "50 modules": Path(base, "small"),
            "200 modules": Path(base, "large"),
        }
        for directory in projects.values():
            directory.mkdir()
        one_file(projects["10,000 lines in one file"], 10_000)
        modules(projects["10,000 lines in 200 modules"], 200, 2)
        modules(projects["50 modules"], 50, 1)
        modules(projects["200 modules"], 200, 1)
        print(f"{'project':30} {'step':16} {'time':>8} {'budget':>8} {'memory':>8}")
        for name in ("10,000 lines in one file", "10,000 lines in 200 modules"):
            directory = projects[name]
            for step, arguments, budget in (
                ("check", ["check", "app.py"], None),
                ("debug build", ["build", "app.py", "-o", "out"], DEBUG_BUDGET),
                ("release build", ["build", "--release", "app.py", "-o", "out"], RELEASE_BUDGET),
            ):
                took, peak = measure(compiler, directory, arguments)
                shown = f"{budget:7.1f}s" if budget else ""
                print(f"{name:30} {step:16} {took:7.3f}s {shown:>8} {peak:6} MB")
                if budget and took > budget:
                    failures.append(f"{name}: {step} took {took:.3f}s, over {budget}s")
                if peak > MEMORY_BUDGET:
                    failures.append(f"{name}: {step} used {peak} MB, over {MEMORY_BUDGET} MB")
        small, _ = measure(compiler, projects["50 modules"], ["check", "app.py"])
        large, _ = measure(compiler, projects["200 modules"], ["check", "app.py"])
        ratio = large / small
        print(f"{'checking 200 modules / 50':30} {'ratio':16} {ratio:7.1f}x {SCALING_LIMIT:7.1f}x")
        if ratio > SCALING_LIMIT:
            failures.append(
                f"checking 200 modules took {ratio:.1f} times as long as 50, over {SCALING_LIMIT}"
            )
    for failure in failures:
        print(f"over budget: {failure}", file=sys.stderr)
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
