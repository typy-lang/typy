//! Lowers a call of an exception class with `*` or `**` (EXP-22, ADR-0083). The positional
//! values are evaluated into a tuple, or into one list when one of them is `*iterable`; each
//! `**` dict is then merged as the keywords of any call are, and the call raises CPython's
//! `TypeError` when the merged dict holds a keyword, since an exception class takes none. The
//! new exception then takes the values.

use typy_check::{ExceptionClass, ExceptionValues, InitCall, TypedExpr, UnpackedException};
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

    /// Lowers a call of the exception class `class` whose `__init__` of the program's binds
    /// what the call unpacks when it runs (ADR-0073): the arguments are evaluated, the
    /// exception keeps those given by position, or none for an `OSError`, as
    /// `BaseException.__new__` and `OSError.__new__` do, then the call is checked and
    /// `__init__` runs on the exception.
    pub(crate) fn new_exception_bound_when_run(
        &mut self,
        class: &ExceptionClass,
        init: &InitCall,
        span: Span,
    ) -> Lowered {
        let Some(class_id) = class.class_id() else {
            unreachable!("an `__init__` of the program's belongs to a class of the program")
        };
        let info = self.program.class(class_id);
        let called = format!("{}.{}", info.module, info.name);
        let unbound =
            self.unbound_arguments(init.function, &init.arguments, (1, Some(&called)), span);
        let tail = unbound.tail().filter(|_| !class.builtin().is_os_error());
        let exception = if let Some(tail) = tail {
            // The values before the tail go in front of a copy of it, which holds the one type
            // they all have.
            let list = self.call_runtime(RuntimeFunction::ListCopy, vec![tail.value], span);
            for (index, value) in unbound.by_position().iter().enumerate() {
                let index = self.constant(Constant::Int(index.to_string()), span);
                let bits = self.bits(*value, span);
                self.call_runtime(
                    RuntimeFunction::ListInsert,
                    vec![list.value, index, bits],
                    span,
                );
            }
            let class = self.constant(Constant::TypeObject(exception_class(class)), span);
            let exception = self.call_runtime(
                RuntimeFunction::ExceptionFromList,
                vec![class, list.value],
                span,
            );
            self.release(list, span);
            self.store_defaults(exception, class_id, span);
            exception
        } else {
            let kept: &[Lowered] = if class.builtin().is_os_error() {
                &[]
            } else {
                unbound.by_position()
            };
            let kept = kept.to_vec();
            let tuple = self.exception_tuple_of(&kept, span);
            self.exception_of_tuple(class, tuple, span)
        };
        let receiver = Self::borrowed(exception.value, TyirType::EXCEPTION);
        let values = self.bind_when_run(init.function, unbound, (Some(receiver), false), span);
        let done = self.call_function(init.function, values, TyirType::Unit, span);
        self.release(done, span);
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
