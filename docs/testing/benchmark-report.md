# TyPy benchmark report

| Field | Value |
| --- | --- |
| Document | TyPy benchmark report |
| Version | 0.2 |
| Status | Draft |
| Owner | typy-lang contributors |
| Last updated | 2026-11-22 |

## 1. Purpose

This report measures how fast TyPy's programs run, how much memory they
take, how long they take to build and how large they are, against the
languages a TyPy user would otherwise reach for: CPython and the other
compilers of Python, and C, C++, Rust, Go, Java and JavaScript. It builds on
the [test plan](test-plan.md), whose benchmark level it extends from the
budgets of NFR-03 to NFR-05 to a comparison across languages, and it gives
the figures that the performance work of later releases, such as stack
allocation (MEM-03), starts from.

The figures describe one machine on one day. They show where TyPy stands
and where it is slow, not a ranking of languages.

## 2. Method

### 2.1 Workloads

Twelve programs, each written once for each language, in the idiom of the
language and with the same algorithm, in `tests/bench/programs/`. Seven
come from the Benchmarks Game, adapted to TyPy's subset; five are the work a
Python program typically does.

| Workload | What it does | Main size |
| --- | --- | --- |
| nbody | Five bodies in orbit, advanced in steps: floating point, attributes | 4,000,000 steps |
| spectralnorm | The largest eigenvalue of an infinite matrix: floating point, lists | 2,000 |
| fannkuchredux | Flips of every permutation: integer lists | 10 |
| binarytrees | Many short-lived trees, allocated and freed: allocation | depth 16 |
| mandelbrot | Points of a grid in the Mandelbrot set, counted | 1,600 by 1,600 |
| fasta | DNA sequences, repeated and drawn from a generator, written out | 2,000,000 |
| knucleotide | Fragments of a generated sequence counted in a dictionary | 1,000,000 |
| strings | Joining, splitting, case, searching and replacing | 500,000 lines |
| dicts | Words counted in a dictionary, then ranked | 4,000,000 words |
| dispatch | Calls of a method through a base class | 50,000 rounds of 1,000 |
| recursion | The Fibonacci numbers by their definition | fib(36) |
| parser | Expressions written out, tokenized and evaluated | 150,000 |

The mandelbrot and knucleotide programs print counts rather than an image
or a sequence read from standard input, since TyPy 0.5 reads neither binary
output nor standard input this way; fasta writes its sequences, as in the
Benchmarks Game. Every program takes its size as its one argument.

### 2.2 Peers

| Peer | How it builds or runs a program |
| --- | --- |
| CPython 3.12 to 3.15 | Runs the Python source |
| PyPy 3.11 v7.3.23 | Runs the Python source with its JIT compiler |
| mypyc 2.4.1 | Compiles the Python source to a CPython 3.14 extension, which `python -c "import name"` runs |
| Cython 3.3.1 | Compiles the Python source, in pure Python mode, to a CPython 3.14 extension, run the same way |
| Nuitka 4.3.5 | Compiles the Python source to an executable that loads CPython 3.14 |
| Codon 0.20.6 | Compiles the Python source with `-release` |
| TyPy | Compiles the Python source with `--release` |
| C | `gcc -O2` and `clang -O2` |
| C++ | `g++ -std=c++20 -O2` |
| Rust | `rustc -C opt-level=3`, the optimisation of a Cargo release build |
| Go 1.27.4 | `go build` |
| Java (Temurin 25.0.5) | `javac`, then `java` with its JIT compiler |
| JavaScript (Node) | Runs the source |

The Python peers run the same source. Codon takes binarytrees from a copy,
`binarytrees.codon.py`, whose class declares its fields and names itself
unquoted in its annotations, which Codon needs and CPython 3.12 refuses; the
algorithm is the same. No peer gets flags beyond those above, such as
`-march=native`, so every program runs on any x86-64 machine, as TyPy's do.

### 2.3 Measures

`make bench-compare` runs `tests/bench/compare.py`, which for each peer and
each workload:

1. builds the program from a clean state, with caches of its own for Go and
   Nuitka, and records the build time and the size of what it built: the
   executable, the extension modules, the class files, or the source for a
   peer that runs it;
2. runs it at a small size and checks that its output is CPython 3.15's
   output, byte for byte; a peer whose output differs is not timed;
3. runs it three times at the main size, and once at a quarter and at half
   of it, for how its time grows.

Each run is pinned with `taskset`: to the same two cores when one job runs
at a time, and to a physical core of its own, both its hardware threads,
when `BENCH_LANES` runs several jobs at once; the machine table names the
cores and the jobs at once. Its output goes to `/dev/null`, and
`tests/bench/measure.c` forks it and reports its wall time
and its peak resident size from `wait4`. A JIT compiler's warm-up is part of
each run, since each run is a new process; startup is measured apart, as the
median of ten runs of a program that prints one line. `tests/bench/report.py`
writes the tables below from the raw figures. A peer that cannot build or
run a workload is listed with the reason.

### 2.4 Reproducing the figures

`make bench-peers` installs the peers the development image lacks, once, at
the versions above, each checked against its published checksum. `make
bench-compare` then builds the release compiler, runs every workload, which
takes about two hours one job at a time, and rewrites the results below.
`make bench-compare BENCH_LANES=3` runs three jobs at once in about 45
minutes; lanes share the memory bus and the last-level cache, and a busy
chip lowers its clock, so their times run higher and vary more than one
job's. `make bench-compare BENCH_WORKLOADS="nbody dicts"` runs a subset,
whose figures replace the whole results section.

## 3. Results

<!-- results start -->

### Machine and versions

| Item | Value |
| --- | --- |
| Cpu | Intel(R) Core(TM) i5-10310U CPU @ 1.70GHz |
| Logical cpus | 8 |
| Kernel | 6.8.0-149-generic |
| Pinned cores | 1,5 and 2,6 and 3,7 |
| Jobs at once | 3 |
| Date | 2026-11-22 |
| Runs at the main size | 3 |
| CPython 3.12 | Python 3.12.16 |
| CPython 3.13 | Python 3.13.18 |
| CPython 3.14 | Python 3.14.10 |
| CPython 3.15 | Python 3.15.2 |
| PyPy | Python 3.11.15 (194f9f44b505, May 25 2026, 19:34:11) |
| mypyc | mypy 2.4.1 (compiled: yes) |
| Cython | Cython version 3.3.1 |
| Nuitka | 4.3.5 |
| Codon | 0.20.6 |
| TyPy | typy 0.10.0 |
| C (gcc) | gcc (Debian 14.2.0-19) 14.2.0 |
| C (clang) | Debian clang version 19.1.7 (3+b1) |
| C++ (g++) | g++ (Debian 14.2.0-19) 14.2.0 |
| Rust | rustc 1.99.1 (58cf8f9c3 2026-10-13) |
| Go | go version go1.27.4 linux/amd64 |
| Java | openjdk 25.0.5 2026-10-20 LTS |
| JavaScript (Node) | v26.13.0 |

### Summary

Each figure is the geometric mean, over the workloads the peer ran, of its
median time or its peak memory divided by that of C (gcc): 1.00 is as fast,
or as small, as C.

| Peer | Time against C (gcc) | Memory against C (gcc) | Workloads run | Startup (ms) |
| --- | --- | --- | --- | --- |
| CPython 3.12 | 23.27 | 5.92 | 12 of 12 | 18.3 |
| CPython 3.13 | 21.85 | 5.97 | 12 of 12 | 20.3 |
| CPython 3.14 | 20.36 | 5.79 | 12 of 12 | 15.7 |
| CPython 3.15 | 21.01 | 6.37 | 12 of 12 | 24.1 |
| PyPy | 3.16 | 30.72 | 12 of 12 | 33.8 |
| mypyc | 4.47 | 6.23 | 12 of 12 | 23.7 |
| Cython | 14.99 | 6.44 | 12 of 12 | 19.9 |
| Nuitka | 29.19 | 12.68 | 12 of 12 | 162.7 |
| Codon | 1.47 | 5.38 | 12 of 12 | 14.7 |
| TyPy | 7.47 | 1.12 | 12 of 12 | 0.7 |
| C (gcc) | 1.00 | 1.00 | 12 of 12 | 0.7 |
| C (clang) | 1.09 | 1.14 | 12 of 12 | 0.7 |
| C++ (g++) | 1.12 | 1.69 | 12 of 12 | 0.7 |
| Rust | 1.38 | 1.21 | 12 of 12 | 0.9 |
| Go | 1.43 | 1.88 | 12 of 12 | 1.3 |
| Java | 2.02 | 36.03 | 12 of 12 | 46.9 |
| JavaScript (Node) | 2.72 | 29.46 | 12 of 12 | 45.6 |

### nbody, size 4,000,000

| Peer | Median (s) | Fastest to slowest (s) | Against C (gcc) | Peak memory (MiB) | At 1,000,000 (s) | At 2,000,000 (s) | Build (s) | Size (KiB) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CPython 3.12 | 29.8 | 29.2 to 31.7 | 70.08 | 11.2 | 7.587 | 14.6 | 0.00 | 4 |
| CPython 3.13 | 29.8 | 29.4 to 30.6 | 70.08 | 11.3 | 7.274 | 14.8 | 0.00 | 4 |
| CPython 3.14 | 30.4 | 29.3 to 32.4 | 71.56 | 11.0 | 7.535 | 15.2 | 0.00 | 4 |
| CPython 3.15 | 31.6 | 31.2 to 33.9 | 74.31 | 12.2 | 7.860 | 15.7 | 0.00 | 4 |
| PyPy | 1.694 | 1.628 to 1.820 | 3.99 | 59.0 | 0.454 | 0.847 | 0.00 | 4 |
| mypyc | 1.173 | 1.141 to 1.268 | 2.76 | 12.1 | 0.327 | 0.620 | 6.72 | 583 |
| Cython | 48.4 | 46.3 to 50.6 | 113.86 | 12.1 | 12.2 | 23.0 | 6.76 | 1347 |
| Nuitka | 66.3 | 65.6 to 70.9 | 156.10 | 26.1 | 16.6 | 32.8 | 17.35 | 384 |
| Codon | 0.582 | 0.581 to 0.617 | 1.37 | 8.9 | 0.169 | 0.307 | 10.96 | 134 |
| TyPy | 3.207 | 3.146 to 3.470 | 7.55 | 1.6 | 0.796 | 1.578 | 0.13 | 200 |
| C (gcc) | 0.425 | 0.420 to 0.437 | 1.00 | 1.5 | 0.095 | 0.185 | 0.13 | 16 |
| C (clang) | 0.433 | 0.427 to 0.460 | 1.02 | 1.8 | 0.101 | 0.194 | 0.16 | 16 |
| C++ (g++) | 0.396 | 0.393 to 0.400 | 0.93 | 3.5 | 0.094 | 0.188 | 0.60 | 16 |
| Rust | 0.278 | 0.275 to 0.287 | 0.65 | 1.9 | 0.072 | 0.141 | 0.26 | 4465 |
| Go | 0.508 | 0.494 to 0.914 | 1.20 | 2.0 | 0.123 | 0.252 | 7.00 | 2320 |
| Java | 0.629 | 0.618 to 0.684 | 1.48 | 43.4 | 0.225 | 0.358 | 1.02 | 3 |
| JavaScript (Node) | 0.790 | 0.752 to 0.833 | 1.86 | 54.0 | 0.235 | 0.396 | 0.00 | 3 |

### spectralnorm, size 2,000

| Peer | Median (s) | Fastest to slowest (s) | Against C (gcc) | Peak memory (MiB) | At 1,000 (s) | At 1,414 (s) | Build (s) | Size (KiB) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CPython 3.12 | 56.6 | 55.7 to 57.3 | 209.29 | 11.4 | 13.4 | 27.7 | 0.00 | 1 |
| CPython 3.13 | 48.8 | 48.5 to 49.0 | 180.28 | 11.4 | 12.0 | 24.1 | 0.00 | 1 |
| CPython 3.14 | 37.0 | 37.0 to 37.8 | 136.91 | 11.2 | 9.126 | 19.2 | 0.00 | 1 |
| CPython 3.15 | 38.8 | 37.2 to 39.3 | 143.30 | 12.4 | 8.718 | 18.1 | 0.00 | 1 |
| PyPy | 1.713 | 1.709 to 1.782 | 6.33 | 58.8 | 0.460 | 0.871 | 0.00 | 1 |
| mypyc | 3.945 | 3.943 to 4.106 | 14.59 | 12.0 | 0.997 | 1.967 | 6.88 | 548 |
| Cython | 39.9 | 39.3 to 41.8 | 147.59 | 12.4 | 9.853 | 19.2 | 6.04 | 1073 |
| Nuitka | 64.2 | 59.9 to 64.4 | 237.24 | 25.8 | 15.1 | 31.3 | 13.22 | 351 |
| Codon | 0.425 | 0.420 to 0.432 | 1.57 | 9.0 | 0.158 | 0.244 | 13.04 | 130 |
| TyPy | 10.3 | 10.1 to 10.3 | 37.99 | 1.8 | 2.735 | 6.735 | 0.11 | 204 |
| C (gcc) | 0.270 | 0.261 to 0.286 | 1.00 | 1.9 | 0.067 | 0.154 | 0.11 | 16 |
| C (clang) | 0.275 | 0.270 to 0.289 | 1.02 | 1.9 | 0.073 | 0.145 | 0.16 | 16 |
| C++ (g++) | 0.283 | 0.281 to 0.319 | 1.04 | 3.5 | 0.066 | 0.135 | 0.58 | 17 |
| Rust | 0.311 | 0.298 to 0.321 | 1.15 | 2.0 | 0.083 | 0.177 | 0.26 | 4462 |
| Go | 0.430 | 0.378 to 0.721 | 1.59 | 2.0 | 0.090 | 0.180 | 6.97 | 2320 |
| Java | 0.520 | 0.488 to 0.678 | 1.92 | 43.2 | 0.233 | 0.298 | 1.47 | 1 |
| JavaScript (Node) | 0.646 | 0.644 to 0.697 | 2.39 | 53.1 | 0.200 | 0.368 | 0.00 | <1 |

### fannkuchredux, size 10

| Peer | Median (s) | Fastest to slowest (s) | Against C (gcc) | Peak memory (MiB) | At 8 (s) | At 9 (s) | Build (s) | Size (KiB) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CPython 3.12 | 10.3 | 10.2 to 10.4 | 38.07 | 11.1 | 0.108 | 0.914 | 0.00 | 1 |
| CPython 3.13 | 11.2 | 11.0 to 11.3 | 41.37 | 11.0 | 0.111 | 0.975 | 0.00 | 1 |
| CPython 3.14 | 9.338 | 9.239 to 9.632 | 34.62 | 10.6 | 0.095 | 0.822 | 0.00 | 1 |
| CPython 3.15 | 10.6 | 10.4 to 11.0 | 39.41 | 11.9 | 0.107 | 0.985 | 0.00 | 1 |
| PyPy | 1.307 | 1.271 to 1.360 | 4.85 | 58.8 | 0.063 | 0.146 | 0.00 | 1 |
| mypyc | 4.708 | 4.616 to 4.820 | 17.46 | 11.9 | 0.095 | 0.459 | 6.92 | 549 |
| Cython | 8.312 | 8.038 to 12.0 | 30.82 | 12.0 | 0.079 | 0.636 | 5.07 | 835 |
| Nuitka | 12.2 | 10.4 to 13.0 | 45.29 | 25.8 | 0.287 | 1.155 | 14.69 | 338 |
| Codon | 0.375 | 0.373 to 0.380 | 1.39 | 8.8 | 0.012 | 0.052 | 12.50 | 110 |
| TyPy | 7.435 | 7.388 to 10.2 | 27.57 | 1.6 | 0.057 | 0.598 | 0.12 | 178 |
| C (gcc) | 0.270 | 0.265 to 0.272 | 1.00 | 1.5 | 0.003 | 0.027 | 0.11 | 16 |
| C (clang) | 0.300 | 0.295 to 0.318 | 1.11 | 1.8 | 0.004 | 0.033 | 0.12 | 16 |
| C++ (g++) | 0.334 | 0.329 to 0.339 | 1.24 | 3.5 | 0.005 | 0.030 | 0.69 | 17 |
| Rust | 0.315 | 0.312 to 0.319 | 1.17 | 1.8 | 0.004 | 0.034 | 0.25 | 4436 |
| Go | 0.323 | 0.321 to 0.330 | 1.20 | 1.9 | 0.004 | 0.030 | 6.73 | 2319 |
| Java | 0.483 | 0.473 to 0.505 | 1.79 | 42.8 | 0.068 | 0.120 | 0.89 | 2 |
| JavaScript (Node) | 0.909 | 0.894 to 0.937 | 3.37 | 51.7 | 0.052 | 0.128 | 0.00 | 1 |

### binarytrees, size 16

| Peer | Median (s) | Fastest to slowest (s) | Against C (gcc) | Peak memory (MiB) | At 14 (s) | At 15 (s) | Build (s) | Size (KiB) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CPython 3.12 | 9.727 | 9.700 to 9.765 | 18.91 | 31.2 | 1.533 | 3.525 | 0.00 | 1 |
| CPython 3.13 | 7.644 | 7.576 to 7.722 | 14.86 | 35.2 | 1.325 | 2.728 | 0.00 | 1 |
| CPython 3.14 | 6.944 | 6.934 to 7.037 | 13.50 | 34.8 | 1.199 | 2.606 | 0.00 | 1 |
| CPython 3.15 | 7.545 | 7.341 to 7.713 | 14.66 | 36.1 | 1.524 | 2.438 | 0.00 | 1 |
| PyPy | 1.391 | 1.263 to 1.438 | 2.70 | 94.8 | 0.346 | 0.398 | 0.00 | 1 |
| mypyc | 1.072 | 1.063 to 1.085 | 2.08 | 28.0 | 0.287 | 0.501 | 7.00 | 553 |
| Cython | 4.938 | 4.761 to 5.788 | 9.60 | 36.2 | 1.610 | 2.122 | 5.49 | 912 |
| Nuitka | 6.152 | 6.053 to 8.556 | 11.96 | 48.0 | 1.681 | 2.842 | 13.39 | 335 |
| Codon | 0.550 | 0.548 to 0.580 | 1.07 | 23.6 | 0.174 | 0.260 | 11.41 | 106 |
| TyPy | 1.910 | 1.808 to 1.975 | 3.71 | 13.5 | 0.435 | 0.820 | 0.13 | 174 |
| C (gcc) | 0.514 | 0.491 to 0.540 | 1.00 | 9.5 | 0.100 | 0.205 | 0.27 | 20 |
| C (clang) | 0.530 | 0.506 to 0.546 | 1.03 | 9.8 | 0.103 | 0.205 | 0.11 | 16 |
| C++ (g++) | 0.595 | 0.581 to 1.018 | 1.16 | 11.5 | 0.143 | 0.272 | 1.41 | 40 |
| Rust | 0.616 | 0.587 to 0.659 | 1.20 | 9.9 | 0.134 | 0.246 | 0.20 | 4437 |
| Go | 0.922 | 0.906 to 0.999 | 1.79 | 12.9 | 0.120 | 0.312 | 7.24 | 2319 |
| Java | 0.397 | 0.317 to 0.724 | 0.77 | 191.6 | 0.171 | 0.189 | 0.91 | 3 |
| JavaScript (Node) | 0.698 | 0.562 to 0.755 | 1.36 | 100.3 | 0.135 | 0.283 | 0.00 | <1 |

### mandelbrot, size 1,600

| Peer | Median (s) | Fastest to slowest (s) | Against C (gcc) | Peak memory (MiB) | At 800 (s) | At 1,131 (s) | Build (s) | Size (KiB) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CPython 3.12 | 9.298 | 9.013 to 9.725 | 28.96 | 11.1 | 2.287 | 4.389 | 0.00 | <1 |
| CPython 3.13 | 9.113 | 9.007 to 9.258 | 28.38 | 11.1 | 2.223 | 4.601 | 0.00 | <1 |
| CPython 3.14 | 9.428 | 9.236 to 9.444 | 29.37 | 10.5 | 2.382 | 4.799 | 0.00 | <1 |
| CPython 3.15 | 10.4 | 10.0 to 12.8 | 32.32 | 11.9 | 2.655 | 6.709 | 0.00 | <1 |
| PyPy | 1.320 | 1.286 to 1.357 | 4.11 | 58.6 | 0.351 | 0.669 | 0.00 | <1 |
| mypyc | 0.504 | 0.502 to 0.524 | 1.57 | 11.9 | 0.152 | 0.269 | 7.26 | 524 |
| Cython | 0.518 | 0.512 to 0.545 | 1.61 | 11.9 | 0.151 | 0.280 | 4.48 | 564 |
| Nuitka | 18.8 | 17.7 to 21.1 | 58.53 | 26.0 | 4.637 | 9.443 | 17.18 | 326 |
| Codon | 0.315 | 0.313 to 0.338 | 0.98 | 8.9 | 0.084 | 0.156 | 10.34 | 106 |
| TyPy | 1.308 | 0.966 to 1.817 | 4.07 | 1.5 | 0.465 | 0.678 | 0.09 | 170 |
| C (gcc) | 0.321 | 0.303 to 0.323 | 1.00 | 1.5 | 0.086 | 0.163 | 0.07 | 16 |
| C (clang) | 0.344 | 0.344 to 0.344 | 1.07 | 1.8 | 0.086 | 0.172 | 0.12 | 16 |
| C++ (g++) | 0.319 | 0.317 to 0.324 | 1.00 | 1.5 | 0.080 | 0.160 | 0.11 | 16 |
| Rust | 0.325 | 0.321 to 0.328 | 1.01 | 1.9 | 0.083 | 0.168 | 0.19 | 4435 |
| Go | 0.342 | 0.336 to 0.343 | 1.06 | 1.9 | 0.082 | 0.168 | 6.94 | 2302 |
| Java | 0.368 | 0.365 to 0.422 | 1.15 | 40.2 | 0.133 | 0.199 | 1.08 | <1 |
| JavaScript (Node) | 0.375 | 0.374 to 0.389 | 1.17 | 54.3 | 0.132 | 0.224 | 0.00 | <1 |

### fasta, size 2,000,000

| Peer | Median (s) | Fastest to slowest (s) | Against C (gcc) | Peak memory (MiB) | At 500,000 (s) | At 1,000,000 (s) | Build (s) | Size (KiB) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CPython 3.12 | 7.455 | 7.251 to 7.653 | 19.48 | 11.1 | 1.885 | 3.686 | 0.00 | 2 |
| CPython 3.13 | 7.227 | 7.129 to 7.406 | 18.88 | 11.2 | 1.840 | 3.565 | 0.00 | 2 |
| CPython 3.14 | 6.333 | 6.253 to 6.433 | 16.55 | 10.9 | 1.614 | 3.382 | 0.00 | 2 |
| CPython 3.15 | 6.212 | 6.207 to 6.390 | 16.23 | 12.0 | 1.677 | 3.243 | 0.00 | 2 |
| PyPy | 1.242 | 1.206 to 1.288 | 3.25 | 67.5 | 0.346 | 0.618 | 0.00 | 2 |
| mypyc | 3.214 | 3.196 to 3.226 | 8.40 | 12.1 | 0.838 | 1.623 | 7.33 | 595 |
| Cython | 7.939 | 6.746 to 8.473 | 20.74 | 12.1 | 1.670 | 3.573 | 6.42 | 1168 |
| Nuitka | 9.168 | 8.671 to 11.6 | 23.95 | 25.8 | 2.812 | 4.708 | 17.28 | 356 |
| Codon | 1.380 | 1.338 to 1.407 | 3.61 | 9.6 | 0.363 | 0.694 | 11.72 | 118 |
| TyPy | 2.248 | 2.226 to 2.368 | 5.87 | 1.8 | 0.535 | 1.113 | 0.12 | 198 |
| C (gcc) | 0.383 | 0.381 to 0.408 | 1.00 | 1.2 | 0.098 | 0.192 | 0.09 | 16 |
| C (clang) | 0.339 | 0.333 to 0.345 | 0.88 | 1.6 | 0.085 | 0.181 | 0.12 | 16 |
| C++ (g++) | 0.397 | 0.395 to 0.408 | 1.04 | 3.4 | 0.098 | 0.203 | 0.87 | 22 |
| Rust | 0.430 | 0.386 to 0.565 | 1.12 | 2.0 | 0.116 | 0.207 | 0.34 | 4450 |
| Go | 0.435 | 0.416 to 0.436 | 1.14 | 6.1 | 0.100 | 0.200 | 6.56 | 2052 |
| Java | 0.846 | 0.761 to 1.015 | 2.21 | 44.4 | 0.309 | 0.487 | 0.90 | 3 |
| JavaScript (Node) | 1.083 | 1.048 to 1.151 | 2.83 | 123.0 | 0.389 | 0.593 | 0.00 | 2 |

### knucleotide, size 1,000,000

| Peer | Median (s) | Fastest to slowest (s) | Against C (gcc) | Peak memory (MiB) | At 250,000 (s) | At 500,000 (s) | Build (s) | Size (KiB) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CPython 3.12 | 2.588 | 2.479 to 2.697 | 9.27 | 27.3 | 0.756 | 1.270 | 0.00 | 1 |
| CPython 3.13 | 2.555 | 2.474 to 2.610 | 9.15 | 27.8 | 0.650 | 1.312 | 0.00 | 1 |
| CPython 3.14 | 2.415 | 2.381 to 2.452 | 8.65 | 27.7 | 0.597 | 1.229 | 0.00 | 1 |
| CPython 3.15 | 2.167 | 2.049 to 2.279 | 7.76 | 28.6 | 0.605 | 1.135 | 0.00 | 1 |
| PyPy | 1.104 | 1.101 to 1.307 | 3.95 | 155.4 | 0.321 | 0.567 | 0.00 | 1 |
| mypyc | 2.155 | 1.812 to 2.168 | 7.72 | 28.6 | 0.769 | 1.530 | 7.20 | 582 |
| Cython | 1.753 | 1.623 to 2.062 | 6.28 | 29.1 | 0.450 | 1.587 | 6.28 | 1081 |
| Nuitka | 2.930 | 2.909 to 3.191 | 10.50 | 39.6 | 0.977 | 1.505 | 14.57 | 347 |
| Codon | 0.523 | 0.513 to 0.888 | 1.87 | 79.6 | 0.181 | 0.286 | 14.63 | 155 |
| TyPy | 1.804 | 1.627 to 1.804 | 6.46 | 16.5 | 0.425 | 0.833 | 0.34 | 249 |
| C (gcc) | 0.279 | 0.266 to 0.295 | 1.00 | 18.4 | 0.107 | 0.173 | 0.16 | 16 |
| C (clang) | 0.284 | 0.271 to 0.341 | 1.02 | 18.7 | 0.097 | 0.158 | 0.23 | 16 |
| C++ (g++) | 0.552 | 0.531 to 0.561 | 1.98 | 12.4 | 0.273 | 0.329 | 1.35 | 23 |
| Rust | 0.582 | 0.558 to 0.869 | 2.08 | 15.3 | 0.146 | 0.337 | 0.46 | 4498 |
| Go | 0.413 | 0.375 to 0.414 | 1.48 | 17.1 | 0.124 | 0.200 | 7.16 | 2351 |
| Java | 1.234 | 1.080 to 1.600 | 4.42 | 218.4 | 0.466 | 0.653 | 1.94 | 4 |
| JavaScript (Node) | 2.337 | 1.872 to 2.998 | 8.37 | 139.8 | 0.563 | 0.980 | 0.00 | 1 |

### strings, size 500,000

| Peer | Median (s) | Fastest to slowest (s) | Against C (gcc) | Peak memory (MiB) | At 125,000 (s) | At 250,000 (s) | Build (s) | Size (KiB) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CPython 3.12 | 1.497 | 1.459 to 1.575 | 5.12 | 11.2 | 0.354 | 0.688 | 0.00 | <1 |
| CPython 3.13 | 1.342 | 1.271 to 1.393 | 4.59 | 11.2 | 0.319 | 0.633 | 0.00 | <1 |
| CPython 3.14 | 1.218 | 1.190 to 1.372 | 4.17 | 10.5 | 0.316 | 0.621 | 0.00 | <1 |
| CPython 3.15 | 1.335 | 1.269 to 1.340 | 4.57 | 12.0 | 0.320 | 0.609 | 0.00 | <1 |
| PyPy | 0.547 | 0.520 to 0.561 | 1.87 | 58.8 | 0.184 | 0.295 | 0.00 | <1 |
| mypyc | 0.869 | 0.727 to 0.936 | 2.98 | 12.0 | 0.302 | 0.421 | 6.60 | 540 |
| Cython | 0.946 | 0.896 to 0.981 | 3.24 | 12.1 | 0.258 | 0.490 | 4.56 | 686 |
| Nuitka | 1.348 | 1.340 to 1.948 | 4.61 | 26.0 | 0.614 | 1.282 | 15.78 | 322 |
| Codon | 0.410 | 0.407 to 0.503 | 1.40 | 9.6 | 0.192 | 0.462 | 14.83 | 154 |
| TyPy | 0.892 | 0.848 to 1.176 | 3.05 | 1.8 | 0.206 | 0.394 | 0.09 | 237 |
| C (gcc) | 0.292 | 0.275 to 0.307 | 1.00 | 1.5 | 0.082 | 0.153 | 0.10 | 16 |
| C (clang) | 0.304 | 0.236 to 0.390 | 1.04 | 1.8 | 0.103 | 0.194 | 0.20 | 16 |
| C++ (g++) | 0.450 | 0.419 to 0.451 | 1.54 | 3.5 | 0.105 | 0.209 | 2.03 | 21 |
| Rust | 0.499 | 0.485 to 0.513 | 1.71 | 2.0 | 0.121 | 0.226 | 0.56 | 4448 |
| Go | 0.715 | 0.584 to 0.834 | 2.45 | 8.2 | 0.152 | 0.253 | 7.39 | 2353 |
| Java | 1.088 | 1.029 to 1.369 | 3.72 | 195.1 | 0.667 | 0.787 | 1.49 | 2 |
| JavaScript (Node) | 1.696 | 1.553 to 2.132 | 5.81 | 54.6 | 0.363 | 0.673 | 0.00 | <1 |

### dicts, size 4,000,000

| Peer | Median (s) | Fastest to slowest (s) | Against C (gcc) | Peak memory (MiB) | At 1,000,000 (s) | At 2,000,000 (s) | Build (s) | Size (KiB) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CPython 3.12 | 2.730 | 2.614 to 3.070 | 5.22 | 11.9 | 0.701 | 1.313 | 0.00 | <1 |
| CPython 3.13 | 2.788 | 2.724 to 2.996 | 5.33 | 12.0 | 0.723 | 1.339 | 0.00 | <1 |
| CPython 3.14 | 2.676 | 2.575 to 2.937 | 5.12 | 11.5 | 0.698 | 1.297 | 0.00 | <1 |
| CPython 3.15 | 2.831 | 2.764 to 3.019 | 5.41 | 12.6 | 0.701 | 1.310 | 0.00 | <1 |
| PyPy | 0.753 | 0.719 to 0.785 | 1.44 | 67.5 | 0.212 | 0.401 | 0.00 | <1 |
| mypyc | 1.697 | 1.643 to 1.775 | 3.24 | 12.8 | 0.409 | 0.754 | 6.81 | 547 |
| Cython | 2.394 | 2.359 to 2.432 | 4.58 | 13.0 | 0.529 | 0.976 | 4.80 | 715 |
| Nuitka | 4.422 | 3.590 to 4.710 | 8.45 | 26.0 | 0.923 | 1.615 | 19.03 | 326 |
| Codon | 0.627 | 0.593 to 0.649 | 1.20 | 10.4 | 0.222 | 0.628 | 10.71 | 118 |
| TyPy | 1.645 | 1.613 to 1.938 | 3.14 | 2.2 | 0.452 | 0.848 | 0.08 | 205 |
| C (gcc) | 0.523 | 0.460 to 0.564 | 1.00 | 2.0 | 0.113 | 0.214 | 0.11 | 16 |
| C (clang) | 0.644 | 0.475 to 0.953 | 1.23 | 2.4 | 0.290 | 0.496 | 0.14 | 16 |
| C++ (g++) | 0.420 | 0.408 to 0.433 | 0.80 | 4.1 | 0.108 | 0.208 | 2.61 | 31 |
| Rust | 0.649 | 0.613 to 0.824 | 1.24 | 2.6 | 0.444 | 0.486 | 0.86 | 4473 |
| Go | 0.556 | 0.526 to 0.592 | 1.06 | 7.2 | 0.141 | 0.269 | 7.07 | 2346 |
| Java | 0.732 | 0.683 to 1.184 | 1.40 | 183.1 | 0.388 | 0.609 | 1.02 | 3 |
| JavaScript (Node) | 1.016 | 0.947 to 1.257 | 1.94 | 54.6 | 0.321 | 0.522 | 0.00 | <1 |

### dispatch, size 50,000

| Peer | Median (s) | Fastest to slowest (s) | Against C (gcc) | Peak memory (MiB) | At 12,500 (s) | At 25,000 (s) | Build (s) | Size (KiB) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CPython 3.12 | 5.934 | 5.877 to 6.156 | 31.61 | 11.2 | 1.414 | 3.002 | 0.00 | 1 |
| CPython 3.13 | 5.336 | 5.315 to 5.569 | 28.43 | 11.2 | 1.398 | 2.555 | 0.00 | 1 |
| CPython 3.14 | 5.625 | 5.590 to 5.674 | 29.97 | 10.9 | 1.593 | 2.866 | 0.00 | 1 |
| CPython 3.15 | 5.642 | 5.399 to 5.753 | 30.06 | 12.0 | 1.274 | 2.957 | 0.00 | 1 |
| PyPy | 0.553 | 0.529 to 0.639 | 2.95 | 58.5 | 0.158 | 0.284 | 0.00 | 1 |
| mypyc | 0.493 | 0.486 to 0.597 | 2.63 | 12.0 | 0.130 | 0.242 | 6.85 | 579 |
| Cython | 6.467 | 5.790 to 7.806 | 34.46 | 12.2 | 1.616 | 3.239 | 5.80 | 1030 |
| Nuitka | 9.185 | 8.409 to 12.3 | 48.94 | 26.1 | 2.388 | 4.560 | 16.09 | 361 |
| Codon | 0.167 | 0.167 to 0.171 | 0.89 | 9.1 | 0.059 | 0.106 | 12.66 | 130 |
| TyPy | 1.447 | 1.425 to 1.505 | 7.71 | 1.8 | 0.502 | 0.722 | 0.10 | 196 |
| C (gcc) | 0.188 | 0.187 to 0.205 | 1.00 | 1.6 | 0.052 | 0.095 | 0.08 | 16 |
| C (clang) | 0.294 | 0.292 to 0.297 | 1.57 | 2.0 | 0.070 | 0.145 | 0.12 | 16 |
| C++ (g++) | 0.193 | 0.185 to 0.205 | 1.03 | 3.5 | 0.047 | 0.091 | 1.11 | 18 |
| Rust | 0.287 | 0.220 to 0.388 | 1.53 | 2.0 | 0.059 | 0.114 | 0.39 | 4462 |
| Go | 0.206 | 0.202 to 0.218 | 1.10 | 1.9 | 0.053 | 0.104 | 7.38 | 2319 |
| Java | 0.729 | 0.646 to 0.994 | 3.88 | 44.5 | 0.325 | 0.462 | 1.06 | 3 |
| JavaScript (Node) | 0.404 | 0.398 to 0.427 | 2.15 | 54.3 | 0.137 | 0.178 | 0.00 | 1 |

### recursion, size 36

| Peer | Median (s) | Fastest to slowest (s) | Against C (gcc) | Peak memory (MiB) | At 33 (s) | At 34 (s) | Build (s) | Size (KiB) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CPython 3.12 | 3.854 | 3.709 to 3.949 | 69.85 | 11.1 | 1.018 | 1.488 | 0.00 | <1 |
| CPython 3.13 | 3.374 | 3.347 to 3.474 | 61.15 | 11.0 | 0.796 | 1.383 | 0.00 | <1 |
| CPython 3.14 | 3.139 | 3.067 to 3.207 | 56.90 | 10.5 | 0.698 | 1.131 | 0.00 | <1 |
| CPython 3.15 | 2.911 | 2.838 to 3.108 | 52.75 | 11.9 | 0.658 | 1.101 | 0.00 | <1 |
| PyPy | 0.313 | 0.311 to 0.313 | 5.67 | 62.5 | 0.117 | 0.166 | 0.00 | <1 |
| mypyc | 0.341 | 0.259 to 0.359 | 6.17 | 11.9 | 0.056 | 0.079 | 6.28 | 522 |
| Cython | 1.905 | 1.789 to 2.289 | 34.52 | 11.9 | 0.419 | 0.624 | 4.08 | 554 |
| Nuitka | 3.398 | 3.151 to 4.855 | 61.59 | 25.8 | 0.918 | 1.316 | 17.29 | 314 |
| Codon | 0.105 | 0.104 to 0.106 | 1.91 | 8.9 | 0.030 | 0.043 | 9.43 | 106 |
| TyPy | 1.043 | 0.997 to 2.436 | 18.90 | 1.6 | 0.436 | 0.428 | 0.07 | 174 |
| C (gcc) | 0.055 | 0.055 to 0.072 | 1.00 | 1.5 | 0.013 | 0.019 | 0.09 | 16 |
| C (clang) | 0.065 | 0.063 to 0.069 | 1.17 | 1.6 | 0.017 | 0.027 | 0.10 | 16 |
| C++ (g++) | 0.064 | 0.054 to 0.070 | 1.16 | 1.5 | 0.022 | 0.022 | 0.13 | 16 |
| Rust | 0.122 | 0.122 to 0.124 | 2.22 | 1.9 | 0.029 | 0.046 | 0.30 | 4435 |
| Go | 0.153 | 0.150 to 0.153 | 2.77 | 1.9 | 0.037 | 0.059 | 7.93 | 2302 |
| Java | 0.199 | 0.150 to 0.253 | 3.60 | 39.6 | 0.144 | 0.164 | 0.99 | <1 |
| JavaScript (Node) | 0.490 | 0.471 to 0.499 | 8.89 | 51.5 | 0.161 | 0.327 | 0.00 | <1 |

### parser, size 150,000

| Peer | Median (s) | Fastest to slowest (s) | Against C (gcc) | Peak memory (MiB) | At 37,500 (s) | At 75,000 (s) | Build (s) | Size (KiB) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CPython 3.12 | 3.975 | 3.832 to 4.475 | 7.75 | 11.1 | 0.960 | 2.071 | 0.00 | 2 |
| CPython 3.13 | 3.755 | 3.649 to 3.985 | 7.32 | 11.0 | 1.091 | 1.873 | 0.00 | 2 |
| CPython 3.14 | 3.723 | 3.526 to 3.732 | 7.25 | 10.8 | 0.840 | 1.863 | 0.00 | 2 |
| CPython 3.15 | 3.871 | 3.654 to 3.927 | 7.54 | 11.8 | 0.901 | 2.034 | 0.00 | 2 |
| PyPy | 0.654 | 0.631 to 0.678 | 1.27 | 62.5 | 0.308 | 0.404 | 0.00 | 2 |
| mypyc | 1.413 | 1.318 to 1.621 | 2.75 | 12.0 | 0.386 | 0.691 | 7.19 | 597 |
| Cython | 3.603 | 3.037 to 4.996 | 7.02 | 12.2 | 0.876 | 1.735 | 7.03 | 1300 |
| Nuitka | 5.664 | 5.560 to 7.617 | 11.04 | 26.1 | 1.952 | 2.807 | 12.66 | 360 |
| Codon | 0.886 | 0.847 to 0.904 | 1.73 | 9.8 | 0.256 | 0.448 | 13.07 | 134 |
| TyPy | 2.433 | 2.388 to 2.461 | 4.74 | 1.9 | 0.588 | 1.135 | 0.15 | 229 |
| C (gcc) | 0.513 | 0.478 to 0.520 | 1.00 | 1.5 | 0.122 | 0.232 | 0.12 | 16 |
| C (clang) | 0.518 | 0.509 to 0.562 | 1.01 | 1.8 | 0.124 | 0.240 | 0.15 | 16 |
| C++ (g++) | 0.483 | 0.472 to 0.623 | 0.94 | 3.5 | 0.149 | 0.316 | 1.11 | 29 |
| Rust | 1.340 | 1.176 to 1.399 | 2.61 | 2.0 | 0.223 | 0.667 | 0.42 | 4445 |
| Go | 0.632 | 0.626 to 0.679 | 1.23 | 7.5 | 0.162 | 0.315 | 7.06 | 2314 |
| Java | 0.739 | 0.724 to 0.757 | 1.44 | 192.4 | 0.416 | 0.599 | 1.72 | 3 |
| JavaScript (Node) | 0.708 | 0.633 to 0.732 | 1.38 | 56.1 | 0.221 | 0.372 | 0.00 | 2 |

<!-- results end -->

## 4. Observations

These notes read the figures of 2026-11-22 above, measured with three jobs
at once. Jobs that share the chip run slower than one alone: on the same
day, nbody took 31.6 seconds under CPython 3.15 here and 23.0 seconds as
the only job, and TyPy's took 3.21 and 2.30 seconds. The ratios move less
than the times: TyPy's nbody took 0.102 of CPython 3.15's time here and
0.100 alone, and 7.5 times C's here and 8.6 times alone.

### 4.1 Where TyPy stands

TyPy's programs take 7.5 times as long as C's, as the geometric mean of the
twelve workloads, against 20 to 23 times for CPython 3.12 to 3.15; on
2026-11-16 TyPy's mean was 14 times. Among the compilers of Python, Codon
(1.5), PyPy (3.2) and mypyc (4.5) are faster, and Cython in pure Python
mode (15) and Nuitka (29) are slower. NFR-22 asks for 5 times by 0.12 and
NFR-03 for 1.5 times by 1.0.

Every workload built with `--release` runs faster than with CPython 3.15,
as NFR-21 asks of 0.11: from 0.10 of CPython's time for nbody to 0.83 for
knucleotide, whose three runs all ended before CPython's fastest. The
geometric mean is 0.36 of CPython's time.

TyPy is closest to C where a program works on strings, dicts, objects it
allocates and frees, or floating-point numbers: strings at 3.1 times, dicts
at 3.1, binarytrees at 3.7 and mandelbrot at 4.1. It is furthest where the
work is integer arithmetic and list indexing: spectralnorm at 38 times,
fannkuchredux at 28 and recursion at 19.

### 4.2 What changed since 2026-11-16

On 2026-11-16, five workloads ran slower than with CPython 3.15. Each is
now faster:

| Workload | TyPy against CPython 3.15 on 2026-11-16 | On 2026-11-22 |
| --- | --- | --- |
| strings | 3.44 | 0.67 |
| knucleotide | 2.38 | 0.83 |
| dicts | 1.35 | 0.58 |
| parser | 1.11 | 0.63 |
| fannkuchredux | 1.09 | 0.70 |

Four changes to the runtime account for it. The runtime that debug and
`--release` builds link is compiled for speed rather than size, so its
hashing, dict lookups and string methods are inlined and unrolled
(ADR-0100). The methods of a `str` of one byte a code point, and its
slices, work on runs of bytes rather than one code point at a time. A
`str` hashes and compares by its stored data and keeps its hash once
computed (ADR-0101). And every `str` of one character up to U+00FF is one
shared object, as in CPython.

### 4.3 Why integer work is slow

The optimised code TyPy writes for `fibonacci` calls the runtime for each
integer operation, `typy_rt_int_compare`, `typy_rt_int_subtract` and
`typy_rt_int_add`, changes the reference count of each integer it holds, and
checks the stack on each call. An `int` is arbitrary precision, held in place
while it is small, but the test for a small value and its arithmetic are never
written in place, and the runtime is linked as an archive, so the optimiser
cannot see into those calls. Writing the small-integer case in place, with a
call to the runtime only when a result leaves it, is the largest single gain
these figures point to; list indexing and the reference counts of values held
in place would follow.

### 4.4 Memory

TyPy's peak memory is 1.12 times C's as the geometric mean, the smallest of
the Python peers, which hold an interpreter or a runtime of several megabytes
before the program starts: 6 times for CPython and 31 for PyPy. Its startup,
0.7 ms, is that of C. knucleotide peaks at 16.5 MiB against CPython's 28.6;
on 2026-11-16 it peaked at 60 MiB, when each one-character string was a new
object.

### 4.5 Building

TyPy builds each workload in 0.13 seconds as the mean, as fast as gcc, while
the other compilers of Python take 5.6 to 15.7 seconds and Go 7.1 from a
clean cache; three jobs at once lengthen these as they lengthen the runs.
Its executables are 201 KiB as the mean, against 16 KiB for C, since each
holds the part of TyPy's runtime it uses, linked statically; Rust's, built
by `rustc` without stripping, are 4.3 MiB, and Go's 2.2 MiB.

### 4.6 What the figures do not show

The machine is a laptop processor whose clock follows its temperature, and
three jobs at once share its memory bus and last-level cache, so two runs
of one program differ by a few percent and now and then by more; the
fastest and slowest of the three runs are given beside each median. Every
program is single threaded, and Java's and Node's figures include their
JIT compilers' warm-up, which a long-running server would not pay for each
request.
