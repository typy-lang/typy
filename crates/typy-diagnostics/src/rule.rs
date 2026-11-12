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
    /// `for` shall iterate over `range`, `str`, `bytes`, `list`, `tuple`, `dict`, `set` and any class with `__iter__` and `__next__`.
    pub const CST_03: Self = Self("CST-03");
    /// `try` with `except`, `else` and `finally` clauses shall be accepted, and `finally` shall run on `return`, `break`, `continue` and exceptions.
    pub const CST_04: Self = Self("CST-04");
    /// `with` shall call `__enter__` and `__exit__` as the Language Reference defines, also when the block exits early.
    pub const CST_05: Self = Self("CST-05");
    /// `match` shall be accepted with literal, capture, wildcard, value, sequence, mapping and class patterns, guards and `\\|` alternatives.
    pub const CST_06: Self = Self("CST-06");
    /// `for` shall iterate over any class with `__iter__` and `__next__`, and end when `__next__` raises `StopIteration` (OI-17).
    pub const CST_08: Self = Self("CST-08");
    /// `iter` and `next`, with and without a default, shall be available for the built-in iterables.
    pub const CST_09: Self = Self("CST-09");
    /// The parameters of `__exit__` after `self` may be left without annotations, and shall then have the types `type[BaseException] \| None`, `BaseException \| None` and `TracebackType \| None`; the traceback shall be one object of type `traceback`, written as CPython writes one, which holds no frames.
    pub const CST_10: Self = Self("CST-10");
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
    /// A program shall be built from several `.py` files, found from the main file's directory and from paths set in `typy.toml`.
    /// A program shall be built from several `.py` files, found from the main file's directory and from paths set in `typy.toml`.
    pub const MOD_01: Self = Self("MOD-01");
    /// Packages with `__init__.py` and relative imports shall be accepted.
    pub const MOD_02: Self = Self("MOD-02");
    /// A circular import shall be rejected with the cycle shown.
    pub const MOD_05: Self = Self("MOD-05");
    /// `lazy import` and `lazy from ... import` are accepted at module level when the Python floor is 3.15 or later.
    pub const MOD_06: Self = Self("MOD-06");
    /// A name bound by a lazy import shall be usable from another module, assignable through a lazily imported module, and bindable by more than one lazy import.
    pub const MOD_11: Self = Self("MOD-11");
    /// The compiler shall not crash on any input; an internal error shall be reported as `E9xxx` with a hint to report it.
    pub const NFR_09: Self = Self("NFR-09");
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
        Self::CST_03,
        Self::CST_04,
        Self::CST_05,
        Self::CST_06,
        Self::CST_08,
        Self::CST_09,
        Self::CST_10,
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
        Self::LEX_01,
        Self::LEX_06,
        Self::LEX_07,
        Self::LEX_08,
        Self::LEX_09,
        Self::LIB_01,
        Self::LIB_02,
        Self::LIB_03,
        Self::LIB_04,
        Self::LIB_05,
        Self::LIB_06,
        Self::LIB_07,
        Self::LIB_08,
        Self::LIB_10,
        Self::MOD_01,
        Self::MOD_02,
        Self::MOD_05,
        Self::MOD_06,
        Self::MOD_11,
        Self::NFR_09,
        Self::SMT_02,
        Self::SMT_04,
        Self::SMT_05,
        Self::SMT_06,
        Self::SMT_07,
        Self::SMT_08,
        Self::SMT_09,
        Self::SMT_10,
        Self::SMT_11,
        Self::STD_01,
        Self::STD_03,
        Self::STD_04,
        Self::TYP_01,
        Self::TYP_03,
        Self::TYP_05,
        Self::TYP_06,
        Self::TYP_09,
        Self::TYP_10,
        Self::TYP_12,
        Self::TYP_13,
        Self::TYP_14,
        Self::TYP_15,
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
