# TyPy benchmark report

| Field | Value |
| --- | --- |
| Document | TyPy benchmark report |
| Version | 0.1 |
| Status | Draft |
| Owner | typy-lang contributors |
| Last updated | 2026-11-16 |

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

Each run is pinned to the same two cores with `taskset`, its output goes to
`/dev/null`, and `tests/bench/measure.c` forks it and reports its wall time
and its peak resident size from `wait4`. A JIT compiler's warm-up is part of
each run, since each run is a new process; startup is measured apart, as the
median of ten runs of a program that prints one line. `tests/bench/report.py`
writes the tables below from the raw figures. A peer that cannot build or
run a workload is listed with the reason.

### 2.4 Reproducing the figures

`make bench-peers` installs the peers the development image lacks, once, at
the versions above, each checked against its published checksum. `make
bench-compare` then builds the release compiler, runs every workload, which
takes about an hour, and rewrites the results below. `make bench-compare
BENCH_WORKLOADS="nbody dicts"` runs a subset, whose figures replace the
whole results section.

## 3. Results

<!-- results start -->

### Machine and versions

| Item | Value |
| --- | --- |
| Cpu | Intel(R) Core(TM) i5-10310U CPU @ 1.70GHz |
| Logical cpus | 8 |
| Kernel | 6.8.0-138-generic |
| Pinned cores | 2,3 |
| Date | 2026-11-16 |
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
| TyPy | typy 0.4.0 |
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
| CPython 3.12 | 25.57 | 5.97 | 12 of 12 | 13.6 |
| CPython 3.13 | 23.77 | 5.98 | 12 of 12 | 15.3 |
| CPython 3.14 | 22.18 | 6.13 | 12 of 12 | 17.0 |
| CPython 3.15 | 22.09 | 6.39 | 12 of 12 | 16.9 |
| PyPy | 3.21 | 30.85 | 12 of 12 | 25.9 |
| mypyc | 4.35 | 6.58 | 12 of 12 | 23.2 |
| Cython | 15.48 | 6.79 | 12 of 12 | 21.7 |
| Nuitka | 30.02 | 12.82 | 12 of 12 | 200.6 |
| Codon | 1.33 | 5.47 | 12 of 12 | 5.7 |
| TyPy | 14.27 | 1.22 | 12 of 12 | 0.6 |
| C (gcc) | 1.00 | 1.00 | 12 of 12 | 0.5 |
| C (clang) | 1.04 | 1.17 | 12 of 12 | 0.6 |
| C++ (g++) | 1.13 | 1.72 | 12 of 12 | 0.5 |
| Rust | 1.25 | 1.21 | 12 of 12 | 0.7 |
| Go | 1.44 | 1.98 | 12 of 12 | 1.2 |
| Java | 1.69 | 35.76 | 12 of 12 | 30.7 |
| JavaScript (Node) | 2.48 | 29.62 | 12 of 12 | 31.6 |

### nbody, size 4,000,000

| Peer | Median (s) | Fastest to slowest (s) | Against C (gcc) | Peak memory (MiB) | At 1,000,000 (s) | At 2,000,000 (s) | Build (s) | Size (KiB) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CPython 3.12 | 24.7 | 22.6 to 26.3 | 93.83 | 11.4 | 5.497 | 11.2 | 0.00 | 4 |
| CPython 3.13 | 21.6 | 20.7 to 23.1 | 81.98 | 11.3 | 5.078 | 10.1 | 0.00 | 4 |
| CPython 3.14 | 21.1 | 20.9 to 21.8 | 79.82 | 11.8 | 5.244 | 10.5 | 0.00 | 4 |
| CPython 3.15 | 21.4 | 21.3 to 21.9 | 81.07 | 12.4 | 5.283 | 10.6 | 0.00 | 4 |
| PyPy | 1.103 | 1.099 to 1.155 | 4.18 | 59.2 | 0.296 | 0.562 | 0.00 | 4 |
| mypyc | 0.806 | 0.793 to 0.839 | 3.05 | 12.8 | 0.235 | 0.457 | 4.70 | 583 |
| Cython | 31.2 | 30.7 to 32.2 | 118.44 | 12.9 | 7.932 | 15.9 | 4.65 | 1347 |
| Nuitka | 45.7 | 45.1 to 46.9 | 173.36 | 26.1 | 11.3 | 22.5 | 8.50 | 384 |
| Codon | 0.358 | 0.353 to 0.414 | 1.36 | 8.9 | 0.092 | 0.184 | 7.34 | 134 |
| TyPy | 3.079 | 3.020 to 3.134 | 11.68 | 1.6 | 0.753 | 1.516 | 0.09 | 171 |
| C (gcc) | 0.264 | 0.237 to 0.273 | 1.00 | 1.5 | 0.066 | 0.157 | 0.09 | 16 |
| C (clang) | 0.263 | 0.253 to 0.272 | 1.00 | 1.9 | 0.068 | 0.136 | 0.13 | 16 |
| C++ (g++) | 0.256 | 0.251 to 0.272 | 0.97 | 3.6 | 0.064 | 0.130 | 0.39 | 16 |
| Rust | 0.173 | 0.173 to 0.179 | 0.66 | 1.9 | 0.043 | 0.090 | 0.19 | 4465 |
| Go | 0.338 | 0.337 to 0.391 | 1.28 | 2.0 | 0.084 | 0.161 | 3.95 | 2320 |
| Java | 0.423 | 0.414 to 0.440 | 1.60 | 43.6 | 0.154 | 0.225 | 0.56 | 3 |
| JavaScript (Node) | 0.521 | 0.510 to 0.543 | 1.97 | 54.2 | 0.144 | 0.268 | 0.00 | 3 |

### spectralnorm, size 2,000

| Peer | Median (s) | Fastest to slowest (s) | Against C (gcc) | Peak memory (MiB) | At 1,000 (s) | At 1,414 (s) | Build (s) | Size (KiB) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CPython 3.12 | 38.4 | 36.1 to 38.9 | 225.08 | 11.5 | 9.050 | 19.1 | 0.00 | 1 |
| CPython 3.13 | 33.1 | 32.9 to 33.1 | 194.07 | 11.3 | 8.195 | 16.7 | 0.00 | 1 |
| CPython 3.14 | 25.7 | 25.3 to 25.7 | 150.23 | 11.8 | 6.482 | 12.9 | 0.00 | 1 |
| CPython 3.15 | 25.1 | 24.9 to 25.2 | 146.80 | 12.4 | 5.976 | 12.2 | 0.00 | 1 |
| PyPy | 1.090 | 1.084 to 1.153 | 6.38 | 59.0 | 0.287 | 0.550 | 0.00 | 1 |
| mypyc | 2.672 | 2.655 to 2.700 | 15.65 | 12.7 | 0.685 | 1.327 | 4.57 | 548 |
| Cython | 26.3 | 26.2 to 27.0 | 154.24 | 12.9 | 6.538 | 13.4 | 3.85 | 1073 |
| Nuitka | 40.6 | 40.4 to 40.9 | 237.77 | 26.1 | 10.3 | 20.0 | 8.25 | 351 |
| Codon | 0.252 | 0.250 to 0.255 | 1.47 | 9.1 | 0.067 | 0.131 | 7.11 | 130 |
| TyPy | 11.4 | 11.2 to 11.5 | 66.52 | 1.8 | 2.853 | 5.684 | 0.07 | 175 |
| C (gcc) | 0.171 | 0.167 to 0.178 | 1.00 | 2.0 | 0.040 | 0.100 | 0.07 | 16 |
| C (clang) | 0.170 | 0.169 to 0.177 | 0.99 | 2.0 | 0.049 | 0.082 | 0.10 | 16 |
| C++ (g++) | 0.172 | 0.167 to 0.174 | 1.01 | 3.5 | 0.044 | 0.088 | 0.38 | 17 |
| Rust | 0.196 | 0.194 to 0.199 | 1.15 | 2.0 | 0.050 | 0.093 | 0.17 | 4462 |
| Go | 0.274 | 0.237 to 0.305 | 1.60 | 2.0 | 0.061 | 0.121 | 3.82 | 2320 |
| Java | 0.301 | 0.292 to 0.303 | 1.77 | 43.2 | 0.124 | 0.170 | 0.53 | 1 |
| JavaScript (Node) | 0.442 | 0.427 to 0.446 | 2.59 | 53.7 | 0.136 | 0.231 | 0.00 | <1 |

### fannkuchredux, size 10

| Peer | Median (s) | Fastest to slowest (s) | Against C (gcc) | Peak memory (MiB) | At 8 (s) | At 9 (s) | Build (s) | Size (KiB) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CPython 3.12 | 7.063 | 7.031 to 7.067 | 39.42 | 11.1 | 0.070 | 0.611 | 0.00 | 1 |
| CPython 3.13 | 7.597 | 7.579 to 7.599 | 42.40 | 10.9 | 0.080 | 0.654 | 0.00 | 1 |
| CPython 3.14 | 6.462 | 6.394 to 6.470 | 36.06 | 11.3 | 0.068 | 0.560 | 0.00 | 1 |
| CPython 3.15 | 7.112 | 7.040 to 7.114 | 39.69 | 11.9 | 0.074 | 0.628 | 0.00 | 1 |
| PyPy | 0.833 | 0.816 to 0.846 | 4.65 | 59.0 | 0.043 | 0.097 | 0.00 | 1 |
| mypyc | 3.148 | 3.100 to 3.158 | 17.57 | 12.6 | 0.044 | 0.281 | 4.59 | 549 |
| Cython | 5.404 | 5.303 to 5.420 | 30.16 | 12.8 | 0.060 | 0.483 | 3.43 | 835 |
| Nuitka | 7.079 | 7.006 to 7.210 | 39.50 | 26.3 | 0.251 | 0.816 | 8.18 | 338 |
| Codon | 0.243 | 0.235 to 0.248 | 1.35 | 9.0 | 0.007 | 0.026 | 6.83 | 110 |
| TyPy | 7.754 | 7.682 to 7.816 | 43.27 | 1.6 | 0.062 | 0.644 | 0.15 | 160 |
| C (gcc) | 0.179 | 0.174 to 0.180 | 1.00 | 1.5 | 0.002 | 0.016 | 0.06 | 16 |
| C (clang) | 0.198 | 0.193 to 0.201 | 1.10 | 1.8 | 0.002 | 0.017 | 0.09 | 16 |
| C++ (g++) | 0.219 | 0.218 to 0.227 | 1.22 | 3.5 | 0.003 | 0.020 | 0.43 | 17 |
| Rust | 0.209 | 0.201 to 0.212 | 1.17 | 1.9 | 0.002 | 0.018 | 0.15 | 4436 |
| Go | 0.242 | 0.217 to 0.252 | 1.35 | 2.0 | 0.003 | 0.019 | 3.89 | 2319 |
| Java | 0.312 | 0.311 to 0.316 | 1.74 | 43.0 | 0.039 | 0.064 | 0.53 | 2 |
| JavaScript (Node) | 0.606 | 0.604 to 0.632 | 3.38 | 51.7 | 0.033 | 0.080 | 0.00 | 1 |

### binarytrees, size 16

| Peer | Median (s) | Fastest to slowest (s) | Against C (gcc) | Peak memory (MiB) | At 14 (s) | At 15 (s) | Build (s) | Size (KiB) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CPython 3.12 | 6.886 | 6.833 to 6.903 | 20.71 | 31.2 | 1.007 | 2.216 | 0.00 | 1 |
| CPython 3.13 | 5.641 | 5.602 to 5.649 | 16.97 | 35.2 | 0.836 | 1.957 | 0.00 | 1 |
| CPython 3.14 | 5.029 | 4.988 to 5.037 | 15.13 | 35.4 | 0.748 | 1.645 | 0.00 | 1 |
| CPython 3.15 | 4.950 | 4.929 to 4.975 | 14.89 | 36.1 | 0.738 | 1.674 | 0.00 | 1 |
| PyPy | 0.836 | 0.829 to 0.873 | 2.51 | 95.0 | 0.201 | 0.212 | 0.00 | 1 |
| mypyc | 0.722 | 0.707 to 0.730 | 2.17 | 28.7 | 0.158 | 0.289 | 4.54 | 553 |
| Cython | 3.054 | 3.027 to 3.066 | 9.18 | 36.9 | 0.653 | 1.274 | 3.50 | 912 |
| Nuitka | 4.047 | 4.008 to 4.070 | 12.17 | 48.5 | 0.974 | 1.854 | 8.41 | 335 |
| Codon | 0.345 | 0.345 to 0.351 | 1.04 | 23.6 | 0.077 | 0.187 | 6.38 | 106 |
| TyPy | 1.465 | 1.464 to 1.557 | 4.41 | 13.6 | 0.307 | 0.611 | 0.07 | 160 |
| C (gcc) | 0.332 | 0.305 to 0.344 | 1.00 | 9.5 | 0.066 | 0.130 | 0.17 | 20 |
| C (clang) | 0.318 | 0.317 to 0.323 | 0.96 | 9.9 | 0.066 | 0.129 | 0.08 | 16 |
| C++ (g++) | 0.375 | 0.374 to 0.375 | 1.13 | 11.6 | 0.079 | 0.152 | 0.87 | 40 |
| Rust | 0.407 | 0.388 to 0.436 | 1.22 | 9.8 | 0.079 | 0.159 | 0.13 | 4437 |
| Go | 0.451 | 0.427 to 0.498 | 1.36 | 14.8 | 0.068 | 0.158 | 3.90 | 2319 |
| Java | 0.175 | 0.174 to 0.179 | 0.53 | 190.3 | 0.079 | 0.121 | 0.59 | 3 |
| JavaScript (Node) | 0.278 | 0.276 to 0.278 | 0.84 | 100.4 | 0.091 | 0.130 | 0.00 | <1 |

### mandelbrot, size 1,600

| Peer | Median (s) | Fastest to slowest (s) | Against C (gcc) | Peak memory (MiB) | At 800 (s) | At 1,131 (s) | Build (s) | Size (KiB) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CPython 3.12 | 6.184 | 6.094 to 6.281 | 33.54 | 11.2 | 1.549 | 3.069 | 0.00 | <1 |
| CPython 3.13 | 6.211 | 6.115 to 6.239 | 33.69 | 11.2 | 1.535 | 3.087 | 0.00 | <1 |
| CPython 3.14 | 6.196 | 6.150 to 6.245 | 33.60 | 11.2 | 1.602 | 3.091 | 0.00 | <1 |
| CPython 3.15 | 6.444 | 6.362 to 6.492 | 34.95 | 12.0 | 1.624 | 3.205 | 0.00 | <1 |
| PyPy | 0.827 | 0.816 to 0.834 | 4.48 | 58.8 | 0.219 | 0.432 | 0.00 | <1 |
| mypyc | 0.354 | 0.344 to 0.358 | 1.92 | 12.6 | 0.108 | 0.185 | 4.47 | 524 |
| Cython | 0.341 | 0.339 to 0.350 | 1.85 | 12.7 | 0.098 | 0.183 | 2.62 | 564 |
| Nuitka | 12.0 | 11.8 to 12.5 | 65.26 | 26.1 | 3.158 | 6.037 | 8.16 | 326 |
| Codon | 0.192 | 0.191 to 0.202 | 1.04 | 9.0 | 0.052 | 0.100 | 6.41 | 106 |
| TyPy | 0.866 | 0.861 to 0.893 | 4.70 | 1.5 | 0.220 | 0.429 | 0.06 | 152 |
| C (gcc) | 0.184 | 0.184 to 0.189 | 1.00 | 1.5 | 0.046 | 0.094 | 0.04 | 16 |
| C (clang) | 0.199 | 0.196 to 0.203 | 1.08 | 1.8 | 0.052 | 0.098 | 0.08 | 16 |
| C++ (g++) | 0.195 | 0.185 to 0.207 | 1.06 | 1.5 | 0.052 | 0.092 | 0.06 | 16 |
| Rust | 0.196 | 0.188 to 0.198 | 1.06 | 1.9 | 0.048 | 0.095 | 0.13 | 4435 |
| Go | 0.239 | 0.206 to 0.255 | 1.30 | 2.0 | 0.053 | 0.101 | 3.94 | 2302 |
| Java | 0.222 | 0.220 to 0.231 | 1.21 | 40.1 | 0.073 | 0.124 | 0.45 | <1 |
| JavaScript (Node) | 0.231 | 0.227 to 0.231 | 1.25 | 54.5 | 0.083 | 0.127 | 0.00 | <1 |

### fasta, size 2,000,000

| Peer | Median (s) | Fastest to slowest (s) | Against C (gcc) | Peak memory (MiB) | At 500,000 (s) | At 1,000,000 (s) | Build (s) | Size (KiB) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CPython 3.12 | 5.115 | 5.073 to 5.131 | 21.53 | 11.2 | 1.245 | 2.480 | 0.00 | 2 |
| CPython 3.13 | 4.944 | 4.928 to 5.053 | 20.81 | 11.2 | 1.237 | 2.440 | 0.00 | 2 |
| CPython 3.14 | 4.362 | 4.275 to 4.435 | 18.36 | 11.6 | 1.124 | 2.181 | 0.00 | 2 |
| CPython 3.15 | 4.257 | 4.218 to 4.351 | 17.92 | 12.0 | 1.093 | 2.116 | 0.00 | 2 |
| PyPy | 0.763 | 0.749 to 0.779 | 3.21 | 67.6 | 0.226 | 0.402 | 0.00 | 2 |
| mypyc | 2.021 | 1.992 to 2.083 | 8.50 | 12.8 | 0.542 | 1.003 | 4.82 | 595 |
| Cython | 4.626 | 4.575 to 4.686 | 19.47 | 12.8 | 1.191 | 2.357 | 4.30 | 1168 |
| Nuitka | 6.012 | 6.001 to 6.286 | 25.30 | 26.4 | 1.683 | 3.070 | 8.46 | 356 |
| Codon | 0.872 | 0.865 to 0.883 | 3.67 | 9.9 | 0.226 | 0.443 | 6.71 | 118 |
| TyPy | 3.353 | 3.330 to 3.377 | 14.11 | 1.8 | 0.823 | 1.650 | 0.07 | 166 |
| C (gcc) | 0.238 | 0.237 to 0.241 | 1.00 | 1.2 | 0.056 | 0.121 | 0.06 | 16 |
| C (clang) | 0.217 | 0.207 to 0.218 | 0.91 | 1.6 | 0.055 | 0.105 | 0.09 | 16 |
| C++ (g++) | 0.260 | 0.258 to 0.303 | 1.09 | 3.4 | 0.069 | 0.129 | 0.53 | 22 |
| Rust | 0.266 | 0.265 to 0.270 | 1.12 | 2.0 | 0.066 | 0.131 | 0.21 | 4450 |
| Go | 0.287 | 0.286 to 0.305 | 1.21 | 6.0 | 0.069 | 0.131 | 3.58 | 2052 |
| Java | 0.383 | 0.359 to 0.384 | 1.61 | 44.4 | 0.147 | 0.248 | 0.51 | 3 |
| JavaScript (Node) | 0.664 | 0.657 to 0.672 | 2.80 | 123.2 | 0.211 | 0.371 | 0.00 | 2 |

### knucleotide, size 1,000,000

| Peer | Median (s) | Fastest to slowest (s) | Against C (gcc) | Peak memory (MiB) | At 250,000 (s) | At 500,000 (s) | Build (s) | Size (KiB) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CPython 3.12 | 1.630 | 1.596 to 1.711 | 9.41 | 27.6 | 0.408 | 0.808 | 0.00 | 1 |
| CPython 3.13 | 1.680 | 1.675 to 1.728 | 9.70 | 27.8 | 0.427 | 0.811 | 0.00 | 1 |
| CPython 3.14 | 1.534 | 1.513 to 1.549 | 8.86 | 28.3 | 0.382 | 0.764 | 0.00 | 1 |
| CPython 3.15 | 1.372 | 1.339 to 1.391 | 7.92 | 28.6 | 0.350 | 0.671 | 0.00 | 1 |
| PyPy | 0.740 | 0.718 to 0.788 | 4.27 | 155.5 | 0.220 | 0.402 | 0.00 | 1 |
| mypyc | 0.889 | 0.877 to 0.961 | 5.13 | 29.1 | 0.234 | 0.440 | 4.83 | 582 |
| Cython | 1.208 | 1.188 to 1.238 | 6.97 | 29.6 | 0.310 | 0.602 | 4.08 | 1081 |
| Nuitka | 1.958 | 1.925 to 2.009 | 11.30 | 39.9 | 0.622 | 1.046 | 8.09 | 347 |
| Codon | 0.277 | 0.273 to 0.287 | 1.60 | 83.1 | 0.078 | 0.145 | 8.47 | 155 |
| TyPy | 3.261 | 3.157 to 3.326 | 18.83 | 59.9 | 0.763 | 1.628 | 0.10 | 199 |
| C (gcc) | 0.173 | 0.172 to 0.173 | 1.00 | 18.4 | 0.070 | 0.103 | 0.10 | 16 |
| C (clang) | 0.179 | 0.177 to 0.180 | 1.03 | 18.8 | 0.067 | 0.107 | 0.12 | 16 |
| C++ (g++) | 0.349 | 0.343 to 0.350 | 2.02 | 12.5 | 0.118 | 0.199 | 0.76 | 23 |
| Rust | 0.332 | 0.291 to 0.347 | 1.92 | 15.5 | 0.086 | 0.153 | 0.30 | 4498 |
| Go | 0.274 | 0.214 to 0.416 | 1.58 | 15.5 | 0.076 | 0.116 | 3.87 | 2351 |
| Java | 0.565 | 0.554 to 0.574 | 3.26 | 218.4 | 0.247 | 0.321 | 0.63 | 4 |
| JavaScript (Node) | 1.192 | 1.163 to 1.255 | 6.88 | 141.0 | 0.357 | 0.623 | 0.00 | 1 |

### strings, size 500,000

| Peer | Median (s) | Fastest to slowest (s) | Against C (gcc) | Peak memory (MiB) | At 125,000 (s) | At 250,000 (s) | Build (s) | Size (KiB) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CPython 3.12 | 0.934 | 0.922 to 0.963 | 4.55 | 11.2 | 0.239 | 0.460 | 0.00 | <1 |
| CPython 3.13 | 0.842 | 0.826 to 0.866 | 4.11 | 11.0 | 0.212 | 0.425 | 0.00 | <1 |
| CPython 3.14 | 0.801 | 0.791 to 0.841 | 3.91 | 11.3 | 0.207 | 0.412 | 0.00 | <1 |
| CPython 3.15 | 0.833 | 0.824 to 0.850 | 4.06 | 11.9 | 0.217 | 0.417 | 0.00 | <1 |
| PyPy | 0.349 | 0.338 to 0.353 | 1.70 | 58.9 | 0.107 | 0.187 | 0.00 | <1 |
| mypyc | 0.542 | 0.526 to 0.555 | 2.64 | 12.7 | 0.148 | 0.290 | 4.57 | 540 |
| Cython | 0.671 | 0.660 to 0.674 | 3.27 | 12.9 | 0.200 | 0.380 | 2.99 | 686 |
| Nuitka | 0.969 | 0.949 to 1.068 | 4.73 | 26.2 | 0.396 | 0.578 | 8.46 | 322 |
| Codon | 0.215 | 0.215 to 0.227 | 1.05 | 10.0 | 0.057 | 0.115 | 8.34 | 154 |
| TyPy | 2.868 | 2.816 to 2.983 | 13.99 | 1.6 | 0.706 | 1.422 | 0.06 | 178 |
| C (gcc) | 0.205 | 0.195 to 0.241 | 1.00 | 1.5 | 0.062 | 0.100 | 0.07 | 16 |
| C (clang) | 0.133 | 0.128 to 0.136 | 0.65 | 1.9 | 0.034 | 0.067 | 0.11 | 16 |
| C++ (g++) | 0.290 | 0.285 to 0.293 | 1.42 | 3.6 | 0.067 | 0.140 | 0.57 | 21 |
| Rust | 0.333 | 0.325 to 0.359 | 1.62 | 2.0 | 0.084 | 0.160 | 0.39 | 4448 |
| Go | 0.307 | 0.297 to 0.442 | 1.50 | 11.1 | 0.081 | 0.174 | 4.00 | 2353 |
| Java | 0.540 | 0.527 to 0.574 | 2.63 | 195.0 | 0.276 | 0.373 | 0.63 | 2 |
| JavaScript (Node) | 0.810 | 0.802 to 0.868 | 3.95 | 54.5 | 0.218 | 0.418 | 0.00 | <1 |

### dicts, size 4,000,000

| Peer | Median (s) | Fastest to slowest (s) | Against C (gcc) | Peak memory (MiB) | At 1,000,000 (s) | At 2,000,000 (s) | Build (s) | Size (KiB) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CPython 3.12 | 1.835 | 1.826 to 1.915 | 6.49 | 12.0 | 0.449 | 0.983 | 0.00 | <1 |
| CPython 3.13 | 1.834 | 1.830 to 1.911 | 6.48 | 12.0 | 0.435 | 0.894 | 0.00 | <1 |
| CPython 3.14 | 1.805 | 1.790 to 1.809 | 6.38 | 12.2 | 0.453 | 0.864 | 0.00 | <1 |
| CPython 3.15 | 1.799 | 1.786 to 1.866 | 6.36 | 12.6 | 0.486 | 0.895 | 0.00 | <1 |
| PyPy | 0.442 | 0.440 to 0.459 | 1.56 | 67.6 | 0.135 | 0.246 | 0.00 | <1 |
| mypyc | 1.103 | 1.096 to 1.114 | 3.90 | 13.4 | 0.336 | 0.574 | 4.58 | 547 |
| Cython | 1.579 | 1.568 to 1.658 | 5.58 | 13.6 | 0.391 | 0.770 | 3.08 | 715 |
| Nuitka | 2.208 | 2.162 to 2.218 | 7.80 | 26.1 | 0.699 | 1.153 | 8.44 | 326 |
| Codon | 0.354 | 0.351 to 0.354 | 1.25 | 10.4 | 0.088 | 0.179 | 7.28 | 118 |
| TyPy | 2.434 | 2.432 to 2.444 | 8.60 | 2.2 | 0.613 | 1.171 | 0.06 | 176 |
| C (gcc) | 0.283 | 0.283 to 0.296 | 1.00 | 2.0 | 0.075 | 0.152 | 0.07 | 16 |
| C (clang) | 0.302 | 0.285 to 0.312 | 1.07 | 2.3 | 0.074 | 0.154 | 0.10 | 16 |
| C++ (g++) | 0.267 | 0.266 to 0.271 | 0.94 | 4.2 | 0.065 | 0.140 | 0.95 | 31 |
| Rust | 0.417 | 0.404 to 0.428 | 1.47 | 2.5 | 0.116 | 0.203 | 0.33 | 4473 |
| Go | 0.354 | 0.328 to 0.408 | 1.25 | 7.4 | 0.083 | 0.155 | 4.00 | 2346 |
| Java | 0.339 | 0.332 to 0.347 | 1.20 | 163.9 | 0.146 | 0.221 | 0.60 | 3 |
| JavaScript (Node) | 0.667 | 0.665 to 0.677 | 2.36 | 54.8 | 0.179 | 0.327 | 0.00 | <1 |

### dispatch, size 50,000

| Peer | Median (s) | Fastest to slowest (s) | Against C (gcc) | Peak memory (MiB) | At 12,500 (s) | At 25,000 (s) | Build (s) | Size (KiB) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CPython 3.12 | 4.022 | 3.901 to 4.153 | 33.21 | 11.4 | 0.966 | 1.912 | 0.00 | 1 |
| CPython 3.13 | 3.429 | 3.391 to 3.476 | 28.31 | 11.2 | 0.863 | 1.782 | 0.00 | 1 |
| CPython 3.14 | 3.818 | 3.817 to 3.963 | 31.52 | 11.6 | 0.942 | 1.959 | 0.00 | 1 |
| CPython 3.15 | 3.757 | 3.655 to 3.851 | 31.02 | 12.0 | 0.930 | 1.859 | 0.00 | 1 |
| PyPy | 0.346 | 0.342 to 0.356 | 2.86 | 58.6 | 0.106 | 0.186 | 0.00 | 1 |
| mypyc | 0.304 | 0.302 to 0.305 | 2.51 | 12.8 | 0.094 | 0.160 | 4.81 | 579 |
| Cython | 3.906 | 3.885 to 4.017 | 32.25 | 12.8 | 1.011 | 2.085 | 3.78 | 1030 |
| Nuitka | 5.772 | 5.526 to 5.856 | 47.66 | 26.2 | 1.598 | 3.045 | 8.27 | 361 |
| Codon | 0.069 | 0.065 to 0.070 | 0.57 | 9.1 | 0.026 | 0.034 | 6.90 | 130 |
| TyPy | 1.703 | 1.701 to 1.801 | 14.06 | 1.6 | 0.416 | 0.875 | 0.06 | 167 |
| C (gcc) | 0.121 | 0.113 to 0.123 | 1.00 | 1.6 | 0.038 | 0.076 | 0.06 | 16 |
| C (clang) | 0.181 | 0.180 to 0.189 | 1.49 | 2.0 | 0.047 | 0.092 | 0.09 | 16 |
| C++ (g++) | 0.118 | 0.116 to 0.124 | 0.98 | 3.6 | 0.031 | 0.061 | 0.61 | 18 |
| Rust | 0.133 | 0.129 to 0.135 | 1.10 | 2.0 | 0.034 | 0.067 | 0.15 | 4462 |
| Go | 0.149 | 0.148 to 0.150 | 1.23 | 2.0 | 0.035 | 0.062 | 4.05 | 2319 |
| Java | 0.355 | 0.340 to 0.357 | 2.93 | 44.4 | 0.125 | 0.195 | 0.57 | 3 |
| JavaScript (Node) | 0.186 | 0.184 to 0.197 | 1.53 | 53.9 | 0.074 | 0.104 | 0.00 | 1 |

### recursion, size 36

| Peer | Median (s) | Fastest to slowest (s) | Against C (gcc) | Peak memory (MiB) | At 33 (s) | At 34 (s) | Build (s) | Size (KiB) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CPython 3.12 | 2.528 | 2.526 to 2.609 | 80.60 | 11.1 | 0.599 | 0.942 | 0.00 | <1 |
| CPython 3.13 | 2.233 | 2.185 to 2.244 | 71.18 | 11.1 | 0.534 | 0.848 | 0.00 | <1 |
| CPython 3.14 | 2.069 | 2.048 to 2.096 | 65.97 | 11.2 | 0.492 | 0.785 | 0.00 | <1 |
| CPython 3.15 | 1.932 | 1.927 to 1.975 | 61.58 | 11.9 | 0.447 | 0.836 | 0.00 | <1 |
| PyPy | 0.196 | 0.194 to 0.211 | 6.24 | 62.6 | 0.077 | 0.098 | 0.00 | <1 |
| mypyc | 0.127 | 0.127 to 0.127 | 4.04 | 12.7 | 0.045 | 0.062 | 4.43 | 522 |
| Cython | 1.167 | 1.149 to 1.211 | 37.20 | 12.7 | 0.273 | 0.437 | 2.62 | 554 |
| Nuitka | 2.203 | 2.162 to 2.231 | 70.25 | 26.1 | 0.655 | 0.945 | 8.07 | 314 |
| Codon | 0.045 | 0.042 to 0.084 | 1.42 | 9.0 | 0.016 | 0.021 | 6.31 | 106 |
| TyPy | 0.821 | 0.820 to 0.839 | 26.18 | 1.5 | 0.196 | 0.309 | 0.05 | 151 |
| C (gcc) | 0.031 | 0.030 to 0.032 | 1.00 | 1.5 | 0.008 | 0.012 | 0.07 | 16 |
| C (clang) | 0.042 | 0.040 to 0.042 | 1.35 | 1.9 | 0.010 | 0.016 | 0.07 | 16 |
| C++ (g++) | 0.031 | 0.031 to 0.032 | 1.00 | 1.5 | 0.009 | 0.012 | 0.08 | 16 |
| Rust | 0.037 | 0.036 to 0.042 | 1.17 | 1.9 | 0.009 | 0.015 | 0.12 | 4435 |
| Go | 0.086 | 0.084 to 0.094 | 2.74 | 2.0 | 0.023 | 0.035 | 3.99 | 2302 |
| Java | 0.086 | 0.083 to 0.088 | 2.75 | 39.7 | 0.036 | 0.042 | 0.49 | <1 |
| JavaScript (Node) | 0.225 | 0.224 to 0.226 | 7.16 | 51.6 | 0.080 | 0.102 | 0.00 | <1 |

### parser, size 150,000

| Peer | Median (s) | Fastest to slowest (s) | Against C (gcc) | Peak memory (MiB) | At 37,500 (s) | At 75,000 (s) | Build (s) | Size (KiB) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CPython 3.12 | 2.677 | 2.663 to 2.844 | 8.44 | 11.2 | 0.644 | 1.300 | 0.00 | 2 |
| CPython 3.13 | 2.471 | 2.458 to 2.569 | 7.79 | 11.2 | 0.626 | 1.237 | 0.00 | 2 |
| CPython 3.14 | 2.425 | 2.402 to 2.428 | 7.65 | 11.4 | 0.626 | 1.176 | 0.00 | 2 |
| CPython 3.15 | 2.482 | 2.434 to 2.491 | 7.82 | 11.9 | 0.636 | 1.245 | 0.00 | 2 |
| PyPy | 0.416 | 0.399 to 0.420 | 1.31 | 62.5 | 0.185 | 0.254 | 0.00 | 2 |
| mypyc | 0.919 | 0.905 to 0.961 | 2.90 | 12.8 | 0.243 | 0.470 | 4.83 | 597 |
| Cython | 2.161 | 2.107 to 2.199 | 6.81 | 13.1 | 0.539 | 1.058 | 4.43 | 1300 |
| Nuitka | 3.722 | 3.677 to 3.765 | 11.73 | 26.1 | 1.036 | 1.974 | 8.36 | 360 |
| Codon | 0.540 | 0.527 to 0.586 | 1.70 | 9.9 | 0.131 | 0.271 | 7.26 | 134 |
| TyPy | 2.766 | 2.731 to 2.873 | 8.72 | 1.8 | 0.681 | 1.363 | 0.09 | 166 |
| C (gcc) | 0.317 | 0.314 to 0.337 | 1.00 | 1.4 | 0.084 | 0.154 | 0.08 | 16 |
| C (clang) | 0.337 | 0.336 to 0.341 | 1.06 | 1.8 | 0.084 | 0.166 | 0.12 | 16 |
| C++ (g++) | 0.324 | 0.322 to 0.324 | 1.02 | 3.6 | 0.076 | 0.158 | 0.72 | 29 |
| Rust | 0.603 | 0.598 to 0.608 | 1.90 | 2.0 | 0.151 | 0.297 | 0.25 | 4445 |
| Go | 0.424 | 0.391 to 0.465 | 1.34 | 7.4 | 0.099 | 0.188 | 3.99 | 2314 |
| Java | 0.416 | 0.408 to 0.436 | 1.31 | 192.5 | 0.216 | 0.272 | 0.58 | 3 |
| JavaScript (Node) | 0.480 | 0.474 to 0.490 | 1.51 | 56.9 | 0.153 | 0.246 | 0.00 | 2 |

<!-- results end -->

## 4. Observations

These notes read the figures of 2026-11-16 above; a later run may move
them.

### 4.1 Where TyPy stands

TyPy's programs take 14 times as long as C's, as the geometric mean of the
twelve workloads, against 22 to 26 times for CPython 3.12 to 3.15. Among the
compilers of Python, Codon (1.3), PyPy (3.2) and mypyc (4.4) are faster, and
Cython in pure Python mode (15.5) and Nuitka (30) are slower. NFR-03 asks for
1.5 times by 1.0, so most of that distance is still ahead.

TyPy is closest to C where a program allocates and frees objects or works on
floating-point numbers in a loop: binarytrees at 4.4 times and mandelbrot at
4.7. It is furthest where the work is integer arithmetic and list indexing:
spectralnorm at 66 times and fannkuchredux at 43, where it is slower than
CPython 3.15, and recursion at 26.

### 4.2 Why integer work is slow

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

### 4.3 Memory

TyPy's peak memory is 1.22 times C's as the geometric mean, the smallest of
the Python peers, which hold an interpreter or a runtime of several megabytes
before the program starts: 6 times for CPython and 31 for PyPy. Its startup,
0.6 ms, is that of C. The exception is knucleotide, where TyPy peaks at 60 MiB
against CPython's 29: `"ACGT"[k]` makes a new one-character string for each of
the million characters, where CPython shares one object for each character.

### 4.4 Building

TyPy builds each workload in 0.08 seconds as the mean, as fast as gcc, while
the other compilers of Python take 3.6 to 8.3 seconds and Go 3.9 from a clean
cache. Its executables are 168 KiB as the mean, against 16 KiB for C, since
each holds the part of TyPy's runtime it uses, linked statically; Rust's,
built by `rustc` without stripping, are 4.4 MiB, and Go's 2.3 MiB.

### 4.5 What the figures do not show

The machine is a laptop processor whose clock follows its temperature, so
two runs of one program differ by a few percent; the fastest and slowest of
the three runs are given beside each median. Every program is single
threaded, and Java's and Node's figures include their JIT compilers'
warm-up, which a long-running server would not pay for each request.
