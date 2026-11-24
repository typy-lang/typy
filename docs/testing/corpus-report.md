# TyPy corpus report

| Field | Value |
| --- | --- |
| Document | TyPy corpus report |
| Version | 0.3 |
| Status | Draft |
| Owner | typy-lang contributors |
| Last updated | 2026-11-24 |

## 1. Purpose

This report gives the share of a fixed corpus of typed Python that TyPy
compiles unchanged and runs with CPython's output, as NFR-24 asks of each
release, and the share must not fall from one release to the next. It
builds on the [test plan](test-plan.md), whose corpus level it describes,
and it shows which refusals keep the most code out, which the planning of
later releases reads.

## 2. Method

### 2.1 Corpora

Two corpora of Python written by others, each pinned to one commit and
fetched once into the cache volume, outside the repository:

| Corpus | Repository | Commit | Part read |
| --- | --- | --- | --- |
| TheAlgorithms/Python | `github.com/TheAlgorithms/Python` | `30507bbe` of 2026-11-11 | Every algorithm script, its tests left out |
| mypyc run tests | `github.com/python/mypy` | `1572a188` of 2026-11-20 | `mypyc/test-data/run-*.test` |

The algorithm scripts are programs written to be read, one algorithm a
file, with type hints. The run tests are the end-to-end tests of mypyc, a
compiler of typed Python: each case whose main module stands alone becomes
a program that calls its test functions in order, as mypyc's harness does,
and prints `done`.

### 2.2 The corpus

A module joins the corpus when it:

1. imports only modules of the standard library,
2. runs something under `if __name__ == "__main__":`,
3. passes `mypy --strict` for Python 3.15, with mypy 2.4.1, and
4. runs under CPython 3.15 to an exit status of 0 within 20 seconds,
   printing the same output on two runs.

`scripts/corpus.py --select` applies these rules and writes the names of
the modules it chooses to `tests/corpus/modules.txt`. It runs when a pin
changes, with the mypy that `make bench-peers` installs. Between pins the
list stays as it is, so each release is measured on the same modules and
the release workflow needs no mypy.

### 2.3 What counts

A module of the list counts when TyPy builds it unchanged, in a debug
build, and the program prints what CPython 3.15 prints and exits with the
same status, within 20 seconds. The share is the modules that count over
the modules of the list.

### 2.4 Running it

| Command | What it does |
| --- | --- |
| `make corpus-select` | Fetches the corpora the first time, about 19 MB, chooses the corpus and writes its list and section 3.1 |
| `make corpus` | Builds and runs the modules of the list and writes section 3.2, unless a share falls below the one section 3.2 holds, which NFR-24 forbids |
| `make corpus-check` | Builds and runs them as `make corpus` does, and fails when a share falls below the one section 3.2 holds |

The release workflow runs `make corpus-check` before it builds the
release archive.

## 3. Results

### 3.1 Choosing the corpus

<!-- corpus selection -->

| Corpus | Commit | Modules read | Chosen |
| --- | --- | --- | --- |
| TheAlgorithms/Python | 30507bb | 1346 | 584 |
| mypyc run tests | 1572a18 | 245 | 83 |

| Corpus | Left out because | Modules |
| --- | --- | --- |
| TheAlgorithms/Python | fails mypy --strict | 273 |
| TheAlgorithms/Python | imports a module outside the standard library | 248 |
| TheAlgorithms/Python | fails or runs too long under CPython | 145 |
| TheAlgorithms/Python | runs nothing as a program | 76 |
| TheAlgorithms/Python | prints differently each run under CPython | 20 |
| mypyc run tests | imports a module outside the standard library | 133 |
| mypyc run tests | fails mypy --strict | 25 |
| mypyc run tests | fails or runs too long under CPython | 4 |

<!-- end of corpus selection -->

### 3.2 Results

<!-- corpus results -->

| Corpus | Commit | Modules | Built | Same output | Share |
| --- | --- | --- | --- | --- | --- |
| TheAlgorithms/Python | 30507bb | 584 | 85 | 85 | 14.6 % |
| mypyc run tests | 1572a18 | 83 | 19 | 19 | 22.9 % |

| Corpus | Outcome | Modules |
| --- | --- | --- |
| TheAlgorithms/Python | refused: E0201 | 295 |
| TheAlgorithms/Python | refused: E0206 | 56 |
| TheAlgorithms/Python | refused: E0412 | 26 |
| TheAlgorithms/Python | refused: E0404 | 23 |
| TheAlgorithms/Python | refused: E0302 | 21 |
| TheAlgorithms/Python | refused: E0202 | 17 |
| TheAlgorithms/Python | refused: E0103 | 9 |
| TheAlgorithms/Python | refused: E0411 | 8 |
| TheAlgorithms/Python | refused: E0402 | 7 |
| TheAlgorithms/Python | refused: E0410 | 7 |
| TheAlgorithms/Python | refused: E0303 | 5 |
| TheAlgorithms/Python | refused: E0306 | 4 |
| TheAlgorithms/Python | refused: E0401 | 4 |
| TheAlgorithms/Python | refused: E0405 | 4 |
| TheAlgorithms/Python | refused: E0205 | 3 |
| TheAlgorithms/Python | refused: E0407 | 3 |
| TheAlgorithms/Python | refused: E0408 | 2 |
| TheAlgorithms/Python | refused: E0409 | 2 |
| TheAlgorithms/Python | refused: E0301 | 1 |
| TheAlgorithms/Python | refused: E0305 | 1 |
| TheAlgorithms/Python | refused: E0309 | 1 |
| mypyc run tests | refused: E0201 | 29 |
| mypyc run tests | refused: E0202 | 9 |
| mypyc run tests | refused: E0104 | 4 |
| mypyc run tests | refused: E0206 | 4 |
| mypyc run tests | refused: E0404 | 3 |
| mypyc run tests | refused: E0205 | 2 |
| mypyc run tests | refused: E0309 | 2 |
| mypyc run tests | refused: E0412 | 2 |
| mypyc run tests | refused: E0414 | 2 |
| mypyc run tests | refused: E0301 | 1 |
| mypyc run tests | refused: E0303 | 1 |
| mypyc run tests | refused: E0402 | 1 |
| mypyc run tests | refused: E0407 | 1 |
| mypyc run tests | refused: E0409 | 1 |
| mypyc run tests | refused: E0411 | 1 |
| mypyc run tests | refused: E0425 | 1 |

<!-- end of corpus results -->

## 4. Reading the results

The reasons in section 3.1 say why a module read is not in the corpus.
In section 3.2, "refused" names the first diagnostic of a module TyPy
does not build, and "differs" counts a program that runs to another
output or exit status than CPython's, or for longer than 20 seconds.

### 4.1 What keeps the most code out

On 2026-11-24, with TyPy 0.12, every module of the corpus that TyPy built
ran as CPython runs it. The shares rose from 11.0 % and 18.1 % in 0.11, as
25 more modules built: `from __future__ import annotations` (LIB-29),
empty collections typed from their first use (TYP-27), the builtins of
LIB-28 and the modules of LIB-27 reached them. The first error of the
modules it refused, in both corpora, comes mostly from these causes:

| First error | Modules | Plan |
| --- | --- | --- |
| `import doctest`, whose `testmod()` is the module's main program | 248 | After 1.0 (OI-43) |
| Another module of the standard library, such as `collections`, `itertools`, `functools` or `re`, E0206 | 60 | Open (OI-49) |
| `typing.Any` | 25 | 0.13 (TYP-25) |
| A default other than a constant | 23 | 1.0 (EXP-20) |
| An `int` where a `float` is declared | 21 | Kept: CPython keeps the value an `int` (TYP-09, OI-12) |
| `x ** y` whose result CPython types by the exponent's value, E0302 | 21 | Kept (EXP-04, OI-09) |
| An optional that `mypy --strict` narrows and TyPy does not, E0411 or E0412 | 17 | Open (OI-51) |
| A method of a built-in type TyPy lacks: `tuple.count`, `int.bit_length` or `int.from_bytes` | 14 | Open (OI-50) |
| `random`, `statistics`, `copy` or `time` | 10 | 0.13 (LIB-30, LIB-33) |
| `sum` of floats without a float start | 8 | Kept: CPython returns the `int` 0 for an empty list (LIB-02) |
