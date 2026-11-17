//! IDs of the specification's requirements that a diagnostic can name (DIA-03).

use std::fmt;

/// The ID of a requirement in `docs/requirements/srs.md`, such as `TYP-03`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuleId(&'static str);

impl RuleId {
    /// The host has a C toolchain that can link for the target.
    pub const A_01: Self = Self("A-01");
    /// Project settings shall be read from `typy.toml`, and a command line flag shall override the same setting in the file.
    pub const CLI_07: Self = Self("CLI-07");
    /// The setting `python-floor` in `typy.toml` shall set the program's Python floor to one of `3.12`, `3.13`, `3.14` or `3.15`, and any other value shall be rejected.
    pub const CLI_12: Self = Self("CLI-12");
    /// Classes with methods and an `__init__` shall be accepted, with each instance attribute assigned in `__init__` or declared in the class body.
    pub const CLS_01: Self = Self("CLS-01");
    /// Adding an attribute that the class does not declare shall be rejected.
    pub const CLS_02: Self = Self("CLS-02");
    /// Single inheritance between user classes shall be accepted, with overriding methods type checked against the base.
    pub const CLS_04: Self = Self("CLS-04");
    /// `@dataclass`, including `frozen`, `order` and `field(default=...)`, shall be accepted.
    pub const CLS_05: Self = Self("CLS-05");
    /// `@property`, `@staticmethod` and `@classmethod` shall be accepted.
    pub const CLS_06: Self = Self("CLS-06");
    /// Operator methods such as `__add__`, `__eq__`, `__lt__`, `__len__`, `__getitem__` and `__contains__` shall be called for the matching operators.
    pub const CLS_07: Self = Self("CLS-07");
    /// `__str__` and `__repr__` shall be used by `str()`, `repr()`, `print()` and f-strings.
    pub const CLS_08: Self = Self("CLS-08");
    /// `isinstance` shall be accepted with a class or a tuple of classes.
    pub const CLS_09: Self = Self("CLS-09");
    /// Class variables, given a value in the class body, shall be accepted.
    pub const CLS_11: Self = Self("CLS-11");
    /// `type(x)` shall give the type object of the value `x`, and a class name or the name of a built-in type shall be its type object as a value; a type object shall print as `<class 'name'>`, give its name as `__name__`, and compare with `is`, `is not`, `==` and `!=`.
    pub const CLS_12: Self = Self("CLS-12");
    /// A class derived from an exception class shall be accepted with new methods and attributes and with its own `__init__`, `__str__` and `__repr__`; redefining another method of a base shall be rejected until CLS-04.
    pub const CLS_13: Self = Self("CLS-13");
    /// `super().__init__(...)` shall be accepted in the `__init__` of an exception class and shall set the exception's arguments; any other use of `super()` shall be rejected until 0.4.
    pub const CLS_14: Self = Self("CLS-14");
    /// `field(default_factory=f)` in a dataclass shall call `f`, a class or a function that takes no arguments, for each instance whose `__init__` is given no value for the field.
    pub const CLS_15: Self = Self("CLS-15");
    /// A dataclass that is frozen with `eq=True`, or given `unsafe_hash=True`, shall hash as the tuple of its fields, and one with `eq=True` that is neither shall be unhashable, as CPython's `dataclasses` writes `__hash__`.
    pub const CLS_16: Self = Self("CLS-16");
    /// `isinstance` and a class pattern shall accept a class held in a variable or computed.
    pub const CLS_18: Self = Self("CLS-18");
    /// `issubclass` shall be accepted with a class or a tuple of classes.
    pub const CLS_19: Self = Self("CLS-19");
    /// An augmented assignment to an instance shall call the class's in-place method, such as `__iadd__`, and the binary method when the class has none, as CPython does.
    pub const CLS_20: Self = Self("CLS-20");
    /// An operator or comparison method that returns `NotImplemented` shall give way to the reflected method, and then to CPython's default or its `TypeError`.
    pub const CLS_21: Self = Self("CLS-21");
    /// Comparison and hash methods that may raise, a chain such as `a < b < c` of instances, and an operator whose method and reflected method return different types shall be accepted.
    pub const CLS_22: Self = Self("CLS-22");
    /// `in` on an instance whose class has no `__contains__` shall iterate it, and an instance shall be sliced through its `__getitem__`, as CPython does.
    pub const CLS_23: Self = Self("CLS-23");
    /// The truth of an instance shall come from the `__bool__` or `__len__` of its own class, also where its static type is a base or a union that tests truth otherwise, and in `any` and `all`.
    pub const CLS_24: Self = Self("CLS-24");
    /// A class method shall be callable through an instance and through a subclass defined in another module, a class variable shall be assignable through a subclass, and a static method shall be callable through any expression that gives its class or an instance.
    pub const CLS_25: Self = Self("CLS-25");
    /// `super(Class, instance)` shall be accepted with any class of the instance's chain of bases.
    pub const CLS_26: Self = Self("CLS-26");
    /// A class derived from `typing.NamedTuple` shall be accepted, its fields read by name and by position.
    pub const CLS_27: Self = Self("CLS-27");
    /// `@dataclass` shall accept the options `kw_only` and `weakref_slot` and options given other than as `True` or `False`, `field` its options `init`, `repr`, `hash`, `compare`, `kw_only` and `metadata`, and a generic dataclass a field with a `default_factory`.
    pub const CLS_28: Self = Self("CLS-28");
    /// `super(Class, instance)` shall be accepted with an instance other than the method's own, and outside a method.
    pub const CLS_29: Self = Self("CLS-29");
    /// A class method of a generic class shall be callable through an instance.
    pub const CLS_30: Self = Self("CLS-30");
    /// A class variable shall be assignable through a variable that holds a class derived from the class that gives it, such as `cls` in a class method, giving that class one of its own.
    pub const CLS_31: Self = Self("CLS-31");
    /// An augmented assignment shall call the in-place method that only a class derived from its target's class defines, when the target holds an instance of that class.
    pub const CLS_32: Self = Self("CLS-32");
    /// An operator or comparison method that may return both `None` and `NotImplemented` shall be accepted.
    pub const CLS_33: Self = Self("CLS-33");
    /// An override whose parameter is held otherwise than its base method's, such as `int | None` for `int`, shall be accepted with a default of its own for that parameter.
    pub const CLS_34: Self = Self("CLS-34");
    /// A static method that overrides a method, or a method that overrides a static method, shall be accepted with a parameter or result held otherwise than the one it overrides.
    pub const CLS_35: Self = Self("CLS-35");
    /// `super()` shall be usable as a value: held in a variable, passed and returned.
    pub const CLS_36: Self = Self("CLS-36");
    /// `for` shall iterate over `range`, `str`, `bytes`, `list`, `tuple`, `dict`, `set` and any class with `__iter__` and `__next__`.
    pub const CST_03: Self = Self("CST-03");
    /// `try` with `except`, `else` and `finally` clauses shall be accepted, and `finally` shall run on `return`, `break`, `continue` and exceptions.
    pub const CST_04: Self = Self("CST-04");
    /// `with` shall call `__enter__` and `__exit__` as the Language Reference defines, also when the block exits early.
    pub const CST_05: Self = Self("CST-05");
    /// `match` shall be accepted with literal, capture, wildcard, value, sequence, mapping and class patterns, guards and `\\|` alternatives.
    pub const CST_06: Self = Self("CST-06");
    /// A `match` whose cases without a guard leave a member of the subject's union, optional or `bool` type unmatched shall report a warning that names the members (ADR-0033).
    pub const CST_07: Self = Self("CST-07");
    /// `for` shall iterate over any class with `__iter__` and `__next__`, and end when `__next__` raises `StopIteration` (OI-17).
    pub const CST_08: Self = Self("CST-08");
    /// `iter` and `next`, with and without a default, shall be available for the built-in iterables.
    pub const CST_09: Self = Self("CST-09");
    /// The parameters of `__exit__` after `self` may be left without annotations, and shall then have the types `type[BaseException] \| None`, `BaseException \| None` and `TracebackType \| None`; the traceback shall be one object of type `traceback`, written as CPython writes one, which holds no frames.
    pub const CST_10: Self = Self("CST-10");
    /// A class with `__getitem__` and no `__iter__` shall be iterable through `__getitem__` with the positions 0, 1, 2 and so on until it raises `IndexError`, in `for`, `in` and every builtin that iterates, as CPython's sequence protocol does.
    pub const CST_11: Self = Self("CST-11");
    /// `bool` shall have the values `True` and `False`.
    pub const DAT_03: Self = Self("DAT-03");
    /// `str` shall be an immutable sequence of Unicode code points. `len()` and indexing shall count code points, and indexing shall return a `str` of length 1.
    pub const DAT_05: Self = Self("DAT-05");
    /// `bytes` shall be an immutable sequence of integers from 0 to 255.
    pub const DAT_06: Self = Self("DAT-06");
    /// `list[T]` shall be a mutable sequence.
    pub const DAT_07: Self = Self("DAT-07");
    /// `tuple[...]` shall be an immutable sequence with a fixed number of items, each with its own type.
    pub const DAT_08: Self = Self("DAT-08");
    /// `dict[K, V]` shall be a mutable mapping that iterates in insertion order.
    pub const DAT_09: Self = Self("DAT-09");
    /// `set[T]` shall be a mutable set.
    pub const DAT_10: Self = Self("DAT-10");
    /// Slicing shall return a new object with the items selected by start, stop and step, as in Python.
    pub const DAT_12: Self = Self("DAT-12");
    /// Assigning to a slice of a list shall replace the selected items with the items of a list, as in Python.
    pub const DAT_13: Self = Self("DAT-13");
    /// `tuple[T, ...]` shall be an immutable sequence of any number of items of type `T`, and a tuple of a fixed number of items of type `T` shall be accepted where one is expected.
    pub const DAT_14: Self = Self("DAT-14");
    /// The methods of `set` that combine a set with others, `union`, `intersection`, `difference`, `symmetric_difference`, `update`, `intersection_update`, `difference_update`, `symmetric_difference_update`, `issubset`, `issuperset` and `isdisjoint`, shall accept any iterable of the set's item type, and as many arguments as CPython accepts, with CPython's results and order.
    pub const DAT_15: Self = Self("DAT-15");
    /// `reversed` of a dict and of its views, `dict` with keyword arguments, `dict.fromkeys`, and the comparisons and set operations of `dict.keys()` and `dict.items()` shall be available with Python's results.
    pub const DAT_16: Self = Self("DAT-16");
    /// `range` shall be a value as well as a loop source: a sequence of `int` with `len`, indexing, slicing, `in`, `==` and `repr`, as in Python.
    pub const DAT_17: Self = Self("DAT-17");
    /// Where CPython 3.12 to 3.15 give a `SyntaxWarning` while compiling a program TyPy accepts, TyPy shall give a warning: for an invalid escape sequence and for an assertion that is always true.
    pub const DIA_08: Self = Self("DIA-08");
    /// Raising and catching exceptions shall follow the Language Reference, including matching on base classes.
    pub const EXC_01: Self = Self("EXC-01");
    /// A `return`, `break` or `continue` that leaves a `finally` block shall be rejected, as PEP 765 makes CPython 3.14 warn about it (ADR-0013).
    pub const EXC_06: Self = Self("EXC-06");
    /// A `raise` without an expression shall re-raise the exception being handled, and shall be rejected outside an `except` clause.
    pub const EXC_07: Self = Self("EXC-07");
    /// An `except` clause shall name one exception class or a tuple of them, with or without `as`, or none for a bare `except`; any other expression shall be rejected.
    pub const EXC_09: Self = Self("EXC-09");
    /// Calling `UnicodeDecodeError` or `UnicodeEncodeError`, or deriving a class from them, shall be rejected; catching them shall be accepted.
    pub const EXC_12: Self = Self("EXC-12");
    /// `OSError` and its subclasses built with two to five arguments shall give the subclass for the `errno`, and the attributes `errno`, `strerror` and `filename`, as CPython does.
    pub const EXC_14: Self = Self("EXC-14");
    /// `args` of an exception and `code` of a `SystemExit` shall be values of their own: stored, passed, indexed and iterated.
    pub const EXC_15: Self = Self("EXC-15");
    /// An `OSError` shall take arguments of any type, more than five of them, and two or more through `super().__init__`, as CPython does.
    pub const EXC_16: Self = Self("EXC-16");
    /// `args` of an exception shall hold the `args` of an exception of a related class, a type that nests without end, as CPython allows.
    pub const EXC_17: Self = Self("EXC-17");
    /// `characters_written` of a `BlockingIOError`, and a number as the third argument of `OSError` or `BlockingIOError`, shall behave as CPython's do.
    pub const EXC_18: Self = Self("EXC-18");
    /// `+`, `-`, `*`, `/`, `//`, `%` and `**` shall give Python's results for `int` and `float`, including `/` on two `int` values returning `float` and `%` taking the sign of the divisor.
    pub const EXP_02: Self = Self("EXP-02");
    /// `int ** int` shall be accepted only when the exponent is a non-negative integer literal, because CPython returns `float` for a negative exponent.
    pub const EXP_04: Self = Self("EXP-04");
    /// Comparisons shall be accepted between values of the same type, and between `int` and `float`.
    pub const EXP_06: Self = Self("EXP-06");
    /// `in` and `not in` shall be accepted on `str`, `bytes`, `list`, `tuple`, `dict` and `set`.
    pub const EXP_07: Self = Self("EXP-07");
    /// `is` and `is not` shall be accepted only when one operand is `None` or both operands are class instances.
    pub const EXP_08: Self = Self("EXP-08");
    /// `and` and `or` shall short-circuit and return one of their operands, and shall be accepted only when both operands have the same type.
    pub const EXP_09: Self = Self("EXP-09");
    /// A value shall be accepted in a truth test when its type is `bool`, `int`, `float`, `str`, `bytes`, a built-in container, `T \\| None`, or a class that defines `__bool__` or `__len__`.
    pub const EXP_10: Self = Self("EXP-10");
    /// Conditional expressions `x if c else y` shall be accepted when both branches have the same type.
    pub const EXP_11: Self = Self("EXP-11");
    /// List, dict and set comprehensions shall be accepted, each with its own scope.
    pub const EXP_12: Self = Self("EXP-12");
    /// Assignment expressions `:=` shall be accepted.
    pub const EXP_13: Self = Self("EXP-13");
    /// `lambda` expressions shall be accepted where the expected type is a `Callable`.
    pub const EXP_14: Self = Self("EXP-14");
    /// Generator expressions shall be accepted.
    pub const EXP_15: Self = Self("EXP-15");
    /// Calls with positional, keyword and default arguments shall be accepted, and default values shall be evaluated once, when the function is defined.
    pub const EXP_16: Self = Self("EXP-16");
    /// `*args` and `**kwargs` with annotated element types shall be accepted.
    pub const EXP_17: Self = Self("EXP-17");
    /// A module-level `ctypes.CDLL` or `ctypes.cdll.LoadLibrary` call with a string literal shall link the named library.
    pub const FFI_01: Self = Self("FFI-01");
    /// Module-level function definitions shall be accepted.
    pub const FUN_01: Self = Self("FUN-01");
    /// Keyword-only and positional-only parameters shall be accepted.
    pub const FUN_03: Self = Self("FUN-03");
    /// Nested functions and closures shall be accepted, capturing variables as Python does.
    pub const FUN_04: Self = Self("FUN-04");
    /// Functions shall be values of type `Callable[[A, ...], R]`.
    pub const FUN_05: Self = Self("FUN-05");
    /// User-defined decorators whose types can be checked shall be accepted.
    pub const FUN_06: Self = Self("FUN-06");
    /// Generic functions and classes shall be written with PEP 695 type parameter lists; `TypeVar` and `Generic` shall be rejected.
    pub const GEN_01: Self = Self("GEN-01");
    /// Type parameters with a bound or with constraints shall be accepted and checked.
    pub const GEN_02: Self = Self("GEN-02");
    /// Generics shall compile to one copy per concrete type, with no run-time type information.
    pub const GEN_03: Self = Self("GEN-03");
    /// Classes derived from `typing.Protocol` shall be satisfied by any class with matching methods, checked when the program is compiled.
    pub const GEN_04: Self = Self("GEN-04");
    /// `isinstance` with a protocol shall be rejected.
    pub const GEN_05: Self = Self("GEN-05");
    /// `*Ts` and `**P` type parameters, and defaults of type parameters, shall be accepted.
    pub const GEN_06: Self = Self("GEN-06");
    /// A generic function or class of another module shall be usable, made from its source as ADR-0016's interfaces carry it.
    pub const GEN_07: Self = Self("GEN-07");
    /// A generic class shall be accepted with a base other than `object`, as the base of another class, and with class variables and defaults.
    pub const GEN_08: Self = Self("GEN-08");
    /// Type parameters shall be accepted on a method of a generic class, a special method, a property, a class method, and a method that overrides or is overridden.
    pub const GEN_09: Self = Self("GEN-09");
    /// A protocol shall be usable as the type of a variable, an attribute, a result and a container item outside a parameter, and of a parameter of a special method or a property.
    pub const GEN_10: Self = Self("GEN-10");
    /// A class variable or an attribute default of a generic class whose annotation names a type parameter of the class shall be accepted.
    pub const GEN_11: Self = Self("GEN-11");
    /// A generic class shall be accepted with an exception class as its base.
    pub const GEN_12: Self = Self("GEN-12");
    /// A generic protocol shall be accepted, satisfied by the classes whose members fit it for its type arguments.
    pub const GEN_13: Self = Self("GEN-13");
    /// `isinstance` and a class pattern with a generic class shall narrow a variable of a base's type to the copies of the class the program makes.
    pub const GEN_14: Self = Self("GEN-14");
    /// A class derived from a copy of a generic class shall inherit the generic class's class methods, and a dataclass derived from a copy of a generic dataclass its fields.
    pub const GEN_15: Self = Self("GEN-15");
    /// Type parameters shall be accepted on a special method, on a property, and on a method that overrides or is overridden by one with other type parameters or in a generic class, and a class method with type parameters shall be callable through an instance.
    pub const GEN_16: Self = Self("GEN-16");
    /// A protocol shall be usable as the base of a class, with properties, static methods, class methods, class variables and defaults, and in the signatures of its own methods; an attribute shall be assignable through a protocol value, and a value of a built-in type shall be held as a protocol.
    pub const GEN_17: Self = Self("GEN-17");
    /// Source files shall be UTF-8 with the `.py` extension.
    pub const LEX_01: Self = Self("LEX-01");
    /// Bytes literals shall be accepted.
    pub const LEX_06: Self = Self("LEX-06");
    /// f-strings, with conversions and format specifications, shall be accepted.
    pub const LEX_07: Self = Self("LEX-07");
    /// Imaginary literals shall be rejected.
    pub const LEX_08: Self = Self("LEX-08");
    /// A source file larger than 16 MiB shall be rejected with a diagnostic before it is parsed, so input size cannot exhaust the compiler's memory.
    pub const LEX_09: Self = Self("LEX-09");
    /// A `\u` or `\U` escape of a surrogate code point, U+D800 to U+DFFF, in a `str` literal shall give that code point, as CPython keeps it.
    pub const LEX_10: Self = Self("LEX-10");
    /// Brackets nested more than 200 deep and blocks nested more than 99 deep shall be rejected with CPython's messages, and expressions and statements nested more than 25000 deep shall be rejected, so that no program nests past what the compiler's stack holds.
    pub const LEX_11: Self = Self("LEX-11");
    /// `print` with `sep` and `end`, `len`, `range`, `abs`, `min`, `max`, `int`, `float`, `str`, `bool`, `repr`, `ord` and `chr` shall be available.
    pub const LIB_01: Self = Self("LIB-01");
    /// `list`, `dict`, `set`, `tuple`, `sorted`, `reversed`, `enumerate`, `zip`, `sum`, `any`, `all` and `round` shall be available.
    pub const LIB_02: Self = Self("LIB-02");
    /// The methods of `str`, `bytes`, `list`, `dict` and `set` listed in the design shall be available with Python's results.
    pub const LIB_03: Self = Self("LIB-03");
    /// `sys.argv`, `sys.exit`, `sys.stdout`, `sys.stderr`, the `file` argument of `print`, `input` and `open` for text files shall be available.
    pub const LIB_04: Self = Self("LIB-04");
    /// The `math` module shall be available.
    pub const LIB_05: Self = Self("LIB-05");
    /// The `dataclasses` names in CLS-05 and the `typing` names in TYP and GEN shall be available.
    pub const LIB_06: Self = Self("LIB-06");
    /// `os.environ`, `os.path`, `time` and `random` shall be available.
    pub const LIB_07: Self = Self("LIB-07");
    /// Importing a standard library module that TyPy does not provide shall be rejected with the list of modules it provides.
    pub const LIB_08: Self = Self("LIB-08");
    /// `str % values`, printf-style formatting with a value, a tuple of values or a dict of named values, shall be accepted with Python's results.
    pub const LIB_10: Self = Self("LIB-10");
    /// `format % values` shall be accepted with a format that is not a string literal, and with a `tuple[T, ...]` of values.
    pub const LIB_11: Self = Self("LIB-11");
    /// `math.fsum`, `math.prod`, `math.sumprod`, `math.dist`, `math.gamma` and `math.lgamma` shall be available.
    pub const LIB_12: Self = Self("LIB-12");
    /// `sys.maxsize`, `sys.byteorder`, `sys.platform`, `sys.version`, `sys.version_info`, `sys.hexversion`, `sys.maxunicode`, `sys.getrecursionlimit`, `sys.setrecursionlimit` and `sys.stdin` shall be available.
    pub const LIB_13: Self = Self("LIB-13");
    /// `open` shall accept binary modes, `+` modes, a mode that is not a literal, encodings other than UTF-8 and the arguments `buffering`, `errors`, `newline`, `closefd` and `opener`; a text file shall have `seek`, `tell`, `truncate`, `fileno`, `isatty`, `readable`, `writable` and `seekable`, and the attributes `name`, `mode` and `closed`; closing `sys.stdout` or `sys.stderr` shall close it.
    pub const LIB_14: Self = Self("LIB-14");
    /// `math.fsum`, `math.dist`, `math.prod` and `math.sumprod` shall take items of a union of number types, and `math.prod` a `bool` start.
    pub const LIB_15: Self = Self("LIB-15");
    /// `sys.version_info` shall be a value as CPython's: printed, stored, passed and imported with `from sys import version_info`.
    pub const LIB_16: Self = Self("LIB-16");
    /// The names `Optional`, `Union`, `List`, `Dict`, `Set`, `Tuple` and `Type` of `typing` shall be accepted in annotations as the types they spell; `Callable` follows function values (FUN-05).
    pub const LIB_17: Self = Self("LIB-17");
    /// The `opener` argument of `open` shall be accepted.
    pub const LIB_18: Self = Self("LIB-18");
    /// `open` with a binary mode shall accept a `buffering` that is not a literal, as `typing.BinaryIO`.
    pub const LIB_19: Self = Self("LIB-19");
    /// The abstract collections of `collections.abc` and `typing`, such as `Sequence`, `Mapping`, `MutableSequence` and `Collection`, shall be accepted as types, and a value of type `Iterable[T]` shall be testable against `Iterator`.
    pub const LIB_20: Self = Self("LIB-20");
    /// `math.prod` shall take items and a start other than numbers, multiplying them in turn as CPython does.
    pub const LIB_21: Self = Self("LIB-21");
    /// The allocator shall be replaceable per project through three C functions, allocate, reallocate and free, that `typy.toml` names.
    pub const MEM_08: Self = Self("MEM-08");
    /// A program shall be built from several `.py` files, found from the main file's directory and from paths set in `typy.toml`.
    pub const MOD_01: Self = Self("MOD-01");
    /// Packages with `__init__.py` and relative imports shall be accepted.
    pub const MOD_02: Self = Self("MOD-02");
    /// A circular import shall be rejected with the cycle shown.
    pub const MOD_05: Self = Self("MOD-05");
    /// `lazy import` and `lazy from ... import` are accepted at module level when the Python floor is 3.15 or later.
    pub const MOD_06: Self = Self("MOD-06");
    /// `sys.set_lazy_imports`, `sys.set_lazy_imports_filter` and `sys.lazy_modules` shall be rejected, because they change import behaviour from outside the importing module.
    pub const MOD_09: Self = Self("MOD-09");
    /// A name bound by a lazy import shall be usable from another module, assignable through a lazily imported module, and bindable by more than one lazy import.
    pub const MOD_11: Self = Self("MOD-11");
    /// A function or a class shall be bindable by more than one import of a module, eager or lazy, and by an import inside a block, a call running the one the import that ran last bound, and raising `NameError` when none has run.
    pub const MOD_12: Self = Self("MOD-12");
    /// A module shall be importable inside a function of a module that it imports at module level.
    pub const MOD_13: Self = Self("MOD-13");
    /// A name that imports bind to more than one function or class shall be usable as a value, in an annotation and from another module.
    pub const MOD_14: Self = Self("MOD-14");
    /// An `__init__` shall be able to import a module that imports its own module at module level.
    pub const MOD_15: Self = Self("MOD-15");
    /// The compiler shall not crash on any input; an internal error shall be reported as `E9xxx` with a hint to report it.
    pub const NFR_09: Self = Self("NFR-09");
    /// Assignment, annotated assignment and augmented assignment shall be accepted.
    pub const SMT_01: Self = Self("SMT-01");
    /// Unpacking assignment to several targets shall be accepted for tuples and lists.
    pub const SMT_02: Self = Self("SMT-02");
    /// `assert` shall raise `AssertionError` when its test is false, in every build mode.
    pub const SMT_04: Self = Self("SMT-04");
    /// `del` shall be accepted on list items, list slices and dict keys, and rejected on names and attributes.
    pub const SMT_05: Self = Self("SMT-05");
    /// `raise` shall be accepted with an exception instance or class, with and without `from`.
    pub const SMT_06: Self = Self("SMT-06");
    /// `import` and `from ... import` are accepted for TyPy modules and the supported standard library.
    pub const SMT_07: Self = Self("SMT-07");
    /// `global` shall be accepted for module-level variables.
    pub const SMT_08: Self = Self("SMT-08");
    /// `nonlocal` shall be accepted in nested functions.
    pub const SMT_09: Self = Self("SMT-09");
    /// The `type` statement shall define a type alias, with or without type parameters.
    pub const SMT_10: Self = Self("SMT-10");
    /// `yield` shall be accepted in generator functions.
    pub const SMT_11: Self = Self("SMT-11");
    /// A type alias shall be usable from another module, readable as a value as CPython's `TypeAliasType`, definable inside a function, a class or a block, and given type parameters with a bound or constraints.
    pub const SMT_12: Self = Self("SMT-12");
    /// TyPy shall accept only source that every oracle version, from the program's Python floor up, parses without a syntax error.
    pub const STD_01: Self = Self("STD-01");
    /// STD-02 shall cover only behaviour the Language Reference or the library documentation defines.
    pub const STD_03: Self = Self("STD-03");
    /// TyPy shall reject, with a diagnostic, any program that uses a feature it cannot compile with CPython's meaning.
    pub const STD_04: Self = Self("STD-04");
    /// Every function parameter except `self` and `cls`, and every function return, shall carry a type annotation.
    pub const TYP_01: Self = Self("TYP-01");
    /// Assigning a value of another type to a variable shall be rejected.
    pub const TYP_03: Self = Self("TYP-03");
    /// An empty list, dict or set display shall need an annotation that names its element types.
    pub const TYP_05: Self = Self("TYP-05");
    /// The annotations `list[T]`, `dict[K, V]`, `set[T]`, `tuple[...]` and `T \\| None` shall be accepted.
    pub const TYP_06: Self = Self("TYP-06");
    /// A name annotated `Final` shall not be assigned again.
    pub const TYP_08: Self = Self("TYP-08");
    /// An `int` value shall be rejected where a `float` is declared, because CPython keeps it an `int`: `x: float = 1` prints `1`.
    pub const TYP_09: Self = Self("TYP-09");
    /// `typing.Any`, `typing.cast` and unannotated `object` values shall be rejected.
    pub const TYP_10: Self = Self("TYP-10");
    /// A read of a local variable shall be rejected unless the variable is assigned on every path that reaches the read, so `UnboundLocalError` cannot occur.
    pub const TYP_12: Self = Self("TYP-12");
    /// A local variable or parameter of type `T | None` shall narrow to `T` where an `is None` or `is not None` test, or a truth test that holds, shows it is not `None`, and using it as `T` anywhere else shall be rejected.
    pub const TYP_13: Self = Self("TYP-13");
    /// Annotations that join two or more types other than `None` with `|`, such as `int | str`, shall be accepted, with narrowing by `isinstance` (TYP-07).
    pub const TYP_14: Self = Self("TYP-14");
    /// The annotations `Iterator[T]` and `Iterable[T]`, imported from `collections.abc` or `typing`, shall be accepted for iterators and for the built-in iterables of items of type `T`; their use as values is planned for 0.4 (ADR-0014, ADR-0015).
    pub const TYP_15: Self = Self("TYP-15");
    /// One value assigned to several targets, as in `a = b[0] = value`, shall be converted to each target's type, such as the union an item holds.
    pub const TYP_16: Self = Self("TYP-16");
    /// The names `Iterable` and `Iterator` of `typing` and `collections.abc`, and `List`, `Dict`, `Set`, `Tuple`, `Type`, `Optional[...]` and `Union[...]` of `typing`, shall be accepted as the classes of `isinstance` and of a class pattern, with CPython's answers.
    pub const TYP_17: Self = Self("TYP-17");
    /// The names of `typing` and `collections.abc` shall be values of their own: stored, passed, compared and written as CPython writes them; so shall a type alias that a function or a class defines, an alias given its type arguments, and an alias's `__value__` and `__type_params__`.
    pub const TYP_18: Self = Self("TYP-18");
    /// An operation on a union whose members the runtime cannot tell apart by class or kind, such as two iterators, two file kinds or an iterable, shall be accepted.
    pub const TYP_19: Self = Self("TYP-19");

    /// Every rule a diagnostic can name, in ID order.
    pub const ALL: &'static [Self] = &[
        Self::A_01,
        Self::CLI_07,
        Self::CLI_12,
        Self::CLS_01,
        Self::CLS_02,
        Self::CLS_04,
        Self::CLS_05,
        Self::CLS_06,
        Self::CLS_07,
        Self::CLS_08,
        Self::CLS_09,
        Self::CLS_11,
        Self::CLS_12,
        Self::CLS_13,
        Self::CLS_14,
        Self::CLS_15,
        Self::CLS_16,
        Self::CLS_18,
        Self::CLS_19,
        Self::CLS_20,
        Self::CLS_21,
        Self::CLS_22,
        Self::CLS_23,
        Self::CLS_24,
        Self::CLS_25,
        Self::CLS_26,
        Self::CLS_27,
        Self::CLS_28,
        Self::CLS_29,
        Self::CLS_30,
        Self::CLS_31,
        Self::CLS_32,
        Self::CLS_33,
        Self::CLS_34,
        Self::CLS_35,
        Self::CLS_36,
        Self::CST_03,
        Self::CST_04,
        Self::CST_05,
        Self::CST_06,
        Self::CST_07,
        Self::CST_08,
        Self::CST_09,
        Self::CST_10,
        Self::CST_11,
        Self::DAT_03,
        Self::DAT_05,
        Self::DAT_06,
        Self::DAT_07,
        Self::DAT_08,
        Self::DAT_09,
        Self::DAT_10,
        Self::DAT_12,
        Self::DAT_13,
        Self::DAT_14,
        Self::DAT_15,
        Self::DAT_16,
        Self::DAT_17,
        Self::DIA_08,
        Self::EXC_01,
        Self::EXC_06,
        Self::EXC_07,
        Self::EXC_09,
        Self::EXC_12,
        Self::EXC_14,
        Self::EXC_15,
        Self::EXC_16,
        Self::EXC_17,
        Self::EXC_18,
        Self::EXP_02,
        Self::EXP_04,
        Self::EXP_06,
        Self::EXP_07,
        Self::EXP_08,
        Self::EXP_09,
        Self::EXP_10,
        Self::EXP_11,
        Self::EXP_12,
        Self::EXP_13,
        Self::EXP_14,
        Self::EXP_15,
        Self::EXP_16,
        Self::EXP_17,
        Self::FFI_01,
        Self::FUN_01,
        Self::FUN_03,
        Self::FUN_04,
        Self::FUN_05,
        Self::FUN_06,
        Self::GEN_01,
        Self::GEN_02,
        Self::GEN_03,
        Self::GEN_04,
        Self::GEN_05,
        Self::GEN_06,
        Self::GEN_07,
        Self::GEN_08,
        Self::GEN_09,
        Self::GEN_10,
        Self::GEN_11,
        Self::GEN_12,
        Self::GEN_13,
        Self::GEN_14,
        Self::GEN_15,
        Self::GEN_16,
        Self::GEN_17,
        Self::LEX_01,
        Self::LEX_06,
        Self::LEX_07,
        Self::LEX_08,
        Self::LEX_09,
        Self::LEX_10,
        Self::LEX_11,
        Self::LIB_01,
        Self::LIB_02,
        Self::LIB_03,
        Self::LIB_04,
        Self::LIB_05,
        Self::LIB_06,
        Self::LIB_07,
        Self::LIB_08,
        Self::LIB_10,
        Self::LIB_11,
        Self::LIB_12,
        Self::LIB_13,
        Self::LIB_14,
        Self::LIB_15,
        Self::LIB_16,
        Self::LIB_17,
        Self::LIB_18,
        Self::LIB_19,
        Self::LIB_20,
        Self::LIB_21,
        Self::MEM_08,
        Self::MOD_01,
        Self::MOD_02,
        Self::MOD_05,
        Self::MOD_06,
        Self::MOD_09,
        Self::MOD_11,
        Self::MOD_12,
        Self::MOD_13,
        Self::MOD_14,
        Self::MOD_15,
        Self::NFR_09,
        Self::SMT_01,
        Self::SMT_02,
        Self::SMT_04,
        Self::SMT_05,
        Self::SMT_06,
        Self::SMT_07,
        Self::SMT_08,
        Self::SMT_09,
        Self::SMT_10,
        Self::SMT_11,
        Self::SMT_12,
        Self::STD_01,
        Self::STD_03,
        Self::STD_04,
        Self::TYP_01,
        Self::TYP_03,
        Self::TYP_05,
        Self::TYP_06,
        Self::TYP_08,
        Self::TYP_09,
        Self::TYP_10,
        Self::TYP_12,
        Self::TYP_13,
        Self::TYP_14,
        Self::TYP_15,
        Self::TYP_16,
        Self::TYP_17,
        Self::TYP_18,
        Self::TYP_19,
    ];

    /// Returns the ID as text, such as `TYP-03`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl fmt::Display for RuleId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::RuleId;

    fn specification() -> String {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/requirements/srs.md"
        );
        std::fs::read_to_string(path).expect("the specification is part of the repository")
    }

    #[test]
    fn every_rule_is_an_id_defined_in_the_specification() {
        let specification = specification();
        for rule in RuleId::ALL {
            let row = format!("\n| {rule} |");
            assert!(
                specification.contains(&row),
                "{rule} is not defined in srs.md"
            );
        }
    }

    #[test]
    fn the_list_of_rules_is_sorted_and_has_no_duplicates() {
        let ids: Vec<&str> = RuleId::ALL.iter().map(|rule| rule.as_str()).collect();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(ids, sorted);
    }
}
