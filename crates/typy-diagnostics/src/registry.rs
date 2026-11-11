//! The table of every diagnostic code, its summary, its rule and its status.

use crate::code::DiagnosticCode;
use crate::rule::RuleId;

/// Whether a code is in use (DIA-06).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodeStatus {
    /// The compiler reports this code.
    Active,
    /// No longer reported; the code stays reserved and is never reused.
    Retired {
        /// The code that took its place, if any.
        replaced_by: Option<DiagnosticCode>,
    },
}

/// What the registry knows about one code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CodeInfo {
    /// The code.
    pub code: DiagnosticCode,
    /// One line that says what the code means.
    pub summary: &'static str,
    /// The requirement the code enforces, if it enforces one.
    pub rule: Option<RuleId>,
    /// Whether the code is in use.
    pub status: CodeStatus,
}

const fn active(number: u16, summary: &'static str, rule: RuleId) -> CodeInfo {
    CodeInfo {
        code: DiagnosticCode::error(number),
        summary,
        rule: Some(rule),
        status: CodeStatus::Active,
    }
}

/// Every diagnostic code, sorted by code (detailed design section 4.3).
pub const CODES: &[CodeInfo] = &[
    active(101, "source file larger than 16 MiB", RuleId::LEX_09),
    active(102, "source file is not valid UTF-8", RuleId::LEX_01),
    active(
        103,
        "Python syntax error, or syntax newer than the Python floor",
        RuleId::STD_01,
    ),
    active(104, "imaginary literal", RuleId::LEX_08),
    CodeInfo {
        code: DiagnosticCode::error(105),
        summary: "source file cannot be read",
        rule: None,
        status: CodeStatus::Active,
    },
    CodeInfo {
        code: DiagnosticCode::error(106),
        summary: "output file cannot be written",
        rule: None,
        status: CodeStatus::Active,
    },
    active(
        201,
        "feature planned for a later TyPy release",
        RuleId::STD_04,
    ),
    active(
        202,
        "feature TyPy cannot compile with CPython's meaning",
        RuleId::STD_04,
    ),
    active(203, "parameter without a type annotation", RuleId::TYP_01),
    active(204, "function without a return annotation", RuleId::TYP_01),
    active(205, "name is not defined", RuleId::STD_04),
    active(
        206,
        "import of a module TyPy does not provide",
        RuleId::LIB_08,
    ),
    active(
        301,
        "`is` used on values other than `None` or class instances",
        RuleId::EXP_08,
    ),
    active(
        302,
        "`**` whose result in CPython could have another type",
        RuleId::EXP_04,
    ),
    active(
        303,
        "`and` or `or` with operands of different types",
        RuleId::EXP_09,
    ),
    active(
        304,
        "truth test on a value of an unsupported type",
        RuleId::EXP_10,
    ),
    active(
        305,
        "conditional expression with branches of different types",
        RuleId::EXP_11,
    ),
    active(
        306,
        "variable read before it is assigned on every path",
        RuleId::TYP_12,
    ),
    active(
        307,
        "tuple indexed other than by an integer literal inside it",
        RuleId::DAT_08,
    ),
    active(
        308,
        "unpacking into a different number of targets",
        RuleId::SMT_02,
    ),
    active(
        309,
        "instance used, or an attribute read, before `__init__` assigns every attribute",
        RuleId::CLS_01,
    ),
    active(
        310,
        "attribute whose type is not known yet where it is read",
        RuleId::CLS_01,
    ),
    active(
        401,
        "operator not defined for these operand types",
        RuleId::EXP_02,
    ),
    active(
        402,
        "assignment changes the type of a variable",
        RuleId::TYP_03,
    ),
    active(403, "`Any`, `cast` or an untyped value", RuleId::TYP_10),
    active(404, "argument of the wrong type", RuleId::TYP_01),
    active(405, "returned value of the wrong type", RuleId::TYP_01),
    active(406, "arithmetic on `bool`", RuleId::DAT_03),
    active(407, "comparison between unsupported types", RuleId::EXP_06),
    active(
        408,
        "`tuple`, `list`, `set` or `dict` annotation without its item types",
        RuleId::TYP_06,
    ),
    active(
        409,
        "list, set or dict display with items of different types",
        RuleId::DAT_07,
    ),
    active(
        410,
        "empty collection without an annotation that names its item types",
        RuleId::TYP_05,
    ),
    active(
        411,
        "`for` over a value that cannot be iterated",
        RuleId::CST_03,
    ),
    active(412, "method a built-in type does not have", RuleId::LIB_03),
    active(
        413,
        "set item or dict key of a type without a hash",
        RuleId::DAT_10,
    ),
    active(
        414,
        "attribute or method a class does not have",
        RuleId::CLS_02,
    ),
    active(
        415,
        "`raise` or `from` of a value that is not an exception",
        RuleId::SMT_06,
    ),
    active(
        416,
        "bare `raise` outside an `except` clause",
        RuleId::EXC_07,
    ),
    active(
        417,
        "call of an exception class TyPy does not let a program build",
        RuleId::EXC_12,
    ),
    active(
        418,
        "`except` clause that names something other than exception classes",
        RuleId::EXC_09,
    ),
    active(
        419,
        "`return`, `break` or `continue` that leaves a `finally` block",
        RuleId::EXC_06,
    ),
    active(
        420,
        "`with` on a value whose class has no `__enter__` and `__exit__`",
        RuleId::CST_05,
    ),
    active(
        421,
        "`del` of something other than a list item, a list slice or a dict key",
        RuleId::SMT_05,
    ),
    active(701, "C toolchain program not found", RuleId::A_01),
    active(702, "link failed", RuleId::A_01),
    CodeInfo {
        code: DiagnosticCode::error(703),
        summary: "runtime archive missing, or from another TyPy version",
        rule: None,
        status: CodeStatus::Active,
    },
    active(9001, "internal compiler error", RuleId::NFR_09),
];

/// Returns what the registry knows about `code`, or `None` for an unknown code.
#[must_use]
pub fn lookup(code: DiagnosticCode) -> Option<&'static CodeInfo> {
    CODES
        .binary_search_by(|info| info.code.cmp(&code))
        .ok()
        .map(|index| &CODES[index])
}

#[cfg(test)]
mod tests {
    use super::{CODES, lookup};
    use crate::code::DiagnosticCode;

    #[test]
    fn codes_are_sorted_and_unique() {
        for pair in CODES.windows(2) {
            assert!(
                pair[0].code < pair[1].code,
                "{} then {}",
                pair[0].code,
                pair[1].code
            );
        }
    }

    #[test]
    fn every_code_is_found_by_lookup() {
        for info in CODES {
            assert_eq!(lookup(info.code), Some(info));
        }
    }

    #[test]
    fn unknown_code_is_not_found() {
        assert_eq!(lookup(DiagnosticCode::error(9999)), None);
        assert_eq!(lookup(DiagnosticCode::warning(101)), None);
    }

    #[test]
    fn summaries_start_in_lower_case_unless_with_a_proper_name_and_have_no_final_period() {
        const PROPER_NAMES: [&str; 2] = ["C", "Python"];
        for info in CODES {
            let first_word = info.summary.split(' ').next().unwrap();
            let first = first_word.chars().next().unwrap();
            assert!(
                !first.is_uppercase() || PROPER_NAMES.contains(&first_word),
                "{}",
                info.code
            );
            assert!(!info.summary.ends_with('.'), "{}", info.code);
        }
    }
}
