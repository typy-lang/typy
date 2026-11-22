//! Tests of `runtime`.

use super::RuntimeFunction;

#[test]
fn names_are_unique_lower_case_identifiers() {
    let mut names: Vec<&str> = RuntimeFunction::ALL
        .iter()
        .map(|function| function.name())
        .collect();
    // The C names of `math`, such as `log10`, hold digits after a letter.
    assert!(
        names
            .iter()
            .all(|name| name.starts_with(|c: char| c.is_ascii_lowercase()))
    );
    assert!(names.iter().all(|name| {
        name.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    }));
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), RuntimeFunction::ALL.len());
}

#[test]
fn operations_that_can_fail_in_python_may_raise() {
    let raising = [
        RuntimeFunction::IntFloorDivide,
        RuntimeFunction::StrIndex,
        RuntimeFunction::CheckStack,
        RuntimeFunction::IntToStr,
        RuntimeFunction::RangeCheckStep,
        RuntimeFunction::ExceptionThrow,
    ];
    for function in raising {
        assert!(function.signature().may_raise, "{function:?}");
    }
    let safe = [
        RuntimeFunction::StrConcat,
        RuntimeFunction::WriteStdout,
        RuntimeFunction::FloatAbsolute,
    ];
    for function in safe {
        assert!(!function.signature().may_raise, "{function:?}");
    }
}
