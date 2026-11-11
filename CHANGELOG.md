# Changelog

All notable changes to TyPy are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and TyPy uses
[semantic versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `raise` with an exception or an exception class, and `raise ... from
  cause` and `raise ... from None`. An uncaught exception prints its cause
  or its context first, with CPython's line between them (SMT-06, EXC-04,
  STD-05).
- The built-in exception classes as values, annotations and calls, with
  CPython's bases, so a `KeyError` may be stored where a `LookupError` is
  declared; `NotImplementedError` and `AttributeError` are new (EXC-01,
  EXC-02, EXC-10).
- `str`, `repr`, `print` and f-strings of an exception give CPython's text,
  computed from its arguments; `repr` of a `UnicodeDecodeError` shows its
  five arguments (EXC-08).
- Exception classes of your own: a class derived from a built-in
  exception class, or from such a class defined before it, with
  attributes, methods and its own `__init__`, `__str__` and `__repr__`.
  Calling it keeps the arguments passed by position as the exception's
  arguments, and `super().__init__(...)` in `__init__` calls the base's.
  `except` catches it by any of its bases, and `is` compares exceptions
  (CLS-03, CLS-13, CLS-14, EXP-08).
- Assigning a list to a slice of a list, `xs[a:b:c] = items`, with
  CPython's results: a simple slice grows or shrinks the list, and an
  extended slice takes exactly as many items or raises `ValueError`
  (DAT-13).
- `del` of list items, list slices with or without a step, and dict keys,
  with CPython's errors; E0421 for `del` of a variable, an attribute or
  part of a tuple, `str` or `bytes` (SMT-05).
- `is` and `is not` between `bool` and `bool | None` values, such as
  `done is True` or `flag is False`. `True`, `False` and `None` are each a
  single object, as in CPython, so the result is that of `==` (EXP-08).
- `dict.keys()` and `dict.items()` as sets: `|`, `&`, `-` and `^` with a
  set, another view or a list give a new set in CPython's order, and `==`,
  `<` and the other comparisons compare their items. `dict.values()`
  compares by identity, as in CPython (DAT-16).
- `dict(name=value, ...)`, with or without a source, and
  `dict.fromkeys(keys)` and `dict.fromkeys(keys, value)`, with CPython's
  order of evaluation (DAT-16).
- `reversed` of a dict, of `dict.keys()`, `dict.values()` and
  `dict.items()`, from the last entry to the first; a dict that changes
  size raises `RuntimeError`, as in CPython (DAT-16).
- `range` as a value and an annotation: `len`, indexing, slicing, `in`,
  `==`, use as a dict key or set item, and CPython's text. `for`,
  `reversed`, `iter` and `next` read it, and its bounds may be of any size
  (DAT-17).
- `with`, with one or more context managers and `as` targets, calling
  `__exit__` on every way out of the body; a true result of `__exit__`
  suppresses the exception. The parameters of `__exit__` may be left
  without annotations, and its traceback is an object that holds no
  frames (CST-05, CST-10).
- E0420 for `with` on a value whose class has no `__enter__` and
  `__exit__` (CST-05).
- `for` over an instance of a class that defines `__iter__` and
  `__next__`, which ends when `__next__` raises `StopIteration` (CST-08).
- `iter()` and `next()`, with or without a default, for lists, tuples,
  `str`, `bytes`, sets, dicts and views of dicts, and for classes with
  `__iter__` and `__next__`, with CPython's iterator behaviour: a list
  iterator sees appended items, and a set or dict that changes size raises
  `RuntimeError` (CST-09).
- `assert test` and `assert test, message`, which raise `AssertionError`
  when the test is false in every build mode, evaluating the message only
  then; what the test shows holds after it (SMT-04).
- `finally`, which runs on every way out of a `try`: falling through,
  `return`, `break`, `continue` and an exception, which keeps going after
  it. E0419 refuses a `return`, `break` or `continue` that would leave a
  `finally` block, as CPython 3.14 warns about it (CST-04, EXC-06).
- `try` with `except` clauses and `else`: `except C`, `except (A, B)`,
  `except C as name` and a bare `except`. A clause catches its classes and
  every class derived from them, the name after `as` is deleted when the
  clause ends, and a bare `raise` inside a clause re-raises the exception.
  An exception raised while another is handled keeps it as its context,
  and an uncaught one prints it first (CST-04, EXC-01, EXC-04, EXC-07,
  EXC-09).
- E0418 for an `except` clause that names something other than exception
  classes (EXC-09).
- E0415 for `raise` of a value that is not an exception, E0416 for a bare
  `raise` outside an `except` clause, and E0417 for a call of
  `UnicodeDecodeError` or `UnicodeEncodeError` (EXC-07, EXC-12).

### Fixed

- A list, set or dict display, a comprehension, or a `list`, `set`,
  `tuple` or `dict` call is accepted where an optional collection is
  expected, such as `items: list[int] | None = []` or an argument of type
  `list[int] | None`; it stopped with E9001 or E0410.
- A function that returns a value on both branches of an `if` and `else`,
  with nothing after them, compiles; it stopped with E9001.

## [0.2.0] - 2026-11-10

### Added

- Tuples: displays such as `(1, "a")`, annotations such as
  `tuple[int, str]` and `tuple[()]`, items read at a literal position such
  as `pair[0]` or `pair[-1]`, `+`, `len`, comparisons item by item, truth
  tests, `str` and `repr`, and tuples as arguments and return values
  (DAT-08, TYP-06).
- Unpacking assignment into names and nested tuples of names, such as
  `a, b = b, a` and `x, (y, z) = t` (SMT-02).
- Lists of `int`, `float`, `bool`, `str` and lists: displays such as
  `[1, 2]`, annotations such as `list[str]`, an empty `[]` where an
  annotation, a return type or a parameter gives its type, items read and
  assigned at positions counted from either end with `IndexError` outside,
  `append`, `len`, `in` and `not in`, `==`, truth tests, `str` and `repr`,
  and `for` over a list. Two names bound to one list share it (DAT-07,
  DAT-11, TYP-05, EXP-07, CST-03). A list stores each item in eight bytes,
  so a `list[float]` holds raw doubles (MEM-02), and a million `int` items
  fit in 8.5 MB (NFR-13).
- Slices of strings and lists, such as `text[1:4]`, `xs[::-1]` and
  `xs[-2:]`, with Python's defaults and clamping, and `ValueError` for a
  zero step (DAT-12).
- The list methods `pop`, `insert`, `extend`, `remove`, `index`, `count`,
  `clear`, `copy` and `reverse`, with CPython's errors (LIB-03).
- `+` and `*` on lists, ordering of lists with `<`, `<=`, `>` and `>=`,
  `xs += ys` and `xs *= n` changing the list in place, and augmented
  assignment of a list item such as `counts[i] += 1` (EXP-02, EXP-06,
  SMT-01).
- An empty `[]` in a comparison takes the type of the other operand, so
  `xs == []` needs no annotation, and one inside a display takes the type
  of the other items, as in `[[1], []]` (TYP-05).
- `sorted`, with `reverse`, `list.sort`, `sum`, with an exact result for
  `int` items and CPython's compensated result for `float` items, `any`,
  `all`, and `min` and `max` of a list, with `default` (LIB-02, LIB-03).
  `sum` of a `list[float]` takes a start such as `0.0`, because CPython
  returns the `int` 0 for an empty list without one. The `key` argument
  arrives with function values in 1.0 (FUN-05).
- Lists of tuples and of `None`, such as `list[tuple[int, str]]`, with every
  list operation. A tuple is stored inline in the list, so a million pairs
  of `int` values take 16 MB (DAT-07, MEM-02).
- `list()`, `list(xs)`, and `list` of a `str`, `enumerate`, `zip` or
  `reversed`; `sorted`, `min`, `max`, `sum`, `any` and `all` take those
  sources too (LIB-02).
- `in` on a `str`, a substring test, and on a tuple, and slices of a tuple
  with literal bounds such as `pair[1:]` (EXP-07, DAT-12).
- `tuple[T, ...]`, a tuple of any length, with `tuple(source)`, indexing,
  slicing, `in`, `len`, `+`, `*`, comparisons, `count`, `index` and `for`.
  A tuple such as `(1, 2)` is accepted where a `tuple[int, ...]` is
  expected, and `for` goes over a tuple whose items have one type (DAT-14,
  CST-03).
- `bytes`: literals such as `b"a\x00"`, `bytes()`, `bytes(n)` and
  `bytes(values)`, items as `int` values, slices, `len`, `+`, `*`,
  comparisons, `in`, `for`, and `str` and `repr` as CPython writes them
  (DAT-06).
- The methods of `bytes` that `str` has too, `find` to `join`, `strip`,
  `replace`, padding, `upper`, `lower`, `swapcase`, `capitalize`, `title`
  and the `is` tests, with CPython's ASCII meaning, and `bytes.hex`
  (LIB-03).
- `str.encode`, `bytes.decode` and `bytes(text, encoding, errors)` with the
  codecs `utf-8`, `ascii` and `latin-1` under all their CPython names, and
  the error handlers `strict`, `ignore`, `replace`, `backslashreplace`,
  `xmlcharrefreplace` and `surrogateescape`, raising `UnicodeEncodeError`
  and `UnicodeDecodeError` with CPython's messages (LIB-03, EXC-02).
- The `str` methods `find`, `rfind`, `index`, `rindex`, `count`,
  `startswith`, `endswith`, `split`, `rsplit`, `splitlines`, `partition`,
  `rpartition`, `join`, `strip`, `lstrip`, `rstrip`, `removeprefix`,
  `removesuffix`, `replace`, `center`, `ljust`, `rjust`, `zfill`,
  `expandtabs` and `isascii`, with arguments by keyword where CPython allows
  them (LIB-03).
- The `str` methods `upper`, `lower`, `casefold`, `swapcase`, `capitalize`
  and `title`, and `isalpha`, `isalnum`, `isdecimal`, `isdigit`,
  `isnumeric`, `isspace`, `isupper`, `islower`, `istitle`, `isidentifier`
  and `isprintable`, from CPython 3.15's Unicode 17.0 data, with CPython's
  final sigma and its title case of words such as `they're` (LIB-03,
  OI-15).
- `round`, with and without `ndigits`, rounding half to even on the exact
  value as CPython does, so `round(2.675, 2)` is `2.67` (LIB-02).
- `for` over the characters of a `str`, and over `enumerate`, with
  `start`, `zip`, with `strict`, and `reversed` of lists and strings,
  nested as in `enumerate(zip(a, b))`, with CPython's errors (CST-03,
  LIB-02).
- Sets of numbers, `bool`, `None`, `str`, `bytes` and tuples of them:
  displays such as `{1, 2}`, annotations such as `set[int]`, `set()` and
  `set(source)`, `in`, `len`, truth tests, `str` and `repr`, `for`,
  `|`, `&`, `-` and `^` and their in-place forms, `==` and the subset
  comparisons, and the methods `add`, `remove`, `discard`, `pop`,
  `clear`, `copy`, `union`, `intersection`, `difference`,
  `symmetric_difference`, `update`, the `_update` methods, `issubset`,
  `issuperset` and `isdisjoint`, which take any iterable of the items, and
  several where Python takes several (DAT-10, DAT-15, LIB-03). A
  set hashes its items as CPython does and iterates and prints them in
  CPython's order (ADR-0010). A list or a set as an item of a set is
  rejected, since it has no hash. `remove` of a missing item and `pop` of
  an empty set raise `KeyError`, and a set that changes size while a loop
  reads it raises `RuntimeError`.
- `set()` in a comparison takes the type of the other operand, as `[]`
  does, so `s == set()` needs no annotation (TYP-05).
- Dicts: displays such as `{"a": 1}`, annotations such as
  `dict[str, int]`, `{}` and `dict()` where an annotation or the other
  operand of a comparison gives the type, `dict(source)` from a dict or
  from `(key, value)` pairs, `d[k]` with `KeyError` for a missing key,
  `d[k] = v` and `d[k] += v`, `in`, `len`, truth tests, `str` and `repr`,
  `for` over a dict, `|`, `|=`, `==` and `!=`, the views `keys()`,
  `values()` and `items()`, which follow the dict as it changes, and the
  methods `get` and `setdefault` with a default, `pop`, `popitem`,
  `update`, `clear` and `copy` (DAT-09, LIB-03). A dict keeps insertion
  order, leaves holes where entries were removed until it is resized, and
  raises CPython's `RuntimeError` when it changes while a loop reads it.
  Sets take a dict wherever they take an iterable, with CPython's steps
  for a dict (DAT-15).
- List, set and dict comprehensions, with several `for` clauses and `if`
  conditions, each in its own scope, so a comprehension variable does not
  leak and the first source is read in the enclosing scope, as in CPython
  (EXP-12).
- Optional values: annotations such as `int | None` and `None | str`,
  `None` or a value where one is expected, in variables, parameters,
  defaults, returns, lists, sets, dicts and tuples, `is None` and
  `is not None`, truth tests, `str` and `repr`, and `dict.get(key)` without
  a default (TYP-06). A variable narrows to its value where an
  `is not None` test, an `is None` test that failed, a truth test, or an
  assignment shows it is not `None`, and using it unnarrowed is rejected
  with a hint (TYP-13). `None` costs no memory: it is held in a value the
  type never takes (ADR-0011).
- A display with `None` among its items, and a conditional expression with a
  `None` branch, have the optional type, such as `list[int | None]`.
- `range(...)` and `reversed(range(...))` wherever an iterable is read: in
  comprehensions, `list`, `set`, `dict`, `sum`, `sorted`, `enumerate`,
  `zip` and the set methods, as well as in `for` (CST-03).
- Classes: `class` statements with `__init__`, methods, attributes declared
  in the class body or assigned in `__init__`, and instances in variables,
  parameters, returns, lists, sets, dicts and tuples (CLS-01). Assigning or
  calling a name the class does not have is rejected (CLS-02), and so is
  any use of `self` in `__init__` before every attribute is assigned.
  `__str__` and `__repr__` write instances for `print`, `str`, `repr` and
  containers, and a class without them is written as CPython writes it
  (CLS-08). Instances are equal only to themselves, hashed by identity and
  compared with `is`. A signature names a class defined below it, or its
  own class, in quotes, such as `"Node | None"`.
- Every type has a type object whose bases end at `object`, the base of
  each class, ready for inheritance and `isinstance` in 0.4 (CLS-10,
  ADR-0012).
- `type(x)` gives the type object of a value, and class names and the
  built-in type names, such as `int` and `dict`, are type objects as values.
  A type object prints as `<class 'int'>` or `<class '__main__.Point'>`,
  has `__name__`, compares with `is` and `==`, and can be a dict key; the
  annotations `type` and `type[int]` hold one (CLS-12).
- f-strings, with `!s`, `!r` and `!a`, `{x = }`, nested fields in a format
  specification, and the whole format specification mini-language for
  `int`, `bool`, `float` and `str`: fill, alignment, sign, `z`, `#`, `0`,
  width, `,` and `_` grouping, precision and the types `b`, `c`, `d`, `o`,
  `x`, `X`, `n`, `e`, `E`, `f`, `F`, `g`, `G`, `%` and `s`. Every result
  and error message matches CPython 3.12 to 3.15 on a table of 4,628
  cases, and `float` digits are rounded exactly (LEX-07).
- Passing a variable or a constant as an argument no longer counts a
  reference, and `return name` moves the variable's value to the caller
  (MEM-04). An argument read from a container is kept alive for the call,
  so a callee that empties the container still sees it, as in CPython.
- User documentation in `docs/user`: getting started, a language guide with
  one page per area of the specification, and a troubleshooting page for
  every diagnostic code. Every Python sample is checked by the test suite,
  and an accepted one must print what CPython prints (DOC-01 to DOC-04).

### Changed

- `%` formatting of strings is planned for 0.3 (LIB-10), and `iter` and
  `next` for 0.3 (CST-09); an f-string formats a value now. An annotation
  of a generic type other than `list`, `dict`, `set`, `tuple` and `type`,
  such as `frozenset[int]`, is reported as not provided.
- The runtime's size budget (NFR-02) is measured on the runtime a hello
  world links, 10.3 KB, rather than on the whole runtime archive, so the
  runtime can hold the built-in collections without enlarging programs
  that do not use them (ADR-0009).

### Fixed

- A program function named `range` is called by `for i in range(...)`,
  as in CPython, instead of the builtin (CST-02).
- A traceback shows an entry repeated in a row at most three times, then
  `[Previous line repeated N more times]`, as CPython does, so a
  `RecursionError` writes a few lines instead of one for each frame
  (STD-05).

## [0.1.0] - 2026-11-07

The first release. TyPy 0.1 compiles programs made of module-level
functions and statements over `int`, `float`, `str`, `bool` and `None` to
native executables for x86_64 Linux, with glibc or statically with musl.
Each program it accepts prints what CPython 3.12 to 3.15 prints. The
compiler needs glibc 2.39 or later and a C toolchain.

### Added

- Engineering documents: research, requirements specification, use case
  and domain models, architecture, decision records ADR-0001 to ADR-0008,
  detailed design for 0.1, test plan and development guide.
- A development container with Rust 1.99.1, LLVM 23, musl, CPython 3.12 to
  3.15 for conformance testing, and the documentation tools.
- The `typy-diagnostics` crate: source spans, diagnostic codes, the code
  registry and text rendering, with unit and property tests.
- The `typy-source` crate: reading source files with the 16 MiB and UTF-8
  checks, line and column positions, parsing with the program's Python
  floor, and TyPy's own syntax tree with its text dump.
- The `typy-check` crate: the 0.1 feature table, name resolution with
  Python's scoping, type inference within a function, definite assignment,
  and the typing rules for the operators and builtins of 0.1. Every
  rejection names its rule.
- The `typy-tyir` crate: TyIR's types, a function builder, a verifier that
  checks definitions by dominance and the types of every operation, and the
  text form for `--emit=tyir`.
- The `typy-lower` crate: lowers a checked program to TyIR with reference
  counting, error returns through `invoke`, and the semantics of CPython for
  loops, `and`, `or`, comparison chains and the builtins of 0.1.
- The `typy-codegen` crate: turns TyIR into LLVM 23 IR, assembly and object
  code, with static immortal constants and traceback entries on raising
  calls; `--release-small` marks functions `minsize`, as LLVM 23 has no `Oz`.
- The `typy-runtime` crate, without the standard library: object headers,
  reference counting, `str` objects and their operations, exceptions with
  tracebacks and the uncaught-exception report, buffered output, the stack
  check and `main`. `make runtime` builds the archive programs link.
- Unlimited-size `int` in the runtime: arithmetic, `//`, `%` and `**` with
  Python's floor rules, bitwise operators and shifts, comparisons, and
  conversion to and from `str` with `int()`'s parsing rules and CPython's
  4300-digit limit. Error messages match CPython's.
- `float` in the runtime: `/`, `//`, `%`, `**` and `abs` with CPython's
  special cases; `int / int`, `float(int)` and `float(str)` rounded once to
  the nearest double; exact `int(float)` and exact comparison of `int` with
  `float`; and `repr` with CPython's shortest digits and layout.
- Every division by zero raises `ZeroDivisionError: division by zero`, the
  wording of CPython 3.14 and later (OI-14).
- `repr` of a string escapes exactly the code points CPython 3.15 escapes,
  from Unicode 17.0 tables generated from CPython's own data (OI-15).
- `chr` and `ord` in the runtime, with CPython's messages.
- `int()` and `float()` read decimal digits of every script, such as
  `int("\u0661\u0662")`, which is `12`.
- The `typy-build` crate: build requests, modes and `--emit` stages, and
  the link of an object file with the runtime archive through `cc` or
  `musl-gcc`. The executable appears whole or not at all; a missing
  toolchain program is E0701, a failed link E0702, and a missing runtime
  archive, or one from another TyPy version, E0703.
- Diagnostics without a source location, such as a failed link.
- The `typy` command: `typy check FILE`, `typy build FILE` with `--emit`,
  `-o`, `--release` and `--release-small`, and `typy run FILE -- ARGS`,
  which keeps each build in a cache under `TYPY_HOME` and runs it again
  without compiling. Exit status 0 on success, 1 for program errors, 2 for
  wrong usage and 101 for an internal error, which a panic also becomes.
- The `typy-conformance` crate and a conformance suite of 89 programs in
  `tests/conformance/`: each accepted program runs under CPython 3.12 to
  3.15 and under TyPy in debug and release mode, with the same output,
  exit status and last line of standard error; each rejected program gets
  exactly its diagnostic code at its line and column. `make test` runs it.
- Every accepted conformance program also runs linked with AddressSanitizer
  and LeakSanitizer and must show no report. A program that ends with an
  uncaught exception frees the exception before it exits, so it leaks
  nothing.
- The sanitizer run uses a runtime built with AddressSanitizer, so every
  memory access the runtime makes is checked. `make miri` runs the
  runtime's unit tests under Miri, and `make fuzz` fuzzes the source,
  checking and lowering stages.
- Continuous integration: the quality gate and the release archive on
  every pull request, Miri and 30 minutes of fuzzing on demand before a
  release, code scanning, dependency review, and monthly grouped Dependabot
  updates. A tag `vX.Y.Z` releases the
  version after a maintainer approves it.
- `make dist` builds a reproducible release archive of the compiler and its
  runtime archives, and runs a program with it unpacked.
- `make check` measures branch coverage as well as line coverage and fails
  below 95 percent of either (NFR-11); `make coverage-branches` runs that
  measurement alone, and `make test-int`, `make test-e2e` and
  `make test-all` run one test level each.
- Golden tests of every text stage of `--emit` and of rendered
  diagnostics in `tests/golden/`; `make bless` rewrites them.
- End-to-end tests for a missing or failing C toolchain, an empty program,
  the 16 MiB limit, a file that is not UTF-8, two builds at once, argument
  forwarding by `typy run`, and environment variables that must not change
  the output.
- The stack check reads the stack pointer register, so `RecursionError`
  is raised in time also when locals do not live on the machine stack.
- The runtime archive is built for musl as well as glibc, and `make check`
  checks its size: 40.3 KB, and 29.8 KB without large integers (NFR-02). A
  hello world built with `--release-small` and linked statically with musl
  is 75 KB (NFR-01).
- E0106 when an output file cannot be written; the linker reports a
  directory it cannot write with E0106 instead of E0702.
- The `typy-driver` crate: runs loading, checking, lowering, code
  generation and linking in order, and stops after the stage `--emit`
  names. `--emit=typed-ast` writes every node with its position and type,
  and `--emit=tyir` leaves out the `inc` and `dec` that `--emit=tyir-mem`
  shows.
- A quality gate, `make check`, that checks formatting, runs Clippy and the
  tests with at least 95 percent line coverage, lints the documents, checks for
  non-ASCII text, checks that every requirement has a row in the test plan,
  and checks that every diagram is rendered from its source.

[Unreleased]: https://github.com/typy-lang/typy/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/typy-lang/typy/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/typy-lang/typy/releases/tag/v0.1.0
