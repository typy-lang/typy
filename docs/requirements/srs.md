# TyPy software requirements specification

| Field | Value |
| --- | --- |
| Document | TyPy software requirements specification |
| Version | 1.2 |
| Status | Approved |
| Owner | typy-lang contributors |
| Last updated | 2026-11-06 |

## 1. Introduction

### 1.1 Purpose

This document states what TyPy shall do, release by release, up to 1.0. It
builds on the approved [research](../research/research.md), and every later
document traces back to the IDs defined here: the use case model, the domain
model, the architecture, the decision records, the design and the test plan.

### 1.2 Scope

TyPy is an ahead-of-time compiler for a strict subset of standard Python. It
turns typed Python source into native executables and libraries through
LLVM, with no interpreter and no tracing garbage collector in the output.

TyPy 1.0 covers the typeable part of the Python 3.10 language, plus the type
parameter syntax of PEP 695 for generics. It is an independent
implementation of Python: it adds no modules, functions or types of its own,
and it provides its own implementation of the standard library modules it
supports. It rejects any program it cannot compile with the same meaning
that CPython gives it. Section 6 lists what is
out of scope, and what is planned for after 1.0.

### 1.3 Definitions

| Term | Meaning |
| --- | --- |
| Accepted program | A program that `typy check` reports no errors for |
| LR x.y | Section x.y of The Python Language Reference for Python 3.12 |
| Oracle | The CPython versions from the program's Python floor up to 3.15, run on the same source with nothing else installed |
| Python floor | The oldest CPython version a program supports: 3.12, 3.13, 3.14 or 3.15, and 3.12 unless `typy.toml` sets it (CLI-12) |
| Release | A TyPy version number, 0.1 to 1.0 |
| Runs alike | Produces the same standard output, the same exit status and the same last line of standard error as the oracle |
| Foreign call | A call through `ctypes` into a C function |
| Safe code | Code that makes no foreign call |
| Typeable | Every expression has one type that the compiler can determine without running the program |

### 1.4 References

| Reference | Version |
| --- | --- |
| TyPy research | 1.0, 2026-11-05 |
| TyPy-Lang Specification Pack, called "the pack" here | 0.1 draft, 2026-11-02 |
| The Python Language Reference | 3.12, <https://docs.python.org/3.12/reference/> |
| The Python Standard Library | 3.12, <https://docs.python.org/3.12/library/> |
| PEP 484, 526, 544, 557, 585, 591, 604, 634, 695 | As published on <https://peps.python.org> |
| RFC 2119, key words for requirement levels | <https://www.rfc-editor.org/rfc/rfc2119> |

### 1.5 Requirement format

Each requirement is one testable sentence with "shall". Its priority is
MoSCoW: M (must), S (should) or C (could). Its release is the first TyPy
release that meets it. The "LR" column names the Language Reference section
whose meaning the requirement keeps. An ID is never reused or renumbered.

## 2. Overall description

### 2.1 Product perspective

A TyPy program is an ordinary Python program. You write it, type check it
and format it with the usual Python tools, and you can run it with CPython.
TyPy adds one thing: it compiles the program to a native binary that behaves
the same way.

The product has three parts: the `typy` command line compiler, a small
runtime library linked into each binary, and TyPy's implementation of the
supported standard library modules. Nothing is installed into CPython.

### 2.2 Users

| User | Goal |
| --- | --- |
| Application developer | Ship a Python program as one small native binary |
| Library binder | Call a C library from Python code, or export Python code to C |
| Compiler contributor | Change TyPy with each rule traced to a test |
| Tool integrator | Read TyPy diagnostics in an editor or a CI system |

### 2.3 Operating environment

| Release | Host and target platforms |
| --- | --- |
| 0.1 | Linux x86_64 with glibc or musl |
| 0.5 | Adds Linux AArch64 and macOS AArch64 |
| 1.0 | Adds Windows x86_64 |

Building needs a system C linker driver (`cc`), or `lld`. Running the
oracle needs CPython 3.12 or later.

### 2.4 Constraints

| ID | Constraint | Reason |
| --- | --- | --- |
| C-01 | The compiler is written in Rust. | Decided by the project owner on 2026-11-05; meets NFR-04 and NFR-05 without a rewrite. |
| C-02 | Python source is parsed with `ruff_python_parser`, pinned to one exact version and upgraded at most once per TyPy minor release, after the full conformance suite passes on the new version. | The grammar is Python's; the crate has no stability promise and is released about weekly (research 5.1). |
| C-03 | LLVM is used through `inkwell`, with one LLVM major version per TyPy release. TyPy 0.1 uses LLVM 23, installed from apt.llvm.org. | Research 5.2 and 5.3; LLVM 23 is the latest major version, and Debian's LLVM 19 is four behind. |
| C-04 | Runtime and dependencies carry licences compatible with Apache-2.0 and MIT. | Binaries are linked with the runtime. |
| C-05 | The compiler makes no network access. | Builds are reproducible and work offline (NFR-06). |

### 2.5 Assumptions

| ID | Assumption |
| --- | --- |
| A-01 | The host has a C toolchain that can link for the target. |
| A-02 | Conformance testing has CPython 3.12 to 3.15 available, with no extra packages. |
| A-03 | Programs are single-threaded up to 1.0. |
| A-04 | Users type check with a standard checker, such as mypy, in addition to `typy check`. |

## 3. Functional requirements

### 3.1 Standard-Python rule (STD)

| ID | Requirement | Priority | Release | LR |
| --- | --- | --- | --- | --- |
| STD-01 | TyPy shall accept only source that every oracle version, from the program's Python floor up, parses without a syntax error (ADR-0008). | M | 0.1 | 10 |
| STD-02 | An accepted program shall run alike under TyPy and under the oracle, for every input on which the oracle finishes without an uncaught exception. | M | 0.1 | 9.1 |
| STD-03 | STD-02 shall cover only behaviour the Language Reference or the library documentation defines. It shall not cover `id()` values, `hash()` values, set iteration order, memory addresses in default `repr()` output, or the recursion limit. | M | 0.1 | 3.1 |
| STD-04 | TyPy shall reject, with a diagnostic, any program that uses a feature it cannot compile with CPython's meaning. It shall never compile such a feature with a different meaning. | M | 0.1 | |
| STD-05 | When the oracle ends with an uncaught exception, the TyPy binary shall end with the same exception type and message on the last line of standard error, and exit status 1. | M | 0.3 | 4.3 |
| STD-06 | A program shall run its module-level statements in order, as `python file.py` does, with `__name__` equal to `"__main__"` in the main module. | M | 0.1 | 9.1 |
| STD-07 | TyPy shall add no module, function, type, decorator or syntax of its own. A program shall use only the Python language and the standard library modules TyPy provides (LIB). | M | 0.1 | |

### 3.2 Command line and builds (CLI)

| ID | Requirement | Priority | Release |
| --- | --- | --- | --- |
| CLI-01 | `typy build FILE` shall produce a native executable for the host. | M | 0.1 |
| CLI-02 | `typy run FILE [-- ARGS]` shall build and run the program, passing `ARGS` as `sys.argv[1:]`. | M | 0.1 |
| CLI-03 | `typy check PATH` shall report every diagnostic without generating code. | M | 0.1 |
| CLI-04 | `typy build --emit=STAGE` shall write the output of one stage as deterministic text, for `ast`, `typed-ast`, `tyir`, `tyir-mem`, `llvm-ir` and `asm`, and as an object file for `obj`. | M | 0.1 |
| CLI-05 | `typy build` shall offer the modes `debug` (default), `--release` and `--release-small`, with the LLVM levels `-O0`, `-O2` and `-Oz`. | M | 0.1 |
| CLI-06 | The exit status of `typy` shall be 0 on success, 1 when the program has errors, 2 on wrong command usage, and 101 on an internal compiler error. | M | 0.1 |
| CLI-07 | Project settings shall be read from `typy.toml`, and a command line flag shall override the same setting in the file. | S | 0.3 |
| CLI-08 | `typy build --target TRIPLE` shall cross-compile for every supported target. | S | 0.5 |
| CLI-09 | `typy build --lib static` shall produce a static library and `--lib shared` a shared library. | M | 0.5 |
| CLI-10 | `typy explain CODE` shall print an explanation of the diagnostic with a wrong and a corrected example. | S | 0.3 |
| CLI-11 | `typy` shall read no environment variable that changes the output, except `TYPY_HOME`. | M | 0.1 |
| CLI-12 | The setting `python-floor` in `typy.toml` shall set the program's Python floor to one of `3.12`, `3.13`, `3.14` or `3.15`, and any other value shall be rejected. | S | 0.3 |

TyPy has no formatter of its own. TyPy source is Python, so a standard
formatter applies; Ruff's formatter is the project's recommendation.

### 3.3 Lexical structure (LEX)

| ID | Requirement | Priority | Release | LR |
| --- | --- | --- | --- | --- |
| LEX-01 | Source files shall be UTF-8 with the `.py` extension. | M | 0.1 | 2.1.4 |
| LEX-02 | Comments, explicit and implicit line joining, and indentation shall follow the Language Reference. | M | 0.1 | 2.1 |
| LEX-03 | Integer literals shall be accepted in decimal, `0x`, `0o` and `0b` forms with `_` separators. | M | 0.1 | 2.4.5 |
| LEX-04 | Floating-point literals shall be accepted in every form the Language Reference defines. | M | 0.1 | 2.4.6 |
| LEX-05 | String literals with every quote style, the `r` prefix, and literal concatenation shall be accepted. | M | 0.1 | 2.4.1 |
| LEX-06 | Bytes literals shall be accepted. | M | 0.2 | 2.4.1 |
| LEX-07 | f-strings, with conversions and format specifications, shall be accepted. | M | 0.2 | 2.4.3 |
| LEX-08 | Imaginary literals shall be rejected. | M | 0.1 | 2.4.7 |
| LEX-09 | A source file larger than 16 MiB shall be rejected with a diagnostic before it is parsed, so input size cannot exhaust the compiler's memory (NFR-09, NFR-14). | M | 0.1 | |

### 3.4 Types and annotations (TYP)

| ID | Requirement | Priority | Release | LR |
| --- | --- | --- | --- | --- |
| TYP-01 | Every function parameter except `self` and `cls`, and every function return, shall carry a type annotation. | M | 0.1 | 8.7 |
| TYP-02 | A local variable without an annotation shall take the type of its first assignment. | M | 0.1 | 7.2 |
| TYP-03 | Assigning a value of another type to a variable shall be rejected. | M | 0.1 | 7.2 |
| TYP-04 | Type inference shall stay within one function; it shall not use the bodies of other functions. | M | 0.1 | |
| TYP-05 | An empty list, dict or set display shall need an annotation that names its element types. | M | 0.2 | 6.2.4 |
| TYP-06 | The annotations `list[T]`, `dict[K, V]`, `set[T]`, `tuple[...]` and `T \| None` shall be accepted. | M | 0.2 | 3.3.5 |
| TYP-07 | A value of type `T \| None` shall narrow to `T` after `is None`, `is not None` and `isinstance` tests, and using it unnarrowed shall be rejected. | M | 0.4 | |
| TYP-08 | A name annotated `Final` shall not be assigned again. | M | 0.4 | |
| TYP-09 | `int` shall be accepted where `float` is expected, as PEP 484 allows. | M | 0.1 | |
| TYP-10 | `typing.Any`, `typing.cast` and unannotated `object` values shall be rejected. | M | 0.1 | |
| TYP-11 | The type of an expression shall be fixed when the program is compiled, and no run-time type information shall be generated except for classes used with `isinstance` or in `except` clauses. | M | 0.1 | |
| TYP-12 | A read of a local variable shall be rejected unless the variable is assigned on every path that reaches the read, so `UnboundLocalError` cannot occur. | M | 0.1 | 4.2.2 |

### 3.5 Built-in types and values (DAT)

| ID | Requirement | Priority | Release | LR |
| --- | --- | --- | --- | --- |
| DAT-01 | `int` shall be an integer without a size limit, as in Python. A value that fits in 63 bits shall be held inline, without a heap allocation. | M | 0.1 | 3.2.4 |
| DAT-02 | `float` shall be an IEEE 754 double, and `str()` and `repr()` of a `float` shall give the same text as CPython. | M | 0.1 | 3.2.4 |
| DAT-03 | `bool` shall have the values `True` and `False`. Arithmetic operators on `bool` operands shall be rejected; `&`, `\|`, `^` and `not` shall be accepted. | M | 0.1 | 3.2.4 |
| DAT-04 | `None` shall be the only value of its type. | M | 0.1 | 3.2.1 |
| DAT-05 | `str` shall be an immutable sequence of Unicode code points. `len()` and indexing shall count code points, and indexing shall return a `str` of length 1. | M | 0.1 | 3.2.5 |
| DAT-06 | `bytes` shall be an immutable sequence of integers from 0 to 255. | M | 0.2 | 3.2.5 |
| DAT-07 | `list[T]` shall be a mutable sequence. Assigning a list to a second name shall share the one list, so a change through either name is seen through both. | M | 0.2 | 3.2.5 |
| DAT-08 | `tuple[...]` shall be an immutable sequence with a fixed number of items, each with its own type. | M | 0.2 | 3.2.5 |
| DAT-09 | `dict[K, V]` shall be a mutable mapping that iterates in insertion order. | M | 0.2 | 3.2.7 |
| DAT-10 | `set[T]` shall be a mutable set. | M | 0.2 | 3.2.6 |
| DAT-11 | Indexing a sequence with a negative index shall count from the end, and an index outside the sequence shall raise `IndexError`. | M | 0.2 | 6.3.2 |
| DAT-12 | Slicing shall return a new object with the items selected by start, stop and step, as in Python. | M | 0.2 | 6.3.3 |

### 3.6 Expressions (EXP)

| ID | Requirement | Priority | Release | LR |
| --- | --- | --- | --- | --- |
| EXP-01 | Expressions shall evaluate in the order the Language Reference defines. | M | 0.1 | 6.16 |
| EXP-02 | `+`, `-`, `*`, `/`, `//`, `%` and `**` shall give Python's results for `int` and `float`, including `/` on two `int` values returning `float` and `%` taking the sign of the divisor. | M | 0.1 | 6.7 |
| EXP-03 | Division or modulo by zero, for `int` and for `float`, shall raise `ZeroDivisionError`. | M | 0.1 | 6.7 |
| EXP-04 | `int ** int` shall be accepted only when the exponent is a non-negative integer literal, because CPython returns `float` for a negative exponent. Other exponents shall use `float` or `pow()`. | M | 0.1 | 6.5 |
| EXP-05 | Shifts and bitwise operators shall give Python's results for `int`, and a negative shift count shall raise `ValueError`. | M | 0.1 | 6.8 |
| EXP-06 | Comparisons shall be accepted between values of the same type, and between `int` and `float`. Chained comparisons shall evaluate each operand once. | M | 0.1 | 6.10 |
| EXP-07 | `in` and `not in` shall be accepted on `str`, `bytes`, `list`, `tuple`, `dict` and `set`. | M | 0.2 | 6.10.2 |
| EXP-08 | `is` and `is not` shall be accepted only when one operand is `None` or both operands are class instances. | M | 0.1 | 6.10.3 |
| EXP-09 | `and` and `or` shall short-circuit and return one of their operands, and shall be accepted only when both operands have the same type. | M | 0.1 | 6.11 |
| EXP-10 | A value shall be accepted in a truth test when its type is `bool`, `int`, `float`, `str`, `bytes`, a built-in container, `T \| None`, or a class that defines `__bool__` or `__len__`. Other values shall be rejected. | M | 0.1 | 6.11 |
| EXP-11 | Conditional expressions `x if c else y` shall be accepted when both branches have the same type. | M | 0.1 | 6.13 |
| EXP-12 | List, dict and set comprehensions shall be accepted, each with its own scope. | M | 0.2 | 6.2.4 |
| EXP-13 | Assignment expressions `:=` shall be accepted. | M | 0.5 | 6.12 |
| EXP-14 | `lambda` expressions shall be accepted where the expected type is a `Callable`. | M | 1.0 | 6.14 |
| EXP-15 | Generator expressions shall be accepted. | M | 1.0 | 6.2.8 |
| EXP-16 | Calls with positional, keyword and default arguments shall be accepted, and default values shall be evaluated once, when the function is defined. | M | 0.1 | 6.3.4 |
| EXP-17 | `*args` and `**kwargs` with annotated element types shall be accepted. | S | 1.0 | 8.7 |

### 3.7 Simple statements (SMT)

| ID | Requirement | Priority | Release | LR |
| --- | --- | --- | --- | --- |
| SMT-01 | Assignment, annotated assignment and augmented assignment shall be accepted. | M | 0.1 | 7.2 |
| SMT-02 | Unpacking assignment to several targets shall be accepted for tuples and lists. | M | 0.2 | 7.2 |
| SMT-03 | `pass`, `return`, `break` and `continue` shall be accepted. | M | 0.1 | 7.4 |
| SMT-04 | `assert` shall raise `AssertionError` when its test is false, in every build mode. | M | 0.3 | 7.3 |
| SMT-05 | `del` shall be accepted on list items, list slices and dict keys, and rejected on names and attributes. | M | 0.3 | 7.5 |
| SMT-06 | `raise` shall be accepted with an exception instance or class, with and without `from`. | M | 0.3 | 7.8 |
| SMT-07 | `import` and `from ... import` shall be accepted for TyPy modules and for the supported standard library (LIB). | M | 0.3 | 7.11 |
| SMT-08 | `global` shall be accepted for module-level variables. | M | 0.3 | 7.12 |
| SMT-09 | `nonlocal` shall be accepted in nested functions. | M | 1.0 | 7.13 |
| SMT-10 | The `type` statement shall define a type alias, with or without type parameters. | M | 0.4 | 7.14 |
| SMT-11 | `yield` shall be accepted in generator functions. | M | 1.0 | 7.7 |

### 3.8 Compound statements (CST)

| ID | Requirement | Priority | Release | LR |
| --- | --- | --- | --- | --- |
| CST-01 | `if`, `elif` and `else` shall be accepted. | M | 0.1 | 8.1 |
| CST-02 | `while` and `for`, each with an optional `else` clause, shall be accepted. | M | 0.1 | 8.2 |
| CST-03 | `for` shall iterate over `range`, `str`, `bytes`, `list`, `tuple`, `dict`, `set` and any class with `__iter__` and `__next__`. | M | 0.2 | 8.3 |
| CST-04 | `try` with `except`, `else` and `finally` clauses shall be accepted, and `finally` shall run on `return`, `break`, `continue` and exceptions. | M | 0.3 | 8.4 |
| CST-05 | `with` shall call `__enter__` and `__exit__` as the Language Reference defines, also when the block exits early. | M | 0.3 | 8.5 |
| CST-06 | `match` shall be accepted with literal, capture, wildcard, value, sequence, mapping and class patterns, guards and `\|` alternatives. | M | 0.5 | 8.6 |
| CST-07 | A `match` on an enumeration-like union of classes shall report a warning when cases are missing. | S | 0.5 | 8.6 |

### 3.9 Functions (FUN)

| ID | Requirement | Priority | Release | LR |
| --- | --- | --- | --- | --- |
| FUN-01 | Module-level function definitions shall be accepted. | M | 0.1 | 8.7 |
| FUN-02 | Recursion shall be accepted. Exhausting the stack shall end the program with `RecursionError` on standard error and exit status 1. | M | 0.1 | 8.7 |
| FUN-03 | Keyword-only and positional-only parameters shall be accepted. | S | 1.0 | 8.7 |
| FUN-04 | Nested functions and closures shall be accepted, capturing variables as Python does. | M | 1.0 | 4.2 |
| FUN-05 | Functions shall be values of type `Callable[[A, ...], R]`. | M | 1.0 | 3.2.8 |
| FUN-06 | User-defined decorators whose types can be checked shall be accepted. | C | 1.0 | 8.7 |
| FUN-07 | Generator functions shall return `Iterator[T]` or `Generator[T, None, None]`. | M | 1.0 | 8.7 |

### 3.10 Classes (CLS)

| ID | Requirement | Priority | Release | LR |
| --- | --- | --- | --- | --- |
| CLS-01 | Classes with methods and an `__init__` shall be accepted, with each instance attribute assigned in `__init__` or declared in the class body. | M | 0.2 | 8.8 |
| CLS-02 | Adding an attribute that the class does not declare shall be rejected. | M | 0.2 | 3.3.2 |
| CLS-03 | Classes derived from `Exception` and its subclasses shall be accepted. | M | 0.3 | 8.8 |
| CLS-04 | Single inheritance between user classes shall be accepted, with overriding methods type checked against the base. | M | 0.4 | 8.8 |
| CLS-05 | `@dataclass`, including `frozen`, `order` and `field(default=...)`, shall be accepted. | M | 0.4 | |
| CLS-06 | `@property`, `@staticmethod` and `@classmethod` shall be accepted. | M | 0.4 | 3.3.2 |
| CLS-07 | Operator methods such as `__add__`, `__eq__`, `__lt__`, `__len__`, `__getitem__` and `__contains__` shall be called for the matching operators. | M | 0.4 | 3.3 |
| CLS-08 | `__str__` and `__repr__` shall be used by `str()`, `repr()`, `print()` and f-strings. | M | 0.2 | 3.3.1 |
| CLS-09 | `isinstance` shall be accepted with a class or a tuple of classes. | M | 0.4 | |

### 3.11 Generics and protocols (GEN)

| ID | Requirement | Priority | Release | LR |
| --- | --- | --- | --- | --- |
| GEN-01 | Generic functions and classes shall be written with PEP 695 type parameter lists; `TypeVar` and `Generic` shall be rejected. | M | 0.4 | 8.10 |
| GEN-02 | Type parameters with a bound or with constraints shall be accepted and checked. | M | 0.4 | 8.10 |
| GEN-03 | Generics shall compile to one copy per concrete type, with no run-time type information. | M | 0.4 | |
| GEN-04 | Classes derived from `typing.Protocol` shall be satisfied by any class with matching methods, checked when the program is compiled. | M | 0.4 | |
| GEN-05 | `isinstance` with a protocol shall be rejected. | M | 0.4 | |

### 3.12 Exceptions (EXC)

| ID | Requirement | Priority | Release | LR |
| --- | --- | --- | --- | --- |
| EXC-01 | Raising and catching exceptions shall follow the Language Reference, including matching on base classes. | M | 0.3 | 4.3 |
| EXC-02 | The built-in exception classes the runtime raises shall exist with their Python names: `ArithmeticError`, `AssertionError`, `IndexError`, `KeyError`, `OverflowError`, `RecursionError`, `RuntimeError`, `StopIteration`, `TypeError`, `ValueError`, `ZeroDivisionError` and their bases. | M | 0.3 | 4.3 |
| EXC-03 | An exception that is not raised shall cost no more than a compare and branch at each call that may raise. | M | 0.3 | |
| EXC-04 | Exception chaining with `from` and the implicit context shall be kept, and shown in the uncaught exception output. | S | 0.3 | 7.8 |
| EXC-05 | An exception shall not cross a function exported to C (FFI-05); the program shall abort with the exception printed. | M | 0.5 | |

### 3.13 Modules and imports (MOD)

| ID | Requirement | Priority | Release | LR |
| --- | --- | --- | --- | --- |
| MOD-01 | A program shall be built from several `.py` files, found from the main file's directory and from paths set in `typy.toml`. | M | 0.3 | 5.3 |
| MOD-02 | Packages with `__init__.py` and relative imports shall be accepted. | M | 0.3 | 5.7 |
| MOD-03 | Each module shall be initialised once: at its first eager import, or at the first use of a name bound by a lazy import, in the order CPython 3.15 initialises it. | M | 0.3 | 5.4 |
| MOD-04 | An import whose name is computed at run time shall be rejected. | M | 0.3 | 5.1 |
| MOD-05 | A circular import shall be rejected with the cycle shown. | M | 0.3 | 5.4 |
| MOD-06 | `lazy import` and `lazy from ... import` (PEP 810) shall be accepted at module level when the program's Python floor is 3.15 or later, and shall bind the name without initialising the module. | S | 0.3 | |
| MOD-07 | A module-level `__lazy_modules__` list of string literals shall make the listed imports of that module lazy, as PEP 810 defines for Python 3.15, for every Python floor. | S | 0.3 | |
| MOD-08 | An exception raised while a lazily imported module initialises shall be raised at the first use of the name, and a later use shall try the initialisation again, as PEP 810 defines. | S | 0.3 | |
| MOD-09 | `sys.set_lazy_imports`, `sys.set_lazy_imports_filter` and `sys.lazy_modules` shall be rejected, because they change import behaviour from outside the importing module. | S | 0.3 | |

### 3.14 Built-in functions and standard library (LIB)

| ID | Requirement | Priority | Release |
| --- | --- | --- | --- |
| LIB-01 | `print` with `sep` and `end`, `len`, `range`, `abs`, `min`, `max`, `int`, `float`, `str`, `bool`, `repr`, `ord` and `chr` shall be available. | M | 0.1 |
| LIB-02 | `list`, `dict`, `set`, `tuple`, `sorted`, `reversed`, `enumerate`, `zip`, `sum`, `any`, `all` and `round` shall be available. | M | 0.2 |
| LIB-03 | The methods of `str`, `bytes`, `list`, `dict` and `set` listed in the design shall be available with Python's results. | M | 0.2 |
| LIB-04 | `sys.argv`, `sys.exit`, `sys.stdout`, `sys.stderr`, the `file` argument of `print`, `input` and `open` for text files shall be available. | M | 0.3 |
| LIB-05 | The `math` module shall be available. | M | 0.3 |
| LIB-06 | The `dataclasses` names in CLS-05 and the `typing` names in TYP and GEN shall be available. | M | 0.4 |
| LIB-07 | `os.environ`, `os.path`, `time` and `random` shall be available. | S | 1.0 |
| LIB-08 | Importing a standard library module that TyPy does not provide shall be rejected with the list of modules it provides. | M | 0.3 |
| LIB-09 | Each standard library module TyPy provides shall be TyPy's own implementation, with the names and results the Python library documentation defines. | M | 0.1 |

### 3.15 Memory management (MEM)

| ID | Requirement | Priority | Release |
| --- | --- | --- | --- |
| MEM-01 | An object shall be freed when its last reference goes, with no tracing garbage collector and no pause. | M | 0.1 |
| MEM-02 | `float`, `bool` and inline `int` values shall be held without a heap allocation in variables, fields and containers, so a `list[float]` stores 8 bytes per item. | M | 0.2 |
| MEM-03 | An object the compiler proves does not outlive its function shall be allocated without a heap call. | S | 0.4 |
| MEM-04 | Passing an argument shall cost no reference count operation, and the last use of a variable shall move its value. | M | 0.2 |
| MEM-05 | Freed memory of one object shall be reused for a new object of the same size in the same function when the compiler can prove it safe. | C | 1.0 |
| MEM-06 | Reference cycles shall be collected, without a background thread, and only objects whose class can be part of a cycle shall pay for cycle tracking (ADR-0001). | S | 1.0 |
| MEM-07 | `weakref.ref` shall be available. | S | 1.0 |
| MEM-08 | The allocator shall be replaceable per project through three functions: allocate, reallocate and free. | S | 0.5 |

### 3.16 C interoperability (FFI)

C interoperability uses the standard `ctypes` module, so a program that
calls C also runs under CPython. TyPy compiles the declarations below into
direct calls and links the library when the program is built.

| ID | Requirement | Priority | Release |
| --- | --- | --- | --- |
| FFI-01 | A module-level `ctypes.CDLL` or `ctypes.cdll.LoadLibrary` call with a string literal shall link the named library. | M | 0.5 |
| FFI-02 | A C function shall be callable when its `argtypes` and `restype` are assigned at module level, with ctypes types, before its first call. A call to a function without both shall be rejected. | M | 0.5 |
| FFI-03 | The ctypes types `c_int8` to `c_uint64`, `c_int`, `c_long`, `c_size_t`, `c_ssize_t`, `c_float`, `c_double`, `c_bool`, `c_char_p`, `c_void_p`, `POINTER(T)` and `Structure` subclasses with a literal `_fields_` list shall be accepted, with the conversions and the wrap-around that ctypes defines. | M | 0.5 |
| FFI-04 | Whether a library is linked statically or dynamically shall be set in `typy.toml`, not in the source. | M | 0.5 |
| FFI-05 | A function listed under `[export]` in `typy.toml` shall be exported with an unmangled C symbol and the C types given there, and the build shall write a matching C header. | M | 0.5 |
| FFI-06 | `ctypes.CFUNCTYPE` callbacks shall be accepted. | C | 1.0 |
| FFI-07 | A C header shall be convertible into a module of ctypes declarations. | C | 1.0 |

### 3.17 Diagnostics (DIA)

| ID | Requirement | Priority | Release |
| --- | --- | --- | --- |
| DIA-01 | Every diagnostic shall have a stable code, a `file:line:column` location with columns counted in code points, and one sentence that says what is wrong. | M | 0.1 |
| DIA-02 | A diagnostic shall show a corrected line under `help:` when the fix is certain. | S | 0.1 |
| DIA-03 | A diagnostic that rejects valid Python shall name the TyPy rule, by ID, that the program falls outside. | M | 0.1 |
| DIA-04 | One root cause shall give one error, and errors that follow from it shall be suppressed. | M | 0.1 |
| DIA-05 | `--error-format=json` shall write the same diagnostics as JSON, one object per line. | S | 0.3 |
| DIA-06 | A code shall never be reused, and a retired code shall stay reserved. | M | 0.1 |
| DIA-07 | Warnings shall not fail a build unless `--deny-warnings` is given. | M | 0.1 |

Code ranges follow the pack: `E01xx` lexical, `E02xx` declarations, `E03xx`
expressions and statements, `E04xx` types, `E05xx` exceptions and control
flow, `E06xx` memory, `E07xx` C interoperability, `E08xx` modules, `E9xxx`
internal errors, and `Wxxxx` warnings in the same ranges.

## 4. Non-functional requirements

Measurements run on the reference machine: Linux x86_64, 8 cores, warm file
cache, unless a row says otherwise.

| ID | Requirement | Release | Verified by |
| --- | --- | --- | --- |
| NFR-01 | A hello world program built with `--release-small` and linked statically with musl shall be smaller than 100 KB. | 0.1 | Size check in CI |
| NFR-02 | The runtime library, excluding libc, shall be smaller than 48 KB, and smaller than 32 KB without its large-integer and cycle-collection parts. | 0.1 | Size check in CI, for both figures |
| NFR-03 | The geometric mean run time of the benchmark suite in `--release` mode shall be within 1.5 times that of the equivalent C programs. | 1.0 | Weekly benchmark run |
| NFR-04 | A debug build of a 10,000-line project shall take under 2 seconds. | 1.0 | Benchmark with budget |
| NFR-05 | A release build of a 10,000-line project shall take under 20 seconds. | 1.0 | Benchmark with budget |
| NFR-06 | The same source, flags, target and compiler version shall produce byte-identical output. | 0.1 | Build twice and compare in CI |
| NFR-07 | Safe code shall have no use after free, double free, null dereference, out-of-bounds access or uninitialised read. | 0.1 | Run tests under AddressSanitizer; fuzzing |
| NFR-08 | A program without reference cycles shall free all its memory before exit. | 0.1 | Leak checker on every run test |
| NFR-09 | The compiler shall not crash on any input; an internal error shall be reported as `E9xxx` with a hint to report it. | 0.1 | Nightly fuzzing, 8 hours, zero crashes |
| NFR-10 | Every requirement in section 3 shall have at least one accepted and one rejected test program, and every run test shall run alike on each oracle version from its Python floor up. | 0.1 | Conformance suite in CI |
| NFR-11 | Line and branch coverage of the compiler and runtime shall be at least 95 percent. | 0.1 | Coverage gate in CI |
| NFR-12 | An object header shall take at most 8 bytes. | 0.2 | Layout test |
| NFR-13 | A `list[int]` of one million small values shall use at most 8.5 MB. | 0.2 | Memory benchmark |
| NFR-14 | The compiler shall use at most 1 GB of memory to build a 10,000-line project. | 1.0 | Benchmark with budget |
| NFR-15 | A debug build and a release build of a program shall run alike. | 0.1 | Differential tests in CI |

## 5. Release plan

| Release | Language added | Other content |
| --- | --- | --- |
| 0.1 | Functions, `int`, `float`, `bool`, `str`, `None`, `if`, `while`, `for` over `range`, operators | `build`, `run`, `check`, `--emit`; runtime; diagnostics; Linux x86_64 |
| 0.2 | `list`, `tuple`, `dict`, `set`, `bytes`, classes without inheritance, f-strings, comprehensions, slicing | Unboxed values, reference count elision |
| 0.3 | `try`, `raise`, user exceptions, `with`, `assert`, `del`, `global`, modules and packages, lazy imports | `sys`, `math`, `open`; `typy.toml` with `python-floor`; JSON diagnostics |
| 0.4 | Dataclasses, inheritance, protocols, PEP 695 generics, narrowing, `Final`, `type` aliases | Stack allocation |
| 0.5 | `match`, `:=`, C interoperability through `ctypes` | Libraries and exports; Linux and macOS AArch64; cross-compilation |
| 1.0 | Closures, `lambda`, `nonlocal`, generators, `*args`, keyword-only parameters | Cycle collection, `weakref`; Windows; performance budgets |

## 6. Out of scope

### 6.1 Excluded from TyPy

These features cannot be compiled ahead of time with CPython's meaning, so
TyPy rejects them (STD-04):

- `eval`, `exec`, `compile` and `__import__`.
- Metaclasses, class decorators that change the class, and `type()` with
  three arguments.
- Adding or replacing attributes of classes, modules or functions at run
  time, and `__getattr__`, `__getattribute__`, `__setattr__` and
  `__delattr__`.
- `globals()`, `locals()` and `vars()`.
- `typing.Any`, `typing.cast` and untyped values (TYP-10).
- Complex numbers (LEX-08).
- Running CPython extension modules or CPython packages, such as NumPy.
- Names that are not Python's: TyPy offers no fixed-width integer types, no
  `unsafe` blocks and no decorators of its own (STD-07).

### 6.2 Planned for after 1.0

| Feature | First Python version |
| --- | --- |
| `typing.Self`, exception groups and `except*` | 3.11 |
| `async` and `await` | 3.5 |
| Threads | |
| Multiple inheritance | |
| `__del__` | |
| A language server | |

## 7. Open issues

There are no open issues. A new question gets the next free ID, OI-12.

## 8. Resolved issues

| ID | Question | Answer | Date |
| --- | --- | --- | --- |
| OI-01 | Which Python versions are the baseline and the test matrix? | CPython 3.12 to 3.15 are the oracle. | 2026-11-05 |
| OI-02 | Which LLVM major version does 0.1 pin? | LLVM 23, from apt.llvm.org (C-03). | 2026-11-05 |
| OI-03 | How is `ruff_python_parser` upgraded? | Exact pin, upgraded at most once per minor release after the full conformance suite passes (C-02). | 2026-11-05 |
| OI-04 | How is a `typy` package for CPython shipped? | There is none. TyPy is an independent implementation of Python and adds no names (STD-07), so the oracle is plain CPython. | 2026-11-05 |
| OI-05 | Does an overflowing fixed-width integer raise or panic? | TyPy has no fixed-width integer types (STD-07). ctypes values wrap as ctypes defines (FFI-03). | 2026-11-05 |
| OI-06 | Does `int` have Python's unlimited size? | Yes (DAT-01). Values that fit in 63 bits are held inline; larger values move to the heap. | 2026-11-05 |
| OI-07 | Which Python version sets the scope, and which reference is the rules' text? | TyPy 1.0 covers the typeable Python 3.10 language, with PEP 695 generics. The 3.12 Language Reference is the cited text. | 2026-11-05 |
| OI-08 | How are reference cycles collected? | Only types that can form a cycle are tracked, and a synchronous trial-deletion collector runs at points set by an allocation count (ADR-0001). | 2026-11-05 |
| OI-09 | Is EXP-04 too strict for `2 ** n`? | EXP-04 stands. Before 1.0, the programs it rejects in the conformance suite and the benchmark suite are reviewed, and any change goes through a decision record. | 2026-11-05 |
| OI-10 | Does NFR-02 keep its 32 KB budget? | The core runtime keeps 32 KB. Large-integer arithmetic (DAT-01) and cycle collection (ADR-0001) add an estimated 8 to 15 KB, so the whole runtime gets 48 KB (NFR-02). | 2026-11-05 |
| OI-11 | How does TyPy support PEP 810 lazy imports, whose `lazy` keyword CPython 3.12 to 3.14 reject? | Each program has a Python floor; a program with floor 3.15 may use `lazy import`, and `__lazy_modules__` works for every floor (ADR-0008). | 2026-11-06 |

## Appendix A. Disposition of the specification pack

The pack's rules map onto this document as follows. A rule not listed here
keeps its meaning under the new ID shown.

| Pack rules | Disposition |
| --- | --- |
| REQ-F-001 to REQ-F-004 | Kept as STD-01, CLI-01 and CLI-04. Source files are `.py`, not `.ty`. |
| REQ-F-005, REQ-F-006 | Kept as the language rows of releases 0.1 and 0.2. Enums with payloads become classes in a union, matched with `match`. |
| REQ-F-007 | Kept as MEM-01 to MEM-06. |
| REQ-F-008 to REQ-F-010 | Kept as FFI-01 to FFI-05 through `ctypes`, and GEN-01 to GEN-05. |
| REQ-F-013 | Retired. Python exceptions replace `Result` and `?` (EXC). |
| REQ-F-014, REQ-F-015 | Kept as FFI-07, EXP-12, EXP-15 and FUN-07. |
| REQ-F-016 | Retired. A standard Python formatter applies. |
| REQ-F-017, REQ-F-018 | Planned for after 1.0 (language server). No package manager. |
| REQ-N-001 to REQ-N-010 | Kept as NFR-01 to NFR-09 and NFR-15. |
| LEX-001 to LEX-009 | Replaced by LEX-01 to LEX-08. Tabs, backslash joining and non-ASCII identifiers follow Python. |
| SYN-001 | Kept as TYP-01. |
| SYN-002 to SYN-004 | Retired: `let`, `var`, `struct` and lowercase `true`, `false`, `none` are not Python. |
| SYN-005, GRM-001, GRM-002 | Replaced by CST-06 and EXP-06. Chained comparisons are accepted; precedence is Python's. |
| TYP-001 to TYP-007 | Replaced by DAT, CLS and FUN-05. `T?` becomes `T \| None`. |
| GEN-001 to GEN-003, PRO-001 to PRO-003 | Kept as GEN-01 to GEN-05. `dyn` is planned for after 1.0. |
| INF-001 to INF-004 | Kept as TYP-02 to TYP-05. |
| CNV-001 | Retired. TyPy has one `int`, and `int` is accepted where `float` is expected (TYP-09). |
| CNV-002 to CNV-005 | Replaced by EXP-06 and EXP-10. Truth tests are accepted as listed. |
| SEM-001, SEM-003 | Kept as EXP-01. |
| SEM-002 | Replaced by EXP-09: `and` and `or` return an operand, as in Python. |
| SEM-004 | Replaced by EXP-16: defaults are evaluated once, as in Python. |
| SEM-010 to SEM-015 | Replaced by DAT-01 and EXP-02 to EXP-05. `int` does not overflow, and `/` on integers returns `float`. |
| SEM-020 | Replaced by EXP-03: `float` division by zero raises, as in Python. |
| SEM-030 to SEM-034 | Replaced by DAT-05, DAT-11 and DAT-12: code points, negative indexes and copying slices, as in Python. |
| ERR-001 to ERR-005 | Replaced by EXC-01 to EXC-05. |
| UNS-001 to UNS-003 | Retired. Foreign calls through `ctypes` mark the code that NFR-07 does not cover. |
| MEM-001 to MEM-006 | Replaced by MEM-01 to MEM-04 and DAT-07. Lists are shared, not copied on write. |
| MEM-010 to MEM-015 | Kept as MEM-04 and MEM-05. `own` and `mut` are retired. |
| MEM-020 to MEM-023 | Replaced by MEM-06 and MEM-07. |
| MEM-030 to MEM-032 | Replaced by CST-05: `with` calls `__enter__` and `__exit__`. |
| MEM-040 to MEM-043 | Planned for after 1.0 (threads). |
| FFI-001 to FFI-023 | Replaced by FFI-01 to FFI-07 and EXC-05, through `ctypes` and `typy.toml`. |
| ABI-001 to ABI-003 | Moved to the design. |
| ARC-001 to ARC-010 | Kept as CLI-04 and NFR-15; the rest moves to the architecture. |
| CLI-001 to CLI-003 | Kept as CLI-06, CLI-07 and CLI-11. |
| RT-001 to RT-004 | Moved to the architecture; RT-004 is planned for after 1.0. |
| LIB-001 to LIB-005 | Replaced by LIB-01 to LIB-08, using Python's module names. |
| NAM-001 to NAM-005 | Retired. PEP 8 applies, as for any Python code. |
| DIA-001 to DIA-009 | Kept as DIA-01 to DIA-07 and CLI-10. |
| TST-001 to TST-005 | Moved to the test plan. |
| INV-001 to INV-006 | Kept; INV-007 is replaced by STD-01 to STD-04. |
| AI-001 to AI-006 | Retired. They govern how the work is done, not the product. |
