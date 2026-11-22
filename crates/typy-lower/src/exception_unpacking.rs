//! Lowers a call of an exception class with `*` or `**` (EXP-22, ADR-0083). The positional
//! values are evaluated into a tuple, or into one list when one of them is `*iterable`; each
//! `**` dict is then merged as the keywords of any call are, and the call raises CPython's
//! `TypeError` when the merged dict holds a keyword, since an exception class takes none. The
//! new exception then takes the values.

use typy_check::{ExceptionClass, ExceptionValues, TypedExpr, UnpackedException};
use typy_diagnostics::Span;
use typy_tyir::{Constant, RuntimeFunction, TyirType};

use crate::exception::{ExceptionTuple, exception_class};
use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

/// The positional values of a call of an exception class, evaluated.
enum Values {
    /// Laid out as a tuple, as a call without `*` lays them out.
    Tuple(ExceptionTuple),
    /// One list of them all.
    List(Lowered),
}

impl FunctionLowerer<'_> {
    /// Lowers `call`, a call of an exception class with `*` or `**`.
    pub(crate) fn new_exception_unpacked(
        &mut self,
        call: &UnpackedException,
        span: Span,
    ) -> Lowered {
        let values = match &call.values {
            ExceptionValues::Each(arguments) => {
                Values::Tuple(self.exception_tuple(arguments, span))
            }
            ExceptionValues::Listed(list) => Values::List(self.expression(list)),
        };
        if !call.options.is_empty() {
            self.refuse_keywords(&call.class, &call.options, span);
        }
        let list = match values {
            Values::Tuple(tuple) => return self.exception_of_tuple(&call.class, tuple, span),
            Values::List(list) => list,
        };
        let class = self.constant(Constant::TypeObject(exception_class(&call.class)), span);
        let exception = self.call_runtime(
            RuntimeFunction::ExceptionFromList,
            vec![class, list.value],
            span,
        );
        self.release(list, span);
        if let Some(program_class) = call.class.class_id() {
            self.store_defaults(exception, program_class, span);
        }
        exception
    }

    /// Evaluates and merges the dicts `options` of a call of `class`, raising CPython's
    /// `TypeError` for a key two of them give, then for any keyword at all.
    fn refuse_keywords(&mut self, class: &ExceptionClass, options: &[TypedExpr], span: Span) {
        let (name, full) = match class {
            ExceptionClass::Builtin(builtin) => {
                (builtin.name().to_owned(), builtin.name().to_owned())
            }
            ExceptionClass::Class(class) => {
                let info = self.program.class(class.id);
                (info.name.clone(), format!("{}.{}", info.module, info.name))
            }
        };
        // A description without parameters, which names the class in the error of a key
        // given twice.
        let description = self.string(&format!("{name}\n{full}"), span);
        let (_, value) = self.dict_types(&options[0].ty);
        let names = self.new_dict(TyirType::STR, TyirType::Int, 0, span);
        let merged = self.new_dict(TyirType::STR, value, 0, span);
        for option in options {
            let dict = self.expression(option);
            let kwargs = self.null_bits(span);
            self.call_runtime(
                RuntimeFunction::CallMerge,
                vec![description, names.value, merged.value, dict.value, kwargs],
                span,
            );
            self.release(dict, span);
        }
        let class = self.constant(Constant::TypeObject(exception_class(class)), span);
        self.call_runtime(
            RuntimeFunction::ExceptionKeywords,
            vec![class, merged.value],
            span,
        );
        self.release(merged, span);
        self.release(names, span);
    }
}
