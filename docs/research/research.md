# TyPy research: compiling standard Python ahead of time

| Field | Value |
| --- | --- |
| Document | TyPy research |
| Version | 1.1 |
| Status | Approved |
| Owner | typy-lang contributors |
| Last updated | 2026-11-05 |

## 1. Purpose

This document records what already exists for compiling Python ahead of time
to native code, and where each option falls short of TyPy's goals. It is the
input to the software requirements specification, which comes next.

It builds on the TyPy-Lang Specification Pack, version 0.1 (draft) of
2026-11-02, referred to here as "the pack". The pack's goals G1 to G5 and its
requirement IDs are quoted where they apply. This document replaces one rule
of the pack. The pack treats Python compatibility as a tiebreaker (principle
4 and INV-007). TyPy instead follows the standard-Python rule:

> Every TyPy program is a valid Python program. When CPython runs it without
> error, it produces the same output and exit code as the TyPy binary. TyPy
> may reject a program that CPython accepts, but it never gives a valid Python
> program a different meaning.

TyPy is therefore a strict subset of Python, not a variant of it.

## 2. Method

All facts were collected on 2026-11-05. Versions come from the GitHub
releases API, crates.io, PyPI, python.org, the Debian trixie archive and
apt.llvm.org. Behaviour comes from each project's own documentation, quoted
in section 4. The experiments in section 7 ran in the project devcontainer,
image `typy-lang-dev:1.99.1` (Rust 1.99.1 on Debian trixie), and in the
`python:3.13-slim-bookworm` image.

A finding that rests on one source only says so.

## 3. Criteria

Each existing compiler is measured against the criteria below. They come
from the standard-Python rule and from the pack's goals.

| ID | Criterion | Source |
| --- | --- | --- |
| K1 | Every accepted program is valid Python syntax | Standard-Python rule |
| K2 | An accepted program behaves as it does under CPython | Standard-Python rule |
| K3 | Output is a native binary that needs no CPython at run time | G1, NG4 |
| K4 | No tracing garbage collector | G3 |
| K5 | Calls C functions and links `.so` and `.a` files directly | G4 |
| K6 | Released in the last 12 months | Project health |

## 4. Existing compilers

### 4.1 LPython

LPython compiles a typed subset of Python through LLVM, C, C++ or WASM
backends. Its site states the guarantee closest to TyPy's rule: "If LPython
compiles and runs a code, then it will run in CPython." It describes itself
as "in alpha stage".

The latest release is v0.22.0 of 2024-07-08, and the last commit was on
2025-12-11. The PyPI package `lpython` is at 0.0.16.0 (2025-06-28). The
guarantee covers syntax and that the program runs. The site does not promise
the same output.

Where it falls short: no release in 28 months (K6), and the guarantee is
one-way and does not cover output (K2).

### 4.2 Codon

Codon, from Exaloop, compiles Python-like source through LLVM to native
code. The latest release is v0.20.6 of 2026-11-01, under Apache-2.0. Its
documentation lists deliberate differences from Python, under "Differences
with Python":

- "Codon's `int` is a 64-bit signed integer, whereas Python's (after
  version 3) can be arbitrarily large."
- "For performance reasons, some numeric operations use C semantics rather
  than Python semantics." A `-numerics=py` flag restores Python numerics.
- Ordered dictionaries are now the default. The old unordered dictionary is
  still available through the `-unordered-dict` flag.
- Tuple lengths must be known at compile time.
- Modules without a native implementation load from CPython through
  `from python import`.

Where it falls short: valid Python can give a different result, for example
integer overflow under C numerics (K2). The same source means different
things under different flags, which also conflicts with the pack's INV-001.
`from python import` brings CPython into the binary (K3).

### 4.3 mypyc

mypyc compiles type-annotated Python into CPython C extension modules. It
ships with mypy, at 2.4.1 (2026-10-28). Its documentation lists the
differences: unboxed integers can change the runtime type ("a `bool`
passed through a `list[int]` comes back as `1`"), compiled modules cannot
run as scripts, and nested classes, conditional definitions and generator
expressions are not supported.

Where it falls short: the output is an extension module loaded by CPython,
not a standalone binary (K3), and it keeps CPython's garbage collector (K4).

### 4.4 Cython

Cython compiles Python and its own `.pyx` dialect into C extension modules.
The latest release is 3.3.1 of 2026-11-01, under Apache-2.0. Its pure Python
mode accepts standard Python syntax with types from the `cython` module.

Where it falls short: the output runs inside CPython (K3, K4), and most
speed comes from the `.pyx` dialect, which is not Python (K1).

### 4.5 SPy

SPy is described in its repository as a variant of Python designed to be
statically compilable. It compiles to C, then to native or WebAssembly
binaries. The license is MIT. Its only release is v0.0.0 of 2026-01-08, and
it is under active development (last commit 2026-11-05). Its README says
"the documentation is very scarce at the moment". The design is discussed in
the "Inside SPy" posts by Antonio Cuni.

Where it falls short: it is a variant by design, not a subset (K1, K2), and
it is pre-release.

### 4.6 Summary

| Compiler | K1 | K2 | K3 | K4 | K5 | K6 |
| --- | --- | --- | --- | --- | --- | --- |
| LPython 0.22.0 | Yes | Partly | Yes | Yes | Yes | No |
| Codon 0.20.6 | Mostly | No | Partly | Yes | Yes | Yes |
| mypyc in mypy 2.4.1 | Yes | Mostly | No | No | No | Yes |
| Cython 3.3.1 | Partly | Mostly | No | No | Yes | Yes |
| SPy 0.0.0 | No | No | Yes | Yes | Yes | Yes |

No maintained project meets K1 to K5 together. LPython comes closest in
intent but has stopped releasing. This is the gap TyPy fills.

## 5. Building blocks

### 5.1 Python parser for a Rust compiler

| Option | Version and date | Python syntax covered | Notes |
| --- | --- | --- | --- |
| `ruff_python_parser` | 0.0.20, 2026-10-27 | Up to 3.15 | Published by Astral from the Ruff repository, MIT. Its crates.io description reads "This is an internal component crate of Ruff". |
| `rustpython-parser` | 0.4.0, 2024-08-06 | Up to 3.12 at that date | No release in 27 months. |
| A parser of TyPy's own | None | As written | The pack's Accepted choice: recursive descent with Pratt parsing for expressions. |

`ruff_python_parser` is used by Ruff (0.17.3, 2026-10-27) and by Astral's
type checker, so it tracks new Python syntax quickly. The cost is that it
makes no stability promise: it is a 0.0.x crate, with versions 0.0.18,
0.0.19 and 0.0.20 published within 12 days. A compiler that uses it shall
pin an exact version and upgrade on purpose.

The parser already reports the source spans and the recovery behaviour the
pack asks for (pack section 8, stage 2). See experiment X1.

A parser of TyPy's own would follow every change to Python's grammar by
hand. Under the standard-Python rule, the grammar is Python's, so writing
one buys nothing that `ruff_python_parser` does not already give.

### 5.2 LLVM binding

`inkwell` 0.11.0 (2026-10-20, Apache-2.0) is a typed Rust wrapper over
`llvm-sys` (231.0.2, 2026-10-30). Its feature flags select one LLVM major
version: `llvm19-1`, `llvm20-1`, `llvm21-1`, `llvm22-1` and `llvm23-1` are
all supported. This fits the pack's Accepted choice of one pinned LLVM major
version per compiler release.

### 5.3 LLVM availability

| Source | Versions for Debian trixie |
| --- | --- |
| Debian archive | `llvm-19-dev` 1:19.1.7-3+b1 only |
| apt.llvm.org | 19, 20, 21, 22 and 23 |
| LLVM upstream | Latest release llvmorg-23.1.5, 2026-11-03 |

LLVM releases a new major version about every six months. Debian's LLVM 19
is four major versions behind. apt.llvm.org packages every current version
for trixie, so the devcontainer can pin any of them.

### 5.4 CPython as the oracle

Under the standard-Python rule, CPython's output is the expected output of
every run test. The python.org downloads page lists these series on
2026-11-05:

| Series | Latest | Status | End of support |
| --- | --- | --- | --- |
| 3.15 | 3.15.2 | Bug fixes | 2031-10 |
| 3.14 | 3.14.10 | Bug fixes | 2030-10 |
| 3.13 | 3.13.18 | Security fixes | 2029-10 |
| 3.12 | 3.12.16 | Security fixes | 2028-10 |

Python 3.12 is the lowest version that can be the baseline. It introduced
the type parameter syntax of PEP 695 (`def first[T](xs: list[T])`) and the
`type` statement, which TyPy uses for generics and type aliases.

Debian trixie ships Python 3.13.5 only. Testing several versions therefore
needs the official `python:3.X-slim` images or a tool such as `uv` that
installs pinned interpreters.

Python 3.14 changed when annotations are evaluated (PEP 649, deferred
annotations). Under 3.12 and 3.13, annotations on module-level names and
class fields are evaluated when the line runs. So every name in such an
annotation shall exist under CPython. Experiment X2 shows that this needs a
`typy` package for CPython, providing `i32`, `ptr`, `extern` and the other
TyPy names with matching behaviour.

### 5.5 Static Linux binaries

The pack's size budget (REQ-N-001, a hello world binary under 100 KB) is
measured against musl. Debian trixie provides `musl-tools` 1.2.5, and Rust
provides the `x86_64-unknown-linux-musl` target for the compiler's own
runtime tests.

### 5.6 Which Python version defines the language

Three Python versions play different parts. Python 3.10 sets the scope of
TyPy 1.0. The Language Reference for 3.12 is the text TyPy's rules cite.
CPython 3.12 to 3.15 decide what a program does.

Python 3.10 is the first version in which typed Python reads naturally. The
notation for types arrived in this order:

| Typing notation | Example | First version | PEP |
| --- | --- | --- | --- |
| Function annotations | `def f(x: int) -> int:` | 3.0 | 3107 |
| `typing` module | `from typing import Final` | 3.5 | 484 |
| Variable annotations | `total: int = 0` | 3.6 | 526 |
| Generic built-in types | `list[int]`, `dict[str, int]` | 3.9 | 585 |
| Union operator | `int \| None` | 3.10 | 604 |
| Type aliases, `ParamSpec`, `TypeGuard` | `Pair: TypeAlias = tuple[int, int]` | 3.10 | 613, 612, 647 |
| `Self`, `Never`, `assert_never` | `def copy(self) -> Self:` | 3.11 | 673 |
| Type parameters, `type` statement | `def first[T](xs: list[T]) -> T:` | 3.12 | 695 |

The run-time features of the 3.10 language arrived in this order:

| First version | Feature | PEP |
| --- | --- | --- |
| 3.6 | f-strings | 498 |
| 3.7 | Data classes | 557 |
| 3.8 | Assignment expressions `:=` | 572 |
| 3.8 | `typing.Protocol` and `Final` | 544, 591 |
| 3.10 | `match` statement | 634 |

The one gap in 3.10 is generics. In 3.10 a type parameter is created by a
call at run time, `T = TypeVar("T")`, and a generic class inherits from
`Generic[T]`. A compiler has to treat that call as a declaration. PEP 695
gives generics their own syntax, and every oracle version accepts it, so
TyPy can use the PEP 695 form for generics while keeping 3.10 as its scope.

The earlier references cannot serve as the rules' text. CPython 3.0 is long
out of support, and the meaning of some 3.0 features has changed since:

| Behaviour | In 3.0 | Since |
| --- | --- | --- |
| Dictionary iteration order | Not defined | Insertion order, guaranteed from 3.7 |
| `StopIteration` raised inside a generator | Ends the generator | Becomes `RuntimeError` from 3.7 (PEP 479) |
| `async` and `await` | Ordinary names | Reserved words from 3.7 |

A rule that cited the 3.0 text would contradict a test run on CPython 3.12.
The 3.12 reference describes every 3.10 feature with the meaning the oracle
gives it, so the rules cite the 3.12 reference.

## 6. Exceptions without an unwinding runtime

TyPy keeps Python's `try`, `except` and `raise`. The pack bans them (ERR-004)
to keep the runtime small (REQ-N-002, under 32 KB). Three ways to compile
exceptions are documented in Swift's error handling rationale
(`docs/ErrorHandlingRationale.md` in the Swift repository):

| Strategy | Cost when nothing is raised | Cost when raising | Runtime support |
| --- | --- | --- | --- |
| Table-based unwinding, as in C++ | Low, but blocks some optimisations | High: a table lookup in every frame on the way up | Unwinder and tables in every binary |
| `setjmp` and `longjmp` | Work on every frame that has cleanups | Low | Small |
| Error returned to the caller and checked after each call | A compare and branch after each call that may raise | Low | None |

The rationale finds table-based unwinding expensive to raise through, and
says the "zero-cost" label is "something of a misnomer, because it does have
several costs that can affect non-error performance".

Swift returns the error to the caller. LLVM supports this directly: the
`swifterror` parameter attribute lets "the calling convention optimize
access to swifterror variables by associating them with a specific register
at call boundaries rather than placing them in memory" (LLVM Language
Reference). The register approach changes the calling convention, so it is
not ABI-compatible with plain C functions.

For TyPy, returning the error needs no unwinder and no tables, which keeps
REQ-N-002 within reach. The compiler sees the whole program, so it can work
out which functions may raise without new syntax. Exported functions
(`@export_c`) do not pass errors across the C boundary. They abort, as the
pack already requires for panics (FFI-023).

## 7. Experiments

### 7.1 X1: parsing TyPy written as standard Python

A Rust program linked `ruff_python_parser` 0.0.20 and parsed two files in
the devcontainer.

`sample.py` expresses the pack's features in standard Python: a frozen
dataclass in place of `struct`, a `Protocol`, PEP 695 generics with a bound,
`T | None` in place of `T?`, a `type` alias, `@link` and `@extern` on a stub
function, `try` and `except`, `with unsafe():`, and `match`. An excerpt:

```python
@dataclass(frozen=True)
class Point:
    x: f64
    y: f64


def first[T](xs: list[T]) -> T | None:
    return xs[0] if len(xs) > 0 else None


@link("c")
@extern
def strlen(s: cstr) -> usize: ...
```

`bad.py` uses the pack's own syntax (`let x = 1` and `struct Point:`).

| File | Top-level statements | Syntax errors |
| --- | --- | --- |
| `sample.py` | 11, each recognised: `ImportFrom`, `AnnAssign`, `ClassDef`, `FunctionDef`, `TypeAlias` | 0 |
| `bad.py` | 5, with recovery | 4, the first "Simple statements must be separated by newlines or semicolons" |

Result: every TyPy feature in the pack has a standard Python form that the
parser accepts. The pack's own syntax is rejected, with several errors
reported in one pass.

### 7.2 X2: the same file under CPython

`sample.py` ran under CPython 3.13.16 with a small stand-in `typy` package.
The stand-in maps `f64` to `float`, makes `i32` and `u8` integers that raise
`OverflowError` outside their range, and binds `@extern` functions through
`ctypes`. The program printed `total = 50.0 1 3` and exited with code 0.
`ast.parse` rejected `bad.py` with `SyntaxError: invalid syntax`.

Result: CPython can serve as the oracle, but only with a `typy` package
installed. That package is part of the product, and its behaviour defines
what fixed-width types do under CPython.

The stand-in also shows a way to keep overflow consistent. If a fixed-width
value that overflows raises `OverflowError` under both CPython and TyPy,
then the pack's overflow panic (SEM-010) becomes an ordinary Python
exception. It can then be caught and tested like any other.

## 8. Findings

1. No maintained compiler meets K1 to K5 together (section 4.6). TyPy is
   not duplicating existing work.
2. Standard Python syntax covers every feature in the pack. The forms are
   decorators, annotations, `typing.Protocol`, dataclasses, PEP 695
   generics and context managers (X1).
3. A Rust compiler can use `ruff_python_parser` instead of its own parser,
   provided the version is pinned exactly (section 5.1).
4. `inkwell` 0.11.0 supports LLVM 19 to 23, and apt.llvm.org packages each of
   them for trixie (sections 5.2 and 5.3).
5. CPython works as the test oracle only with a `typy` package for CPython
   that defines the TyPy-specific names (X2).
   The requirements specification removes those names: TyPy is an
   independent implementation of Python and adds no names of its own
   (STD-07). The oracle is therefore plain CPython, and no package is
   needed.
6. Returning errors to the caller supports Python exceptions with no
   unwinder, consistent with the 32 KB runtime budget (section 6).
7. Python 3.10 is a sound scope for TyPy 1.0: it is the first version in
   which typed Python reads naturally. PEP 695 from 3.12 fills its one gap,
   generics. The rules cite the 3.12 reference, because its text matches
   the oracle's behaviour and the 3.0 text does not (section 5.6).

## 9. Questions for the requirements

The specification settles each of these as an open issue. The project
owner answered OI-01 and OI-07 on 2026-11-05; the specification records them
as resolved.

| ID | Question | Input from this research |
| --- | --- | --- |
| OI-01 | Which Python versions are the baseline and the test matrix? | 3.12 is the floor (PEP 695). 3.14 changes annotation evaluation. 3.12 reaches end of support in 2028-10. Answered: CPython 3.12 to 3.15 are the oracle. |
| OI-02 | Which LLVM major version does the first release pin? | 19 is in Debian; 23 is the latest; `inkwell` supports both. |
| OI-03 | How is `ruff_python_parser` pinned and upgraded? | It is internal to Ruff and released about weekly. |
| OI-04 | What does the `typy` package for CPython contain, and how is it shipped? | Required for the oracle (X2). |
| OI-05 | Does fixed-width overflow raise `OverflowError` instead of panicking? | Keeps behaviour the same under CPython and TyPy (X2). |
| OI-06 | What does TyPy's `int` mean? | Python's `int` is unbounded. A 64-bit `int` gives the same result only when nothing overflows. |
| OI-07 | Which Python version sets the scope, and which reference is the rules' text? | Section 5.6. Answered: TyPy 1.0 covers the typeable Python 3.10 language, with PEP 695 generics. The 3.12 Language Reference is the cited text. |

## 10. References

| Source | Version or date | Location |
| --- | --- | --- |
| TyPy-Lang Specification Pack | 0.1, 2026-11-02 | Provided by the project owner |
| LPython | v0.22.0, 2024-07-08 | <https://lpython.org> and <https://github.com/lcompilers/lpython> |
| Codon, "Differences with Python" | v0.20.6, 2026-11-01 | <https://docs.exaloop.io/language/overview/> |
| mypyc, "Differences from Python" | mypy 2.4.1, 2026-10-28 | <https://mypyc.readthedocs.io/en/latest/differences_from_python.html> |
| Cython | 3.3.1, 2026-11-01 | <https://github.com/cython/cython> |
| SPy | v0.0.0, 2026-01-08 | <https://github.com/spylang/spy> |
| Ruff and `ruff_python_parser` | 0.17.3 and 0.0.20, 2026-10-27 | <https://github.com/astral-sh/ruff> |
| `inkwell` | 0.11.0, 2026-10-20 | <https://github.com/TheDan64/inkwell> |
| LLVM | llvmorg-23.1.5, 2026-11-03 | <https://github.com/llvm/llvm-project> |
| LLVM Language Reference, `swifterror` | Read 2026-11-05 | <https://llvm.org/docs/LangRef.html> |
| Swift error handling rationale | Read 2026-11-05 | <https://github.com/swiftlang/swift/blob/main/docs/ErrorHandlingRationale.md> |
| Python downloads | Read 2026-11-05 | <https://www.python.org/downloads/> |
| The Python Language Reference | 3.12 | <https://docs.python.org/3.12/reference/index.html> |
| The Python Language Reference | 3.0.1 | <https://docs.python.org/3.0/reference/index.html> |
| PEP 604, union operator for types | Python 3.10 | <https://peps.python.org/pep-0604/> |
| PEP 634, structural pattern matching | Python 3.10 | <https://peps.python.org/pep-0634/> |
| PEP 361, Python 2.6 and 3.0 release schedule | 3.0 final on 2008-12-03 | <https://peps.python.org/pep-0361/> |
| PEP 3107, function annotations | Python 3.0 | <https://peps.python.org/pep-3107/> |
| PEP 479, `StopIteration` inside generators | Python 3.7 | <https://peps.python.org/pep-0479/> |
| PEP 695, type parameter syntax | Python 3.12 | <https://peps.python.org/pep-0695/> |
| PEP 649, deferred evaluation of annotations | Python 3.14 | <https://peps.python.org/pep-0649/> |
| apt.llvm.org | Read 2026-11-05 | <https://apt.llvm.org> |
