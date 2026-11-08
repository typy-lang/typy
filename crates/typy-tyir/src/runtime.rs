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
    FloatAbsolute,
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
    RangeCheckStep,
    ListNew,
    ListAppend,
    ListLength,
    ListGet,
    ListSet,
    ListEqual,
    ListContains,
    ListRepr,
    ListSlice,
    StrSlice,
    ListPop,
    ListInsert,
    ListExtend,
    ListRemove,
    ListIndex,
    ListCount,
    ListClear,
    ListCopy,
    ListReverse,
    ListConcat,
    ListRepeat,
    ListCompare,
    ListSort,
    ListExtreme,
    ListSumInt,
    ListSumFloat,
    ListAny,
    ListAll,
    ZipUnequal,
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
    pub const ALL: [Self; 59] = [
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
        Self::FloatAbsolute,
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
        Self::RangeCheckStep,
        Self::ListNew,
        Self::ListAppend,
        Self::ListLength,
        Self::ListGet,
        Self::ListSet,
        Self::ListEqual,
        Self::ListContains,
        Self::ListRepr,
        Self::ListSlice,
        Self::StrSlice,
        Self::ListPop,
        Self::ListInsert,
        Self::ListExtend,
        Self::ListRemove,
        Self::ListIndex,
        Self::ListCount,
        Self::ListClear,
        Self::ListCopy,
        Self::ListReverse,
        Self::ListConcat,
        Self::ListRepeat,
        Self::ListCompare,
        Self::ListSort,
        Self::ListExtreme,
        Self::ListSumInt,
        Self::ListSumFloat,
        Self::ListAny,
        Self::ListAll,
        Self::ZipUnequal,
    ];

    /// Returns the function's signature. Comparisons take the comparison as an `int` operand, numbered as `Comparison::code`.
    #[must_use]
    pub const fn signature(self) -> RuntimeSignature {
        use TyirType::{Bits, Bool, Float, Int, Unit};
        const STR: TyirType = TyirType::STR;
        const LIST: TyirType = TyirType::LIST;
        match self {
            Self::IntFloorDivide
            | Self::IntModulo
            | Self::IntPower
            | Self::IntLeftShift
            | Self::IntRightShift => signature(INT_INT, Int, true),
            Self::IntTrueDivide => signature(INT_INT, Float, true),
            Self::IntToFloat => signature(&[Int], Float, true),
            Self::IntAbsolute => signature(&[Int], Int, false),
            // CPython refuses to convert an int of more than 4300 digits (sys.int_info), and
            // `chr` refuses a code point outside Unicode.
            Self::IntToStr | Self::Chr => signature(&[Int], STR, true),
            Self::IntFromStr => signature(&[STR, Int], Int, true),
            Self::CompareIntFloat => signature(&[Int, Float, Int], Bool, false),
            Self::FloatDivide | Self::FloatFloorDivide | Self::FloatModulo | Self::FloatPower => {
                signature(FLOAT_FLOAT, Float, true)
            }
            Self::FloatAbsolute => signature(&[Float], Float, false),
            Self::FloatToInt => signature(&[Float], Int, true),
            Self::FloatToStr => signature(&[Float], STR, false),
            Self::FloatFromStr => signature(&[STR], Float, true),
            Self::StrConcat => signature(&[STR, STR], STR, false),
            Self::StrRepeat | Self::StrIndex => signature(&[STR, Int], STR, true),
            Self::StrLength => signature(&[STR], Int, false),
            Self::StrCompare => signature(&[STR, STR, Int], Bool, false),
            Self::StrRepr => signature(&[STR], STR, false),
            Self::Ord => signature(&[STR], Int, true),
            Self::WriteStdout => signature(&[STR], Unit, false),
            Self::CheckStack => signature(&[], Unit, true),
            Self::RangeCheckStep => signature(&[Int], Unit, true),
            Self::ListNew => signature(&[Int, Int, Bits], LIST, false),
            Self::ListAppend => signature(&[LIST, Bits], Unit, false),
            Self::ListLength => signature(&[LIST], Int, false),
            Self::ListGet | Self::ListExtreme => signature(&[LIST, Int], Bits, true),
            Self::ListSet => signature(&[LIST, Int, Bits], Unit, true),
            Self::ListEqual => signature(&[LIST, LIST], Bool, false),
            Self::ListContains => signature(&[LIST, Bits], Bool, false),
            Self::ListRepr => signature(&[LIST], STR, true),
            Self::ListSlice => signature(&[LIST, Int, Int, Int, Int], LIST, true),
            Self::StrSlice => signature(&[STR, Int, Int, Int, Int], STR, true),
            Self::ListPop => signature(&[LIST, Int, Int], Bits, true),
            Self::ListInsert => signature(&[LIST, Int, Bits], Unit, false),
            Self::ListExtend => signature(&[LIST, LIST], Unit, false),
            Self::ListRemove => signature(&[LIST, Bits], Unit, true),
            Self::ListIndex => signature(&[LIST, Bits], Int, true),
            Self::ListCount => signature(&[LIST, Bits], Int, false),
            Self::ListCopy => signature(&[LIST], LIST, false),
            Self::ListClear | Self::ListReverse => signature(&[LIST], Unit, false),
            Self::ListConcat => signature(&[LIST, LIST], LIST, false),
            Self::ListRepeat => signature(&[LIST, Int], LIST, true),
            Self::ListCompare => signature(&[LIST, LIST, Int], Bool, false),
            Self::ListSort => signature(&[LIST, Int], Unit, false),
            Self::ListSumInt => signature(&[LIST, Int], Int, false),
            Self::ListSumFloat => signature(&[LIST, Float], Float, false),
            Self::ListAny | Self::ListAll => signature(&[LIST], Bool, false),
            Self::ZipUnequal => signature(INT_INT, Unit, true),
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
            Self::FloatAbsolute => "float_absolute",
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
            Self::RangeCheckStep => "range_check_step",
            Self::ListNew => "list_new",
            Self::ListAppend => "list_append",
            Self::ListLength => "list_length",
            Self::ListGet => "list_get",
            Self::ListSet => "list_set",
            Self::ListEqual => "list_equal",
            Self::ListContains => "list_contains",
            Self::ListRepr => "list_repr",
            Self::ListSlice => "list_slice",
            Self::StrSlice => "str_slice",
            Self::ListPop => "list_pop",
            Self::ListInsert => "list_insert",
            Self::ListExtend => "list_extend",
            Self::ListRemove => "list_remove",
            Self::ListIndex => "list_index",
            Self::ListCount => "list_count",
            Self::ListClear => "list_clear",
            Self::ListCopy => "list_copy",
            Self::ListReverse => "list_reverse",
            Self::ListConcat => "list_concat",
            Self::ListRepeat => "list_repeat",
            Self::ListCompare => "list_compare",
            Self::ListSort => "list_sort",
            Self::ListExtreme => "list_extreme",
            Self::ListSumInt => "list_sum_int",
            Self::ListSumFloat => "list_sum_float",
            Self::ListAny => "list_any",
            Self::ListAll => "list_all",
            Self::ZipUnequal => "zip_unequal",
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
        let raising = [
            RuntimeFunction::IntFloorDivide,
            RuntimeFunction::StrIndex,
            RuntimeFunction::CheckStack,
            RuntimeFunction::IntToStr,
            RuntimeFunction::RangeCheckStep,
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
}
