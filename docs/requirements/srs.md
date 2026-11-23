# TyPy software requirements specification

| Field | Value |
| --- | --- |
| Document | TyPy software requirements specification |
| Version | 1.127 |
| Status | Approved |
| Owner | typy-lang contributors |
| Last updated | 2026-11-23 |

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
| PEP 318, 484, 526, 544, 557, 585, 591, 604, 614, 634, 695 | As published on <https://peps.python.org> |
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
| 0.6 | Adds Linux AArch64 with glibc |
| 0.7 | Adds macOS 11 or later on Apple silicon (AArch64), which builds programs for macOS |
| 1.0 | Adds Windows x86_64 |

Building needs a C linker driver for the target: `cc` for the host's own
target, and for another target the cross toolchain a Linux distribution
packages, such as Debian's `gcc-aarch64-linux-gnu` (A-01, ADR-0051). On
macOS, `cc` is the one of Xcode or its Command Line Tools. A Linux host does
not build for macOS, since linking for it needs Apple's SDK, whose licence
keeps it to Apple hardware (ADR-0055). Running the oracle needs CPython 3.12
or later.

### 2.4 Constraints

| ID | Constraint | Reason |
| --- | --- | --- |
| C-01 | The compiler is written in Rust. | Decided by the project owner on 2026-11-05; meets NFR-04 and NFR-05 without a rewrite. |
| C-02 | Python source is parsed with `ruff_python_parser`, pinned to one exact version and upgraded at most once per TyPy minor release, after the full conformance suite passes on the new version. | The grammar is Python's; the crate has no stability promise and is released about weekly (research 5.1). |
| C-03 | LLVM is used through `inkwell`, with one LLVM major version per TyPy release. TyPy uses LLVM 23: from apt.llvm.org up to 0.6, and from 0.7 a build of LLVM 23.1.6 that TyPy makes from its sources for each host and pins by its SHA-256 (ADR-0054). | Research 5.2 and 5.3; LLVM 23 is the latest major version, and Debian's LLVM 19 is four behind. A build of TyPy's own is the same bytes on every machine and in every CI run, which apt.llvm.org's dated snapshots were not (OI-28). |
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
| STD-05 | When the oracle ends with an uncaught exception, the TyPy binary shall end with the same exception type on the last line of standard error, with the same message where every oracle version words it alike and otherwise with the wording of the newest (OI-14), and exit status 1. | M | 0.3 | 4.3 |
| STD-06 | A program shall run its module-level statements in order, as `python file.py` does, with `__name__` equal to `"__main__"` in the main module. | M | 0.1 | 9.1 |
| STD-07 | TyPy shall add no module, function, type, decorator or syntax of its own. A program shall use only the Python language and the standard library modules TyPy provides (LIB). | M | 0.1 | |
| STD-08 | The first minor release after a CPython version's final release shall add it to the oracle and to the Python floors, with a requirement for each of its language changes, and the first minor release after a version's end of life shall stop accepting it as a floor (ADR-0095). | S | 0.11 | |

### 3.2 Command line and builds (CLI)

| ID | Requirement | Priority | Release |
| --- | --- | --- | --- |
| CLI-01 | `typy build FILE` shall produce a native executable for the host. | M | 0.1 |
| CLI-02 | `typy run FILE [-- ARGS]` shall build and run the program, starting it with `ARGS` as its command-line arguments. A program reads them as `sys.argv[1:]` from 0.3 (LIB-04). | M | 0.1 |
| CLI-03 | `typy check PATH` shall report every diagnostic without generating code. | M | 0.1 |
| CLI-04 | `typy build --emit=STAGE` shall write the output of one stage as deterministic text, for `ast`, `typed-ast`, `tyir`, `tyir-mem`, `llvm-ir` and `asm`, and as an object file for `obj`. | M | 0.1 |
| CLI-05 | `typy build` shall offer the modes `debug` (default), `--release` and `--release-small`: LLVM's `O0` pipeline, its `O2` pipeline, and its `O2` pipeline with every function marked `minsize`, because LLVM 23 has no `Oz` pipeline. | M | 0.1 |
| CLI-06 | The exit status of `typy` shall be 0 on success, 1 when the program has errors, 2 on wrong command usage, and 101 on an internal compiler error. | M | 0.1 |
| CLI-07 | Project settings shall be read from `typy.toml`, and a command line flag shall override the same setting in the file. | S | 0.3 |
| CLI-08 | `typy build --target TRIPLE` and `typy check --target TRIPLE` shall build and check for each target the host supports, each Linux target on a Linux host and `aarch64-apple-darwin` on a macOS host, and another triple shall be a usage error that lists the host's targets. | S | 0.6 |
| CLI-09 | `typy build --lib static` shall produce a static library and `--lib shared` a shared library. | M | 0.5 |
| CLI-10 | `typy explain CODE` shall print an explanation of the diagnostic with a wrong and a corrected example. | S | 0.3 |
| CLI-11 | `typy` shall read no environment variable that changes the output, except `TYPY_HOME`, and `NO_COLOR`, which only takes the colour from diagnostics on a terminal (CLI-26). | M | 0.1 |
| CLI-12 | The setting `python-floor` in `typy.toml` shall set the program's Python floor to one of `3.12`, `3.13`, `3.14` or `3.15`, and any other value shall be rejected. | S | 0.3 |
| CLI-13 | A library build shall carry the release of the version that its module's `__version__`, or else `version` in `typy.toml`, gives in its file names, as Linux names libraries: `libNAME.so.X.Y` with the soname `libNAME.so.X`, and `libNAME.X.Y.a`, each with links of the names a linker finds. | S | 0.5 |
| CLI-14 | The C linker driver, `ar` and `objcopy` that build for a target shall be the host's own tools for the host's target and the tools named after the target's GNU triple for another, such as `aarch64-linux-gnu-gcc`, unless `typy.toml` names others for that target. | S | 0.6 |
| CLI-15 | `typy` shall run on macOS 11 or later on Apple silicon and build executables and libraries for `aarch64-apple-darwin` that run on macOS 11 or later. | M | 0.7 |
| CLI-16 | A library built for macOS shall be named as macOS names libraries: `libNAME.X.dylib`, whose install name is that file name and whose compatibility and current versions are its release, with a link `libNAME.dylib`, and `libNAME.a`. | S | 0.7 |
| CLI-17 | A development build shall compile each module of the standard library TyPy provides once for each release, target and mode, keep it in the user's cache directory, and link it from there in later builds (ADR-0016). | C | 1.0 |
| CLI-18 | `typy` shall run on Windows on x86-64 and build executables and libraries for `x86_64-pc-windows-msvc` that run on Windows 10 or later. | S | 1.0 |
| CLI-19 | `typy` shall build programs for WebAssembly with WASI 0.2, `wasm32-wasip2`, on each host it runs on. | C | 1.0 |
| CLI-20 | `typy build` and `typy run` shall take `-O0`, `-O1`, `-O2`, `-O3`, `-Os` and `-Oz`, each selecting LLVM's pipeline of that level, with `--release` standing for `-O2` and `--release-small` for `-Oz` (ADR-0104). | S | 0.12 |
| CLI-21 | `--target-cpu NAME` shall build for the processor NAME of the target, `native` for the build host's, and a build without it shall target the baseline processor of its triple (NFR-06). | S | 0.12 |
| CLI-22 | `--timings` shall write to standard error the time each stage of the command took. | S | 0.12 |
| CLI-23 | `-v` shall write to standard error each command `typy` runs, such as the C linker driver with its arguments, before it runs it. | S | 0.12 |
| CLI-24 | `--save-temps` shall keep a build's object files and other intermediate files beside its output. | S | 0.12 |
| CLI-25 | `typy print WHAT` shall write what `typy` would use: `targets`, `target-cpus` of a target, `host`, `paths` of the runtime archives and the cache, `settings` of a project and its command line, and `version`. | S | 0.12 |
| CLI-26 | `--color auto`, `always` or `never` shall decide whether diagnostics written as text are coloured, `auto` colouring them only on a terminal and when `NO_COLOR` is not set. | S | 0.12 |
| CLI-27 | `-W CODE`, `-A CODE` and `-D CODE` shall make the warning CODE a warning, allowed or an error, `-D warnings` standing for `--deny-warnings`, and the last of them for a code shall decide. | S | 0.12 |
| CLI-28 | `typy run -c CODE` shall build and run CODE as the main module, and `typy run -m MODULE` the module MODULE of the project, as CPython's `-c` and `-m` run them. | S | 0.12 |
| CLI-29 | `--emit` shall take several stages separated by commas, each written beside the output, and the stages `llvm-ir-unoptimised`, the IR before the optimisation pipeline, and `llvm-bc`. | S | 0.12 |
| CLI-30 | `-C KEY=VALUE` shall set a code generation option, `-C help` shall list them, and `opt-level`, `target-cpu` and `target-feature` shall be among them; a flag and its key shall be one setting, and two values for it shall be wrong usage (ADR-0106). | S | 0.12 |
| CLI-31 | `-C inline-runtime=yes` shall link the runtime into the program as LLVM bitcode, so the optimiser inlines it, and shall be off by default (ADR-0104). | C | 0.13 |
| CLI-32 | `typy NAME ARGS`, for a NAME that is not a command of `typy`, shall run the program `typy-NAME` found on `PATH` with ARGS, `TYPY` set to the path of the running `typy`, and report an unknown command with exit status 2 when there is none (ADR-0105). | S | 0.12 |
| CLI-33 | Every option that changes a build's output shall be part of the key of `typy run`'s cache. | M | 0.12 |
| CLI-34 | `-g` shall write DWARF debug information whose lines are the program's Python lines, and `-C debuginfo=0`, `1` or `2` its level. | S | 0.13 |
| CLI-35 | `--sanitize address` shall build the program with the AddressSanitizer runtime, which reports invalid memory use and leaks. | C | 0.13 |
| CLI-36 | `-C remark=PASS` shall report what the optimisation pass PASS did and did not do, at the program's Python lines. | C | 0.14 |
| CLI-37 | `-C profile-generate=DIR` and `-C profile-use=FILE` shall build with profile-guided optimisation. | C | 1.0 |

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
| LEX-10 | A `\u` or `\U` escape of a surrogate code point, U+D800 to U+DFFF, in a `str` literal shall give that code point, as CPython keeps it. | S | 0.5 | 2.4.1.1 |
| LEX-11 | Brackets nested more than 200 deep and blocks nested more than 99 deep shall be rejected with CPython's messages, and expressions and statements nested more than 25000 deep shall be rejected, so that no program nests past what the compiler's stack holds. | M | 0.3 | 2.1.8 |
| LEX-12 | A template string (PEP 750) shall evaluate to a `string.templatelib.Template` whose `strings`, `interpolations` and `values`, iteration and `+` are CPython's, under a Python floor of 3.14 or later. | S | 0.13 | 2.4.3 |

### 3.4 Types and annotations (TYP)

| ID | Requirement | Priority | Release | LR |
| --- | --- | --- | --- | --- |
| TYP-01 | Every function parameter except `self` and `cls`, and every function return that TYP-26 does not infer, shall carry a type annotation. | M | 0.1 | 8.7 |
| TYP-02 | A local variable without an annotation shall take the type of its first assignment. | M | 0.1 | 7.2 |
| TYP-03 | Assigning a value of another type to a variable shall be rejected. | M | 0.1 | 7.2 |
| TYP-04 | Type inference shall stay within one function; it shall not use the bodies of other functions. | M | 0.1 | |
| TYP-05 | An empty list, dict or set display shall need an annotation that names its element types. | M | 0.2 | 6.2.4 |
| TYP-06 | The annotations `list[T]`, `dict[K, V]`, `set[T]`, `tuple[...]` and `T \| None` shall be accepted. | M | 0.2 | 3.3.5 |
| TYP-07 | A value of type `T \| None`, or of a union of classes, shall narrow after `isinstance` tests. | M | 0.4 | |
| TYP-08 | A name annotated `Final` shall not be assigned again. | M | 0.4 | |
| TYP-09 | An `int` value shall be rejected where a `float` is declared, because CPython keeps it an `int`: `x: float = 1` prints `1`. Arithmetic that mixes `int` and `float` gives a `float`, as in Python (EXP-02). | M | 0.1 | |
| TYP-10 | A value whose type the program gives neither by an annotation nor by inference shall be rejected. | M | 0.1 | |
| TYP-11 | The type of an expression shall be fixed when the program is compiled, and a value shall carry no type information of its own beyond the type object a heap object refers to (ADR-0012). | M | 0.1 | |
| TYP-12 | A read of a local variable shall be rejected unless the variable is assigned on every path that reaches the read, so `UnboundLocalError` cannot occur. | M | 0.1 | 4.2.2 |
| TYP-13 | A local variable or parameter of type `T \| None` shall narrow to `T` where an `is None` or `is not None` test, or a truth test that holds, shows it is not `None`, and using it as `T` anywhere else shall be rejected. | M | 0.2 | |
| TYP-14 | Annotations that join two or more types other than `None` with `\|`, such as `int \| str`, shall be accepted, with narrowing by `isinstance` (TYP-07). | M | 0.4 | 3.3.5 |
| TYP-15 | The annotations `Iterator[T]` and `Iterable[T]`, imported from `collections.abc` or `typing`, shall be accepted for iterators and for the built-in iterables of items of type `T` (ADR-0014, ADR-0015). | S | 0.3 | |
| TYP-16 | One value assigned to several targets, as in `a = b[0] = value`, shall be converted to each target's type, such as the union an item holds. | S | 0.5 | 7.2 |
| TYP-17 | The names `Iterable` and `Iterator` of `typing` and `collections.abc`, and `List`, `Dict`, `Set`, `Tuple`, `Type`, `Optional[...]` and `Union[...]` of `typing`, shall be accepted as the classes of `isinstance` and of a class pattern, with CPython's answers. | S | 0.5 | |
| TYP-18 | The names of `typing` and `collections.abc`, alone or given type arguments, a built-in class given type arguments, and a union of types written with `\|`, shall be values of their own: stored, passed, compared and written as CPython writes them, where every oracle from the program's floor writes them alike; so shall a type alias that a function or a class defines, an alias given its type arguments, and an alias's `__value__` and `__type_params__` (ADR-0087). | C | 0.11 | |
| TYP-19 | An operation on a union whose members the runtime cannot tell apart by class or kind, such as two iterators, two file kinds or an iterable, shall be accepted. | C | 1.0 | |
| TYP-20 | A class whose `__next__` returns `T` and whose `__iter__` returns the instance shall be an `Iterator[T]`, and a class whose `__iter__` returns an `Iterator[T]` an `Iterable[T]`, as `typing` reads the protocols, so its instances are held and passed where those are expected (ADR-0065). | S | 0.9 | |
| TYP-21 | `Generator` of `typing` and `collections.abc` shall be accepted as the class of `isinstance` and of a class pattern, and a value of type `Iterable[T]` shall be testable against `Iterator` and `Generator`, with CPython's answers (ADR-0065). | S | 0.9 | |
| TYP-22 | A typing value shall have the annotations `types.GenericAlias`, `types.UnionType` and `typing.TypeAliasType` where every oracle from the program's floor gives it that class, and shall be callable, testable with `isinstance` when held in a variable, and read through `type()`, `__origin__`, `__args__`, `__name__`, `typing.get_origin` and `typing.get_args` as CPython reads them; so shall the attributes of a type parameter, and an alias a function defines given its type arguments. | C | 1.0 | |
| TYP-23 | A parameter, variable, attribute, collection item or result annotated `object` shall hold a value of any type, support `==`, `!=`, `is`, `hash`, `str`, `repr`, truth, `type()` and `isinstance`, and narrow by `isinstance`, a class pattern and `type(x) is C`, as Java narrows `Object` (ADR-0090). | S | 0.11 | 3.1 |
| TYP-24 | `typing.cast(T, value)` shall give `value` as a `T` when it is one, and raise `TypeError` when it is not (ADR-0091). | S | 0.11 | |
| TYP-25 | A value of type `typing.Any` shall support every operation, bound when the program runs, and raise the `TypeError` or `AttributeError` CPython raises for an operation it does not support (ADR-0094). | S | 0.13 | |
| TYP-26 | A function without a result annotation, other than a generator function, one that calls itself, a method that overrides or is overridden, and a function a library exports, shall return the type its `return` statements give (ADR-0092). | S | 0.12 | 8.7 |
| TYP-27 | An unannotated local variable assigned an empty list, dict or set display shall take its item types from the first statement of the same function that adds to it (ADR-0092). | S | 0.12 | 7.2 |
| TYP-28 | `typing.Literal` of `int`, `str`, `bytes`, `bool` and `None` constants shall be accepted, a constant passing for its literal type and `==`, `in` and `match` narrowing to it. | S | 0.12 | |
| TYP-29 | `typing.overload` shall be accepted on functions and methods, each call checked against the signature it matches and the body against every signature. | S | 0.12 | |
| TYP-30 | `typing.Self` shall be accepted in a method's parameters and result as the class the method is called through. | S | 0.12 | |
| TYP-31 | A function annotated to return `typing.TypeGuard[T]` or `typing.TypeIs[T]` shall narrow its argument where a call of it is tested, as CPython's type system reads them. | S | 0.12 | |
| TYP-32 | A class derived from `typing.TypedDict` shall be accepted, its instances being dicts whose keys it names and whose values it types, with `total`, `Required`, `NotRequired` and `ReadOnly`. | S | 0.13 | |

### 3.5 Built-in types and values (DAT)

| ID | Requirement | Priority | Release | LR |
| --- | --- | --- | --- | --- |
| DAT-01 | `int` shall be an integer without a size limit, as in Python. A value that fits in 63 bits shall be held inline, without a heap allocation. | M | 0.1 | 3.2.4 |
| DAT-02 | `float` shall be an IEEE 754 double, and `str()` and `repr()` of a `float` shall give the same text as CPython. | M | 0.1 | 3.2.4 |
| DAT-03 | `bool` shall have the values `True` and `False`, and `&`, `\|`, `^` and `not` shall be accepted on them. | M | 0.1 | 3.2.4 |
| DAT-04 | `None` shall be the only value of its type. | M | 0.1 | 3.2.1 |
| DAT-05 | `str` shall be an immutable sequence of Unicode code points. `len()` and indexing shall count code points, and indexing shall return a `str` of length 1. | M | 0.1 | 3.2.5 |
| DAT-06 | `bytes` shall be an immutable sequence of integers from 0 to 255. | M | 0.2 | 3.2.5 |
| DAT-07 | `list[T]` shall be a mutable sequence. Assigning a list to a second name shall share the one list, so a change through either name is seen through both. | M | 0.2 | 3.2.5 |
| DAT-08 | `tuple[...]` shall be an immutable sequence with a fixed number of items, each with its own type. | M | 0.2 | 3.2.5 |
| DAT-09 | `dict[K, V]` shall be a mutable mapping that iterates in insertion order. | M | 0.2 | 3.2.7 |
| DAT-10 | `set[T]` shall be a mutable set. | M | 0.2 | 3.2.6 |
| DAT-11 | Indexing a sequence with a negative index shall count from the end, and an index outside the sequence shall raise `IndexError`. | M | 0.2 | 6.3.2 |
| DAT-12 | Slicing shall return a new object with the items selected by start, stop and step, each an `int` or `None`, as in Python. | M | 0.2 | 6.3.3 |
| DAT-13 | Assigning to a slice of a list shall replace the selected items with the items of a list, as in Python. | S | 0.3 | 7.2 |
| DAT-14 | `tuple[T, ...]` shall be an immutable sequence of any number of items of type `T`, and a tuple of a fixed number of items of type `T` shall be accepted where one is expected. | M | 0.2 | 3.2.5 |
| DAT-15 | The methods of `set` that combine a set with others, `union`, `intersection`, `difference`, `symmetric_difference`, `update`, `intersection_update`, `difference_update`, `symmetric_difference_update`, `issubset`, `issuperset` and `isdisjoint`, shall accept any iterable of the set's item type, and as many arguments as CPython accepts, with CPython's results and order. | M | 0.2 | 3.2.6 |
| DAT-16 | `reversed` of a dict and of its views, `dict` with keyword arguments, `dict.fromkeys`, and the comparisons and set operations of `dict.keys()` and `dict.items()` shall be available with Python's results. | S | 0.3 | 3.2.7 |
| DAT-17 | `range` shall be a value as well as a loop source: a sequence of `int` with `len`, indexing, slicing, `in`, `==` and `repr`, as in Python. | S | 0.3 | 3.2.5 |
| DAT-18 | `frozendict` (PEP 814) shall be an immutable, hashable mapping with CPython's methods, `\|` and text, under a Python floor of 3.15 or later. | S | 0.12 | 3.2.5 |
| DAT-19 | Arithmetic on `bool` operands, and `abs`, `round` and `sum` of them, shall give CPython's results, a `bool` taken as the `int` 0 or 1, such as `True + 1 == 2` and `-True == -1`; `~` of a `bool`, which CPython deprecates since 3.12, shall be rejected (ADR-0098). | S | 0.11 | 3.2.4 |

### 3.6 Expressions (EXP)

| ID | Requirement | Priority | Release | LR |
| --- | --- | --- | --- | --- |
| EXP-01 | Expressions shall evaluate in the order the Language Reference defines. | M | 0.1 | 6.16 |
| EXP-02 | `+`, `-`, `*`, `/`, `//`, `%` and `**` shall give Python's results for `int` and `float`, including `/` on two `int` values returning `float` and `%` taking the sign of the divisor. | M | 0.1 | 6.7 |
| EXP-03 | Division or modulo by zero, for `int` and for `float`, shall raise `ZeroDivisionError`. | M | 0.1 | 6.7 |
| EXP-04 | `x ** y` shall be accepted only when CPython's result has one type that TyPy represents: `int ** int` when the exponent is a non-negative integer literal, because CPython returns `float` for a negative exponent; and a `float` exponent only when it is a literal with an integral value or the base is a non-negative literal, because CPython returns `complex` for a negative base and a fractional exponent. | M | 0.1 | 6.5 |
| EXP-05 | Shifts and bitwise operators shall give Python's results for `int`, and a negative shift count shall raise `ValueError`. | M | 0.1 | 6.8 |
| EXP-06 | Comparisons shall be accepted between values of the same type, and between `int` and `float`. Chained comparisons shall evaluate each operand once. | M | 0.1 | 6.10 |
| EXP-07 | `in` and `not in` shall be accepted on `str`, `bytes`, `list`, `tuple`, `dict` and `set`. | M | 0.2 | 6.10.2 |
| EXP-08 | `is` and `is not` shall be accepted only when one operand is `None`, both operands are `bool` or `bool \| None`, or both operands are class instances. | M | 0.1 | 6.10.3 |
| EXP-09 | `and` and `or` shall short-circuit and return one of their operands, and shall be accepted only when both operands have the same type. | M | 0.1 | 6.11 |
| EXP-10 | A value shall be accepted in a truth test when its type is `bool`, `int`, `float`, `str`, `bytes`, a built-in container, `T \| None`, or a class that defines `__bool__` or `__len__`. Other values shall be rejected. | M | 0.1 | 6.11 |
| EXP-11 | Conditional expressions `x if c else y` shall be accepted when both branches have the same type. | M | 0.1 | 6.13 |
| EXP-12 | List, dict and set comprehensions shall be accepted, each with its own scope. | M | 0.2 | 6.2.4 |
| EXP-13 | Assignment expressions `:=` shall be accepted. | M | 0.5 | 6.12 |
| EXP-14 | `lambda` expressions shall be accepted where the expected type is a `Callable`. | M | 0.8 | 6.14 |
| EXP-15 | Generator expressions shall be accepted, evaluating the first iterable where the expression runs and the rest as the generator runs, as CPython does. | M | 0.9 | 6.2.8 |
| EXP-16 | Calls with positional, keyword and default arguments shall be accepted, and default values shall be evaluated once, when the function is defined. | M | 0.1 | 6.3.4 |
| EXP-17 | `*args` and `**kwargs` parameters with annotated element types shall be accepted, and so shall unpacking with `*` and `**` in calls of the program's functions, methods and classes and with `*` in calls of `print`, binding as CPython binds them (ADR-0073), in list, tuple, set and dict displays, and in assignment targets. | S | 0.9 | 8.7 |
| EXP-18 | An assignment expression inside a generator expression shall bind the variable of the function around it, as CPython binds it. | C | 1.0 | 6.12 |
| EXP-19 | `*` of a tuple display or of a variable that holds a fixed tuple, and `**` of a dict display whose keys are `str` literals, shall be accepted in a call of a built-in function or method, of a function value, of a method through a protocol value and of an exception class, binding as CPython binds them (ADR-0073, ADR-0083). | C | 0.10 | 6.3.4 |
| EXP-20 | A default value other than a constant, such as `-1`, a tuple or a call, shall be accepted and evaluated once, when the function is defined. | C | 1.0 | 6.3.4 |
| EXP-21 | `**` shall be accepted in a call of a function value, binding the keywords by the names of the function the value holds, as CPython binds them. | C | 1.0 | 6.3.4 |
| EXP-22 | `*` of any iterable shall be accepted in a call of a built-in function or method, of a function of a built-in module, of a function value, of a method through a protocol value and of an exception class, and `**` of any `dict[str, V]` in a call of `print`, of a method through a protocol value and of an exception class, binding when the program runs as CPython binds them (ADR-0083, ADR-0099). | C | 0.11 | 6.3.4 |
| EXP-23 | A list, set or generator comprehension whose element is `*iterable`, and a dict comprehension whose element is `**mapping` (PEP 798), shall unpack each into the result, under a Python floor of 3.15 or later. | S | 0.12 | 6.2.4 |
| EXP-24 | `**` of any `dict[str, V]` shall be accepted in a call of a built-in function or method other than `print` and of a function of a built-in module, each key binding the keyword it names when the program runs, with CPython's error for a value its parameter does not take (ADR-0099). | C | 1.0 | 6.3.4 |

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
| SMT-09 | `nonlocal` shall be accepted in nested functions. | M | 0.8 | 7.13 |
| SMT-10 | The `type` statement shall define a type alias, with or without type parameters. | M | 0.4 | 7.14 |
| SMT-11 | `yield` and `yield from` shall be accepted in generator functions as statements: `yield value`, `yield from iterable`, and an assignment of either, such as `received = yield value`, whose `yield` shall give `S \| None` of a `Generator[Y, S, R]` (ADR-0063, ADR-0067, ADR-0072). | M | 0.9 | 7.7 |
| SMT-12 | A type alias shall be usable from another module, readable as a value as CPython's `TypeAliasType`, definable inside a function, a class or a block, and given type parameters with a bound or constraints. | S | 0.5 | 7.14 |

### 3.8 Compound statements (CST)

| ID | Requirement | Priority | Release | LR |
| --- | --- | --- | --- | --- |
| CST-01 | `if`, `elif` and `else` shall be accepted. | M | 0.1 | 8.1 |
| CST-02 | `while` and `for`, each with an optional `else` clause, shall be accepted. | M | 0.1 | 8.2 |
| CST-03 | `for` shall iterate over `range`, `str`, `bytes`, `list`, `tuple`, `dict` and `set`. | M | 0.2 | 8.3 |
| CST-04 | `try` with `except`, `else` and `finally` clauses shall be accepted, and `finally` shall run on `return`, `break`, `continue` and exceptions. | M | 0.3 | 8.4 |
| CST-05 | `with` shall call `__enter__` and `__exit__` as the Language Reference defines, also when the block exits early. | M | 0.3 | 8.5 |
| CST-06 | `match` shall be accepted with literal, capture, wildcard, value, sequence, mapping and class patterns, guards and `\|` alternatives. | M | 0.5 | 8.6 |
| CST-07 | A `match` whose cases without a guard leave a member of the subject's union, optional or `bool` type unmatched shall report a warning that names the members (ADR-0033). | S | 0.5 | 8.6 |
| CST-08 | `for` shall iterate over any class with `__iter__` and `__next__`, and end when `__next__` raises `StopIteration` (OI-17). | M | 0.3 | 8.3 |
| CST-09 | `iter` and `next`, with and without a default, shall be available for the built-in iterables. | S | 0.3 | 8.3 |
| CST-10 | The parameters of `__exit__` after `self` may be left without annotations, and shall then have the types `type[BaseException] \| None`, `BaseException \| None` and `TracebackType \| None`; the traceback shall be one object of type `traceback`, written as CPython writes one, which holds no frames. | S | 0.3 | |
| CST-11 | A class with `__getitem__` and no `__iter__` shall be iterable through `__getitem__` with the positions 0, 1, 2 and so on until it raises `IndexError`, in `for`, `in` and every builtin that iterates, as CPython's sequence protocol does. | C | 1.0 | |

### 3.9 Functions (FUN)

| ID | Requirement | Priority | Release | LR |
| --- | --- | --- | --- | --- |
| FUN-01 | Module-level function definitions shall be accepted. | M | 0.1 | 8.7 |
| FUN-02 | Recursion shall be accepted. Exhausting the stack shall end the program with `RecursionError` on standard error and exit status 1. | M | 0.1 | 8.7 |
| FUN-03 | Keyword-only and positional-only parameters shall be accepted. | S | 0.8 | 8.7 |
| FUN-04 | Nested functions and closures shall be accepted, capturing variables as Python does. | M | 0.8 | 4.2 |
| FUN-05 | A function, nested function, `lambda` or method the program defines shall be a value of type `Callable[[A, ...], R]`, stored, passed, returned, called and compared as CPython's are, and the `key` argument of `sorted`, `list.sort`, `min` and `max` shall take one; the methods of FUN-09 are excepted. | M | 0.8 | 3.2.8 |
| FUN-06 | User-defined decorators whose types can be checked shall be accepted on module-level and nested functions, the name bound to what the decorators return (ADR-0082). | C | 0.10 | 8.7 |
| FUN-07 | A generator function shall be annotated to return `Generator[Y, S, R]`, `Iterator[Y]` or `Iterable[Y]` of `typing` or `collections.abc`, and a call of it shall give a generator object of that type; only `Generator` shall offer `send`, `throw` and `close`, and a result of type `R`, as `typing` reads them; a generator suspended inside an `except` clause shall keep the exception it handles from its caller, as CPython does (ADR-0063, ADR-0068). | M | 0.9 | 8.7 |
| FUN-08 | A built-in function, a function of a standard library module TyPy provides other than `ctypes` (FFI-10), or a method of a built-in type shall be a value where a `Callable` is expected, such as `sorted(words, key=len)`, with CPython's identity and text (ADR-0076, ADR-0077). | S | 0.10 | 3.2.8 |
| FUN-09 | A class method read as a value, a static method read through an instance whose subclasses define it again, a method read through a protocol value, and a method read through an instance whose subclass overrides it with parameters or a result held otherwise, shall be values as CPython's are (ADR-0079). | S | 0.10 | 3.2.8 |
| FUN-10 | A nested function, `lambda` or generator expression inside a generic function or a function with a protocol parameter shall be accepted, with a copy for each set of types (ADR-0081). | C | 0.10 | 8.10 |
| FUN-11 | A generator closed, or freed while it is suspended, shall raise `GeneratorExit` at its `yield` and run its `except` and `finally` clauses when CPython does; one that yields again then shall raise `RuntimeError`, and one never started shall run nothing (ADR-0069, ADR-0070, ADR-0071). | M | 0.9 | 8.7 |
| FUN-12 | A `yield` or `yield from` inside a larger expression, such as `print((yield value))`, shall be accepted. | C | 1.0 | 6.2.9 |
| FUN-13 | A function of a module with `*args` or `**kwargs` parameters, or with parameters that have defaults, shall be a value where a `Callable` its parameters fit is expected, taking the arguments the `Callable` passes (ADR-0080). | C | 0.10 | 3.2.8 |
| FUN-14 | A built-in type whose call TyPy types, such as `int`, `str` or `list`, shall be a value where a `Callable` is expected, such as `map(int, words)` or `sorted(numbers, key=str)` (ADR-0077). | S | 0.10 | 3.2.8 |
| FUN-15 | User-defined decorators shall be accepted on methods, alone and together with `@staticmethod`, `@classmethod` and `@property`. | C | 0.14 | 8.7 |
| FUN-16 | A class of the program or an exception class shall be a value where a `Callable` is expected, such as `make: Callable[[int], Box] = Box`, its `__init__` taking the arguments the `Callable` passes. | C | 0.14 | 3.2.8 |
| FUN-17 | `property`, `staticmethod` and `classmethod`, and the objects they make, such as a property read through its class, shall be values as CPython's are. | C | 1.0 | 3.2.8 |
| FUN-18 | A nested function with type parameters of its own shall be accepted, with a copy for each set of types its calls give (ADR-0081). | C | 0.11 | 8.10 |
| FUN-19 | A nested function, a method or a `lambda` with `*args` or `**kwargs` parameters, or with parameters that have defaults, shall be a value where a `Callable` its parameters fit is expected, taking the arguments the `Callable` passes (ADR-0080). | C | 0.11 | 3.2.8 |

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
| CLS-10 | Every type, built in or a class, shall have a type object that names it and its base, and the chain of bases of every type shall end at `object`. | M | 0.2 | |
| CLS-11 | Class variables, given a value in the class body, shall be accepted. | S | 0.4 | |
| CLS-12 | `type(x)` shall give the type object of the value `x`, and a class name or the name of a built-in type shall be its type object as a value; a type object shall print as `<class 'name'>`, give its name as `__name__`, and compare with `is`, `is not`, `==` and `!=`. | M | 0.2 | |
| CLS-13 | A class derived from an exception class shall be accepted with new methods and attributes and with its own `__init__`, `__str__` and `__repr__`; redefining another method of a base shall be rejected until CLS-04. | M | 0.3 | 8.8 |
| CLS-15 | `field(default_factory=f)` in a dataclass shall call `f`, a class or a function that takes no arguments, for each instance whose `__init__` is given no value for the field. | S | 0.5 | |
| CLS-16 | A dataclass that is frozen with `eq=True`, or given `unsafe_hash=True`, shall hash as the tuple of its fields, and one with `eq=True` that is neither shall be unhashable, as CPython's `dataclasses` writes `__hash__`. | S | 0.5 | |
| CLS-17 | `@dataclass` shall accept the options `init`, `repr`, `unsafe_hash` and `slots`. | S | 0.5 | |
| CLS-18 | A value of type `type[C]` shall hold `C` or a class derived from it, and `isinstance` and a class pattern shall accept a class held in a variable or computed (ADR-0084). | S | 0.10 | |
| CLS-19 | `issubclass` shall be accepted with a class or a tuple of classes (ADR-0084). | S | 0.10 | |
| CLS-20 | An augmented assignment to an instance shall call the class's in-place method, such as `__iadd__`, and the binary method when the class has none, as CPython does. | S | 0.5 | |
| CLS-21 | An operator or comparison method that returns `NotImplemented` shall give way to the reflected method, and then to CPython's default or its `TypeError`. | S | 0.5 | |
| CLS-22 | Comparison and hash methods that may raise, a chain such as `a < b < c` of instances, and an operator whose method and reflected method return different types shall be accepted. | S | 0.5 | |
| CLS-23 | `in` on an instance whose class has no `__contains__` shall iterate it, and an instance shall be sliced through its `__getitem__`, as CPython does. | S | 0.5 | |
| CLS-24 | The truth of an instance shall come from the `__bool__` or `__len__` of its own class, also where its static type is a base or a union that tests truth otherwise, and in `any` and `all`. | S | 0.5 | |
| CLS-25 | A class method shall be callable through an instance and through a subclass defined in another module, a class variable shall be assignable through a subclass, and a static method shall be callable through any expression that gives its class or an instance. | S | 0.5 | |
| CLS-26 | `super(Class, instance)` shall be accepted with any class of the instance's chain of bases. | S | 0.5 | |
| CLS-27 | A class derived from `typing.NamedTuple` shall be accepted, its fields read by name and by position, and its values compared, hashed and written as CPython's are (ADR-0088). | S | 0.10 | |
| CLS-28 | `@dataclass` shall accept the options `kw_only` and `weakref_slot` and options given other than as `True` or `False`, `field` its options `init`, `repr`, `hash`, `compare`, `kw_only` and `metadata`, and a generic dataclass a field with a `default_factory`. | C | 0.14 | |
| CLS-29 | `super(Class, instance)` shall be accepted with an instance other than the method's own, and outside a method. | C | 0.14 | |
| CLS-30 | A class method of a generic class shall be callable through an instance. | C | 0.14 | |
| CLS-31 | A class variable shall be assignable through a variable that holds a class derived from the class that gives it, such as `cls` in a class method, giving that class one of its own. | C | 0.14 | |
| CLS-32 | An augmented assignment shall call the in-place method that only a class derived from its target's class defines, when the target holds an instance of that class. | C | 0.14 | |
| CLS-33 | An operator or comparison method that may return both `None` and `NotImplemented` shall be accepted. | C | 0.14 | |
| CLS-34 | An override whose parameter is held otherwise than its base method's, such as `int \| None` for `int`, shall be accepted with a default of its own for that parameter. | C | 0.14 | |
| CLS-35 | A static method that overrides a method, or a method that overrides a static method, shall be accepted with a parameter or result held otherwise than the one it overrides. | C | 0.14 | |
| CLS-36 | `super()` shall be usable as a value: held in a variable, passed and returned. | C | 0.14 | |
| CLS-37 | A class derived from an abstract collection of `collections.abc`, such as `Sequence`, shall be accepted, with the mixin methods it inherits. | C | 0.14 | |
| CLS-38 | A class decorator other than `@dataclass` shall be accepted when its type shows that it returns the class it is given, such as `def register[T](cls: T) -> T`. | C | 1.0 | |
| CLS-39 | A value of type `type[C]` that may hold a class derived from `C` shall be callable, and its class methods and class variables reachable through it, as CPython reaches those of the class it holds. | C | 0.14 | |
| CLS-40 | A class pattern with a class held in a variable shall accept positional sub-patterns, read by the `__match_args__` of the class it holds. | C | 0.14 | |
| CLS-41 | A named tuple made by a call, such as `NamedTuple("P", [("x", int)])`, a subclass of a named tuple with `__new__`, `__init__` or attributes of its own, and a named tuple whose class defines `__len__`, `__getitem__`, `__iter__` or `__contains__` shall be accepted. | C | 1.0 | |
| CLS-42 | A named tuple shall be accepted where a plain tuple type or an abstract collection is expected, keeping its class. | C | 1.0 | |
| CLS-43 | A named tuple shall have `_make` and `_field_defaults`, and its `_replace` shall be a value, be called through its class and take `**` arguments, as CPython's are and do. | C | 0.14 | |
| CLS-44 | A class derived from `enum.Enum`, `IntEnum`, `StrEnum`, `Flag` or `IntFlag` shall be accepted, with its members, `name`, `value`, `auto()`, iteration, `len`, lookup by value and by name, comparison, hashing and the `Flag` operators as CPython gives them (ADR-0093). | S | 0.12 | |
| CLS-45 | A class derived from `abc.ABC` shall accept `@abstractmethod`, and making an instance of a class that leaves an abstract method without an override shall be rejected where the class is named and raise CPython's `TypeError` otherwise (ADR-0093). | S | 0.12 | |
| CLS-46 | A class method called through `super()` shall run with the class of the instance as `cls` also when a class derived from the class of the enclosing method defines the class method again. | C | 0.14 | |
| CLS-14 | `super().__init__(...)` shall be accepted in the `__init__` of an exception class and shall set the exception's arguments; any other use of `super()` shall be rejected until 0.4. | M | 0.3 | |

### 3.11 Generics and protocols (GEN)

| ID | Requirement | Priority | Release | LR |
| --- | --- | --- | --- | --- |
| GEN-01 | Generic functions and classes shall be written with PEP 695 type parameter lists; `TypeVar` and `Generic` shall be rejected. | M | 0.4 | 8.10 |
| GEN-02 | Type parameters with a bound or with constraints shall be accepted and checked. | M | 0.4 | 8.10 |
| GEN-03 | Generics shall compile to one copy per concrete type, with no run-time type information. | M | 0.4 | |
| GEN-04 | Classes derived from `typing.Protocol` shall be satisfied by any class with matching methods, checked when the program is compiled. | M | 0.4 | |
| GEN-05 | `isinstance` with a protocol shall be rejected. | M | 0.4 | |
| GEN-06 | `*Ts` and `**P` type parameters, and defaults of type parameters, shall be accepted. | C | 0.13 | 8.10 |
| GEN-07 | A generic function or class of another module shall be usable, made from its source as ADR-0016's interfaces carry it. | S | 0.5 | |
| GEN-08 | A generic class shall be accepted with a base other than `object`, as the base of another class, and with class variables and defaults. | S | 0.5 | 8.10 |
| GEN-09 | Type parameters shall be accepted on a method of a generic class, a special method, a property, a class method, and a method that overrides or is overridden. | S | 0.5 | 8.10 |
| GEN-10 | A protocol shall be usable as the type of a variable, an attribute, a result and a container item outside a parameter, and of a parameter of a special method or a property. | S | 0.5 | 8.10 |
| GEN-11 | A class variable or an attribute default of a generic class whose annotation names a type parameter of the class shall be accepted. | C | 0.13 | 8.10 |
| GEN-12 | A generic class shall be accepted with an exception class as its base. | C | 0.13 | 8.10 |
| GEN-13 | A generic protocol shall be accepted, satisfied by the classes whose members fit it for its type arguments. | C | 0.13 | 8.10 |
| GEN-14 | `isinstance` and a class pattern with a generic class shall narrow a variable of a base's type to the copies of the class the program makes. | C | 0.13 | 8.10 |
| GEN-15 | A class derived from a copy of a generic class shall inherit the generic class's class methods, and a dataclass derived from a copy of a generic dataclass its fields. | C | 0.13 | 8.10 |
| GEN-16 | Type parameters shall be accepted on a special method, on a property, and on a method that overrides or is overridden by one with other type parameters or in a generic class, and a class method with type parameters shall be callable through an instance. | C | 0.13 | 8.10 |
| GEN-17 | A protocol shall be usable as the base of a class, with properties, static methods, class methods, class variables and defaults, and in the signatures of its own methods; an attribute shall be assignable through a protocol value, and a value of a built-in type shall be held as a protocol. | C | 0.13 | 8.10 |

### 3.12 Exceptions (EXC)

| ID | Requirement | Priority | Release | LR |
| --- | --- | --- | --- | --- |
| EXC-01 | Raising and catching exceptions shall follow the Language Reference, including matching on base classes. | M | 0.3 | 4.3 |
| EXC-02 | The built-in exception classes the runtime raises shall exist with their Python names: `ArithmeticError`, `AssertionError`, `EOFError`, `IndexError`, `KeyError`, `OverflowError`, `RecursionError`, `RuntimeError`, `StopIteration`, `SystemExit`, `TypeError`, `UnicodeDecodeError`, `UnicodeEncodeError`, `ValueError`, `ZeroDivisionError`, `OSError` with each of its subclasses, and their bases. | M | 0.3 | 4.3 |
| EXC-03 | An exception that is not raised shall cost no more than a compare and branch at each call that may raise. | M | 0.3 | |
| EXC-04 | Exception chaining with `from` and the implicit context shall be kept, and shown in the uncaught exception output. | S | 0.3 | 7.8 |
| EXC-05 | An exception shall not cross a function exported to C (FFI-05): a `SystemExit` shall end the process with its status, and any other exception shall abort it with the exception printed. | M | 0.5 | |
| EXC-06 | A `return`, `break` or `continue` that leaves a `finally` block shall be rejected, as PEP 765 makes CPython 3.14 warn about it (ADR-0013). | M | 0.3 | 8.4 |
| EXC-07 | A `raise` without an expression shall re-raise the exception being handled, and shall be rejected outside an `except` clause. | M | 0.3 | 7.8 |
| EXC-08 | `str()`, `repr()`, `print()` and f-strings of an exception shall give CPython's text, computed from the arguments it was built with. | M | 0.3 | |
| EXC-09 | An `except` clause shall name one exception class or a tuple of them, with or without `as`, or none for a bare `except`; any other expression shall be rejected. | M | 0.3 | 8.4 |
| EXC-10 | `NotImplementedError` and `AttributeError` shall exist with their Python names and bases, for programs to raise. | M | 0.3 | |
| EXC-11 | The `args` attribute of an exception shall give the tuple of its arguments. | S | 0.4 | |
| EXC-12 | Calling `UnicodeDecodeError` or `UnicodeEncodeError`, or deriving a class from them, shall be rejected; catching them shall be accepted. | S | 0.3 | |
| EXC-13 | The `code` attribute of `SystemExit` shall give the argument it was built with, or `None` without one. | S | 0.4 | |
| EXC-14 | `OSError` and its subclasses built with two to five arguments shall give the subclass for the `errno`, and the attributes `errno`, `strerror` and `filename`, as CPython does. | S | 0.4 | |
| EXC-15 | `args` of an exception and `code` of a `SystemExit` shall be values of their own: stored, passed, indexed and iterated. | S | 0.5 | |
| EXC-16 | An `OSError` shall take arguments of any type, more than five of them, and two or more through `super().__init__`, as CPython does. | C | 0.5 | |
| EXC-17 | `args` of an exception shall hold the `args` of an exception of a related class, a type that nests without end, as CPython allows. | C | 1.0 | |
| EXC-18 | `characters_written` of a `BlockingIOError`, and a number as the third argument of `OSError` or `BlockingIOError`, shall behave as CPython's do. | C | 1.0 | |
| EXC-19 | The text and the class of an `OSError` the C library reports shall be those CPython gives on the program's operating system: on macOS, `strerror`'s text and macOS's error numbers. | M | 0.7 | |
| EXC-20 | The special methods an exception class of the program defines, such as `__eq__`, `__lt__`, `__hash__` and `__len__`, shall be called by the operators and the built-in functions as they are for other classes. | C | 1.0 | |

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
| MOD-09 | `sys.set_lazy_imports`, `sys.set_lazy_imports_filter`, `sys.get_lazy_imports`, `sys.get_lazy_imports_filter` and `sys.lazy_modules` shall be rejected, because they change or depend on import behaviour set from outside the importing module. | S | 0.3 | |
| MOD-10 | An imported module shall be an object of type `module` whose attributes are its module-level names: `m.x` shall read and `m.x = v` assign them, the object may be bound to other names and compared with `is`, and its text shall name its path from the project root (OI-20, ADR-0015). | M | 0.3 | 5.3 |
| MOD-11 | A name a module binds by a lazy import shall be usable from another module, a module variable shall be assignable as `m.x = v` through a lazily imported module `m`, and a name shall be bindable by more than one lazy import of the module. | S | 0.4 | |
| MOD-12 | A function or a class shall be bindable by more than one import of a module, eager or lazy, and by an import inside a block, a call running the one the import that ran last bound, and raising `NameError` when none has run. | C | 0.5 | |
| MOD-14 | A name that imports bind to more than one function or class shall be usable as a value, in an annotation and from another module. | C | 1.0 | |
| MOD-15 | An `__init__` shall be able to import a module that imports its own module at module level. | C | 1.0 | |
| MOD-16 | From 1.0, TyPy modules shall link to each other as shared libraries against one shared runtime, through a stable ABI of object layouts, exception state and type identity that every 1.x release keeps (ADR-0074). | S | 1.0 | |
| MOD-13 | A module shall be importable inside a function of a module that it imports at module level. | S | 0.5 | |

### 3.14 Built-in functions and standard library (LIB)

| ID | Requirement | Priority | Release |
| --- | --- | --- | --- |
| LIB-01 | `print` with `sep` and `end`, `len`, `range`, `abs`, `min`, `max`, `int`, `float`, `str`, `bool`, `repr`, `ord` and `chr` shall be available. | M | 0.1 |
| LIB-02 | `list`, `dict`, `set`, `tuple`, `sorted`, `reversed`, `enumerate`, `zip`, `sum`, `any`, `all` and `round` shall be available. | M | 0.2 |
| LIB-03 | The methods of `str`, `bytes`, `list`, `dict` and `set` listed in the design shall be available with Python's results. | M | 0.2 |
| LIB-04 | `sys.argv`, `sys.exit`, `sys.stdout`, `sys.stderr`, the `file` argument of `print`, `input` and `open` for text files shall be available. | M | 0.3 |
| LIB-05 | The `math` module shall be available with its constants and its functions of numbers, with CPython's results and errors; a name the module gained after the program's Python floor shall be rejected. | M | 0.3 |
| LIB-06 | The `dataclasses` names in CLS-05 and the `typing` names in TYP and GEN shall be available. | M | 0.4 |
| LIB-07 | `os.environ` and `os.path` shall be available. | S | 1.0 |
| LIB-08 | Importing a standard library module that TyPy does not provide shall be rejected with the list of modules it provides. | M | 0.3 |
| LIB-09 | Each standard library module TyPy provides shall be TyPy's own implementation, with the names and results the Python library documentation defines. | M | 0.1 |
| LIB-10 | `str % values`, printf-style formatting of a string literal with a value, a tuple of values or a dict of named values, shall be accepted with Python's results, and a format that does not fit its values shall be rejected with CPython's message. | S | 0.3 |
| LIB-11 | `format % values` shall be accepted with a format that is not a string literal, and with a `tuple[T, ...]` of values. | S | 0.4 |
| LIB-12 | `math.fsum`, `math.prod`, `math.sumprod`, `math.dist`, `math.gamma` and `math.lgamma` shall be available. | S | 0.4 |
| LIB-13 | `sys.maxsize`, `sys.byteorder`, `sys.platform`, `sys.version`, `sys.version_info`, `sys.hexversion`, `sys.maxunicode`, `sys.getrecursionlimit`, `sys.setrecursionlimit` and `sys.stdin` shall be available. | S | 0.4 |
| LIB-14 | `open` shall accept binary modes, `+` modes, a mode that is not a literal, a file descriptor as the file, the encodings `ascii` and `latin-1`, and the arguments `buffering`, `errors`, `newline` and `closefd`; a text file shall have `seek`, `tell`, `truncate`, `fileno`, `isatty`, `readable`, `writable` and `seekable`, and the attributes `name`, `mode` and `closed`; closing `sys.stdout` or `sys.stderr` shall close it. | S | 0.4 |
| LIB-15 | `math.fsum`, `math.dist`, `math.prod` and `math.sumprod` shall take items of a union of number types, and `math.prod` a `bool` start. | C | 0.5 |
| LIB-16 | `sys.version_info` shall be a value as CPython's: printed, stored, passed and imported with `from sys import version_info`. | C | 0.5 |
| LIB-17 | The names `Optional`, `Union`, `List`, `Dict`, `Set`, `Tuple` and `Type` of `typing` shall be accepted in annotations as the types they spell; `Callable` follows function values (FUN-05). | S | 0.5 |
| LIB-18 | `open` shall accept a callable `opener`, which opens the file and returns its descriptor. | C | 1.0 |
| LIB-19 | `open` with a binary mode shall accept a `buffering` that is not a literal, giving a `typing.BinaryIO`. | C | 1.0 |
| LIB-20 | The abstract collections of `collections.abc` and `typing`, such as `Sequence`, `Mapping`, `MutableSequence` and `Collection`, shall be accepted as types holding the built-in types CPython registers with them (ADR-0086). | S | 0.10 |
| LIB-21 | `math.prod` shall take items and a start other than numbers, multiplying them in turn as CPython does. | C | 1.0 |
| LIB-22 | `sys.platform` shall be `"darwin"` in a program built for macOS and `"linux"` in one built for Linux. | M | 0.7 |
| LIB-23 | `map`, `filter` and `callable` shall be available, `map` and `filter` giving iterators that go on after the function raises, as CPython's do (ADR-0077, ADR-0089). | S | 0.10 |
| LIB-24 | `Sized`, `Hashable` and `MappingView` shall be accepted as types, and a class of the program with the methods of `Collection`, `Container` or `Reversible` shall be accepted where that collection is expected. | C | 0.14 |
| LIB-25 | `map` and `filter` themselves shall be values, held in a variable, passed, written and given to `isinstance`, and `map` shall take `*` arguments, as CPython's classes are and do. | C | 1.0 |
| LIB-26 | `math.integer` (PEP 791) shall be available with `comb`, `factorial`, `gcd`, `isqrt`, `lcm` and `perm`, under a Python floor of 3.15 or later. | S | 0.12 |
| LIB-27 | `textwrap`, `base64`, `heapq`, `bisect`, `statistics`, `time`, `random`, `copy` and `string` shall be available, written in typed Python that CPython also runs, with signatures a test compares with typeshed's (ADR-0007, ADR-0097). | S | 0.12 |
| LIB-28 | The builtins `divmod`, `bin`, `hex`, `oct`, `pow`, `format` and `bytearray` shall be available, as CPython gives them. | S | 0.12 |
| LIB-29 | `from __future__ import annotations` shall be accepted and shall defer the module's annotations, as CPython does; the other features of `__future__` that Python 3 already has shall be accepted and change nothing; and a future import CPython's compiler refuses shall be E0103. | S | 0.12 |

### 3.15 Memory management (MEM)

| ID | Requirement | Priority | Release |
| --- | --- | --- | --- |
| MEM-01 | An object shall be freed when its last reference goes, with no tracing garbage collector and no pause. | M | 0.1 |
| MEM-02 | `float`, `bool` and inline `int` values shall be held without a heap allocation in variables, fields and containers, so a `list[float]` stores 8 bytes per item. | M | 0.2 |
| MEM-03 | An instance of a class of the program, other than an exception, that the compiler proves does not outlive its function shall be allocated without a heap call. | S | 0.5 |
| MEM-04 | Passing a variable or a constant as an argument shall cost no reference count operation, and returning a variable shall move its value to the caller. | M | 0.2 |
| MEM-05 | Freed memory of one object shall be reused for a new object of the same size in the same function when the compiler can prove it safe. | C | 1.0 |
| MEM-06 | Reference cycles shall be collected, without a background thread, and only objects whose class can be part of a cycle shall pay for cycle tracking (ADR-0001). | S | 1.0 |
| MEM-07 | `weakref.ref` shall be available. | S | 1.0 |
| MEM-08 | The allocator shall be replaceable per project through three C functions, allocate, reallocate and free, that `typy.toml` names. | S | 0.5 |
| MEM-09 | The last use of a variable in an assignment, or as an item a collection stores, shall move its value instead of counting it. | S | 0.3 |
| MEM-10 | A list, dict, set, string or exception object that the compiler proves does not outlive its function shall be allocated without a heap call. | C | 1.0 |
| MEM-11 | The attributes of an instance and the items of a tuple, a closure's captures and a cell's value shall be stored in decreasing order of alignment, while the program reads, writes, compares, hashes and writes out each in its declared order. | M | 0.8 |
| MEM-12 | Each object's header shall hold the thread that owns it and a shared reference count, so a count changes with plain instructions in the owning thread and atomically in others, ready for threads that run in parallel over shared objects (ADR-0096). | S | 1.0 |

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
| FFI-05 | A function that `__all__` of a module built with `--lib` lists shall be exported with an unmangled C symbol and the C types its annotations give, and the build shall write a matching C header (ADR-0032). | M | 0.5 |
| FFI-06 | `ctypes.CFUNCTYPE` callbacks shall be accepted. | C | 0.13 |
| FFI-07 | A C header shall be convertible into a module of ctypes declarations. | C | 0.13 |
| FFI-08 | A library build shall write, beside the library and its C header, a `.pyi` stub of its exported functions and a `ctypes` module that loads the library and declares them, so CPython and TyPy programs call it with the same source (ADR-0032). | S | 0.5 |
| FFI-09 | A library a `ctypes` declaration loads, and its functions, shall be values a program stores, passes and prints. | C | 0.13 |
| FFI-10 | The ctypes names `c_char`, `c_wchar`, `c_wchar_p`, `create_string_buffer`, `cast` and `addressof`, pointers to and arrays of pointers and arrays, arrays whose length is not an integer literal, array slices, `byref` other than as a C function's argument, assigning a pointer's `contents`, ctypes classes as values, in annotations, `isinstance` and `type`, and `POINTER`, `pointer`, `byref`, `sizeof`, `CDLL` and `cdll` as values shall be accepted. | C | 0.13 |
| FFI-11 | Methods and class attributes other than one `_fields_` list in a `Structure` subclass, `_fields_` assigned after the class statement, bit fields, a subclass of a `Structure` subclass, and `ctypes.Union` shall be accepted. | C | 0.13 |
| FFI-12 | A `ctypes` structure passed to or returned from a C function by value shall follow the C calling convention of the target: System V on x86_64 Linux and AAPCS64 on AArch64 Linux. | M | 0.6 |
| FFI-13 | A `ctypes` call shall pass each integer and `c_bool` argument as `libffi` passes it on the target, a 64-bit word widened by its type in a register and, on Apple's arm64, its own size on the stack, so a C function reads the values it reads under CPython. | M | 0.7 |
| FFI-14 | A module-level `ctypes.CDLL(None)` or `ctypes.cdll.LoadLibrary(None)` shall declare the C library and the C maths library that every program links, whose functions it calls on every target, with no other library linked. | M | 0.7 |
| FFI-15 | A TyPy library shall raise `RecursionError`, never overflow its stack, on any thread its host calls it from, also one created with a stack smaller than `RLIMIT_STACK` (ADR-0075). | S | 0.10 |

### 3.17 Diagnostics (DIA)

| ID | Requirement | Priority | Release |
| --- | --- | --- | --- |
| DIA-01 | Every diagnostic shall have a stable code, a `file:line:column` location with columns counted in code points, and one sentence that says what is wrong. | M | 0.1 |
| DIA-02 | A diagnostic shall show a corrected line under `help:` when the fix is certain. | S | 0.1 |
| DIA-03 | A diagnostic that rejects valid Python shall name the TyPy rule, by ID, that the program falls outside. | M | 0.1 |
| DIA-04 | One root cause shall give one error, and errors that follow from it shall be suppressed. | M | 0.1 |
| DIA-05 | `--error-format=json` shall write the same diagnostics as JSON, one object per line. | S | 0.3 |
| DIA-06 | A code shall never be reused, and a retired code shall stay reserved. | M | 0.1 |
| DIA-07 | Warnings shall not fail a build unless `--deny-warnings`, or `-D` for their code, is given (CLI-27). | M | 0.3 |
| DIA-08 | Where CPython 3.12 to 3.15 give a `SyntaxWarning` while compiling a program TyPy accepts, TyPy shall give a warning: for an invalid escape sequence and for an assertion that is always true. | S | 0.3 |

Code ranges follow the pack: `E01xx` lexical, `E02xx` declarations, `E03xx`
expressions and statements, `E04xx` types, `E05xx` exceptions and control
flow, `E06xx` memory, `E07xx` C interoperability, `E08xx` modules, `E9xxx`
internal errors, and `Wxxxx` warnings in the same ranges.

### 3.18 User documentation (DOC)

The user documentation is written as Markdown under `docs/`, in the
sections of `docs/index.md`, beside the engineering documents, and
describes the release it ships with.

| ID | Requirement | Priority | Release |
| --- | --- | --- | --- |
| DOC-01 | A getting-started page shall show how to install TyPy from a release archive, and how to check, build and run a first program. | M | 0.2 |
| DOC-02 | The language and library references shall have one page per area of the specification, listing what the release accepts with the rule ID of each item, and an accepted and a rejected example. | M | 0.2 |
| DOC-03 | A troubleshooting page shall list each active diagnostic code with its cause and how to change the program. | M | 0.2 |
| DOC-04 | Every Python sample in the user documentation shall be checked by the test suite against the current compiler, with the outcome the page states. | M | 0.2 |
| DOC-05 | The documentation shall be published at <https://typy-lang.org>, built in strict mode from the sources of each release, with every page in the site's navigation. | M | 0.6 |

## 4. Non-functional requirements

Measurements run on the reference machine: Linux x86_64, 8 cores, warm file
cache, unless a row says otherwise.

| ID | Requirement | Release | Verified by |
| --- | --- | --- | --- |
| NFR-01 | A hello world program built with `--release-small` and linked statically with musl shall be smaller than 100 KB. | 0.1 | Size check in CI |
| NFR-02 | The runtime code and data that a hello world built with `--release-small` links, excluding libc, shall be smaller than 48 KB, and smaller than 32 KB without its large-integer and cycle-collection parts (ADR-0009). | 0.1 | Size check in CI, for both figures |
| NFR-03 | The geometric mean run time of the benchmark suite in `--release` mode shall be within 1.5 times that of the equivalent C programs. | 1.0 | Weekly benchmark run |
| NFR-04 | A debug build of a 10,000-line project shall take under 2 seconds. | 1.0 | Benchmark with budget |
| NFR-05 | A release build of a 10,000-line project shall take under 20 seconds. | 1.0 | Benchmark with budget |
| NFR-06 | The same source, flags, target and compiler version shall produce byte-identical output. | 0.1 | Build twice and compare in CI |
| NFR-07 | Safe code shall have no use after free, double free, null dereference, out-of-bounds access or uninitialised read. | 0.1 | Run tests under AddressSanitizer; fuzzing |
| NFR-08 | A program without reference cycles shall free all its memory before exit. | 0.1 | Leak checker on every run test |
| NFR-09 | The compiler shall not crash on any input; an internal error shall be reported as `E9xxx` with a hint to report it. | 0.1 | Fuzzing for 30 minutes before each release, zero crashes |
| NFR-10 | Every requirement in section 3 shall have at least one accepted and one rejected test program, and every run test shall run alike on each oracle version from its Python floor up. | 0.1 | Conformance suite in CI |
| NFR-11 | Line and branch coverage of the compiler and runtime shall be at least 96 percent. | 0.10 | Coverage gate in CI |
| NFR-12 | An object header shall take at most 8 bytes. | 0.2 | Layout test |
| NFR-13 | A `list[int]` of one million small values shall use at most 8.5 MB. | 0.2 | Layout test of the runtime's list |
| NFR-14 | The compiler shall use at most 1 GB of memory to build a 10,000-line project. | 1.0 | Benchmark with budget |
| NFR-15 | A debug build and a release build of a program shall run alike. | 0.1 | Differential tests in CI |
| NFR-16 | Every run test of the conformance suite shall give the same output and exit status when built for each supported target and run on it. | 0.6 | Conformance suite on an arm64 CI runner and under qemu-user |
| NFR-17 | Every conformance test shall give on macOS the output and exit status CPython gives on macOS. | 0.7 | Conformance suite on a macOS CI runner |
| NFR-18 | An instance of a class shall take at most its 16-byte prefix and the sizes of its attributes, each tuple attribute at its own size, rounded up to a multiple of 8 bytes. | 0.8 | Layout test of each descriptor's size and offsets against LLVM's |
| NFR-19 | A value of type `Callable[[A, ...], R]` shall hold only a function that takes arguments of the types `A, ...` and returns an `R`, so a call through it passes and returns the values its code expects. | 0.8 | Checker tests that reject each assignment, argument and return that would break it; the sanitizer run of every conformance test |
| NFR-20 | A generator object shall be one allocation that holds its frame and its position, with no machine stack of its own, and a step of a generator shall allocate nothing but the values it yields. | 0.9 | `typy-codegen` test of a generator object's layout; a runtime test that counts the allocations of a loop over a generator |
| NFR-21 | No program of the benchmark suite built with `--release` shall take longer to run than CPython 3.15 takes to run it. | 0.11 | Weekly benchmark run |
| NFR-22 | The geometric mean run time of the benchmark suite in `--release` mode shall be within 5 times that of the equivalent C programs. | 0.12 | Weekly benchmark run |
| NFR-23 | The geometric mean run time of the benchmark suite in `--release` mode shall be within 3 times that of the equivalent C programs. | 0.15 | Weekly benchmark run |
| NFR-24 | Each release shall report the share of a fixed corpus of Python modules that pass `mypy --strict` which TyPy compiles unchanged and runs with CPython's output, and the share shall not fall from one release to the next. | 0.11 | Corpus run in the release workflow |
| NFR-25 | Each release archive shall come with a software bill of materials, in CycloneDX or SPDX form, that lists every component the archive holds with its version and licence. | 0.10 | Release workflow check |
| NFR-26 | Each release archive shall be signed with Sigstore's keyless signing and carry SLSA build provenance from the release workflow, so a user verifies it without a key the project keeps. | 0.10 | `gh attestation verify` in the release workflow |
| NFR-27 | On one thread, biased reference counts (MEM-12) shall cost no more than 2% of the geometric mean run time of the benchmark suite, against plain counts. | 1.0 | Benchmark with budget |

## 5. Release plan

| Release | Language added | Other content |
| --- | --- | --- |
| 0.1 | Functions, `int`, `float`, `bool`, `str`, `None`, `if`, `while`, `for` over `range`, operators | `build`, `run`, `check`, `--emit`; runtime; diagnostics; Linux x86_64 |
| 0.2 | `list`, `tuple`, `dict`, `set`, `bytes`, classes without inheritance, f-strings, comprehensions, slicing | Unboxed values, reference count elision; user documentation |
| 0.3 | `try`, `raise`, user exceptions, `with`, `assert`, `del`, `global`, modules and packages, lazy imports | `sys`, `math`, `open`; `typy.toml` with `python-floor`; JSON diagnostics; the first warnings and `--deny-warnings` |
| 0.4 | Dataclasses, inheritance, protocols, PEP 695 generics, narrowing, `Final`, `type` aliases | Binary files and the rest of `open` |
| 0.5 | `match`, `:=`, dataclass `default_factory`, hashing and options, the rest of the class protocols (CLS-20 to CLS-26), C interoperability through `ctypes` | Stack allocation; libraries and exports described by their annotations (ADR-0032) |
| 0.6 | | Linux AArch64 as host and target; cross-compilation between the Linux targets; the documentation site |
| 0.7 | | macOS AArch64 as host and target; LLVM built and pinned by TyPy for every host |
| 0.8 | Functions as values: nested functions and closures, `lambda`, `nonlocal`, keyword-only and positional-only parameters, the `key` argument of `sorted`, `list.sort`, `min` and `max` | Attributes, tuple items, captures and cells stored by decreasing alignment |
| 0.9 | Generators: `yield` and `yield from`, generator functions of `Generator`, `Iterator` and `Iterable` types, generator expressions; classes with `__iter__` and `__next__` as iterators; `*args`, `**kwargs` and unpacking with `*` and `**` | Generator objects of one allocation |
| 0.10 | `Sequence` and the other abstract collections; built-in functions, built-in types, class methods, protocol methods and functions of a module with defaults, `*args` or `**kwargs` as values; `map`, `filter` and `callable`; decorators of functions; unpacking written out in full in the calls of built-ins and function values; nested functions inside generic functions; `NamedTuple`; `type[C]` holding derived classes, `isinstance` with a class held in a variable, and `issubclass` | Per-thread stacks for libraries; a bill of materials, signing and provenance for each release; coverage gate at 96 percent |
| 0.11 | `typing` names and aliases as values (TYP-18); nested functions with type parameters of their own (FUN-18); nested functions, methods and `lambda`s with defaults, `*args` or `**kwargs` as values (FUN-19); unpacking of any iterable in the calls of built-ins and function values, and of a dict in a call of `print` (EXP-22); `object` as a type (TYP-23) and a checked `cast` (TYP-24); arithmetic on `bool` as CPython does it (DAT-19) | No benchmark program slower than CPython 3.15; the share of a `mypy --strict` corpus TyPy compiles; the concurrency model settled (OI-31) |
| 0.12 | Results inferred from `return` statements and empty collections from their first use (TYP-26, TYP-27); `enum` and `abc` classes (CLS-44, CLS-45); `Literal`, `@overload`, `Self`, `TypeGuard` and `TypeIs` (TYP-28 to TYP-31); unpacking in comprehensions, `frozendict` and `math.integer` for the Python floor 3.15 (EXP-23, DAT-18, LIB-26); the first typed standard library modules (LIB-27); the builtins `divmod`, `bin`, `hex`, `oct`, `pow`, `format` and `bytearray` (LIB-28); `from __future__ import annotations` (LIB-29); optimisation levels, a target processor, timings, shown commands, kept files, `typy print`, colour, per-code warning levels, `run -c` and `-m`, more `--emit` stages, `-C` options and external `typy-NAME` commands (CLI-20 to CLI-30, CLI-32, CLI-33) | The benchmark suite within 5 times C |
| 0.13 | Generics: `*Ts` and `**P`, defaults of type parameters, generic exceptions and protocols, type parameters on special methods; `typing.Any` bound when the program runs (TYP-25); `TypedDict` (TYP-32); template strings for the Python floor 3.14 (LEX-12); `ctypes` callbacks, headers and the rest of `ctypes`; debug information, AddressSanitizer builds and runtime inlining (CLI-31, CLI-34, CLI-35) | |
| 0.14 | The rest of the class rules (CLS-28 to CLS-37) and of named tuples (CLS-43); method decorators; classes and `type[C]` values as callables; positional class patterns through a variable; structural abstract collections for classes of the program; optimisation remarks (CLI-36) | |
| 0.15 | | The benchmark suite within 3 times C |
| 1.0 | The remaining requirements of section 3 marked 1.0; profile-guided optimisation (CLI-37) | Cycle collection, `weakref`; the object header of biased reference counts (MEM-12); Windows; performance budgets |

## 6. Out of scope

### 6.1 Excluded from TyPy

TyPy rejects these features (STD-04). The research note "values whose type
is known late" separates the limits of compiling ahead of time from the
costs TyPy has not taken on.

These need what no ahead-of-time compiler gives without CPython or an
interpreter inside the program:

- `eval`, `exec`, `compile` and `__import__` of code that exists only when
  the program runs.
- Running CPython extension modules or CPython packages, such as NumPy,
  which need CPython's object layout and C API.

These are costs TyPy has not taken on, which a later decision may plan:

- Metaclasses other than those `enum` and `abc` give (CLS-44, CLS-45),
  class decorators that change the class, and `type()` with three
  arguments. A class decorator that returns the class it is given is
  planned for 1.0 (CLS-38).
- Adding or replacing attributes of classes, modules or functions at run
  time, and `__getattr__`, `__getattribute__`, `__setattr__` and
  `__delattr__`.
- `globals()`, `locals()` and `vars()`.
- A value whose type the program gives neither by an annotation nor by
  inference (TYP-10). `object`, `typing.cast` and `typing.Any` are
  planned (TYP-23 to TYP-25).
- Complex numbers (LEX-08).

By design, TyPy adds no names of its own: no fixed-width integer types, no
`unsafe` blocks and no decorators of its own (STD-07).

### 6.2 Planned for after 1.0

| Feature | First Python version |
| --- | --- |
| Exception groups and `except*` | 3.11 |
| `async` and `await` | 3.5 |
| Threads running in parallel over shared objects, through `threading`, `queue`, `concurrent.futures` and `concurrent.interpreters` (ADR-0096) | |
| Multiple inheritance | |
| `__del__` | |
| A language server | |
| `enumerate`, `zip` and `reversed` objects held as values | |
| `from module import *` | |
| Running the code that defines classes in the compiler, for metaclasses other than those `enum` and `abc` give (OI-34) | |
| `doctest`, which runs the examples in a module's docstrings through `exec` when the program runs (OI-43) | |

## 7. Open issues

A new question gets the next unused issue number, after the last of the
resolved issues below. No question is open.

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
| OI-12 | May an `int` be stored where a `float` is declared, as PEP 484 allows? | No. CPython keeps the value an `int`, so `x: float = 1` prints `1`; TyPy would print `1.0`. TYP-09 rejects it, with the fix `1.0` for a literal. Checked on CPython 3.12 to 3.15. | 2026-11-06 |
| OI-13 | Can `x ** y` with a `float` exponent give a type TyPy does not have? | Yes: `(-8.0) ** 0.5` is `complex` in CPython 3.12 to 3.15. EXP-04 accepts a `float` exponent only when it is an integral literal or the base is a non-negative literal. | 2026-11-07 |
| OI-14 | Which wording does an exception message take when the oracle versions word it differently? | The wording of the newest oracle, CPython 3.15. CPython 3.14 reworded every division by zero to `division by zero` and `0.0 ** -1.0` to `zero to a negative power`, where 3.12 and 3.13 name the operation, and 3.12 and 3.13 differ from each other for `float` modulo. A conformance test cannot check such a message, because the oracles disagree on it, so the runtime's unit tests check it (STD-03, STD-05). | 2026-11-07 |
| OI-15 | Which Unicode version decides `repr`, `str.isprintable` and the digits `int()` reads, when CPython 3.12 to 3.15 ship Unicode 15.0 to 17.0? | Unicode 17.0, the version of the newest oracle, as OI-14 decides for messages. A character assigned after Unicode 15.0 is shown unescaped by TyPy and by CPython 3.15 but escaped by older oracles, so a conformance test of such a character is invalid (STD-03). | 2026-11-07 |
| OI-16 | How are generated code and the runtime instrumented with AddressSanitizer, and the runtime's unit tests run under Miri, when both need a nightly Rust toolchain? | The dev image holds one pinned nightly, `nightly-2026-11-05`, used only for these checks and for fuzzing; every build still uses stable. The runtime is built with `-Zsanitizer=address` for the conformance suite's sanitizer run, and `make miri` runs its unit tests. Instrumenting generated code needs LLVM's AddressSanitizer pass in code generation and is planned for 0.2. | 2026-11-07 |
| OI-17 | How does a 0.2 program end a `for` over a class with `__iter__` and `__next__`, when `raise` arrives in 0.3? | It does not: CST-03 covers the built-in iterables in 0.2, and CST-08 adds iterator classes in 0.3 together with `raise`, so `StopIteration` works as in CPython. | 2026-11-08 |
| OI-18 | How do items of type `float` compare inside containers? CPython compares container items by identity before value, so with `x = float("nan")`, `(x,) == (x,)`, `[x] == [x]` and `x in [x]` are `True`, while TyPy holds floats without an object. | By value, as `x == x` does: a NaN item is never equal to another, in a tuple, a list, `in`, `count`, `index`, a `dict` key or a `set`. This gives CPython's result for NaN values made separately, and differs from it only when one NaN object is placed in a container more than once, which the user documentation names as a known difference. | 2026-11-09 |
| OI-19 | In which order does a set display of constant items iterate, when the oracle versions build it differently? A display of three to thirty items that are all constants compiles to a frozenset of the items, which is merged into a new set, and the merge can place the items in other slots than adding them one by one does. CPython 3.12 and 3.13 also rebuild the frozenset once more from its own order when they merge the code's constants, and CPython 3.14 iterates a `for` loop over such a display in the order of the items added one by one. | The order of the newest oracle, CPython 3.15, as OI-14 decides for messages: such a display, and a loop over it, holds the items added in order to a set, then merged into a new set. A constant item is a literal, `None`, `True`, `False`, a number with a sign, or a tuple of constant items; CPython also folds arithmetic on constants, which TyPy computes when the program runs, so a display such as `{1 + 2, 4, 5}` may iterate in another order than CPython's. A conformance test of such a display is invalid unless every oracle prints it alike (STD-03, DAT-10, ADR-0010). | 2026-11-09 |
| OI-20 | What does the text of a module object show, when CPython's `<module 'm' from '/abs/path/m.py'>` names the absolute path of the file, which would make the build depend on its directory (NFR-06)? | The path from the project root, `<module 'a.b' from 'a/b.py'>`. It is the one known difference from CPython's text, named in the user documentation (MOD-10, ADR-0015). | 2026-11-11 |
| OI-21 | Which imports form the cycle MOD-05 rejects, when an import inside a function runs only once the function is called? | The imports at module level. An import inside a function adds no edge and may find a module still initialising, as in CPython, whose names not yet bound raise CPython's `NameError`, `AttributeError` or `ImportError` (ADR-0015). | 2026-11-11 |
| OI-22 | How is a module checked whose function imports a module that imports it at module level, when each module is checked after the modules it imports? | Such an import is E0426, planned for 0.4: TyPy checks the modules in the order their imports give, an import inside a function counting when it closes no cycle, so the importing function's module comes first and cannot see the other's names yet (ADR-0015). | 2026-11-11 |
| OI-23 | Which ABI lets TyPy modules link dynamically: the layouts of objects, the exception state, the identity of type objects across shared libraries, and one runtime shared by every library (ADR-0016)? It is settled after 1.0, once those layouts are fixed. | From 1.0 TyPy defines a stable ABI: object layouts, the exception state and the identity of type objects are fixed for every 1.x release, and one shared runtime serves every TyPy module in a process (ADR-0074, MOD-16). | 2026-11-20 |
| OI-24 | What is `sys.argv[0]`, when CPython gives the path of the script it runs and a TyPy program has no script? | The path of the executable as the operating system passes it to `main`, which is what a C program receives and what a shell user typed. The other items are the arguments, decoded as CPython decodes them. A conformance test does not print `sys.argv[0]`, because the oracle's differs (LIB-04, STD-03). | 2026-11-12 |
| OI-25 | Should the modules of the standard library TyPy provides ship prebuilt, one static library and interface per module and target, so a development build links them without compiling them? It depends on separate compilation (ADR-0016). | No archive of prebuilt modules: a development build compiles each module of the standard library the first time a project needs it and keeps it in the user's cache directory for each release, target and mode (ADR-0016, CLI-17). | 2026-11-20 |
| OI-26 | Do the AArch64 targets and cross-compilation (CLI-08) stay in 0.5? They need a cross toolchain and an emulator or arm64 runners for their tests. | They move to 0.6, so 0.5 holds the language, stack allocation, libraries and `ctypes`. | 2026-11-15 |
| OI-27 | In which order should `list.sort` and `sorted` call `__lt__`? The order shows when the method prints or raises, and CPython 3.12, CPython 3.13 and 3.14, and CPython 3.15 each compare in another order. | The order of the newest oracle, CPython 3.15, as OI-14 decides for messages: the runtime's sort is a port of CPython 3.15.2's `listsort`, which compares the same items in the same order, and a comparison that raises leaves the list in the order CPython leaves it. A conformance test that prints from `__lt__` while sorting is invalid unless every oracle prints alike (CLS-22, ADR-0039). | 2026-11-16 |
| OI-28 | How is LLVM pinned so that a release built in CI is byte for byte the build a developer makes of the same commit (NFR-06)? The dev image installs LLVM 23 from apt.llvm.org, which publishes dated snapshot builds of 23.1.x and removes old ones, so a CI image built later gets a newer build than a local image, and the 0.5.0 archive rebuilt locally differs from the published one. | TyPy builds LLVM 23.1.6 from its sources for each host in a workflow of its own, keeps each build as an asset of a toolchain release with its SHA-256 recorded in the repository, and the dev image and every CI job install that build (ADR-0054). | 2026-11-18 |
| OI-29 | Should the macOS release archive be notarized, so a browser download runs without clearing the quarantine first? Notarizing needs a paid Apple Developer membership and a signing identity in CI. | No. The archive stays unsigned and not notarized, so CI holds no Apple signing key; the macOS page shows how to clear the quarantine, and a download with `curl` or `gh` carries none (ADR-0055). | 2026-11-20 |
| OI-30 | How should a TyPy library on Linux find the stack of a thread its C host created with a stack smaller than `RLIMIT_STACK`? The stack check measures from the limit, so such a thread can overflow before `RecursionError`; `pthread_getattr_np` gives the thread's own stack but reads `/proc/self/maps` for the main thread on every export call. | The first export call on a thread other than the main one reads the thread's stack with `pthread_getattr_np` and keeps it for the thread; the main thread keeps `getrlimit`, so `/proc/self/maps` is never read (ADR-0075, FFI-15). | 2026-11-20 |
| OI-31 | Which concurrency model does TyPy take, and does safe code promise freedom from data races? The answer decides the object header and how reference counts change, which ADR-0074 fixes at 1.0, so it is settled before 1.0 although threads come after it: CPython's free-threaded build uses biased reference counting, and Swift 6 and Rust check data races when a program compiles. A research note measures three models on TyPy's benchmarks, isolated workers as in Python 3.14's `concurrent.interpreters` (PEP 734), free threads as in CPython's free-threaded build (PEP 703), and sharing that the checker proves safe, and the question is settled in the 0.11 planning. | Threads run in parallel over shared objects, as CPython's free-threaded build runs them: biased reference counts, a lock in each list, dict and set, and memory safety under any interleaving. Safe code does not promise freedom from data races, as neither CPython nor Java does; the checker may warn about races it proves. The header takes the owner and the shared count before 1.0 (MEM-12, NFR-27, ADR-0096). | 2026-11-21 |
| OI-32 | Is `object` a type, whose value of a type held by value is boxed and narrowed by `isinstance` and `match` as Java narrows `Object`, and in which release? TyPy rejects it today (TYP-10), although every program the research measured that mypy accepts with it is typable. | Yes, in 0.11: a value of type `object` is an object, boxed when it is held by value and narrowed by its type ID, as Java narrows `Object` (TYP-23, ADR-0090). | 2026-11-21 |
| OI-33 | Does TyPy infer a function's result from its `return` statements, an empty collection's item type from its first use, and unannotated parameters as type parameters with a copy for each set of argument types, which changes TYP-01? | Results are inferred from `return` statements and empty collections from their first use, in 0.12; parameters keep their annotations, so module interfaces stay readable from signatures (TYP-26, TYP-27, ADR-0092). | 2026-11-21 |
| OI-34 | Which code that defines classes, such as a metaclass, an `Enum` or a class decorator, may the compiler run while it compiles, and how is it shown to give the program CPython's meaning? | The checker expands the classes of `enum` and `abc`, as it expands dataclasses and named tuples, in 0.12; running definition code in the compiler stays planned for after 1.0 (CLS-44, CLS-45, ADR-0093). | 2026-11-21 |
| OI-35 | Does TyPy accept `Any` as a value whose operations bind when the program runs, with CPython's `TypeError` and `AttributeError`, paid for only where `Any` appears? | Yes, in 0.13: the operations of an `Any` value bind when the program runs, with CPython's errors, and only code that uses `Any` pays (TYP-25, ADR-0094). | 2026-11-21 |
| OI-36 | Does `typing.cast` compile to a check that raises `TypeError`, as mypyc compiles it, where CPython trusts the cast? | It compiles to a check that raises `TypeError` when the cast is wrong, as mypyc compiles it, in 0.11 (TYP-24, ADR-0091). | 2026-11-21 |
| OI-37 | Which CPython versions join the oracle after 3.15, and when does the lowest Python floor rise above 3.12? | Each CPython version joins the oracle in the first minor release after its final release, and the lowest floor rises in the first minor release after a version's end of life; 3.12 stays the lowest until October 2028 (STD-08, ADR-0095). | 2026-11-21 |
| OI-38 | In which release do template strings (PEP 750) become a planned feature, from the Python floor 3.14? TyPy 0.10 classes them as a form it cannot compile with CPython's meaning, though a template is an immutable object fixed when the expression runs. | Template strings are planned for 0.13, from the Python floor 3.14 (LEX-12). | 2026-11-21 |
| OI-39 | In which releases do `Literal`, `@overload`, `Self`, `TypeGuard`, `TypeIs` and `TypedDict` come, and does `Self` move from after 1.0? About one definition in seven of the standard library's stubs is an overload. | `Literal`, `@overload`, `Self`, `TypeGuard` and `TypeIs` come in 0.12, `Self` moving from after 1.0, and `TypedDict` in 0.13 (TYP-28 to TYP-32). | 2026-11-21 |
| OI-40 | In which release do PEP 798's unpacking in comprehensions, `frozendict` (PEP 814) and `math.integer` (PEP 791) come, from the Python floor 3.15? | All three come in 0.12, from the Python floor 3.15 (EXP-23, DAT-18, LIB-26). | 2026-11-21 |
| OI-41 | In which release does the `typy-stdlib` crate of ADR-0007 start, with which modules first, and are its signatures checked against typeshed? | The crate and `_typy` start in 0.12 with `textwrap`, `base64`, `heapq`, `bisect`, `statistics`, `time`, `random`, `copy` and `string`, their signatures compared with typeshed in a test (LIB-27, ADR-0097). | 2026-11-21 |
| OI-42 | Does TyPy accept `from __future__ import annotations`, and in which release? Under it CPython keeps each annotation as a string and lets an annotation name a class defined later; TyPy reads annotations when it compiles, so the import changes no meaning it gives, but it is E0206 today. It is the first error of 127 of the 584 algorithm scripts of the [corpus](../testing/corpus-report.md) and of 2 of its 83 mypyc run tests. | Yes, in 0.12 (LIB-29); until then it is E0201, planned for 0.12. | 2026-11-22 |
| OI-43 | How does TyPy treat a module whose main program runs `doctest.testmod()`? doctest reads the examples in the module's docstrings and runs them through `exec` when the program runs, which a compiled program cannot do; `doctest` is E0206 today. It is the first error of 165 of the 584 algorithm scripts of the corpus. | `doctest` is planned for a release after 1.0, and is E0201 until then; its modules stay in the corpus, refused. | 2026-11-22 |
| OI-44 | In which release do the builtins `divmod`, `bin`, `hex`, `oct`, `pow`, `format`, `id` and `bytearray` come? Each is E0202 today, as a builtin TyPy does not provide (STD-04), and together they are the first error of 24 modules of the corpus. | `divmod`, `bin`, `hex`, `oct`, `pow`, `format` and `bytearray` come in 0.12 (LIB-28), and are E0201, planned for 0.12, until then; `bytearray` gets its design when it is built. `id` stays refused: its values are not CPython's, which STD-03 leaves out of what a program may print. | 2026-11-22 |
| OI-45 | How does `typy` spell the options the research note "the command lines of compilers" proposes: a flag for each, or `-C KEY=VALUE` as `rustc` does? | Flags for the common options, spelt as `clang` and CPython spell them, and `-C KEY=VALUE` for the rest of code generation (ADR-0104). | 2026-11-23 |
| OI-46 | Which options come in 0.12, and which later? | The small set in 0.12 (CLI-20 to CLI-30, CLI-32, CLI-33); debug information, AddressSanitizer builds and runtime inlining in 0.13; optimisation remarks in 0.14; profile-guided optimisation in 1.0. | 2026-11-23 |
| OI-47 | Does `typy` offer `--target-cpu native`, against NFR-06's portability, and runtime inlining, 6 percent faster at twenty times the build time? | Both, opt-in and off by default (CLI-21, CLI-31, ADR-0104). | 2026-11-23 |
| OI-48 | Does `typy` gain project commands, such as tests, benchmarks, formatting and cleaning the cache, as `cargo` has them? | `typy` stays the compiler; any other command is an external program `typy-NAME` on `PATH`, as `git` and `cargo` find theirs (CLI-32, ADR-0105). | 2026-11-23 |

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
