# TyPy benchmark report

| Field | Value |
| --- | --- |
| Document | TyPy benchmark report |
| Version | 0.3 |
| Status | Draft |
| Owner | typy-lang contributors |
| Last updated | 2026-11-24 |

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
| Pinned cores | 2,3 |
| Jobs at once | 1 |
| Date | 2026-11-24 |
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
| TyPy | typy 0.11.0 |
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
| CPython 3.12 | 24.96 | 5.89 | 12 of 12 | 13.5 |
| CPython 3.13 | 23.76 | 5.97 | 12 of 12 | 15.2 |
| CPython 3.14 | 21.89 | 5.76 | 12 of 12 | 12.9 |
| CPython 3.15 | 22.14 | 6.36 | 12 of 12 | 17.1 |
| PyPy | 3.27 | 30.57 | 12 of 12 | 25.5 |
| mypyc | 4.33 | 6.22 | 12 of 12 | 18.4 |
| Cython | 15.44 | 6.43 | 12 of 12 | 15.5 |
| Nuitka | 29.69 | 12.63 | 12 of 12 | 135.5 |
| Codon | 1.32 | 5.42 | 12 of 12 | 5.4 |
| TyPy | 4.06 | 1.12 | 12 of 12 | 0.6 |
| C (gcc) | 1.00 | 1.00 | 12 of 12 | 0.5 |
| C (clang) | 1.04 | 1.17 | 12 of 12 | 0.6 |
| C++ (g++) | 1.13 | 1.70 | 12 of 12 | 0.5 |
| Rust | 1.26 | 1.21 | 12 of 12 | 0.7 |
| Go | 1.29 | 1.83 | 12 of 12 | 1.1 |
| Java | 1.66 | 35.60 | 12 of 12 | 30.0 |
| JavaScript (Node) | 2.44 | 29.49 | 12 of 12 | 29.6 |

### nbody, size 4,000,000

| Peer | Median (s) | Fastest to slowest (s) | Against C (gcc) | Peak memory (MiB) | At 1,000,000 (s) | At 2,000,000 (s) | Build (s) | Size (KiB) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CPython 3.12 | 20.3 | 20.2 to 20.6 | 80.06 | 11.2 | 5.043 | 10.3 | 0.00 | 4 |
| CPython 3.13 | 21.1 | 20.1 to 22.1 | 83.16 | 11.3 | 5.667 | 10.6 | 0.00 | 4 |
| CPython 3.14 | 21.0 | 20.9 to 21.0 | 82.82 | 11.1 | 5.269 | 10.7 | 0.00 | 4 |
| CPython 3.15 | 22.3 | 22.2 to 23.1 | 88.07 | 12.2 | 5.402 | 10.7 | 0.00 | 4 |
| PyPy | 1.109 | 1.092 to 1.119 | 4.38 | 59.1 | 0.317 | 0.581 | 0.00 | 4 |
| mypyc | 0.814 | 0.812 to 0.818 | 3.21 | 12.1 | 0.215 | 0.419 | 5.08 | 583 |
| Cython | 31.8 | 30.7 to 32.3 | 125.58 | 12.1 | 7.950 | 15.7 | 4.70 | 1347 |
| Nuitka | 45.6 | 45.2 to 46.8 | 180.06 | 26.1 | 12.2 | 23.5 | 8.67 | 384 |
| Codon | 0.365 | 0.359 to 0.371 | 1.44 | 9.0 | 0.093 | 0.187 | 8.40 | 134 |
| TyPy | 1.082 | 1.005 to 1.276 | 4.27 | 1.6 | 0.265 | 0.514 | 0.13 | 208 |
| C (gcc) | 0.254 | 0.246 to 0.272 | 1.00 | 1.5 | 0.075 | 0.126 | 0.08 | 16 |
| C (clang) | 0.269 | 0.260 to 0.283 | 1.06 | 1.9 | 0.074 | 0.154 | 0.21 | 16 |
| C++ (g++) | 0.261 | 0.255 to 0.274 | 1.03 | 3.5 | 0.065 | 0.127 | 0.53 | 16 |
| Rust | 0.187 | 0.183 to 0.227 | 0.74 | 1.9 | 0.050 | 0.093 | 0.17 | 4465 |
| Go | 0.396 | 0.328 to 0.403 | 1.56 | 1.9 | 0.080 | 0.172 | 4.20 | 2320 |
| Java | 0.426 | 0.414 to 0.440 | 1.68 | 43.7 | 0.144 | 0.222 | 0.67 | 3 |
| JavaScript (Node) | 0.543 | 0.537 to 0.546 | 2.14 | 54.3 | 0.152 | 0.286 | 0.00 | 3 |

### spectralnorm, size 2,000

| Peer | Median (s) | Fastest to slowest (s) | Against C (gcc) | Peak memory (MiB) | At 1,000 (s) | At 1,414 (s) | Build (s) | Size (KiB) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CPython 3.12 | 36.9 | 36.2 to 38.1 | 208.97 | 11.4 | 9.033 | 18.9 | 0.00 | 1 |
| CPython 3.13 | 33.7 | 33.2 to 34.5 | 190.99 | 11.5 | 8.426 | 16.5 | 0.00 | 1 |
| CPython 3.14 | 25.8 | 25.7 to 26.2 | 146.05 | 11.1 | 6.498 | 12.8 | 0.00 | 1 |
| CPython 3.15 | 26.1 | 25.9 to 29.3 | 147.89 | 12.2 | 6.332 | 13.2 | 0.00 | 1 |
| PyPy | 1.173 | 1.131 to 1.184 | 6.64 | 58.9 | 0.313 | 0.587 | 0.00 | 1 |
| mypyc | 2.767 | 2.690 to 2.822 | 15.66 | 12.0 | 0.723 | 1.373 | 4.79 | 548 |
| Cython | 27.2 | 27.1 to 30.2 | 154.26 | 12.4 | 6.820 | 13.4 | 4.07 | 1073 |
| Nuitka | 43.6 | 43.3 to 43.7 | 246.97 | 25.9 | 10.3 | 20.5 | 8.35 | 351 |
| Codon | 0.253 | 0.253 to 0.254 | 1.43 | 9.1 | 0.065 | 0.131 | 7.60 | 130 |
| TyPy | 1.759 | 1.746 to 1.835 | 9.96 | 1.9 | 0.441 | 0.906 | 0.14 | 213 |
| C (gcc) | 0.177 | 0.167 to 0.181 | 1.00 | 2.0 | 0.041 | 0.083 | 0.07 | 16 |
| C (clang) | 0.178 | 0.168 to 0.179 | 1.01 | 2.0 | 0.041 | 0.084 | 0.11 | 16 |
| C++ (g++) | 0.172 | 0.168 to 0.220 | 0.97 | 3.6 | 0.042 | 0.088 | 0.38 | 17 |
| Rust | 0.198 | 0.197 to 0.207 | 1.12 | 2.1 | 0.046 | 0.095 | 0.19 | 4462 |
| Go | 0.234 | 0.233 to 0.244 | 1.32 | 2.0 | 0.061 | 0.118 | 3.90 | 2320 |
| Java | 0.295 | 0.294 to 0.305 | 1.67 | 43.2 | 0.111 | 0.172 | 0.58 | 1 |
| JavaScript (Node) | 0.432 | 0.413 to 0.461 | 2.44 | 53.8 | 0.124 | 0.222 | 0.00 | <1 |

### fannkuchredux, size 10

| Peer | Median (s) | Fastest to slowest (s) | Against C (gcc) | Peak memory (MiB) | At 8 (s) | At 9 (s) | Build (s) | Size (KiB) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CPython 3.12 | 6.961 | 6.882 to 7.157 | 36.44 | 11.1 | 0.066 | 0.610 | 0.00 | 1 |
| CPython 3.13 | 7.556 | 7.532 to 8.010 | 39.56 | 11.2 | 0.072 | 0.661 | 0.00 | 1 |
| CPython 3.14 | 6.712 | 6.422 to 6.878 | 35.14 | 10.6 | 0.069 | 0.555 | 0.00 | 1 |
| CPython 3.15 | 7.147 | 6.999 to 7.166 | 37.42 | 12.0 | 0.071 | 0.608 | 0.00 | 1 |
| PyPy | 0.885 | 0.843 to 0.940 | 4.63 | 58.9 | 0.044 | 0.095 | 0.00 | 1 |
| mypyc | 3.385 | 3.124 to 3.440 | 17.72 | 11.9 | 0.041 | 0.277 | 4.79 | 549 |
| Cython | 5.354 | 5.327 to 5.458 | 28.03 | 12.0 | 0.054 | 0.460 | 3.50 | 835 |
| Nuitka | 7.037 | 6.838 to 7.075 | 36.84 | 25.9 | 0.222 | 0.788 | 7.84 | 338 |
| Codon | 0.235 | 0.234 to 0.241 | 1.23 | 9.0 | 0.009 | 0.031 | 7.00 | 110 |
| TyPy | 1.066 | 1.000 to 1.144 | 5.58 | 1.6 | 0.008 | 0.090 | 0.19 | 191 |
| C (gcc) | 0.191 | 0.182 to 0.198 | 1.00 | 1.5 | 0.002 | 0.016 | 0.09 | 16 |
| C (clang) | 0.205 | 0.204 to 0.245 | 1.08 | 1.9 | 0.003 | 0.020 | 0.09 | 16 |
| C++ (g++) | 0.241 | 0.238 to 0.251 | 1.26 | 3.6 | 0.003 | 0.020 | 0.52 | 17 |
| Rust | 0.219 | 0.206 to 0.221 | 1.15 | 1.9 | 0.003 | 0.019 | 0.16 | 4436 |
| Go | 0.212 | 0.209 to 0.218 | 1.11 | 1.9 | 0.003 | 0.019 | 4.18 | 2319 |
| Java | 0.328 | 0.326 to 0.385 | 1.71 | 43.1 | 0.042 | 0.064 | 0.52 | 2 |
| JavaScript (Node) | 0.609 | 0.596 to 0.610 | 3.19 | 51.9 | 0.033 | 0.087 | 0.00 | 1 |

### binarytrees, size 16

| Peer | Median (s) | Fastest to slowest (s) | Against C (gcc) | Peak memory (MiB) | At 14 (s) | At 15 (s) | Build (s) | Size (KiB) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CPython 3.12 | 6.770 | 6.764 to 6.956 | 22.85 | 31.1 | 1.010 | 2.173 | 0.00 | 1 |
| CPython 3.13 | 5.604 | 5.557 to 5.604 | 18.91 | 35.2 | 0.816 | 1.902 | 0.00 | 1 |
| CPython 3.14 | 4.987 | 4.953 to 5.114 | 16.83 | 34.8 | 0.876 | 1.676 | 0.00 | 1 |
| CPython 3.15 | 5.202 | 4.934 to 5.233 | 17.55 | 36.1 | 0.737 | 1.697 | 0.00 | 1 |
| PyPy | 0.829 | 0.806 to 0.856 | 2.80 | 94.9 | 0.197 | 0.208 | 0.00 | 1 |
| mypyc | 0.698 | 0.687 to 0.721 | 2.35 | 28.0 | 0.148 | 0.286 | 4.50 | 553 |
| Cython | 3.001 | 2.992 to 3.173 | 10.13 | 36.3 | 0.639 | 1.251 | 3.55 | 912 |
| Nuitka | 3.976 | 3.921 to 4.059 | 13.42 | 48.0 | 0.913 | 1.717 | 8.56 | 335 |
| Codon | 0.350 | 0.337 to 0.387 | 1.18 | 23.5 | 0.078 | 0.150 | 6.75 | 106 |
| TyPy | 1.060 | 1.031 to 1.165 | 3.58 | 13.6 | 0.213 | 0.446 | 0.12 | 182 |
| C (gcc) | 0.296 | 0.295 to 0.308 | 1.00 | 9.5 | 0.064 | 0.120 | 0.17 | 20 |
| C (clang) | 0.343 | 0.312 to 0.350 | 1.16 | 9.9 | 0.076 | 0.139 | 0.08 | 16 |
| C++ (g++) | 0.366 | 0.360 to 0.374 | 1.23 | 11.6 | 0.077 | 0.158 | 0.89 | 40 |
| Rust | 0.380 | 0.370 to 0.390 | 1.28 | 9.9 | 0.081 | 0.163 | 0.14 | 4437 |
| Go | 0.420 | 0.413 to 0.435 | 1.42 | 13.8 | 0.060 | 0.155 | 3.94 | 2319 |
| Java | 0.172 | 0.172 to 0.177 | 0.58 | 190.2 | 0.093 | 0.116 | 0.54 | 3 |
| JavaScript (Node) | 0.272 | 0.267 to 0.278 | 0.92 | 100.7 | 0.102 | 0.134 | 0.00 | <1 |

### mandelbrot, size 1,600

| Peer | Median (s) | Fastest to slowest (s) | Against C (gcc) | Peak memory (MiB) | At 800 (s) | At 1,131 (s) | Build (s) | Size (KiB) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CPython 3.12 | 6.265 | 6.127 to 6.560 | 33.86 | 11.1 | 1.564 | 3.011 | 0.00 | <1 |
| CPython 3.13 | 6.168 | 5.970 to 6.194 | 33.34 | 11.0 | 1.537 | 2.986 | 0.00 | <1 |
| CPython 3.14 | 6.214 | 6.119 to 6.252 | 33.59 | 10.5 | 1.562 | 3.064 | 0.00 | <1 |
| CPython 3.15 | 6.205 | 6.161 to 6.255 | 33.54 | 12.0 | 1.580 | 3.062 | 0.00 | <1 |
| PyPy | 0.843 | 0.830 to 0.856 | 4.56 | 58.6 | 0.238 | 0.449 | 0.00 | <1 |
| mypyc | 0.352 | 0.351 to 0.353 | 1.90 | 11.9 | 0.104 | 0.185 | 4.87 | 524 |
| Cython | 0.339 | 0.327 to 0.341 | 1.83 | 12.0 | 0.094 | 0.170 | 2.72 | 564 |
| Nuitka | 11.7 | 11.7 to 12.4 | 63.39 | 26.0 | 3.013 | 6.241 | 8.20 | 326 |
| Codon | 0.190 | 0.190 to 0.197 | 1.03 | 9.0 | 0.052 | 0.099 | 6.86 | 106 |
| TyPy | 0.223 | 0.211 to 0.225 | 1.20 | 1.6 | 0.051 | 0.106 | 0.09 | 178 |
| C (gcc) | 0.185 | 0.182 to 0.193 | 1.00 | 1.5 | 0.047 | 0.094 | 0.05 | 16 |
| C (clang) | 0.201 | 0.197 to 0.209 | 1.09 | 1.8 | 0.053 | 0.104 | 0.08 | 16 |
| C++ (g++) | 0.188 | 0.183 to 0.194 | 1.01 | 1.5 | 0.047 | 0.092 | 0.06 | 16 |
| Rust | 0.194 | 0.190 to 0.200 | 1.05 | 1.9 | 0.048 | 0.094 | 0.12 | 4435 |
| Go | 0.203 | 0.203 to 0.207 | 1.10 | 1.9 | 0.054 | 0.102 | 3.88 | 2302 |
| Java | 0.224 | 0.222 to 0.240 | 1.21 | 40.3 | 0.073 | 0.120 | 0.49 | <1 |
| JavaScript (Node) | 0.234 | 0.220 to 0.235 | 1.26 | 54.4 | 0.081 | 0.133 | 0.00 | <1 |

### fasta, size 2,000,000

| Peer | Median (s) | Fastest to slowest (s) | Against C (gcc) | Peak memory (MiB) | At 500,000 (s) | At 1,000,000 (s) | Build (s) | Size (KiB) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CPython 3.12 | 4.988 | 4.929 to 5.167 | 20.82 | 11.2 | 1.245 | 2.518 | 0.00 | 2 |
| CPython 3.13 | 4.863 | 4.784 to 4.867 | 20.30 | 11.3 | 1.200 | 2.434 | 0.00 | 2 |
| CPython 3.14 | 4.282 | 4.273 to 4.348 | 17.88 | 10.9 | 1.076 | 2.150 | 0.00 | 2 |
| CPython 3.15 | 4.233 | 4.155 to 4.246 | 17.67 | 12.1 | 1.045 | 2.110 | 0.00 | 2 |
| PyPy | 0.760 | 0.742 to 0.761 | 3.17 | 67.6 | 0.222 | 0.394 | 0.00 | 2 |
| mypyc | 2.120 | 2.051 to 2.162 | 8.85 | 12.2 | 0.517 | 1.022 | 4.75 | 595 |
| Cython | 4.528 | 4.523 to 4.574 | 18.91 | 12.2 | 1.148 | 2.324 | 4.29 | 1168 |
| Nuitka | 6.071 | 5.875 to 6.160 | 25.35 | 26.0 | 1.620 | 3.039 | 7.99 | 356 |
| Codon | 0.867 | 0.860 to 0.873 | 3.62 | 9.8 | 0.212 | 0.437 | 7.25 | 118 |
| TyPy | 0.912 | 0.911 to 0.922 | 3.81 | 1.8 | 0.228 | 0.452 | 0.12 | 202 |
| C (gcc) | 0.240 | 0.236 to 0.244 | 1.00 | 1.2 | 0.056 | 0.129 | 0.06 | 16 |
| C (clang) | 0.211 | 0.209 to 0.221 | 0.88 | 1.6 | 0.051 | 0.104 | 0.10 | 16 |
| C++ (g++) | 0.258 | 0.255 to 0.263 | 1.08 | 3.4 | 0.062 | 0.128 | 0.53 | 22 |
| Rust | 0.254 | 0.248 to 0.258 | 1.06 | 2.0 | 0.063 | 0.127 | 0.21 | 4450 |
| Go | 0.261 | 0.255 to 0.261 | 1.09 | 6.0 | 0.062 | 0.128 | 3.61 | 2052 |
| Java | 0.369 | 0.356 to 0.371 | 1.54 | 43.9 | 0.148 | 0.227 | 0.54 | 3 |
| JavaScript (Node) | 0.668 | 0.653 to 0.671 | 2.79 | 123.5 | 0.214 | 0.389 | 0.00 | 2 |

### knucleotide, size 1,000,000

| Peer | Median (s) | Fastest to slowest (s) | Against C (gcc) | Peak memory (MiB) | At 250,000 (s) | At 500,000 (s) | Build (s) | Size (KiB) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CPython 3.12 | 1.715 | 1.659 to 1.819 | 9.68 | 27.5 | 0.424 | 0.946 | 0.00 | 1 |
| CPython 3.13 | 1.733 | 1.730 to 1.817 | 9.78 | 27.8 | 0.409 | 0.796 | 0.00 | 1 |
| CPython 3.14 | 1.492 | 1.468 to 1.522 | 8.42 | 27.7 | 0.389 | 0.752 | 0.00 | 1 |
| CPython 3.15 | 1.382 | 1.351 to 1.404 | 7.80 | 28.6 | 0.335 | 0.680 | 0.00 | 1 |
| PyPy | 0.721 | 0.711 to 0.728 | 4.07 | 155.4 | 0.245 | 0.414 | 0.00 | 1 |
| mypyc | 0.877 | 0.863 to 0.896 | 4.95 | 28.6 | 0.232 | 0.435 | 4.70 | 582 |
| Cython | 1.209 | 1.201 to 1.255 | 6.82 | 29.1 | 0.298 | 0.576 | 4.14 | 1081 |
| Nuitka | 1.887 | 1.829 to 1.981 | 10.65 | 39.7 | 0.559 | 0.960 | 7.77 | 347 |
| Codon | 0.284 | 0.275 to 0.287 | 1.60 | 81.5 | 0.078 | 0.141 | 8.91 | 155 |
| TyPy | 0.962 | 0.959 to 0.967 | 5.43 | 16.4 | 0.236 | 0.490 | 0.18 | 254 |
| C (gcc) | 0.177 | 0.174 to 0.182 | 1.00 | 18.6 | 0.067 | 0.100 | 0.10 | 16 |
| C (clang) | 0.175 | 0.175 to 0.196 | 0.99 | 18.7 | 0.067 | 0.103 | 0.12 | 16 |
| C++ (g++) | 0.340 | 0.340 to 0.365 | 1.92 | 12.5 | 0.121 | 0.231 | 0.81 | 23 |
| Rust | 0.290 | 0.290 to 0.300 | 1.64 | 15.5 | 0.086 | 0.158 | 0.30 | 4498 |
| Go | 0.214 | 0.212 to 0.215 | 1.21 | 15.5 | 0.075 | 0.120 | 3.96 | 2351 |
| Java | 0.558 | 0.557 to 0.568 | 3.15 | 218.3 | 0.239 | 0.322 | 0.61 | 4 |
| JavaScript (Node) | 1.192 | 1.134 to 1.227 | 6.73 | 140.9 | 0.342 | 0.602 | 0.00 | 1 |

### strings, size 500,000

| Peer | Median (s) | Fastest to slowest (s) | Against C (gcc) | Peak memory (MiB) | At 125,000 (s) | At 250,000 (s) | Build (s) | Size (KiB) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CPython 3.12 | 0.946 | 0.924 to 0.946 | 4.76 | 11.1 | 0.253 | 0.484 | 0.00 | <1 |
| CPython 3.13 | 0.852 | 0.830 to 0.874 | 4.29 | 11.2 | 0.210 | 0.422 | 0.00 | <1 |
| CPython 3.14 | 0.789 | 0.788 to 0.805 | 3.97 | 10.6 | 0.207 | 0.392 | 0.00 | <1 |
| CPython 3.15 | 0.827 | 0.813 to 0.829 | 4.16 | 12.0 | 0.209 | 0.424 | 0.00 | <1 |
| PyPy | 0.340 | 0.332 to 0.352 | 1.71 | 58.8 | 0.105 | 0.175 | 0.00 | <1 |
| mypyc | 0.523 | 0.511 to 0.565 | 2.63 | 12.0 | 0.141 | 0.299 | 4.46 | 540 |
| Cython | 0.659 | 0.648 to 0.677 | 3.32 | 12.2 | 0.169 | 0.327 | 2.96 | 686 |
| Nuitka | 0.880 | 0.871 to 0.883 | 4.43 | 26.0 | 0.316 | 0.498 | 7.95 | 322 |
| Codon | 0.218 | 0.217 to 0.246 | 1.10 | 9.9 | 0.055 | 0.114 | 8.71 | 154 |
| TyPy | 0.493 | 0.484 to 0.495 | 2.48 | 1.8 | 0.116 | 0.241 | 0.10 | 242 |
| C (gcc) | 0.199 | 0.196 to 0.204 | 1.00 | 1.5 | 0.045 | 0.097 | 0.07 | 16 |
| C (clang) | 0.130 | 0.129 to 0.133 | 0.65 | 1.9 | 0.031 | 0.062 | 0.11 | 16 |
| C++ (g++) | 0.288 | 0.280 to 0.294 | 1.45 | 3.6 | 0.067 | 0.142 | 0.58 | 21 |
| Rust | 0.331 | 0.328 to 0.332 | 1.66 | 2.0 | 0.083 | 0.158 | 0.38 | 4448 |
| Go | 0.294 | 0.290 to 0.303 | 1.48 | 7.1 | 0.075 | 0.146 | 3.99 | 2353 |
| Java | 0.526 | 0.524 to 0.532 | 2.65 | 202.2 | 0.266 | 0.378 | 0.54 | 2 |
| JavaScript (Node) | 0.818 | 0.809 to 0.843 | 4.12 | 54.8 | 0.218 | 0.417 | 0.00 | <1 |

### dicts, size 4,000,000

| Peer | Median (s) | Fastest to slowest (s) | Against C (gcc) | Peak memory (MiB) | At 1,000,000 (s) | At 2,000,000 (s) | Build (s) | Size (KiB) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CPython 3.12 | 1.811 | 1.792 to 1.949 | 5.89 | 12.0 | 0.434 | 1.048 | 0.00 | <1 |
| CPython 3.13 | 1.820 | 1.813 to 1.838 | 5.92 | 12.2 | 0.424 | 0.865 | 0.00 | <1 |
| CPython 3.14 | 1.758 | 1.744 to 1.793 | 5.71 | 11.5 | 0.427 | 0.851 | 0.00 | <1 |
| CPython 3.15 | 1.783 | 1.770 to 1.871 | 5.80 | 12.6 | 0.460 | 0.918 | 0.00 | <1 |
| PyPy | 0.460 | 0.438 to 0.470 | 1.49 | 67.5 | 0.136 | 0.253 | 0.00 | <1 |
| mypyc | 1.071 | 1.048 to 1.104 | 3.48 | 12.9 | 0.305 | 0.584 | 4.56 | 547 |
| Cython | 1.549 | 1.541 to 1.684 | 5.04 | 13.1 | 0.388 | 0.760 | 3.01 | 715 |
| Nuitka | 2.256 | 2.156 to 2.285 | 7.33 | 26.0 | 0.634 | 1.100 | 8.12 | 326 |
| Codon | 0.360 | 0.347 to 0.364 | 1.17 | 10.5 | 0.087 | 0.186 | 7.77 | 118 |
| TyPy | 1.043 | 1.032 to 1.090 | 3.39 | 2.2 | 0.259 | 0.499 | 0.11 | 209 |
| C (gcc) | 0.308 | 0.286 to 0.311 | 1.00 | 2.0 | 0.079 | 0.135 | 0.07 | 16 |
| C (clang) | 0.289 | 0.284 to 0.320 | 0.94 | 2.4 | 0.068 | 0.137 | 0.10 | 16 |
| C++ (g++) | 0.278 | 0.274 to 0.309 | 0.91 | 4.2 | 0.069 | 0.133 | 1.01 | 31 |
| Rust | 0.422 | 0.409 to 0.430 | 1.37 | 2.5 | 0.105 | 0.197 | 0.32 | 4473 |
| Go | 0.313 | 0.312 to 0.316 | 1.02 | 7.1 | 0.081 | 0.154 | 3.95 | 2346 |
| Java | 0.336 | 0.334 to 0.337 | 1.09 | 164.1 | 0.153 | 0.219 | 0.67 | 3 |
| JavaScript (Node) | 0.607 | 0.593 to 0.629 | 1.97 | 54.8 | 0.177 | 0.321 | 0.00 | <1 |

### dispatch, size 50,000

| Peer | Median (s) | Fastest to slowest (s) | Against C (gcc) | Peak memory (MiB) | At 12,500 (s) | At 25,000 (s) | Build (s) | Size (KiB) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CPython 3.12 | 3.885 | 3.749 to 3.921 | 34.02 | 11.2 | 0.944 | 1.825 | 0.00 | 1 |
| CPython 3.13 | 3.423 | 3.386 to 3.460 | 29.97 | 11.4 | 0.858 | 1.731 | 0.00 | 1 |
| CPython 3.14 | 3.630 | 3.472 to 3.666 | 31.78 | 10.9 | 0.916 | 1.765 | 0.00 | 1 |
| CPython 3.15 | 3.696 | 3.635 to 3.715 | 32.37 | 12.1 | 0.948 | 1.823 | 0.00 | 1 |
| PyPy | 0.327 | 0.326 to 0.338 | 2.87 | 58.5 | 0.103 | 0.185 | 0.00 | 1 |
| mypyc | 0.290 | 0.287 to 0.298 | 2.54 | 12.1 | 0.085 | 0.151 | 4.61 | 579 |
| Cython | 4.036 | 3.996 to 4.117 | 35.34 | 12.2 | 0.977 | 1.958 | 3.80 | 1030 |
| Nuitka | 5.629 | 5.622 to 5.712 | 49.28 | 26.0 | 1.492 | 2.875 | 8.20 | 361 |
| Codon | 0.063 | 0.060 to 0.065 | 0.55 | 9.1 | 0.021 | 0.033 | 7.45 | 130 |
| TyPy | 0.434 | 0.433 to 0.444 | 3.80 | 1.8 | 0.111 | 0.212 | 0.09 | 204 |
| C (gcc) | 0.114 | 0.114 to 0.118 | 1.00 | 1.6 | 0.029 | 0.057 | 0.05 | 16 |
| C (clang) | 0.182 | 0.180 to 0.185 | 1.59 | 1.9 | 0.045 | 0.090 | 0.09 | 16 |
| C++ (g++) | 0.116 | 0.114 to 0.118 | 1.02 | 3.6 | 0.030 | 0.057 | 0.64 | 18 |
| Rust | 0.129 | 0.125 to 0.137 | 1.13 | 2.0 | 0.033 | 0.065 | 0.15 | 4462 |
| Go | 0.130 | 0.127 to 0.132 | 1.14 | 2.0 | 0.033 | 0.059 | 4.18 | 2319 |
| Java | 0.337 | 0.333 to 0.349 | 2.95 | 44.5 | 0.125 | 0.201 | 0.57 | 3 |
| JavaScript (Node) | 0.177 | 0.174 to 0.180 | 1.55 | 54.0 | 0.077 | 0.116 | 0.00 | 1 |

### recursion, size 36

| Peer | Median (s) | Fastest to slowest (s) | Against C (gcc) | Peak memory (MiB) | At 33 (s) | At 34 (s) | Build (s) | Size (KiB) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CPython 3.12 | 2.521 | 2.514 to 2.579 | 79.40 | 11.1 | 0.580 | 0.946 | 0.00 | <1 |
| CPython 3.13 | 2.206 | 2.190 to 2.210 | 69.49 | 11.0 | 0.536 | 0.849 | 0.00 | <1 |
| CPython 3.14 | 2.035 | 2.009 to 2.051 | 64.10 | 10.5 | 0.478 | 0.779 | 0.00 | <1 |
| CPython 3.15 | 1.898 | 1.896 to 1.903 | 59.77 | 12.0 | 0.453 | 0.737 | 0.00 | <1 |
| PyPy | 0.233 | 0.185 to 0.254 | 7.34 | 62.5 | 0.074 | 0.094 | 0.00 | <1 |
| mypyc | 0.124 | 0.120 to 0.133 | 3.90 | 11.9 | 0.041 | 0.053 | 4.30 | 522 |
| Cython | 1.148 | 1.115 to 1.363 | 36.16 | 11.9 | 0.269 | 0.496 | 2.58 | 554 |
| Nuitka | 2.173 | 2.097 to 2.206 | 68.45 | 26.0 | 0.593 | 0.875 | 8.00 | 314 |
| Codon | 0.042 | 0.042 to 0.047 | 1.33 | 9.0 | 0.014 | 0.019 | 6.71 | 106 |
| TyPy | 0.234 | 0.233 to 0.244 | 7.36 | 1.6 | 0.068 | 0.088 | 0.06 | 174 |
| C (gcc) | 0.032 | 0.031 to 0.032 | 1.00 | 1.5 | 0.009 | 0.012 | 0.07 | 16 |
| C (clang) | 0.041 | 0.040 to 0.043 | 1.28 | 1.9 | 0.010 | 0.015 | 0.07 | 16 |
| C++ (g++) | 0.031 | 0.030 to 0.035 | 0.99 | 1.5 | 0.009 | 0.012 | 0.08 | 16 |
| Rust | 0.048 | 0.041 to 0.069 | 1.51 | 1.9 | 0.010 | 0.015 | 0.12 | 4435 |
| Go | 0.073 | 0.071 to 0.076 | 2.31 | 1.9 | 0.018 | 0.027 | 3.88 | 2302 |
| Java | 0.079 | 0.079 to 0.080 | 2.50 | 39.7 | 0.034 | 0.047 | 0.49 | <1 |
| JavaScript (Node) | 0.212 | 0.212 to 0.218 | 6.68 | 51.7 | 0.071 | 0.094 | 0.00 | <1 |

### parser, size 150,000

| Peer | Median (s) | Fastest to slowest (s) | Against C (gcc) | Peak memory (MiB) | At 37,500 (s) | At 75,000 (s) | Build (s) | Size (KiB) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CPython 3.12 | 2.607 | 2.578 to 2.712 | 8.12 | 11.1 | 0.647 | 1.275 | 0.00 | 2 |
| CPython 3.13 | 2.504 | 2.395 to 2.507 | 7.79 | 11.2 | 0.610 | 1.223 | 0.00 | 2 |
| CPython 3.14 | 2.327 | 2.306 to 2.344 | 7.24 | 10.8 | 0.615 | 1.168 | 0.00 | 2 |
| CPython 3.15 | 2.419 | 2.391 to 2.423 | 7.53 | 12.0 | 0.610 | 1.236 | 0.00 | 2 |
| PyPy | 0.409 | 0.402 to 0.415 | 1.27 | 62.5 | 0.172 | 0.252 | 0.00 | 2 |
| mypyc | 0.879 | 0.866 to 0.891 | 2.74 | 12.1 | 0.259 | 0.465 | 4.87 | 597 |
| Cython | 2.123 | 2.099 to 2.183 | 6.61 | 12.4 | 0.537 | 1.063 | 4.43 | 1300 |
| Nuitka | 3.656 | 3.555 to 3.851 | 11.38 | 26.0 | 1.001 | 1.870 | 8.26 | 360 |
| Codon | 0.519 | 0.512 to 0.536 | 1.62 | 9.9 | 0.129 | 0.262 | 7.88 | 134 |
| TyPy | 1.280 | 1.254 to 1.281 | 3.98 | 1.9 | 0.314 | 0.624 | 0.16 | 234 |
| C (gcc) | 0.321 | 0.313 to 0.350 | 1.00 | 1.5 | 0.073 | 0.157 | 0.08 | 16 |
| C (clang) | 0.342 | 0.327 to 0.345 | 1.06 | 1.9 | 0.079 | 0.165 | 0.13 | 16 |
| C++ (g++) | 0.315 | 0.314 to 0.322 | 0.98 | 3.5 | 0.084 | 0.164 | 0.71 | 29 |
| Rust | 0.608 | 0.603 to 0.619 | 1.89 | 2.0 | 0.144 | 0.295 | 0.25 | 4445 |
| Go | 0.378 | 0.375 to 0.382 | 1.18 | 7.3 | 0.098 | 0.191 | 3.98 | 2314 |
| Java | 0.411 | 0.407 to 0.412 | 1.28 | 192.3 | 0.210 | 0.282 | 0.60 | 3 |
| JavaScript (Node) | 0.463 | 0.455 to 0.466 | 1.44 | 58.2 | 0.143 | 0.258 | 0.00 | 2 |

<!-- results end -->

## 4. Observations

These notes read the figures of 2026-11-24 above, measured one job at a
time on cores 2 and 3. TyPy was built from the code of 0.12 before its
version number was raised, so it names itself 0.11.0.

### 4.1 Where TyPy stands

TyPy's programs take 4.06 times as long as C's, as the geometric mean of
the twelve workloads, against 21.9 to 25.0 times for CPython 3.12 to
3.15. NFR-22 asks for 5 times by 0.12, NFR-23 for 3 times by 0.15 and
NFR-03 for 1.5 times by 1.0. Among the compilers of Python, Codon (1.32)
and PyPy (3.27) are faster, and mypyc (4.33), Cython in pure Python mode
(15.4) and Nuitka (29.7) are slower.

Every workload built with `--release` runs faster than with CPython 3.15,
from 0.036 of CPython's time for mandelbrot to 0.70 for knucleotide; the
geometric mean is 0.18 of CPython's time, as NFR-21 asks.

TyPy is closest to C on mandelbrot at 1.20 times, strings at 2.48, dicts
at 3.39 and binarytrees at 3.58. It is furthest on spectralnorm at 9.96,
recursion at 7.36, fannkuchredux at 5.58 and knucleotide at 5.43.

### 4.2 What changed since 2026-11-22

The figures of 2026-11-22 were measured three jobs at once, which
lengthened C's runs more than TyPy's: nbody took 7.5 times C's time
there and 8.6 times alone. The changes below are therefore smaller than
those one job at a time would show:

| Workload | TyPy against C on 2026-11-22 | On 2026-11-24 |
| --- | --- | --- |
| spectralnorm | 38.0 | 9.96 |
| fannkuchredux | 27.6 | 5.58 |
| recursion | 18.9 | 7.36 |
| dispatch | 7.71 | 3.80 |
| nbody | 7.55 | 4.27 |
| fasta | 5.87 | 3.81 |
| mandelbrot | 4.07 | 1.20 |

Three changes to the generated code account for it. `int` arithmetic,
comparisons and reference counts on values that fit in 63 bits run in
place, and the runtime is called only for larger integers (ADR-0102).
Integers above that keep up to 1024 bits in fixed-width storage and
divide by a reciprocal (ADR-0103). And `list[i]`, `list[i] = x`, `//` and
`%` of small integers, `float()` of one and `/` of two floats run in
place, calling the runtime only where they may raise or an item needs a
reference count (ADR-0107).

### 4.3 Where the time goes now

spectralnorm and recursion stay furthest from C. Both spend their time
in calls of small functions on numbers, and both gained the most when
X10 let the optimiser inline the runtime (section 4.5): 1.25 and 1.29
times. The calls into the runtime that an archive compiled apart keeps
are the largest cost these figures point to.

### 4.4 Memory and building

TyPy's peak memory is 1.12 times C's as the geometric mean, the smallest
of the Python peers, which hold an interpreter or a runtime before the
program starts: 6.4 times for CPython 3.15, 6.2 for mypyc and 31 for
PyPy. knucleotide peaks at 16.4 MiB against C's 18.6 and CPython's 28.6,
and binarytrees at 13.6 MiB against C's 9.5. Its startup, 0.6 ms, is
that of C.

TyPy builds each workload in 0.12 seconds as the mean, near gcc's 0.08,
while the other compilers of Python take 3.7 to 8.2 seconds and Go 4.0
from a clean cache. Its executables are 208 KiB as the mean, against
16 KiB for C, since each holds the part of TyPy's runtime it uses,
linked statically; Rust's, built by `rustc` without stripping, are
4.3 MiB, and Go's 2.2 MiB.

### 4.5 Linking the runtime as bitcode

The X10 experiment of the
[research note on command lines](../research/compiler-command-lines.md)
linked the runtime into each program as LLVM bitcode, so the optimiser
could inline it. On 2026-11-23 the geometric mean fell from 5.29 to 4.96
times C's time, about 6 percent: recursion ran 1.29 times and
spectralnorm 1.25 times faster, and nbody 0.89 times as fast. One build
took 1.95 to 3.70 seconds instead of 0.06 to 0.17, since the runtime
functions a program reaches are optimised again on every build. Release
builds keep the archive, and `-C inline-runtime=yes` is planned for 0.13
as an option (CLI-31, ADR-0104).

### 4.6 What the figures do not show

The machine is a laptop processor whose clock follows its temperature,
so two runs of one program differ by a few percent and now and then by
more; the fastest and slowest of the three runs are given beside each
median. Every program is single threaded, and Java's and Node's figures
include their JIT compilers' warm-up, which a long-running server would
not pay for each request.
