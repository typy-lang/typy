//! The runtime functions TyIR calls, with their signatures (detailed design section 14).

use crate::types::TyirType;

/// A function of the runtime library. The runtime's symbol is `typy_rt_` followed by `name()`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[allow(
    missing_docs,
    reason = "each variant is described by its row in `signature`"
)]
pub enum RuntimeFunction {
    IntFloorDivide,
    IntModulo,
    IntPower,
    IntLeftShift,
    IntRightShift,
    IntTrueDivide,
    IntToFloat,
    IntAbsolute,
    IntToStr,
    IntFromStr,
    CompareIntFloat,
    FloatDivide,
    FloatFloorDivide,
    FloatModulo,
    FloatPower,
    FloatToInt,
    FloatToStr,
    FloatFromStr,
    StrConcat,
    StrRepeat,
    StrIndex,
    StrLength,
    StrCompare,
    StrRepr,
    Chr,
    Ord,
    WriteStdout,
    CheckStack,
}

/// The parameter types, result type and raising behaviour of a runtime function.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeSignature {
    /// The parameter types, in order.
    pub parameters: &'static [TyirType],
    /// The result type.
    pub result: TyirType,
    /// True when the function may raise, so it is called with `invoke` (ADR-0004).
    pub may_raise: bool,
}

const fn signature(
    parameters: &'static [TyirType],
    result: TyirType,
    may_raise: bool,
) -> RuntimeSignature {
    RuntimeSignature {
        parameters,
        result,
        may_raise,
    }
}

const INT_INT: &[TyirType] = &[TyirType::Int, TyirType::Int];
const FLOAT_FLOAT: &[TyirType] = &[TyirType::Float, TyirType::Float];

impl RuntimeFunction {
    /// Every runtime function.
    pub const ALL: [Self; 28] = [
        Self::IntFloorDivide,
        Self::IntModulo,
        Self::IntPower,
        Self::IntLeftShift,
        Self::IntRightShift,
        Self::IntTrueDivide,
        Self::IntToFloat,
        Self::IntAbsolute,
        Self::IntToStr,
        Self::IntFromStr,
        Self::CompareIntFloat,
        Self::FloatDivide,
        Self::FloatFloorDivide,
        Self::FloatModulo,
        Self::FloatPower,
        Self::FloatToInt,
        Self::FloatToStr,
        Self::FloatFromStr,
        Self::StrConcat,
        Self::StrRepeat,
        Self::StrIndex,
        Self::StrLength,
        Self::StrCompare,
        Self::StrRepr,
        Self::Chr,
        Self::Ord,
        Self::WriteStdout,
        Self::CheckStack,
    ];

    /// Returns the function's signature. Comparisons take the comparison as an `int` operand, numbered as `Comparison::code`.
    #[must_use]
    pub const fn signature(self) -> RuntimeSignature {
        use TyirType::{Bool, Float, Int, Unit};
        const STR: TyirType = TyirType::STR;
        match self {
            Self::IntFloorDivide
            | Self::IntModulo
            | Self::IntPower
            | Self::IntLeftShift
            | Self::IntRightShift => signature(INT_INT, Int, true),
            Self::IntTrueDivide => signature(INT_INT, Float, true),
            Self::IntToFloat => signature(&[Int], Float, true),
            Self::IntAbsolute => signature(&[Int], Int, false),
            Self::IntToStr => signature(&[Int], STR, false),
            Self::IntFromStr => signature(&[STR, Int], Int, true),
            Self::CompareIntFloat => signature(&[Int, Float, Int], Bool, false),
            Self::FloatDivide | Self::FloatFloorDivide | Self::FloatModulo | Self::FloatPower => {
                signature(FLOAT_FLOAT, Float, true)
            }
            Self::FloatToInt => signature(&[Float], Int, true),
            Self::FloatToStr => signature(&[Float], STR, false),
            Self::FloatFromStr => signature(&[STR], Float, true),
            Self::StrConcat => signature(&[STR, STR], STR, false),
            Self::StrRepeat | Self::StrIndex => signature(&[STR, Int], STR, true),
            Self::StrLength => signature(&[STR], Int, false),
            Self::StrCompare => signature(&[STR, STR, Int], Bool, false),
            Self::StrRepr => signature(&[STR], STR, false),
            Self::Chr => signature(&[Int], STR, true),
            Self::Ord => signature(&[STR], Int, true),
            Self::WriteStdout => signature(&[STR], Unit, false),
            Self::CheckStack => signature(&[], Unit, true),
        }
    }

    /// Returns the name after `typy_rt_`, such as `int_floor_divide`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::IntFloorDivide => "int_floor_divide",
            Self::IntModulo => "int_modulo",
            Self::IntPower => "int_power",
            Self::IntLeftShift => "int_left_shift",
            Self::IntRightShift => "int_right_shift",
            Self::IntTrueDivide => "int_true_divide",
            Self::IntToFloat => "int_to_float",
            Self::IntAbsolute => "int_absolute",
            Self::IntToStr => "int_to_str",
            Self::IntFromStr => "int_from_str",
            Self::CompareIntFloat => "compare_int_float",
            Self::FloatDivide => "float_divide",
            Self::FloatFloorDivide => "float_floor_divide",
            Self::FloatModulo => "float_modulo",
            Self::FloatPower => "float_power",
            Self::FloatToInt => "float_to_int",
            Self::FloatToStr => "float_to_str",
            Self::FloatFromStr => "float_from_str",
            Self::StrConcat => "str_concat",
            Self::StrRepeat => "str_repeat",
            Self::StrIndex => "str_index",
            Self::StrLength => "str_length",
            Self::StrCompare => "str_compare",
            Self::StrRepr => "str_repr",
            Self::Chr => "chr",
            Self::Ord => "ord",
            Self::WriteStdout => "write_stdout",
            Self::CheckStack => "check_stack",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RuntimeFunction;

    #[test]
    fn names_are_unique_lower_case_identifiers() {
        let mut names: Vec<&str> = RuntimeFunction::ALL
            .iter()
            .map(|function| function.name())
            .collect();
        assert!(
            names
                .iter()
                .all(|name| name.chars().all(|c| c.is_ascii_lowercase() || c == '_'))
        );
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), RuntimeFunction::ALL.len());
    }

    #[test]
    fn operations_that_can_fail_in_python_may_raise() {
        for function in [
            RuntimeFunction::IntFloorDivide,
            RuntimeFunction::StrIndex,
            RuntimeFunction::CheckStack,
        ] {
            assert!(function.signature().may_raise, "{function:?}");
        }
        for function in [
            RuntimeFunction::StrConcat,
            RuntimeFunction::WriteStdout,
            RuntimeFunction::IntToStr,
        ] {
            assert!(!function.signature().may_raise, "{function:?}");
        }
    }
}
