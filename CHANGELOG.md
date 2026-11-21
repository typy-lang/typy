# Changelog

All notable changes to TyPy are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and TyPy uses
[semantic versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `Sequence`, `MutableSequence`, `Mapping`, `MutableMapping`, `Set`,
  `MutableSet`, `Collection`, `Container` and `Reversible` of
  `collections.abc` and `typing`, and `typing.AbstractSet`, as types that
  hold the built-in collections CPython registers with them, such as a
  list, a tuple, bytes or a range for `Sequence[int]` (LIB-20). A loop and
  the built-ins of one iterable go over a union of built-in iterables of
  one item type.
- `issubclass(class, classes)` with a class or a tuple of classes, named or
  held in a value, answering as CPython does; a value that is no class is
  refused with CPython's `TypeError` message (CLS-19).
- `isinstance` takes a class held in a value, such as `isinstance(shape,
  kind)` with `kind = type(other)` or a parameter of type `type[Square]`,
  alone or in a tuple with named classes, tested when the program runs; a
  class whose type names one narrows the value to it (CLS-18).
- Decorators on functions at the top of a module or nested in a function:
  any expression that takes the function, evaluated from the top down and
  applied from the bottom up when the `def` runs, the name holding what the
  outermost returns, also inside the function; generic decorators too
  (FUN-06). A call through a decorated name passes its arguments by
  position.
- A nested function, a `lambda` or a generator expression inside a generic
  function, or inside a function with a protocol parameter, is accepted and
  made in each copy of the function, with Python's `__qualname__`, such as
  `pair_with.<locals>.make` (FUN-10). A copy of a generic function or class
  shows its name without type arguments.
- A built-in function is a value where a `Callable` is expected, such as
  `sorted(words, key=len)`, `max(rows, key=len)` or `measure:
  Callable[[str], int] = len`, written `<built-in function len>`, of the
  type `builtin_function_or_method`. `chr` and `ord` are values with no
  `Callable` expected; any other built-in function needs one, E0203
  (FUN-08). Two values of one built-in are one function, also where they
  fill different `Callable` types: `is`, `==` and `hash` treat them as one,
  as CPython does (ADR-0076).
- A method of a built-in type read through the type is a value where a
  `Callable` is expected, such as `sorted(words, key=str.lower)` or `add:
  Callable[[list[int], int], None] = list.append`, written `<method 'lower'
  of 'str' objects>`, of the type `method_descriptor` (FUN-08).
- A function of `math` or `sys` is a value where a `Callable` is expected,
  such as `sorted(values, key=math.fabs)` or `root: Callable[[float],
  float] = math.sqrt`, written `<built-in function sqrt>`; with no
  `Callable` expected it is E0203, where it was planned (FUN-08).
- A method read through a built-in value is a value where a `Callable` is
  expected, such as `add: Callable[[str], None] = words.append` or
  `key="ab".index`, bound to that value and written `<built-in method
  append of list object at 0x...>`; with no `Callable` expected it is
  E0203, where it was E0412 (FUN-08).
- A built-in type whose call TyPy types, such as `int`, `str` or `list`, is a
  function where a `Callable` is expected, such as `sorted(numbers,
  key=str)` or `parse: Callable[[str], int] = int`, written `<class 'int'>`
  (FUN-14). A class of the program there is refused as planned for 0.14
  (FUN-16), where it was E0402 or E0404.
- A function of the module with defaults, `*args` or `**kwargs` is a value
  where a `Callable` its parameters take is expected, such as
  `apply(total, 2, 3)` with `def total(*values: int)`, or `twice:
  Callable[[int], int] = scale` with a default for its second parameter;
  it is the same function to `is` and `==`. Read where no `Callable` is
  expected it is E0203 (FUN-13).
- A class method read as a value, such as `make = Box.make` or `box.make`,
  is a bound method of the class it is read through, or of the instance's
  own class, as CPython binds `cls`: `box.make` of a `BigBox` makes a
  `BigBox` (FUN-09). A static method read through an instance whose
  subclasses define it again is the function of the instance's own class.
- `callable(x)` is true for a function value, a built-in function, method
  or type, and a class, and false for any other value, as CPython answers
  (LIB-23).
- Each release archive holds `sbom.cdx.json`, a CycloneDX bill of materials
  of its files, with their SHA-256, and of the crates and libraries the
  compiler holds, with their licences; a copy is published beside the
  archive (NFR-25).
- Each release archive is attested with its build provenance, signed
  through Sigstore without a key the project keeps; `gh attestation verify
  ARCHIVE --repo typy-lang/typy` checks it (NFR-26).

### Changed

- `from typing import ...` and `from collections.abc import ...` report
  every name a later release gives, not only the first. `typing.AbstractSet`
  is refused as planned for 0.10 (LIB-20), where it was "not given"
  (E0425), and `Sized`, `Hashable` and `MappingView` as planned for 0.14
  (LIB-24). A class derived from `typing.NamedTuple` is refused as planned
  for 0.10 (CLS-27), where the message named CLS-04.
- A decorator on a method is refused as planned for 0.14 (FUN-15), and a
  class decorator other than `@dataclass` as planned for 1.0 (CLS-38);
  0.10 brings decorators of functions (FUN-06). `map`, `filter` and
  `callable` are refused as planned for 0.10 under LIB-23, and
  `property`, `staticmethod` and `classmethod` as values as planned for
  1.0 (FUN-17). The messages named FUN-06 for every decorator and FUN-08
  for those names.
- A keyword argument a builtin does not take is refused with CPython's
  message, such as "Cannot specify a default for max() with multiple
  positional arguments" or "abs() takes no keyword arguments", where the
  message said the builtin "takes no keyword arguments in TyPy 0.1" (LIB-01).
- A name `typing`, `collections.abc` or `dataclasses` does not give is
  refused with the names that module gives; the message listed the names of
  `typing` for each and left out `Generator` and `Callable` (LIB-17).
- The messages for a builtin called with too many or too few arguments, and
  for an annotation that names no type, no longer name TyPy 0.1; the latter
  says the name is not a built-in type, a class or a type alias (STD-04).
- Generics and `ctypes` features planned for 1.0 are now planned for 0.13
  (GEN-06, GEN-11 to GEN-17, FFI-06, FFI-07, FFI-09 to FFI-11), and the
  class rules planned for 1.0 for 0.14 (CLS-28 to CLS-37). The E0201
  messages that refuse them name the new release.
- The refusal of a default value other than a constant names EXP-20, the
  requirement that plans it for 1.0, not EXP-16, which TyPy meets.

### Fixed

- A fixed tuple, such as `(1, 2)`, passed or assigned where a union with a
  `tuple[T, ...]` member is expected, such as `tuple[int, ...] | str`, is
  accepted as that member, as CPython runs it. It was refused with E0404
  or E0402 (TYP-14).
- A generic function whose parameters or result name a type parameter
  inside a `Callable` or a `Generator`, such as `def apply[T](value: T,
  change: Callable[[T], T]) -> T` or a generic generator that yields `T`,
  is accepted, and its type parameters are solved from those arguments.
  It was refused with E0404 or E0405 (GEN-01).
- `type(x)` of an instance whose class has subclasses, such as a `Circle`
  held as a `Shape`, is a class known only when the program runs. Calling
  it, or reading a class variable or a class method through it, is refused
  with E0201, planned for 0.14 (CLS-39), and passing it where a
  `type[Shape]` is expected with E0404. TyPy took it for the static class,
  so `type(shape)()` made a `Shape` and `type(shape).kind` read `Shape`'s
  variable, where CPython uses `Circle`'s. Call the class by its name, or
  pass a function that makes the object.
- A property read through its class as a value, such as `getter = Box.v`,
  is refused with E0201, planned for 1.0 (FUN-17). TyPy took it for the
  getter, so calling the value ran the getter, where CPython raises
  `TypeError: 'property' object is not callable`.
- A library called on a thread its C host created with a stack smaller than
  `RLIMIT_STACK` raises `RecursionError` for a recursion too deep for that
  stack; on Linux the process ended with a segmentation fault (FFI-15).
- A call that unpacks a tuple holding an instance of a derived class into a
  `Base | None` parameter compiles; it ended in an internal error (E9001)
  (EXP-17).
- A fixed tuple, a collection or an iterator passed where a union of
  iterables and iterators is expected, or of an iterable and another type,
  such as `Iterable[int] | int`, compiles and runs as in CPython. A fixed
  tuple aborted the program with "an object has an unknown type", and the
  others ended in an internal error (E9001) (TYP-15).
- `==` and `!=` of two exceptions compare them by identity, as CPython
  does; they ended in an internal error (E9001). An exception class that
  defines `__eq__` or `__ne__` is refused with E0201, planned for 1.0
  (EXC-20), since TyPy does not call it yet (EXP-06).
- A library whose `__all__` exports a function under the name of a C
  function the module calls through `ctypes`, such as `labs` beside
  `libc.labs`, is refused with E0429 at the name, since the export would
  take the C function's place; it ended in an internal error (E9001)
  (FFI-05).
- An f-string `{value:c}` of `-2**63`, the smallest C `long`, raises
  CPython's `OverflowError: %c arg not in range(0x110000)`, not "Python
  int too large to convert to C long", since the value fits a C `long`
  (LEX-07).
- `x += y` where the class of `x` has an `__iadd__` whose parameter does
  not take `y` is refused with E0404. TyPy called `__add__` instead, where
  CPython calls `__iadd__` whatever `y` is (CLS-20).
- An override that takes a parameter by keyword only, or by position only,
  where its base method takes it by position or by keyword is refused with
  E0404. TyPy ran a call through the base that CPython refuses with
  `TypeError` (CLS-04).

## [0.9.0] - 2026-11-20

### Added

- Generator functions: a function whose body has a `yield` statement makes
  a generator when it is called, annotated `Iterator[T]`, `Iterable[T]` or
  `Generator[T, S, R]` of `collections.abc` or `typing`. Each step, a turn
  of `for` or a call of `next()`, runs the body to its next `yield`, and
  `list()`, `sum()` and every builtin that takes an iterable take a
  generator. The generator's variables live in one allocation between
  steps, and `next()` of a running generator and a `StopIteration` its
  body raises give CPython's `ValueError` and `RuntimeError` (SMT-11,
  FUN-07, ADR-0063).
- `send`, `throw` and `close` of a `Generator[Y, S, R]`, and
  `received = yield value`, whose `yield` gives `S | None`, since
  `next()` and `for` resume a generator with `None` (ADR-0067). A value
  sent before the first step, a step after the end, and a generator that
  yields after `close()` raise CPython's `TypeError`, `StopIteration` and
  `RuntimeError` (SMT-11, FUN-07, FUN-11).
- Unpacking any iterable into targets, `first, second = items`, checked
  as the program runs with CPython 3.15's `ValueError`, and a starred
  target that takes the rest as a list, `first, *rest = items` (SMT-02,
  EXP-17).
- `*iterable` in list, set and tuple displays, and `**dict` in dict
  displays (EXP-17).
- `*args: T` and `**kwargs: T` parameters, which take the arguments left
  over as a `tuple[T, ...]` and a `dict[str, T]`, after every argument of
  the call is evaluated in source order, as in CPython (EXP-17). A
  function, nested function or method with them as a value, and a
  `lambda` with them, are planned for 0.10 (FUN-13).
- `Generator` of `typing` or `collections.abc` as the class of
  `isinstance`, and of `collections.abc` in a class pattern, and a value
  held as an `Iterable[T]` tested against `Iterator`, answered by the
  object it holds; a true test reads it as an `Iterator[T]` (TYP-21).
- A class whose `__next__` returns `T` and whose `__iter__` returns the
  instance is an `Iterator[T]`, and a class whose `__iter__` returns an
  iterator is an `Iterable[T]`, as `typing` reads the protocols: its
  instances are passed and held where those are expected, and a loop,
  `next()` and every builtin that reads an iterable call its methods as
  CPython does (TYP-20, ADR-0065).
- `*iterable` and `**dict` in a call of the program's functions, methods
  and classes, and `*iterable` in `print`. A tuple of a fixed number of
  items binds as its items would; anything else binds when the call runs,
  with CPython 3.15's `TypeError` for a call that does not fit (EXP-17,
  ADR-0073). Unpacking in a call of another builtin, of a function value,
  of a protocol method or of an exception class is planned for 0.10
  (EXP-19).
- Generator expressions, `(element for target in source if condition)`:
  a `Generator[Y, None, None]` whose first iterable runs where the
  expression runs and the rest as it runs, reading the variables around it
  as a nested function does (EXP-15). An assignment expression inside one
  is planned for 1.0 (EXP-18).
- `yield from` any iterable, passing on `send`, `throw` and `close()` to a
  generator it delegates to, and `return value` in a generator annotated
  `Generator[Y, S, R]`: `StopIteration(value)` ends it, and `result =
  yield from source` gets the value (SMT-11, FUN-07, ADR-0072).
- A generator freed while it is suspended is closed, as CPython closes
  it: `break` out of a loop over it, the end of a function that holds it
  and a container that drops it run its `finally` blocks and the
  `__exit__` of its `with` statements, in CPython's order. An exception
  raised meanwhile is written to standard error after `Exception ignored
  in:`, and the program goes on (FUN-11, ADR-0069). A generator in a
  variable of a function that an exception leaves closes when the
  exception goes, as CPython's traceback keeps it (ADR-0070), and one in a
  module variable closes when the program ends (ADR-0071).
- A `yield` inside an `except` clause or a `finally` block. The exception
  a suspended generator handles stays with it, so an exception its caller
  raises meanwhile has the caller's own context, as in CPython (ADR-0068).

### Fixed

- A class pattern of `Iterator` over a union that holds an iterator, such
  as `list[int] | Iterator[int]`, matches the iterator, where TyPy
  reported that it never matches.
- A generator or an iterator passed where a union holding an
  `Iterable[T]` is expected no longer stops the compiler with an internal
  error.
- `bool()` of an iterator, and an iterator as the condition of `if` or
  `while`, no longer stop the compiler with an internal error; an
  iterator is true, as in CPython.

### Changed

- A class pattern that names `Iterable`, `Iterator`, `Generator`, `List`,
  `Dict`, `Set`, `Tuple`, `Type`, `Optional` or `Union` of `typing` is
  E0401, since CPython raises `TypeError` for it; name the class of
  `collections.abc` or the built-in type instead.
- `Sequence`, `Mapping` and the other abstract collections of
  `collections.abc`, decorators, built-in functions, class methods and
  protocol methods as values, nested functions inside generic functions,
  `NamedTuple`, `isinstance` with a class held in a variable,
  `issubclass`, and `typing` names as values are given as planned for
  0.10, where TyPy 0.8 named 0.9.

## [0.8.0] - 2026-11-19

### Added

- Positional-only parameters, before `/`, and keyword-only parameters,
  after `*`, in functions and methods. A call binds its arguments as
  CPython does, and a keyword for a positional-only parameter, or a
  positional argument past the others, is E0404 where CPython raises
  `TypeError` (FUN-03).
- Functions as values. A function of the program read without a call is a
  value of type `Callable[[A, ...], R]`, from `typing` or
  `collections.abc`, which a variable, a list or dict item, an attribute
  or an argument holds, and which is called with one argument for each
  parameter, by position. Function values compare and hash by identity,
  are true, and write `<function name at 0x...>`, as in CPython. A function
  fits a `Callable` only when nothing is converted through the call, so a
  function of any `Shape` fits a `Callable[[Circle], None]`, and a function
  of a `float` does not fit a `Callable[[int], float]` (FUN-05, NFR-19,
  ADR-0060). `Callable[..., R]` is E0408, and a generic function or one
  with a keyword-only parameter used as a value is E0202.
- Methods as values. A method read through an instance is a bound method
  of the method of the object's own class, which holds the instance; two
  bound methods of one instance and one method are equal, and `repr`
  writes `<bound method Class.name of ...>`, as in CPython. A method read
  through its class, and a static method, is the function itself, and
  `item.name(...)` for a property calls the value its getter gives
  (FUN-05). Class methods, methods read through a protocol value, and a
  few overrides, are planned for 0.9 (FUN-09).
- Nested functions. A `def` inside a function makes a new function object
  each time it runs, which holds a copy of each variable of the functions
  around it that its body reads, when one assignment before the `def`
  gives that variable its value. Any other captured variable, such as a
  loop variable or one assigned after the `def`, lives in a cell the
  functions share, so functions made in a loop read its last value, and
  one read before it is assigned raises `NameError`, as in CPython. A
  nested function calls itself and the other nested functions of its
  function by name. Nested functions that call one another, defined one
  after another, hold the same captures and never each other, so they are
  freed with their last reference (FUN-04, ADR-0061).
- `lambda` expressions where a `Callable` is expected: an annotated
  variable, a parameter, a result, an item of a typed list or dict, a
  branch of a conditional, and the `key` of `sorted`, `list.sort`, `min`
  and `max`. The parameters take the expected types, and the result the
  expected type or the body's. A `lambda` captures as a nested function
  does, so those made in a loop or a comprehension read the last value of
  its variable, as in CPython (EXP-14).
- `nonlocal` in a nested function, which assigns the variable of the
  function around it that every function sharing it sees, with CPython's
  rules for where it may stand. No test narrows such a variable, since any
  call may assign it (SMT-09).
- The `key` argument of `sorted`, `list.sort`, `min` and `max`, also of
  `min` and `max` of several values: a function value of one item, whose
  results order the items as CPython orders them. `sorted` and
  `list.sort` call it once for each item and sort stably, and `min` and
  `max` compare each key as it is computed (FUN-05).

### Fixed

- An operand read from an object, a container or a module variable stays
  alive while an operand evaluated after it runs, so `current.text +
  replace()`, where `replace` binds `current` again, adds the old text,
  as CPython does, where TyPy 0.7 read freed memory. The same holds for
  comparisons, displays, indexes and slices, the receiver and the
  arguments of a method, the arguments of a builtin and of `print`, and
  the target of an item assignment or of `del` (NFR-07).
- A call of a local variable before its first assignment, such as `x()`
  before `x = ...`, is E0306, where TyPy 0.7 accepted the program and then
  failed while compiling it.
- `except ... as name` where a function declares `name` `global` is
  E0202. CPython deletes the module variable when the clause ends, and
  TyPy 0.7 left it as it was. Name the exception with a variable of its
  own.

### Changed

- Calling a value that is not a function, such as an `int`, is E0401, as
  CPython raises `TypeError`, where TyPy 0.7 gave E0201.
- `typing.NamedTuple` and the abstract collections of `typing`, such as
  `Sequence`, are given as planned for 0.9, where TyPy 0.7 named 1.0.
- An instance and a tuple store their attributes and items by decreasing
  alignment, so a `bool` beside `int`s no longer takes 8 bytes: a class of
  two `int`s, a `float`, an object and two `bool`s takes 56 bytes, not 64,
  and fits a 64-byte allocation (MEM-11, NFR-18, ADR-0062). A program sees
  its declared order everywhere, in `repr`, comparisons, hashing, `match`
  and dataclass fields.

## [0.7.0] - 2026-11-18

### Added

- macOS 11 or later on Apple silicon: `typy` runs there and builds
  programs and libraries for `aarch64-apple-darwin`, which run on every
  Apple silicon Mac from macOS 11 (CLI-15, ADR-0055). A Mac builds for
  macOS alone. Programs link with Apple's linker; a debug build writes
  `NAME.dSYM` beside the program, and a library is named as macOS names
  libraries, `libNAME.X.dylib` with the link `libNAME.dylib`, and
  `libNAME.a` (CLI-16, ADR-0056). `sys.platform` is `"darwin"` (LIB-22),
  an `OSError` takes macOS's numbers and texts (EXC-19, ADR-0057), and a C
  call passes its arguments as Apple's arm64 convention does (FFI-13,
  ADR-0058). Each release has a third archive,
  `typy-X.Y.Z-aarch64-apple-darwin.tar.gz`, signed ad hoc and not
  notarized; [Using TyPy on macOS](https://typy-lang.org/using/macos/)
  shows how to clear the quarantine a browser download carries.
- `ctypes.CDLL(None)` and `ctypes.cdll.LoadLibrary(None)` declare the C
  library and the C maths library, which every program links, so a
  program that calls only their functions, such as `abs`, `strlen` or
  `cbrt`, builds and runs on every system, as under CPython (FFI-14,
  ADR-0059).

### Changed

- The compiler links LLVM 23.1.6 as TyPy builds it from LLVM's signed
  sources, the same bytes in every build of one commit, so a release
  rebuilt from its tag matches the published archive (ADR-0054). It no
  longer needs `libffi`, `zlib` or `zstd`: glibc 2.39 or later and
  `libstdc++` are all `typy` needs, which the release build checks.
- `--target aarch64-apple-darwin` on a Linux host is a usage error that
  says the triple is built on macOS only, and lists the Linux targets
  (CLI-08).

### Fixed

- A function a library built with `--lib` exports takes and returns a
  `bool` as clang passes a C `_Bool` on the target. On AArch64 Linux it no
  longer assumes that the C caller extended the argument, which that
  calling convention does not ask for (ADR-0058).

## [0.6.0] - 2026-11-18

### Added

- The documentation is published at <https://typy-lang.org>, built from
  the sources of each release when the release is published. It is a site
  with navigation, search, and light and dark modes, laid out in the
  sections of Python's documentation: What's new, the tutorial, the
  library and language references, setup and usage, extending and
  embedding, and diagnostics, with the engineering documents and pages
  about the site, its copyright and its licence. `make docs` builds it
  into `site/` with Zensical in strict mode, and `make check` runs it, so
  a broken link or anchor, or a page missing from the navigation, fails
  the quality gate (DOC-05, ADR-0050).
- A What's new page for each release, starting with TyPy 0.5, and the
  changelog as a page of the site.
- `typy build --target TRIPLE` builds for another target, and
  `aarch64-unknown-linux-gnu`, 64-bit Arm Linux with glibc, is one. An
  x86_64 host links with the cross toolchain named after the target's GNU
  triple, such as `aarch64-linux-gnu-gcc`. `typy check` takes `--target`
  too, and an unsupported triple is a usage error that lists the supported
  ones. A table `[target."TRIPLE"]` in `typy.toml` names other tools for a
  target: `linker`, `ar` and `objcopy` (CLI-08, CLI-14, ADR-0051).
- A `ctypes` structure passed to or returned from a C function by value
  follows AAPCS64 on AArch64: up to four `float`s or `double`s in
  floating-point registers, other structures of up to 16 bytes in general
  registers, and a larger one as the address of a copy (FFI-12, ADR-0052).
- Each release has an archive for AArch64 Linux hosts,
  `typy-X.Y.Z-aarch64-unknown-linux-gnu.tar.gz`, beside the one for x86_64
  hosts, and each holds the runtime of every target, so either builds for
  both (ADR-0051).
- On AArch64, `math.gamma` and `math.lgamma` fuse the multiply-adds that
  CPython's compiled code fuses there, so they give CPython's results,
  which differ from those on x86-64 in the last bit.

### Changed

- The user documentation is laid out in sections, as Python's
  documentation is, and its pages moved under `docs/`.
  `user/getting-started.md` is `tutorial/index.md`. `user/guide/index.md`
  is `reference/index.md`, the language reference, which holds the
  guide's pages on the language under their names. `built-in-types.md`
  and `exceptions.md` moved to `library/`, and `library.md` is
  `library/functions-and-modules.md`. `command-line.md` is
  `using/command-line.md`, `c-interop.md` is `extending/c-interop.md`,
  `diagnostics.md` is `diagnostics/index.md`, and
  `user/troubleshooting.md` is `diagnostics/troubleshooting.md`.
  `docs/index.md` is the start page of the user documentation, and the
  list of engineering documents moved to `docs/engineering.md` (DOC-02).

### Fixed

- The release archive carries `THIRD-PARTY-NOTICES.md`, with the licence
  and copyright notice of the Python Software Foundation for the parts of
  the runtime derived from CPython's source (the list sort and parts of
  `math`) and the Unicode licence for its Unicode tables. Releases 0.1.0 to
  0.5.0 shipped these parts without the notices. The runtime's package
  metadata gives its licence as
  `(MIT OR Apache-2.0) AND PSF-2.0 AND Unicode-3.0`.
- The release archive's `licenses/` directory holds the licences of the
  Rust crates and the LLVM that `typy` links and of Rust's standard
  library, which the archives of 0.1.0 to 0.5.0 lacked.

## [0.5.0] - 2026-11-17

### Added

- An instance of a class of the program that never outlives the function
  that makes it is made in the function's frame, with no allocator call:
  one the function reads, assigns attributes of, calls methods on, or
  passes to functions that do no more. `--emit=tyir` ends the line of each
  such instance with `in frame` (MEM-03, ADR-0042).
- The table `[allocator]` in `typy.toml` names the C functions a program
  allocates with, in place of `malloc`, `realloc` and `free`, and the
  library that defines them, such as mimalloc's `mi_malloc`, `mi_realloc`
  and `mi_free` from `mimalloc`. A wrong table is E0107 (MEM-08,
  ADR-0043).
- `typy build --lib static` and `--lib shared` build a module as a C
  library, `libNAME.a` or `libNAME.so`, with the C header `NAME.h`. The
  functions `__all__` lists are exported under their own names, with
  `int`, `float` and `bool` as `int64_t`, `double` and `bool`. The module
  runs when the library is loaded, and is named after its file. An
  exception that would leave an export ends the process: a `SystemExit`
  with its status, any other with its report and an abort. E0429 names a
  module or an export a library cannot have; `typy check --lib` reports it
  too (CLI-09, FFI-05, EXC-05, ADR-0044).
- A library's files carry the release of its version, from the module's
  `__version__` or else `version` in `typy.toml`, as Linux names
  libraries: `libNAME.so.5.0` with the soname `libNAME.so.5` and links
  `libNAME.so.5` and `libNAME.so`, and `libNAME.5.0.a` with a link
  `libNAME.a` (CLI-13, ADR-0045).
- A library build also writes `NAME_binding.py`, a module that loads the
  library with `ctypes` by its soname and defines a typed function for
  each export, and its stub `NAME_binding.pyi`, so a Python program
  imports the binding and calls the library (FFI-08, ADR-0046).
- `ctypes`: a library a module declares with `ctypes.CDLL("name")` or
  `ctypes.cdll.LoadLibrary("name")` is linked when the program is built,
  and a C function whose `argtypes` and `restype` are assigned at module
  level is called directly, with every integer type from `c_int8` to
  `c_uint64`, `c_float`, `c_double`, `c_bool`, `c_char_p` and `c_void_p`
  converted as `ctypes` converts them, also `ctypes.ArgumentError` for an
  `int` too large for a `float`. E0430 names a declaration TyPy cannot
  compile (FFI-01 to FFI-03, ADR-0047).
- `ctypes` objects, pointers and arrays: `c_int(5)` and every scalar type
  of `ctypes` make an object whose `value` a program reads and assigns and
  a C function writes through `byref(x)` or a `POINTER(T)` parameter;
  `pointer(x)`, `POINTER(T)()`, `contents` and indexes read and write
  through a pointer, which a C function may also return; `(T * n)(...)`
  makes an array with `len`, checked indexes, iteration and `in`; and
  `sizeof` gives a class's size, each as in CPython. `argtypes` and
  `restype` take `POINTER(T)` and names bound to a type (FFI-03,
  ADR-0048).
- `ctypes.Structure` subclasses with a literal `_fields_` list, laid out as
  C lays out a struct: fields read and assign as in CPython, a structure,
  array or pointer field reads as an object over its memory, structures
  nest, fill arrays and sit behind pointers, and a C function takes and
  returns a structure by value as the x86-64 System V calling convention
  passes it, in registers or in memory (FFI-03, ADR-0049).
- `library-paths` and `[libraries."NAME"]` with `link = "static"` or
  `"dynamic"` in `typy.toml` say where the linker looks for C libraries and
  how each is linked (FFI-04).
- A benchmark report, `docs/testing/benchmark-report.md`, which compares
  TyPy's programs with CPython 3.12 to 3.15, PyPy, mypyc, Cython, Nuitka,
  Codon, C, C++, Rust, Go, Java and JavaScript on twelve workloads: time,
  peak memory, startup, build time and size. `make bench-peers` installs the
  peers at pinned versions and `make bench-compare` runs it.
- Assignment expressions, `name := value`, with CPython's scoping in
  comprehensions and its syntax errors; a test of `(name := value) is not
  None` narrows `name` (EXP-13).
- `match` statements with literal, value, capture, wildcard, `as`, `|`,
  sequence, mapping and class patterns and guards, with CPython's syntax
  errors for patterns. A class pattern reads `__match_args__` from the
  class body or a base, and a class pattern that CPython would answer
  with `TypeError` is refused with E0401. A case narrows the subject as
  the test it stands for does, so after `case None:` an optional subject
  is no longer `None`, and `case [x, *rest]` narrows a
  `list[int] | None` to the list (CST-06).
- The warning W0401 for a `match` whose cases leave members of the
  subject's union, optional or `bool` type unmatched, naming them. A
  `match` whose cases cover every member is complete, so a function that
  returns from each case needs no `return` after it (CST-07, ADR-0033).
- `@dataclass` writes `__match_args__`, the names of the fields, and
  takes `match_args=False` to leave it out (CLS-05, CST-06).
- The names `Optional`, `Union`, `List`, `Dict`, `Set`, `Tuple` and `Type`
  of `typing` in annotations, as the types they spell (LIB-17).
- `Iterable` and `Iterator` of `typing` and `collections.abc`, and
  `List`, `Dict`, `Set`, `Tuple`, `Type`, `Optional[...]` and
  `Union[...]` of `typing`, as the class of `isinstance` and of a class
  pattern, with CPython's answers (TYP-17).
- A loop, a comprehension, `list`, `sorted`, `sum` and the other builtins
  that read an iterable go over a variable that holds a union of fixed
  tuples, such as `tuple[str] | tuple[str, int]` (TYP-14, ADR-0034).
- `args` of an exception and `code` of a `SystemExit` are values of
  their own: stored, passed, indexed, unpacked and looped over. The type
  of `args` comes from every creation of the class and of the classes
  derived from it, in the program and in TyPy's own raises, such as
  `tuple[str, int] | tuple[str]`; `code` is `None`, the one argument, or
  the tuple. The five items of a Unicode error join only in a program
  that encodes, decodes or opens a file (EXC-15, ADR-0034).
- A class may define the in-place methods, such as `__iadd__`, which
  `x += y` calls, falling back to `__add__`; an augmented assignment that
  calls an operator method takes an item or an attribute of any
  expression as its target, evaluated once (CLS-20, CLS-32).
- Assigning a class variable through a subclass that inherits it gives
  the subclass a variable of its own, which it, its subclasses and their
  instances read from then on, as CPython does; through a variable such
  as `cls` it is planned for 1.0 (CLS-25, CLS-31, ADR-0037).
- A class method may be called through an instance, which passes the
  instance's class as `cls`, and through a subclass defined in another
  module than the class that defines it, which gets a copy of its own.
  Through an instance of a generic class it is planned for 1.0 (CLS-25,
  CLS-30, ADR-0037).
- A static method may be called through any expression that gives an
  instance, such as `make().helper()`, which is evaluated first, as
  CPython does (CLS-25).
- `super(Base, self)` takes any class of the enclosing class's chain of
  bases, and finds the method of the nearest base of `Base`, as CPython
  does; with another instance it is planned for 1.0 (CLS-26, CLS-29).
- Dataclasses take `field(default_factory=f)`, called for each instance
  that is not given the field, hash as the tuple of their fields when
  frozen or given `unsafe_hash=True`, and take the options `init`,
  `repr`, `unsafe_hash` and `slots`, as CPython's `dataclasses` does.
  Other options of `field` and `@dataclass` are planned for 1.0
  (CLS-15, CLS-16, CLS-17, CLS-28).
- `math.fsum`, `dist`, `prod` and `sumprod` take items of a union of
  number types, such as a `list[int | float]`, and `prod` a `bool` start,
  with CPython's results. `prod` of items that are not numbers is planned
  for 1.0 (LIB-15, LIB-21).
- A function may import a module that imports its own module at module
  level, the usual way to break an import cycle, as CPython allows; the
  function is checked once that module is. Inside `__init__` it is
  planned for 1.0 (MOD-13, MOD-15).
- `OSError` takes arguments of any type, more than five of them, and two
  or more through `super().__init__` or to a subclass with an `__init__`
  of its own, as CPython does: `OSError("a", "b")` is `[Errno a] b`.
  `errno`, `strerror`, `filename` and `filename2` take their types from
  every creation of the class, and an error about a file opened from a
  descriptor has the descriptor as `filename`. A number as the third
  argument of `OSError` or `BlockingIOError` is planned for 1.0 (EXC-16,
  EXC-18, ADR-0035).
- A literal index into a variable that holds a union of fixed tuples
  raises `IndexError` when the tuple it holds is too short, as CPython
  does, in place of E0307; a position outside every member is still E0307
  (DAT-08).
- `sys.version_info` as a value of its own: stored, put in containers,
  imported with `from sys import version_info` and printed as CPython
  writes it, and read as the tuple of its items by indexing, slicing,
  comparing, `len`, loops and unpacking (LIB-16).
- Type aliases of another module, by name or through the module, `type`
  statements inside functions, classes and blocks, and bounds and
  constraints on an alias's type parameters, and an alias read as a
  value, a `typing.TypeAliasType` with `__name__` and `__module__`
  (SMT-12).
- One value assigned to several targets, as in `a = items[0] = value`, or
  unpacked into targets, is held by each target as its own type, a union
  or an optional among them (TYP-16).
- A `\u` or `\U` escape of a surrogate in a `str` literal or an
  f-string gives that code point, as CPython keeps it (LEX-10).
- `NotImplemented`. An operator or comparison method that returns it
  gives way to the other operand's reflected method, and an in-place
  method to the binary methods; when every method gives way, `==` and
  `!=` compare identities and other operators raise CPython's
  `TypeError`. Lists, sets and dicts compare through an `__eq__` that
  gives way, and so do ordering methods (CLS-22); a method that also
  returns `None` is planned for 1.0 (CLS-33). `return NotImplemented`
  elsewhere is E0405 (CLS-21, ADR-0038).
- Comparison, hash and truth methods that may raise, or that print, inside
  lists, sets, dicts, `sorted`, `min`, `max`, `any` and `all`: the first
  exception leaves the operation, no later method runs, and a set or a
  dict keeps its items. An ordering method that returns `NotImplemented`
  works in a sort too (CLS-21, CLS-22, ADR-0039).
- A chain of comparisons of instances, such as `low < value < high`,
  calls each comparison's method and evaluates each operand once
  (CLS-22).
- An operator whose method and reflected method return different types;
  the operator's type joins them when the reflected method may run
  (CLS-22).
- `in` on an instance whose class has `__iter__` and no `__contains__`
  goes over its items until one is the value or equal to it, as CPython
  does; over a class with `__getitem__` alone it is planned for 1.0
  (CLS-23, CST-11).
- `slice` objects: `x[a:b:c]` on an instance passes one to `__getitem__`,
  `__setitem__` and `__delitem__`, also in an augmented assignment;
  `slice(a, b, c)` makes one; `start`, `stop`, `step` and `indices` read
  it; and `seq[s]` slices a list, `str`, `bytes`, `tuple[T, ...]` or
  `range`, also as an assignment or `del` target of a list. A slice is
  compared, hashed and written as CPython does (CLS-23).
- A slice bound may be `None` or an `int | None`, which stands for a
  missing bound, as in `items[start:]` with `start = None` (DAT-12).
- An operation on a variable of a union with an iterator, a file, a module
  or a traceback member, such as `Iterator[int] | list[int]`, tests the
  object's kind; members TyPy cannot tell apart, such as two iterators,
  are planned for 1.0 (TYP-14, TYP-19).
- The truth of an instance comes from the `__bool__`, or else the
  `__len__`, of the object's own class, also where the variable's type is
  a base with neither or a union, and in `any` and `all` (CLS-24).
- A static or class method of a generic class called through the class
  takes the type arguments from its arguments, as in `Box.wrap("a")`, or
  from the class, as in `Box[int].make(3)`, and runs that copy's method
  (GEN-01).
- A protocol as the type of a variable, an attribute, a result or a
  container item, such as `shapes: list[Shape]`, and of a parameter of
  `__init__` or another special method. The value holds an instance of
  any class that satisfies the protocol; a method call, an attribute
  read, `==`, `len` and `print` reach the object's class. A class derived
  from a protocol, a protocol's properties, static and class methods and
  class variables, assigning an attribute through a protocol value, and a
  built-in value held as a protocol are planned for 1.0 (GEN-10, GEN-17,
  ADR-0041).
- A generic method that overrides one with as many type parameters: a
  call through the base runs the copy of the object's class's method for
  the same type arguments. Other overrides of or by a generic method are
  planned for 1.0 (GEN-09, GEN-16).
- A class method with type parameters of its own, or with a parameter
  that names a protocol, called through its class. Through an instance,
  and type parameters of a special method or a property, are planned for
  1.0 (GEN-09, GEN-16).
- A method of a generic class with type parameters of its own,
  `def paired[U](self, other: U) -> tuple[T, U]`, takes the class's type
  arguments from the instance and solves its own from the arguments
  (GEN-09).
- A class derived from a copy of a generic class, `class IntBox(Box[int]):`,
  and a generic class derived from one for its own type parameters,
  `class Pair[T](Box[T]):`, whose copies derive from the matching copy.
  Class methods and dataclass fields inherited through a copy are
  planned for 1.0 (GEN-08, GEN-15, ADR-0040).
- A generic class with a base of the program, `class Box[T](Base):`:
  every copy derives from `Base`, takes its methods and may stand where a
  `Base` is expected. `isinstance` with a generic class does not narrow a
  variable of a base's type, and the comparison methods of a generic
  dataclass derived from a dataclass, which need that, are planned for
  1.0, with generic exception classes and generic protocols (GEN-08,
  GEN-12 to GEN-14, ADR-0040).
- Class variables and defaults in the body of a generic class. Every
  copy, such as `Counter[int]` and `Counter[str]`, shares them, as every
  instance shares them in CPython, and a generic dataclass gets
  `__match_args__`. One whose annotation names a type parameter of the
  class, such as `last: T | None = None`, is planned for 1.0 (GEN-08,
  GEN-11).
- A method called through a class with the instance first, such as
  `Shape.describe(square)`, runs the method that class holds or inherits,
  as in CPython. In `__init__`, `Base.__init__(self, ...)` does what
  `super().__init__(...)` does and counts the base's attributes as
  assigned (CLS-06).
- A generic function or class of another module, imported by name or
  used through its module as `tools.first(items)` or `tools.Box[int]`.
  Its copies read the names of the module that defines it (GEN-07).

### Changed

- Lists, dicts, sets, strings and exceptions that do not outlive their
  function are made in its frame from 1.0, as MEM-10; in 0.5 MEM-03 covers
  instances of the program's classes.
- `list.sort` and `sorted` compare the items CPython 3.15 compares, in its
  order, so a `__lt__` that prints or raises shows the same calls, and a
  sort a comparison ends leaves the list as CPython leaves it. TyPy's own
  merge sort compared other pairs (OI-27).

### Fixed

- `==` and `!=` take optional values, as CPython does: `None` equals only
  `None`, and two values compare as their type does. A dataclass with an
  optional field, whose `__eq__` compares its fields, was refused with
  E0407 (EXP-06, CLS-05).
- A frozen dataclass whose field holds a `list`, `dict` or `set` inside a
  tuple, or through a `type` alias, compares as CPython does and has no
  hash; it was refused with E0413 (CLS-16).
- `slice.indices` takes a length of any size, as CPython does; a length
  past 64 bits raised `OverflowError` (CLS-23).
- Printing a string with a surrogate writes it as CPython does in UTF-8
  mode: standard output writes U+DC80 to U+DCFF as bytes, so a stray
  byte of `sys.argv` or of standard input is written back, and raises
  `UnicodeEncodeError` for any other surrogate; standard error and the
  report of an uncaught exception write `\ud800`. TyPy wrote `?` for
  each (LEX-10).
- `encode` with `surrogateescape` starts its `UnicodeEncodeError` at the
  first character it cannot escape, as CPython does, not at the start of
  the run (LIB-03).
- A function that returns before a later loop over `dict.items()`,
  `keys()` or `values()`, or a later comprehension over one, no longer
  leaks a view object of 24 bytes (CST-03).
- A module-level statement that can call a function or a class before
  the import that binds it has run is E0306, as one before its `def` is.
  TyPy called it anyway, where CPython raises `NameError` (TYP-12).
- A function or a class that imports bind more than once, such as
  `from first import run` and later `from second import run`, one in
  each branch of an `if`, or two lazy imports, is called as the import
  that ran last bound it, and a name an import inside a block binds
  raises `NameError` when that import has not run. TyPy called the last
  one in the source wherever the name was used, even before any import
  ran (MOD-12, ADR-0036).
- A name an import binds to a function or a class that the module also
  defines with `def` or `class` or assigns as a variable is E0202, as a
  second `def` is (STD-04).
- An `OSError` subclass with an `__init__` of its own starts with no
  arguments, as CPython's `OSError.__new__` leaves them to
  `OSError.__init__`: `Quiet("q")` is `Quiet()` with `args` `()` unless its
  `__init__` calls `super().__init__`, where TyPy kept `("q",)` (EXC-14).
- `args` of a `KeyError` the runtime raises holds the missing key itself,
  as CPython's does: `(1,)` for `d[1]` and `(('a', 1),)` for a tuple key,
  where TyPy held the text of its `repr`, `('1',)`. The empty-set and
  empty-dict errors hold their message without quotes (EXC-11).
- A sort, `min`, `max`, `in`, `index` and `count` of objects of a class
  and of classes derived from it compare as `a < b` and `a == b` do,
  calling the right object's reflected method first when its class
  derives from the left one's, as CPython does; TyPy called the left
  object's method alone (CLS-07, CLS-22).
- A comparison whose right operand's class derives from the left
  operand's calls the right operand's reflected method first, also when
  the subclass inherits it, as CPython does: `A() < B()` calls `__gt__`
  of `B`, where TyPy called `__lt__` of `A` (CLS-07).
- An arithmetic operator on two instances of one class that defines only
  the reflected method, such as `__radd__`, raises CPython's `TypeError`;
  TyPy called the reflected method (CLS-07).
- `items[f()] %= values` and `make().text %= values` with a format that
  takes its values when the program runs evaluate the target's parts
  once, where TyPy refused them (SMT-01).
- An operator whose right operand's class has no reflected method, such
  as `__radd__`, calls the one a class derived from it defines when the
  right object is of that class, also first when that class derives from
  the left operand's, as CPython does; TyPy refused it (CLS-07).
- A static method of a generic class that names none of its type
  parameters, such as `Box.count()`, is compiled once and called through
  the class or an instance; TyPy stopped with an internal error (GEN-01).
- `issubclass`, `super()` held as a value, and `property`, `staticmethod`
  and `classmethod` called as functions are refused as planned for 1.0
  (CLS-19, CLS-36, FUN-05); TyPy named 0.4 and 0.5 for them. `import
  collections` is E0206, as for any module TyPy does not provide, and the
  help lists `dataclasses` among the modules it provides (LIB-08).
- A dataclass derived from a dataclass of another module named through
  the module, such as `class Labeled(shapes.Point)`, takes the base's
  fields, as with `from shapes import Point`; TyPy left them out of its
  `__init__` (CLS-05).
- A method of a generic class that assigns an attribute, such as
  `self.item = item` in `put`, compiles when a signature names a copy of
  the class, such as `Box[int]`; TyPy refused it with E0310, as if the
  type of the attribute were not known yet (GEN-03).
- Calling a generic class of another module, such as `Box(3)` after
  `from tools import Box`, compiles; TyPy stopped with an internal error
  (GEN-07).
- A subclass may redefine a static method, or a method as a static one;
  a call through an instance runs the one of the object's class, as
  CPython does (CLS-06, CLS-35).
- An override may take or return a type held otherwise than its base's,
  such as `int | None` where the base takes `int`; a call through the base
  reaches it through an adapter. With a default of its own for such a
  parameter it is planned for 1.0 (CLS-04, CLS-34).
- An override may give a parameter another default than its base's; a
  call through the base that leaves the argument out takes the default of
  the object's class, as CPython does (CLS-04).
- An exception class may give its attributes defaults in its body, such
  as `code: int = 1`, which each exception holds until `__init__` or the
  program assigns them (CLS-11).
- `obj.prop += v` of a property calls the getter, then the setter with
  the result; TyPy left the statement out. A property is assigned through
  any expression, as `make().prop = value` (CLS-06).
- `x[k] += v` on an instance whose class has `__getitem__` and
  `__setitem__` calls them, with `x` and `k` evaluated once; TyPy left
  the statement out, and now refuses it with E0401 when the class has no
  `__setitem__` (CLS-07).

## [0.4.0] - 2026-11-15

### Added

- Single inheritance between classes of the program: a class may derive
  from a class defined before it, named by its name or as
  `module.Class`, and holds its base's attributes and methods. An
  instance is accepted wherever its base is, as an argument, in a
  variable, in a `list[Base]` and as a `Base | None`, and `type()`,
  `str`, `repr` and `print` use its own class. A subclass may declare an
  inherited attribute again with the same type (CLS-04, ADR-0021).
- Overriding methods: a subclass, including an exception class, may
  define a method of its base again, and a call runs the method of the
  instance's own class, also for `for`, `next` and `with`. An override
  is checked against the base method as type checkers check it, E0404
  otherwise; one that changes a default, or takes or returns a type
  held differently, such as `int | None` for `int`, is planned for 0.5
  (CLS-04, CLS-13, ADR-0021).
- `super()` in any method: `super().name(...)` and `super(Class, self)`
  call the nearest base's method, and `super().__init__()` in a class
  whose bases have no `__init__` does nothing, as `object.__init__`
  does. Outside a method it is E0414 (CLS-14). It comes after a
  subclass assigns its own attributes when the base's `__init__` uses
  the instance, which could run an override that reads them, E0309
  otherwise (CLS-04).
- `isinstance` with a class of the program, a built-in type or an
  exception class, or a tuple of them; a variable narrows to the tested
  class where the test holds, also after an `if` that returns and in
  the right operand of `and`. A class held in a variable and
  `issubclass` are planned for 0.5 (CLS-09, TYP-07).
- Class variables, `name = value` and `name: ClassVar[T] = value` in a
  class body, read through the class or an instance and assigned
  through the class; and attribute defaults, `name: T = value`, which
  each new instance starts with (CLS-11, ADR-0024).
- Comparison methods `__eq__`, `__ne__`, `__lt__`, `__le__`, `__gt__`
  and `__ge__`, and `__hash__`: operators call them, with CPython's
  reflected methods and the subclass's method first, and lists, sets,
  dicts, `in`, `sorted`, `min` and `max` call them through the class's
  type object. A class with `__eq__` and no `__hash__` has no hash
  (CLS-07, ADR-0026).
- `@dataclass` from `dataclasses`, with `eq`, `order` and `frozen` and
  `field(default=...)`: it writes `__init__`, `__repr__`, `__eq__` and
  the ordering methods as CPython does, and refuses what CPython
  refuses, such as a mutable default. The hash of a frozen dataclass
  and `default_factory` are planned for 0.5 (CLS-05, LIB-06).
- Unions, such as `int | str`, `str | list[int] | None` or
  `Circle | Square`: a union of objects is held as the object itself, and
  any other with a tag that names its member, so an `int` stored in an
  `int | str` allocates nothing. A union can be passed, returned, stored,
  printed, written with
  `str` and `repr`, given to `type()` and tested for truth. `isinstance`
  narrows it in both branches, and a union of classes that share a base
  has that base's attributes and methods. Any other operation on a
  variable that holds a union, such as `value.upper()`, `len(value)` or
  `value + value`, is checked for each member and runs the one for the
  object's member. Lists, sets and dicts of a union compare and hash
  each item as its member, so `1` and `1.0` are one key (TYP-14, TYP-07,
  ADR-0027).
- Generic functions written with PEP 695 type parameter lists, such as
  `def first[T](items: list[T]) -> T`, with bounds, `[S: Shape]`, and
  constraints, `[N: (int, float)]`. A call takes the types from its
  arguments, and TyPy compiles one copy of the function for each set of
  types, so a generic call costs what a written-out call does. `TypeVar`
  and `Generic` are E0202; a function that would need copies without
  end is E0428. Calling a generic function of another module is planned
  for 0.5 (GEN-07), and `*Ts`, `**P` and defaults for 1.0 (GEN-01 to
  GEN-03, GEN-06, ADR-0023).
- Generic classes, `class Box[T]:`, made by `Box(1)`, whose types come
  from `__init__`'s arguments, or by `Box[str]("a")`, and named
  `Box[int]` in annotations. Each copy holds its attributes in place, and
  answers as the generic class: `type(Box(1)) is Box` and
  `isinstance(Box(1), Box)` hold. A generic class with a base, a class
  derived from one, and its class variables and defaults are planned for
  0.5 (GEN-01, GEN-03, GEN-08, ADR-0028).
- Protocols: `class Shape(Protocol):` from `typing` lists methods and
  attributes, and any class that has them satisfies it. A parameter
  typed with a protocol, also inside `list[Shape]`, compiles to one copy
  for each class it is called with, and `[S: Shape]` bounds a type
  parameter by one. A class that falls short is E0404, naming the
  member; calling a protocol and `isinstance` with one are E0401. A
  protocol as any other type, as a base, or with properties, static and
  class methods or class variables is planned for 0.5 (GEN-04, GEN-05,
  GEN-10, ADR-0023).
- `...` alone as a statement does nothing, as `pass` does.
- `open` takes a file descriptor, giving a file whose `name` is that `int`,
  and `closefd`, with CPython's errors (LIB-14).
- `open` takes a mode held in a variable, which opens a text file and is
  checked with CPython's errors when the program runs; a binary mode in a
  variable raises `TypeError` (LIB-14, ADR-0031).
- `open` takes binary modes, which give a `BufferedReader`,
  `BufferedWriter` or `BufferedRandom`, or a `FileIO` with `buffering=0`,
  reading and writing `bytes` with CPython's results and errors (LIB-14,
  ADR-0031).
- `open` takes the modes `"r+"`, `"w+"`, `"a+"` and `"x+"`, which read and
  write one text file as CPython's do (LIB-14).
- `close()` of `sys.stdin`, `sys.stdout` or `sys.stderr` closes it, as in
  CPython: reading it, writing to it, `input()` and `print()` then raise
  `ValueError: I/O operation on closed file.` (LIB-14).
- `open` takes `buffering`, `encoding` (`utf-8`, `ascii` or `latin-1`),
  `errors` (`strict`, `ignore`, `replace`, `backslashreplace`,
  `xmlcharrefreplace` or `surrogateescape`) and `newline`, with CPython's
  results and errors; `buffering=1` writes out each line (LIB-14).
- Text files have `name`, `mode` and `closed`, and the methods `fileno`,
  `isatty`, `readable`, `writable`, `seekable`, `seek`, `tell` and
  `truncate`, with CPython's results and errors; a file opened with `"a"`
  starts at its end (LIB-14).
- Names a module binds by a lazy import can be used from another module,
  `m.x = v` assigns through a lazily imported `m`, and a variable may be
  bound by more than one lazy import; each runs the pending import first,
  as CPython does (MOD-11). A function or a class bound by two lazy
  imports is planned for 0.5 (MOD-12).
- `format % values` with a format that is not a literal, also through
  `%=`, and with a `tuple[T, ...]` of values: the runtime reads the format
  and raises what CPython raises (LIB-11).
- `OSError` and its subclasses built with two to five arguments, read as
  `errno`, `strerror`, `filename`, `winerror` and `filename2`, with the
  attributes `errno`, `strerror`, `filename` and `filename2`. `OSError`
  itself becomes the subclass for `errno`, as in CPython. Other argument
  types are planned for 0.5 (EXC-14, EXC-16).
- `args` of an exception and `code` of a `SystemExit`, where a program
  prints, formats, compares with `==` or `!=`, or tests them, and
  `len(error.args)`. Other uses, such as `error.args[0]`, are planned for
  0.5 (EXC-11, EXC-13, EXC-15, ADR-0030).
- `sys.stdin`, a text file that `input()` reads too, and `sys.maxsize`,
  `sys.maxunicode`, `sys.byteorder`, `sys.platform`,
  `sys.getrecursionlimit` and `sys.setrecursionlimit`. The recursion
  limit is kept for `getrecursionlimit`; recursion still stops at the
  stack's limit (LIB-13, FUN-02).
- `sys.version`, `sys.hexversion` and `sys.version_info` report CPython
  3.15.0, whose behaviour TyPy follows. `sys.version_info` is compared
  with a tuple of `int`s, indexed and read by field; as a value of its
  own it is planned for 0.5 (LIB-13, LIB-16, ADR-0029).
- `math.fsum`, `math.dist`, `math.prod`, `math.sumprod`, `math.gamma`
  and `math.lgamma`, with CPython's results to the last bit. The first
  four take iterables of `int`, `bool` or `float` items; `prod` of
  `float`s with the default start and `sumprod` with a `float` are
  `int | float`, since CPython gives the `int` start or 0 for no items.
  Items of a union and a `bool` start are planned for 0.5 (LIB-12,
  LIB-15).
- Methods with type parameters, `def pair[T](self, value: T)`, also
  static methods, in a class that is not generic. A call takes the types
  from the arguments after `self` and runs the copy for them. Type
  parameters of a method of a generic class, of a special method, a
  property or a class method, and of a method that overrides or is
  overridden are planned for 0.5 (GEN-01, GEN-09).
- `Final` and `Final[T]` from `typing`: a module or function variable, a
  class variable or an attribute that `__init__` declares `Final` is
  assigned only by its declaration, and any other assignment is E0414. A
  `Final` without a value is E0408 (TYP-08).
- The `type` statement at module level: `type Name = annotation` makes
  `Name` stand for the annotation in the module's annotations, and may
  name a class defined below it, as CPython reads the value late. An
  alias may take type parameters, `type Pair[T] = tuple[T, T]`, and
  `Pair[int]` stands for its value with `int` in place of `T`. An alias
  of another module, read as a value, defined inside a function, a class
  or a block, or with a bound on a type parameter is planned for 0.5
  (SMT-10, SMT-12).
- Container methods: `len(x)`, `x[k]`, `x[k] = v`, `del x[k]` and
  `v in x` call `__len__`, `__getitem__`, `__setitem__`, `__delitem__`
  and `__contains__`, and truth tests, `any` and `all` call `__bool__`,
  or test `__len__` against zero. A negative `__len__` raises
  `ValueError` as in CPython (CLS-07).
- Arithmetic methods: the binary operators call `__add__`, `__sub__`,
  `__mul__` and the others, with their reflected forms such as
  `__radd__`; `-x`, `+x`, `~x` and `abs(x)` call `__neg__`, `__pos__`,
  `__invert__` and `__abs__`; and `x += y` rebinds `x` to `x + y`. The
  in-place methods such as `__iadd__` are planned for 0.5 (CLS-07).
- `@classmethod`: `Class.name(...)` passes the class, so `cls(...)` in
  an alternative constructor makes an instance of the class the call
  names, as in CPython; TyPy compiles a copy for each class of the
  module that has it (CLS-06, ADR-0025).
- `@staticmethod`, called through the class or an instance, and
  `@property` with its `@name.setter`: reading the attribute calls the
  getter of the instance's class, and assigning it calls the setter.
  A property without a setter cannot be assigned (CLS-06).
- An exception class may derive from an exception class imported from
  another module, as `from errors import AppError` or `errors.AppError`
  (CLS-03).

### Changed

- Allocation on the stack is planned for 0.5 (MEM-03), with the benchmarks
  that measure what it gains.
- A class decorator other than `@dataclass` is planned for 1.0 with the
  other user-defined decorators (FUN-06), not for 0.4.
- `open(..., encoding="csutf8")` is E0404, as for `str.encode`: CPython
  3.12 does not know the name, so its result depends on the version.
  Write `"utf-8"` instead.
- A binary mode with a `buffering` that is not a literal is planned for 1.0
  (LIB-19), since the value decides the class of the file.
- The `opener` argument of `open` is planned for 1.0 (LIB-18), since it
  takes a function as a value.
- Importing a module inside a function of a module that it imports at
  module level is planned for 0.5 (MOD-13).
- Names of `typing` that TyPy does not give are planned for 0.5 rather
  than 0.4: `Optional`, `Union`, `List`, `Dict` and the other older names
  (LIB-17), and `Iterator` and `Iterable` as values (TYP-17).
  `typing.Any` and `typing.cast` are refused with E0202, since every
  value has one type (TYP-10).
- A surrogate escape in a `str` literal, such as `"\udcff"`, is planned
  for 0.5 rather than 0.4: string constants would have to carry code
  points instead of UTF-8 text through the whole compiler (LEX-10).
- The name of `except (A, B) as name` is the union `A | B`, as type
  checkers read it, and no longer their common base; its attributes and
  methods are still the base's, and it can be stored in a variable
  declared `A | B` (TYP-14).

- A base that is not `object`, an exception class or a class of the
  program, such as `int` or `list`, is E0201 with a message that names
  what a base may be, in place of "planned for TyPy 0.4" (CLS-04).

### Fixed

- A class method with a parameter that names a protocol is E0201, planned
  for 0.5 (GEN-09); it was accepted and every call with a matching class
  was refused.
- A `for` loop whose variable is a `T | None`, such as one bound earlier by
  a loop over `None` and `"a"`, over items of type `T` no longer stops the
  compiler with an internal error; the variable is not `None` in the
  loop's body (CST-03, TYP-13).
- A call that names one keyword twice, such as `print(1, sep="", sep="")`,
  is E0103 with CPython's `SyntaxError` message, `keyword argument
  repeated: sep`; it used to compile, or report the wrong error (STD-01).
- A `%` format whose width is past a C `ssize_t` or whose precision is
  past a C `int`, such as `"%99999999999999999999d"`, is refused with
  CPython's `width too big` or `precision too big` (LIB-10).
- `input()` reads standard input as CPython does: a line that ends with
  `\r\n` or `\r` loses it, where it used to keep the `\r` (LIB-04).
- A truth test of a list, a set, a dict or `bytes` in a recursive
  function, such as `if not items:` before `return f(items[1:])`, stopped
  the compiler with an internal error (EXP-10).

- Repeating a list, a `str` or `bytes` by a count from 2**62 up to the
  largest index-sized integer raised `OverflowError`. An empty sequence
  now stays empty however many times it is repeated, and a longer one is
  refused as CPython refuses it. A list whose repeated length passes the
  address space now runs out of memory, which prints `MemoryError` and
  ends the program, where it used to raise `OverflowError` (DAT-05,
  DAT-06, DAT-07).
- Calling a class whose body declares an attribute that no `__init__`
  assigns is E0309. The program used to compile and crash when the
  attribute was read, where CPython raises `AttributeError` (CLS-01).
- Calling a module variable that holds a value, such as an `int`, from a
  function is E0401. The call used to be dropped, where CPython raises
  `TypeError` (EXP-02).

## [0.3.0] - 2026-11-13

### Added

- `open` for text files in UTF-8, in the modes `r`, `w`, `a` and `x`:
  `read`, `readline`, `readlines`, `write`, `writelines`, `flush` and
  `close`, iteration over the lines, and `with`. Reading translates
  newlines and decodes as CPython does, files still open are written out
  when the program ends, and errors are CPython's `OSError` subclasses
  with its text, such as `[Errno 2] No such file or directory:
  'notes.txt'`. `OSError` and all its subclasses can be caught and
  raised. Binary and `+` modes, other encodings, `seek` and the file's
  attributes are planned for 0.4 (LIB-04, LIB-14, EXC-14, ADR-0019).
- The `sys` module's `argv`, `exit`, `stdout` and `stderr`, `print` with
  `file` and `flush`, and `input`, which raises `EOFError` at the end of
  standard input. An uncaught `SystemExit` ends the program with its
  code as the exit status and no traceback, as in CPython. Other names
  of `sys` are E0201 when planned for 0.4 (LIB-13) and E0202 otherwise;
  the switches of lazy imports are E0202 (LIB-04, MOD-09).
- The `math` module: its constants and its functions of numbers, with
  CPython's results to the last bit and CPython 3.15's error messages;
  a name newer than the program's Python floor is E0425. `fsum`, `prod`,
  `sumprod`, `dist`, `gamma` and `lgamma` are planned for 0.4 (LIB-05,
  LIB-12, ADR-0018).
- `format % values`, printf-style formatting of a string literal, with
  every conversion, flag, width, precision and `*`, and `%(key)s` with a
  dict, giving CPython's results. A format that does not fit its values
  is E0427 with CPython's message; a format that is not a literal is
  planned for 0.4 (LIB-10, LIB-11).
- `typy explain CODE` prints what a diagnostic means, the rule it
  enforces, a program that gives it and the corrected program; the
  troubleshooting page shows the same programs, and the documentation
  test runs them (CLI-10).
- Warnings, where CPython warns while compiling: W0101 for an invalid
  escape sequence such as `"\d"`, with the doubled backslash as its fix,
  and W0301 for `assert (test, "message")`, which is always true. They
  never fail a build unless `--deny-warnings` is given (DIA-07, DIA-08,
  ADR-0017).
- `--error-format=json` writes each diagnostic as one JSON object per
  line, with the same keys every time, for editors and CI (DIA-05).
- `typy.toml` beside the main file sets `python-floor` and `module-paths`,
  the directories where imports look for modules after the main file's
  directory; `--python-floor` on `check`, `build` and `run` overrides the
  file. A setting that is not valid is E0107 at its line, with exit status
  2 (CLI-07, CLI-12, MOD-01).
- Lazy imports: `lazy import m` and `lazy from m import name` with the
  Python floor 3.15, and a module-level `__lazy_modules__` at every floor.
  The module runs at the first use of a name, an error it raises reaches
  that use, and the next use runs it again. Using such a name from another
  module is E0201, planned for 0.4 (MOD-06 to MOD-08, MOD-11).
- `Iterator[T]` and `Iterable[T]` annotations from `typing` or
  `collections.abc`, and `TracebackType` and `ModuleType` from `types`. An
  `Iterable[T]` takes any built-in iterable or iterator of `T` and keeps
  the object, so a list passed as one can be read twice (TYP-15, CST-10,
  MOD-10).
- A truth test, `not` or `bool()` of a value whose truth TyPy cannot know
  is E0304 everywhere, not only in a condition (EXP-10).
- `make bench` times the release-built compiler on generated 10,000-line
  projects, in one file and in 200 modules, against the budgets of NFR-04,
  NFR-05 and NFR-14, and checks that checking time grows linearly with the
  number of modules; CI runs it.
- Programs of several modules: `import m`, `import a.b as c` and
  `from m import name` for functions, classes, module variables and
  submodules, packages with `__init__.py`, and relative imports. Each
  module runs once, at its first import, and runs again at the next import
  when its code raised. A module object prints as
  `<module 'm' from 'm.py'>`, with the path from the main file's directory
  (SMT-07, MOD-01 to MOD-03, MOD-10).
- E0206 for a module the program does not have, E0422 for a circular
  import at module level with each import shown, and E0425 for a name a
  module does not define; standard library modules TyPy provides later
  are E0201 (MOD-05, LIB-08).
- `global` in a function, and functions that read the variables of their
  module. A read before the module binds the variable raises CPython's
  `NameError`, and CPython's `SyntaxError`s for `global` are E0103 (SMT-08).
- `NameError` and `ImportError` as built-in exception classes (EXC-01).
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

### Changed

- The last use of a variable moves its value into an assignment, a
  display, `append`, `insert`, `add` or a dict entry, and a new value such
  as a call's result goes into a collection without being counted, so
  building a collection in a loop does no reference counting of its items
  (MEM-09).
- `--release` and `--release-small` builds leave out debug information; a
  static musl hello world is about 30 KB (CLI-05, NFR-01).

### Fixed

- A format specification on an exception, such as `f"{error:>5}"`,
  raises `TypeError` naming the exception's own class, as CPython does;
  it named `NoneType`.
- A deeply nested program no longer crashes `typy`; it aborted on 1000
  nested brackets. Brackets nested past 200 and blocks past 99 are E0103
  with CPython's messages, and expressions or statements nested past
  25000 are E0103; everything CPython compiles still compiles (LEX-11,
  ADR-0020).
- A function of many blocks, such as a long `elif` chain, compiles in
  time and memory in proportion to its size; 20000 `elif` clauses ran out
  of memory.
- `print(..., file=f)` inside an `except` clause compiles; it failed with
  E9001.
- An import inside a function, or a lazy one, no longer makes a circular
  import with a module that imports the importing module back at module
  level; it gave a false E0422 (MOD-05, OI-21).
- A function whose only `try` has a body that cannot raise compiles;
  it failed with E9001.
- A `\u` or `\U` escape of a surrogate in a `str` literal is E0201,
  planned for 0.4 (LEX-10); it gave U+FFFD instead of the surrogate.
- Programs call the C library's maths functions, as CPython does, and
  not the copies in the runtime archive, whose `cbrt` differed in the
  last bit.
- `typy run` builds again after the runtime archive changes, since the
  cache key now covers the archive as well as the compiler.
- `type(m)` of a module, an `Iterable[T]` or a traceback gives its type
  object; it stopped the compiler with E9001 before.

- `typy run` no longer reuses an executable built from other versions of
  the program's modules or by another build of the compiler: the cache key
  covers every file of the program and the compiler executable.
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

[Unreleased]: https://github.com/typy-lang/typy/compare/v0.9.0...HEAD
[0.9.0]: https://github.com/typy-lang/typy/compare/v0.8.0...v0.9.0
[0.8.0]: https://github.com/typy-lang/typy/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/typy-lang/typy/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/typy-lang/typy/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/typy-lang/typy/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/typy-lang/typy/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/typy-lang/typy/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/typy-lang/typy/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/typy-lang/typy/releases/tag/v0.1.0
