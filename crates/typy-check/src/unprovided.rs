//! How the checker treats the CPython builtin names that TyPy 0.1 does not provide.

use typy_diagnostics::RuleId;

/// How the checker treats a CPython builtin name that 0.1 does not provide.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Unprovided {
    /// Provided by a later release.
    Planned {
        /// The release, such as `0.2`.
        release: &'static str,
        /// The requirement that provides it.
        rule: RuleId,
    },
    /// Not provided: it cannot be compiled with CPython's meaning, or no requirement provides it (STD-04).
    Excluded,
}

/// Returns how a builtin name of CPython 3.12 that 0.1 does not provide is treated, or `None` for a name that is not a builtin.
#[must_use]
pub(crate) fn unprovided(name: &str) -> Option<Unprovided> {
    const fn planned(release: &'static str, rule: RuleId) -> Unprovided {
        Unprovided::Planned { release, rule }
    }
    match name {
        "list" | "dict" | "set" | "tuple" | "sorted" | "reversed" | "enumerate" | "zip" | "sum"
        | "any" | "all" | "round" => Some(planned("0.2", RuleId::LIB_02)),
        "bytes" => Some(planned("0.2", RuleId::DAT_06)),
        "iter" | "next" => Some(planned("0.2", RuleId::CST_03)),
        "input" | "open" => Some(planned("0.3", RuleId::LIB_04)),
        "isinstance" | "issubclass" => Some(planned("0.4", RuleId::CLS_09)),
        "property" | "staticmethod" | "classmethod" | "super" => {
            Some(planned("0.4", RuleId::CLS_01))
        }
        "callable" | "map" | "filter" => Some(planned("1.0", RuleId::FUN_05)),
        _ if is_exception_class(name) => Some(planned("0.3", RuleId::EXC_01)),
        _ if CPYTHON_3_12_BUILTINS.contains(&name) => Some(Unprovided::Excluded),
        _ => None,
    }
}

fn is_exception_class(name: &str) -> bool {
    CPYTHON_3_12_BUILTINS.contains(&name)
        && (name.ends_with("Error")
            || name.ends_with("Exception")
            || name.ends_with("Warning")
            || name.ends_with("ExceptionGroup")
            || matches!(
                name,
                "GeneratorExit"
                    | "KeyboardInterrupt"
                    | "StopIteration"
                    | "StopAsyncIteration"
                    | "SystemExit"
            ))
}

/// The names in CPython 3.12's `builtins` module, with the site module's additions, that a program can refer to.
const CPYTHON_3_12_BUILTINS: &[&str] = &[
    "ArithmeticError",
    "AssertionError",
    "AttributeError",
    "BaseException",
    "BaseExceptionGroup",
    "BlockingIOError",
    "BrokenPipeError",
    "BufferError",
    "BytesWarning",
    "ChildProcessError",
    "ConnectionAbortedError",
    "ConnectionError",
    "ConnectionRefusedError",
    "ConnectionResetError",
    "DeprecationWarning",
    "EOFError",
    "Ellipsis",
    "EncodingWarning",
    "EnvironmentError",
    "Exception",
    "ExceptionGroup",
    "FileExistsError",
    "FileNotFoundError",
    "FloatingPointError",
    "FutureWarning",
    "GeneratorExit",
    "IOError",
    "ImportError",
    "ImportWarning",
    "IndentationError",
    "IndexError",
    "InterruptedError",
    "IsADirectoryError",
    "KeyError",
    "KeyboardInterrupt",
    "LookupError",
    "MemoryError",
    "ModuleNotFoundError",
    "NameError",
    "NotADirectoryError",
    "NotImplemented",
    "NotImplementedError",
    "OSError",
    "OverflowError",
    "PendingDeprecationWarning",
    "PermissionError",
    "ProcessLookupError",
    "RecursionError",
    "ReferenceError",
    "ResourceWarning",
    "RuntimeError",
    "RuntimeWarning",
    "StopAsyncIteration",
    "StopIteration",
    "SyntaxError",
    "SyntaxWarning",
    "SystemError",
    "SystemExit",
    "TabError",
    "TimeoutError",
    "TypeError",
    "UnboundLocalError",
    "UnicodeDecodeError",
    "UnicodeEncodeError",
    "UnicodeError",
    "UnicodeTranslateError",
    "UnicodeWarning",
    "UserWarning",
    "ValueError",
    "Warning",
    "ZeroDivisionError",
    "__build_class__",
    "__debug__",
    "__import__",
    "abs",
    "aiter",
    "all",
    "anext",
    "any",
    "ascii",
    "bin",
    "bool",
    "breakpoint",
    "bytearray",
    "bytes",
    "callable",
    "chr",
    "classmethod",
    "compile",
    "complex",
    "copyright",
    "credits",
    "delattr",
    "dict",
    "dir",
    "divmod",
    "enumerate",
    "eval",
    "exec",
    "exit",
    "filter",
    "float",
    "format",
    "frozenset",
    "getattr",
    "globals",
    "hasattr",
    "hash",
    "help",
    "hex",
    "id",
    "input",
    "int",
    "isinstance",
    "issubclass",
    "iter",
    "len",
    "license",
    "list",
    "locals",
    "map",
    "max",
    "memoryview",
    "min",
    "next",
    "object",
    "oct",
    "open",
    "ord",
    "pow",
    "print",
    "property",
    "quit",
    "range",
    "repr",
    "reversed",
    "round",
    "set",
    "setattr",
    "slice",
    "sorted",
    "staticmethod",
    "str",
    "sum",
    "super",
    "tuple",
    "type",
    "vars",
    "zip",
];

#[cfg(test)]
mod tests {
    use super::{CPYTHON_3_12_BUILTINS, Unprovided, unprovided};
    use typy_diagnostics::RuleId;

    #[test]
    fn later_builtins_name_their_release_and_rule() {
        let cases = [
            ("list", "0.2", RuleId::LIB_02),
            ("bytes", "0.2", RuleId::DAT_06),
            ("next", "0.2", RuleId::CST_03),
            ("open", "0.3", RuleId::LIB_04),
            ("ValueError", "0.3", RuleId::EXC_01),
            ("KeyboardInterrupt", "0.3", RuleId::EXC_01),
            ("isinstance", "0.4", RuleId::CLS_09),
            ("super", "0.4", RuleId::CLS_01),
            ("map", "1.0", RuleId::FUN_05),
        ];
        for (name, release, rule) in cases {
            assert_eq!(
                unprovided(name),
                Some(Unprovided::Planned { release, rule }),
                "{name}"
            );
        }
    }

    #[test]
    fn dynamic_and_unlisted_builtins_are_excluded() {
        for name in [
            "eval",
            "exec",
            "compile",
            "__import__",
            "globals",
            "getattr",
            "id",
            "hash",
            "hex",
            "object",
        ] {
            assert_eq!(unprovided(name), Some(Unprovided::Excluded), "{name}");
        }
    }

    #[test]
    fn names_that_are_not_builtins_are_not_classified() {
        for name in ["frozendict", "sentinel", "total", "Print"] {
            assert_eq!(unprovided(name), None, "{name}");
        }
    }

    #[test]
    fn the_name_list_is_sorted_without_duplicates() {
        let mut sorted = CPYTHON_3_12_BUILTINS.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted, CPYTHON_3_12_BUILTINS);
    }

    #[test]
    fn every_builtin_of_0_1_is_a_cpython_builtin() {
        for builtin in [
            "abs", "bool", "chr", "float", "int", "len", "max", "min", "ord", "print", "range",
            "repr", "str",
        ] {
            assert!(CPYTHON_3_12_BUILTINS.contains(&builtin), "{builtin}");
        }
    }
}
