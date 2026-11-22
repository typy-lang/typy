#!/usr/bin/env python3
"""Compares TyPy with other languages on the workloads of tests/bench/programs, as the
benchmark report describes: each peer builds each workload from a clean state, its output is
checked against CPython 3.15's, and then it is timed at the workload's main size, with its peak
memory, and once at a quarter and at half of it.

Every run is pinned to the same two cores with taskset, one job at a time. `--lanes N` runs N
jobs at once instead, each lane pinned to the hardware threads of a physical core of its own
other than core 0: a quicker run, whose times run higher and vary more, since the lanes share the
caches and a busy chip lowers its clock; the report names the cores and the jobs at once. A
run's time is the wall time of the process, and its memory the peak resident size the kernel
reports through `wait4`. The raw figures go to .tmp/bench/results.json, which
tests/bench/report.py turns into tables. A peer that cannot build a workload is recorded with the
reason, never left out silently.
"""

from __future__ import annotations

import functools
import json
import os
import platform
import queue
import shutil
import statistics
import subprocess
import sys
import tempfile
import time
from collections.abc import Callable
from concurrent.futures import Future, ThreadPoolExecutor
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PROGRAMS = ROOT / "tests" / "bench" / "programs"
WORK = ROOT / ".tmp" / "bench"
TOOLS = Path(os.environ.get("BENCH_TOOLS", "/var/cache/typy/bench"))
TARGET = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target"))
CORES = "2,3"
RUNS = 3
STARTUP_RUNS = 10
MEASURE = WORK / "measure"

# name: (main size, check size, the size divided for the scaling runs at a quarter and a half)
WORKLOADS: dict[str, tuple[int, int, tuple[int, int]]] = {
    "nbody": (4_000_000, 1000, (1_000_000, 2_000_000)),
    "spectralnorm": (2000, 100, (1000, 1414)),
    "fannkuchredux": (10, 7, (8, 9)),
    "binarytrees": (16, 10, (14, 15)),
    "mandelbrot": (1600, 200, (800, 1131)),
    "fasta": (2_000_000, 1000, (500_000, 1_000_000)),
    "knucleotide": (1_000_000, 10000, (250_000, 500_000)),
    "strings": (500_000, 2000, (125_000, 250_000)),
    "dicts": (4_000_000, 20000, (1_000_000, 2_000_000)),
    "dispatch": (50_000, 100, (12_500, 25_000)),
    "recursion": (36, 22, (33, 34)),
    "parser": (150_000, 2000, (37_500, 75_000)),
}


@dataclass
class Built:
    """A workload one peer built: the command that runs it, where, and what the build cost."""

    command: list[str]
    directory: Path
    compile_seconds: float
    size_bytes: int


@dataclass
class Peer:
    """A language and its compiler or interpreter, with how it builds and runs a program."""

    name: str
    language: str
    extension: str
    version: list[str]
    environment: dict[str, str] = field(default_factory=dict)

    def source(self, workload: str) -> Path | None:
        directory = PROGRAMS / workload
        if self.name == "Codon" and (directory / f"{workload}.codon.py").exists():
            return directory / f"{workload}.codon.py"
        if self.extension == ".java":
            found = sorted(directory.glob("*.java"))
            return found[0] if found else None
        path = directory / f"{workload}{self.extension}"
        return path if path.exists() else None


def tool(*parts: str) -> str:
    return str(TOOLS.joinpath(*parts))


PYTHON_314 = tool("python-3.14", "bin", "python")
GO = tool("go-1.27.4", "bin", "go")
JDK = tool("jdk-25.0.5+8", "bin")
PEERS = [
    *[
        Peer(f"CPython {minor}", "Python", ".py", [f"python{minor}", "--version"])
        for minor in ("3.12", "3.13", "3.14", "3.15")
    ],
    Peer("PyPy", "Python", ".py", [tool("pypy-3.11-v7.3.23", "bin", "pypy3"), "--version"]),
    Peer("mypyc", "Python", ".py", [PYTHON_314, "-m", "mypy", "--version"]),
    Peer("Cython", "Python", ".py", [PYTHON_314, "-m", "cython", "--version"]),
    Peer("Nuitka", "Python", ".py", [PYTHON_314, "-m", "nuitka", "--version"]),
    Peer(
        "Codon",
        "Python",
        ".py",
        [tool("codon-0.20.6", "bin", "codon"), "--version"],
        {"LIBRARY_PATH": tool("lib")},
    ),
    Peer("TyPy", "Python", ".py", [str(TARGET / "release" / "typy"), "--version"]),
    Peer("C (gcc)", "C", ".c", ["gcc", "--version"]),
    Peer("C (clang)", "C", ".c", ["clang", "--version"]),
    Peer("C++ (g++)", "C++", ".cpp", ["g++", "--version"]),
    Peer("Rust", "Rust", ".rs", ["rustc", "--version"]),
    Peer("Go", "Go", ".go", [GO, "version"], {"GOTOOLCHAIN": "local"}),
    Peer("Java", "Java", ".java", [f"{JDK}/java", "--version"]),
    Peer("JavaScript (Node)", "JavaScript", ".js", ["node", "--version"]),
]


def run_quietly(command: list[str], directory: Path, environment: dict[str, str]) -> None:
    """Runs a build step and fails with its output when it fails."""
    done = subprocess.run(
        command,
        cwd=directory,
        env={**os.environ, **environment},
        capture_output=True,
        text=True,
        check=False,
    )
    if done.returncode != 0:
        raise RuntimeError(f"{' '.join(command)} failed:\n{done.stdout}{done.stderr}")


def build(peer: Peer, workload: str, source: Path) -> Built:
    """Builds `source` for `peer` in a directory of its own and returns how to run it."""
    directory = WORK / "build" / peer.name.replace(" ", "_") / workload
    shutil.rmtree(directory, ignore_errors=True)
    directory.mkdir(parents=True)
    name = source.name.removesuffix(".codon.py").removesuffix(source.suffix)
    output = directory / name
    environment = dict(peer.environment)
    start = time.perf_counter()
    if peer.name.startswith("CPython") or peer.name == "PyPy":
        interpreter = peer.version[0]
        built = Built([interpreter, str(source)], directory, 0.0, source.stat().st_size)
        return built
    if peer.name in ("mypyc", "Cython"):
        shutil.copy(source, directory / f"{name}.py")
        if peer.name == "mypyc":
            run_quietly([tool("python-3.14", "bin", "mypyc"), f"{name}.py"], directory, environment)
        else:
            cythonize = tool("python-3.14", "bin", "cythonize")
            run_quietly([cythonize, "-i", "-3", f"{name}.py"], directory, environment)
        elapsed = time.perf_counter() - start
        (directory / f"{name}.py").unlink()
        size = sum(path.stat().st_size for path in directory.rglob("*.so"))
        return Built([PYTHON_314, "-c", f"import {name}"], directory, elapsed, size)
    if peer.name == "Nuitka":
        # A cache of its own, so the build starts from nothing.
        environment["NUITKA_CACHE_DIR"] = str(directory / "nuitka-cache")
        shutil.copy(source, directory / f"{name}.py")
        run_quietly(
            [PYTHON_314, "-m", "nuitka", "--quiet", "--output-dir=build", f"{name}.py"],
            directory,
            environment,
        )
        output = directory / "build" / f"{name}.bin"
    elif peer.name == "Codon":
        codon = tool("codon-0.20.6", "bin", "codon")
        run_quietly(
            [codon, "build", "-release", "-exe", "-o", str(output), str(source)],
            directory,
            environment,
        )
    elif peer.name == "TyPy":
        typy = str(TARGET / "release" / "typy")
        run_quietly([typy, "build", "--release", str(source), "-o", str(output)], directory, environment)
    elif peer.name in ("C (gcc)", "C (clang)"):
        compiler = "gcc" if "gcc" in peer.name else "clang"
        run_quietly([compiler, "-O2", "-o", str(output), str(source), "-lm"], directory, environment)
    elif peer.name == "C++ (g++)":
        run_quietly(["g++", "-std=c++20", "-O2", "-o", str(output), str(source)], directory, environment)
    elif peer.name == "Rust":
        run_quietly(
            ["rustc", "--edition", "2024", "-C", "opt-level=3", "-o", str(output), str(source)],
            directory,
            environment,
        )
    elif peer.name == "Go":
        # A cache of its own, so the build starts from nothing.
        environment["GOCACHE"] = str(directory / "go-cache")
        environment["GOPATH"] = str(directory / "go-path")
        run_quietly([GO, "build", "-o", str(output), str(source)], directory, environment)
    elif peer.name == "Java":
        classes = directory / "classes"
        run_quietly([f"{JDK}/javac", "-d", str(classes), str(source)], directory, environment)
        elapsed = time.perf_counter() - start
        size = sum(path.stat().st_size for path in classes.rglob("*.class"))
        command = [f"{JDK}/java", "-cp", str(classes), source.stem]
        return Built(command, directory, elapsed, size)
    elif peer.name == "JavaScript (Node)":
        return Built(["node", str(source)], directory, 0.0, source.stat().st_size)
    elapsed = time.perf_counter() - start
    for cache in ("go-cache", "go-path", "nuitka-cache"):
        shutil.rmtree(directory / cache, ignore_errors=True)
    return Built([str(output)], directory, elapsed, output.stat().st_size)


@dataclass(frozen=True)
class Lane:
    """Where a job runs: the cores its runs are pinned to, and the file measure.c reports in."""

    cores: str
    result: Path


def lanes(count: int) -> list[Lane]:
    """Returns `count` lanes: the benchmark's two cores for one, and otherwise the hardware
    threads of a physical core each, from the last, leaving core 0 to the system."""
    if count == 1:
        return [Lane(CORES, WORK / "measured")]
    cores: dict[int, list[int]] = {}
    for cpu in sorted(os.sched_getaffinity(0)):
        core = int(Path(f"/sys/devices/system/cpu/cpu{cpu}/topology/core_id").read_text())
        cores.setdefault(core, []).append(cpu)
    physical = [cores[core] for core in sorted(cores) if core != 0][-count:]
    return [
        Lane(",".join(str(cpu) for cpu in cpus), WORK / f"measured-{index}")
        for index, cpus in enumerate(physical)
    ]


def measure(built: Built, size: int, environment: dict[str, str], lane: Lane) -> tuple[float, int]:
    """Runs a built program once at `size`, pinned to the lane's cores, its output discarded,
    and returns its wall time and its peak resident size in KiB, as tests/bench/measure.c
    reports them."""
    result = lane.result
    command = ["taskset", "-c", lane.cores, str(MEASURE), str(result), *built.command, str(size)]
    done = subprocess.run(
        command,
        cwd=built.directory,
        env={**os.environ, **environment},
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        check=False,
    )
    if done.returncode != 0:
        message = done.stderr.decode(errors="replace")
        raise RuntimeError(f"{' '.join(command)} exited with {done.returncode}: {message[-400:]}")
    nanoseconds, kib = result.read_text().split()
    return int(nanoseconds) / 1e9, int(kib)


def output_of(built: Built, size: int, environment: dict[str, str]) -> str:
    done = subprocess.run(
        [*built.command, str(size)],
        cwd=built.directory,
        env={**os.environ, **environment},
        capture_output=True,
        text=True,
        check=False,
    )
    return done.stdout


def version_of(peer: Peer) -> str:
    done = subprocess.run(peer.version, capture_output=True, text=True, check=False)
    text = (done.stdout or done.stderr).strip().splitlines()
    return text[0] if text else "unknown"


def machine(running: list[Lane]) -> dict[str, str]:
    model = "unknown"
    for line in Path("/proc/cpuinfo").read_text().splitlines():
        if line.startswith("model name"):
            model = line.split(":", 1)[1].strip()
            break
    return {
        "cpu": model,
        "logical cpus": str(os.cpu_count()),
        "kernel": platform.release(),
        "pinned cores": " and ".join(lane.cores for lane in running),
        "jobs at once": str(len(running)),
        "date": time.strftime("%Y-%m-%d"),
    }


def main() -> None:
    arguments = sys.argv[1:]
    count = 1
    if arguments[:1] == ["--lanes"]:
        count = max(1, int(arguments[1]))
        arguments = arguments[2:]
    chosen = set(arguments) or set(WORKLOADS)
    WORK.mkdir(parents=True, exist_ok=True)
    measure_source = ROOT / "tests" / "bench" / "measure.c"
    subprocess.run(["gcc", "-O2", "-o", str(MEASURE), str(measure_source)], check=True)
    running = lanes(count)
    results: dict[str, object] = {
        "machine": machine(running),
        "versions": {peer.name: version_of(peer) for peer in PEERS},
        "runs": RUNS,
        "workloads": {},
        "startup": {},
    }
    free: queue.Queue[Lane] = queue.Queue()
    for lane in running:
        free.put(lane)

    def in_a_lane(job: Callable[[Lane], dict[str, object]], label: str) -> dict[str, object]:
        """Runs `job` in the first free lane and prints its line when it is done."""
        lane = free.get()
        try:
            entry = job(lane)
        finally:
            free.put(lane)
        print(f"{label} {summary(entry)}", flush=True)
        return entry

    with tempfile.TemporaryDirectory() as scratch, ThreadPoolExecutor(len(running)) as pool:
        reference = Path(scratch)
        # Every reference output comes first, so no job is timed while CPython makes one.
        expected: dict[str, str] = {
            workload: subprocess.run(
                ["python3.15", str(PROGRAMS / workload / f"{workload}.py"), str(check_size)],
                cwd=reference,
                capture_output=True,
                text=True,
                check=True,
            ).stdout
            for workload, (_, check_size, _) in WORKLOADS.items()
            if workload in chosen
        }
        pending: dict[str, dict[str, Future[dict[str, object]]]] = {}
        for workload, output in expected.items():
            size, check_size, scaling = WORKLOADS[workload]
            sizes = (size, check_size, scaling)
            pending[workload] = {
                peer.name: pool.submit(
                    in_a_lane,
                    functools.partial(run_peer, peer, workload, sizes, output),
                    f"{workload:14} {peer.name:20}",
                )
                for peer in PEERS
            }
        for workload, futures in pending.items():
            size, _, scaling = WORKLOADS[workload]
            entries = {name: future.result() for name, future in futures.items()}
            results["workloads"][workload] = {
                "size": size,
                "scaling": list(scaling),
                "peers": entries,
            }
        starts = {
            peer.name: pool.submit(
                in_a_lane, functools.partial(startup, peer), f"{'startup':14} {peer.name:20}"
            )
            for peer in PEERS
        }
        results["startup"] = {name: future.result() for name, future in starts.items()}
    (WORK / "results.json").write_text(json.dumps(results, indent=1))
    print(f"results: {WORK / 'results.json'}")


def run_peer(
    peer: Peer,
    workload: str,
    sizes: tuple[int, int, tuple[int, int]],
    expected: str,
    lane: Lane,
) -> dict[str, object]:
    size, check_size, scaling = sizes
    source = peer.source(workload)
    if source is None:
        return {"error": "no program"}
    try:
        built = build(peer, workload, source)
        if output_of(built, check_size, peer.environment) != expected:
            return {"error": "output differs from CPython 3.15's"}
        runs = [measure(built, size, peer.environment, lane) for _ in range(RUNS)]
        times = [elapsed for elapsed, _ in runs]
        scaled = {str(each): measure(built, each, peer.environment, lane)[0] for each in scaling}
    except (RuntimeError, OSError) as failure:
        return {"error": str(failure).splitlines()[0][:200]}
    return {
        "times": times,
        "median": statistics.median(times),
        "rss_kib": max(rss for _, rss in runs),
        "compile_seconds": built.compile_seconds,
        "size_bytes": built.size_bytes,
        "scaling": scaled,
    }


def startup(peer: Peer, lane: Lane) -> dict[str, object]:
    source = peer.source("hello")
    if source is None:
        return {"error": "no program"}
    try:
        built = build(peer, "hello", source)
        runs = [measure(built, 1, peer.environment, lane) for _ in range(STARTUP_RUNS)]
    except (RuntimeError, OSError) as failure:
        return {"error": str(failure).splitlines()[0][:200]}
    times = [elapsed for elapsed, _ in runs]
    return {
        "times": times,
        "median": statistics.median(times),
        "rss_kib": max(rss for _, rss in runs),
        "size_bytes": built.size_bytes,
    }


def summary(entry: object) -> str:
    if not isinstance(entry, dict):
        return ""
    if "error" in entry:
        return f"not run: {entry['error']}"
    return f"{entry['median']:.3f} s, {entry['rss_kib'] / 1024:.1f} MiB"


if __name__ == "__main__":
    main()
