# Changelog

All notable changes to TyPy are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and TyPy uses
[semantic versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

- `open(..., encoding="csutf8")` is E0404, as for `str.encode`: CPython
  3.12 does not know the name, so its result depends on the version.
  Write `"utf-8"` instead.
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

[Unreleased]: https://github.com/typy-lang/typy/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/typy-lang/typy/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/typy-lang/typy/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/typy-lang/typy/releases/tag/v0.1.0
