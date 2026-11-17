#!/usr/bin/env python3
"""Writes the results of tests/bench/compare.py into docs/testing/benchmark-report.md: the
tables between its `results start` and `results end` markers are replaced, and the rest of the
document is left as it is.
"""

from __future__ import annotations

import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
RESULTS = ROOT / ".tmp" / "bench" / "results.json"
REPORT = ROOT / "docs" / "testing" / "benchmark-report.md"
START = "<!-- results start -->"
END = "<!-- results end -->"
BASELINE = "C (gcc)"


def ran(entry: dict[str, object]) -> bool:
    return "error" not in entry


def geometric_mean(values: list[float]) -> float:
    return math.exp(sum(math.log(value) for value in values) / len(values))


def seconds(value: float) -> str:
    return f"{value:.3f}" if value < 10 else f"{value:.1f}"


def kibibytes(size: int) -> str:
    return "<1" if size < 1024 else f"{size / 1024:.0f}"


def mebibytes(kib: float) -> str:
    return f"{kib / 1024:.1f}"


def summary(results: dict[str, object]) -> list[str]:
    workloads: dict[str, dict[str, object]] = results["workloads"]
    peers = list(results["versions"])
    lines = [
        "| Peer | Time against C (gcc) | Memory against C (gcc) | Workloads run | Startup (ms) |",
        "| --- | --- | --- | --- | --- |",
    ]
    for peer in peers:
        time_ratios: list[float] = []
        memory_ratios: list[float] = []
        for workload in workloads.values():
            entry = workload["peers"][peer]
            baseline = workload["peers"][BASELINE]
            if ran(entry) and ran(baseline):
                time_ratios.append(entry["median"] / baseline["median"])
                memory_ratios.append(entry["rss_kib"] / baseline["rss_kib"])
        start = results["startup"][peer]
        startup = f"{start['median'] * 1000:.1f}" if ran(start) else "not run"
        if not time_ratios:
            lines.append(f"| {peer} | not run | not run | 0 of {len(workloads)} | {startup} |")
            continue
        lines.append(
            f"| {peer} | {geometric_mean(time_ratios):.2f} | {geometric_mean(memory_ratios):.2f} "
            f"| {len(time_ratios)} of {len(workloads)} | {startup} |"
        )
    return lines


def workload_table(name: str, workload: dict[str, object]) -> list[str]:
    peers: dict[str, dict[str, object]] = workload["peers"]
    baseline = peers[BASELINE]
    small, middle = workload["scaling"]
    lines = [
        f"### {name}, size {workload['size']:,}",
        "",
        f"| Peer | Median (s) | Fastest to slowest (s) | Against C (gcc) | Peak memory (MiB) "
        f"| At {small:,} (s) | At {middle:,} (s) | Build (s) | Size (KiB) |",
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- |",
    ]
    for peer, entry in peers.items():
        if not ran(entry):
            lines.append(f"| {peer} | not run: {entry['error']} | | | | | | | |")
            continue
        times = entry["times"]
        ratio = entry["median"] / baseline["median"] if ran(baseline) else math.nan
        scaling = entry["scaling"]
        lines.append(
            f"| {peer} | {seconds(entry['median'])} | {seconds(min(times))} to "
            f"{seconds(max(times))} | {ratio:.2f} | {mebibytes(entry['rss_kib'])} "
            f"| {seconds(scaling[str(small)])} | {seconds(scaling[str(middle)])} "
            f"| {entry['compile_seconds']:.2f} | {kibibytes(entry['size_bytes'])} |"
        )
    return lines


def tables(results: dict[str, object]) -> str:
    machine: dict[str, str] = results["machine"]
    lines = [
        "### Machine and versions",
        "",
        "| Item | Value |",
        "| --- | --- |",
        *[f"| {key.capitalize()} | {value} |" for key, value in machine.items()],
        f"| Runs at the main size | {results['runs']} |",
        *[f"| {peer} | {version} |" for peer, version in results["versions"].items()],
        "",
        "### Summary",
        "",
        "Each figure is the geometric mean, over the workloads the peer ran, of its",
        "median time or its peak memory divided by that of C (gcc): 1.00 is as fast,",
        "or as small, as C.",
        "",
        *summary(results),
        "",
    ]
    for name, workload in results["workloads"].items():
        lines.extend(workload_table(name, workload))
        lines.append("")
    return "\n".join(lines)


def main() -> None:
    results = json.loads(RESULTS.read_text())
    text = REPORT.read_text()
    if START not in text or END not in text:
        sys.exit(f"{REPORT} has no `{START}` and `{END}` markers")
    before, rest = text.split(START, 1)
    _, after = rest.split(END, 1)
    REPORT.write_text(f"{before}{START}\n\n{tables(results)}\n{END}{after}")
    print(f"report: {REPORT}")


if __name__ == "__main__":
    main()
