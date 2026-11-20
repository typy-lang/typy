//! Packs the arguments a call passes to `*args` and `**kwargs` (EXP-17): every argument is
//! evaluated first, in source order, then those of `*args` go into a new tuple and those of
//! `**kwargs` into a new dict, in order, as CPython binds them. A call leaves neither
//! parameter out: one with no argument takes an empty tuple or dict.

use typy_check::{Argument, FunctionId, Type, TypedConstant};
use typy_diagnostics::Span;
use typy_source::syntax::ParameterKind;
use typy_tyir::{RuntimeFunction, TyirType};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

/// An argument evaluated for a call: the index of its parameter, the keyword a `**kwargs`
/// parameter takes it by, and its value.
pub(crate) type Evaluated<'call> = (usize, Option<&'call str>, Lowered);

impl FunctionLowerer<'_> {
    /// Returns the arguments of a call of `function`, evaluated in source order, each with
    /// its parameter; the values of `*args` and `**kwargs` are packed.
    pub(crate) fn call_arguments(
        &mut self,
        function: FunctionId,
        arguments: &[Argument],
        span: Span,
    ) -> Vec<(usize, Lowered)> {
        let evaluated = self.evaluated_arguments(arguments, span);
        self.packed_arguments(function, evaluated, span)
    }

    /// Returns the arguments of a call, evaluated in source order, each kept alive until the
    /// call.
    pub(crate) fn evaluated_arguments<'call>(
        &mut self,
        arguments: &'call [Argument],
        span: Span,
    ) -> Vec<Evaluated<'call>> {
        arguments
            .iter()
            .map(|argument| {
                let lowered = self.expression(&argument.value);
                (
                    argument.parameter,
                    argument.keyword.as_deref(),
                    self.kept_for_call(&argument.value, lowered, span),
                )
            })
            .collect()
    }

    /// Returns `evaluated` with the values of the `*args` parameter of `function` packed into
    /// one tuple and those of its `**kwargs` parameter into one dict.
    pub(crate) fn packed_arguments(
        &mut self,
        function: FunctionId,
        evaluated: Vec<Evaluated<'_>>,
        span: Span,
    ) -> Vec<(usize, Lowered)> {
        let parameters = &self.program.function(function).signature.parameters;
        let variadic: Vec<(usize, ParameterKind, Type)> = parameters
            .iter()
            .enumerate()
            .filter(|(_, parameter)| parameter.kind.is_variadic())
            .map(|(index, parameter)| (index, parameter.kind, parameter.ty.clone()))
            .collect();
        if variadic.is_empty() {
            return evaluated
                .into_iter()
                .map(|(parameter, _, value)| (parameter, value))
                .collect();
        }
        let (packed, mut values): (Vec<Evaluated<'_>>, Vec<Evaluated<'_>>) = evaluated
            .into_iter()
            .partition(|(parameter, _, _)| variadic.iter().any(|(index, ..)| index == parameter));
        let mut bound: Vec<(usize, Lowered)> = values
            .drain(..)
            .map(|(parameter, _, value)| (parameter, value))
            .collect();
        for (index, kind, ty) in variadic {
            let items = packed.iter().filter(|(parameter, ..)| *parameter == index);
            let value = if kind == ParameterKind::VarPositional {
                self.packed_tuple(&ty, items.map(|(_, _, value)| *value), span)
            } else {
                let entries =
                    items.map(|(_, keyword, value)| (keyword.unwrap_or_default(), *value));
                self.packed_dict(&ty, entries, span)
            };
            bound.push((index, value));
        }
        bound
    }

    /// Returns a new `tuple[T, ...]` of `values`, which it releases.
    fn packed_tuple(
        &mut self,
        ty: &Type,
        values: impl Iterator<Item = Lowered>,
        span: Span,
    ) -> Lowered {
        let Type::TupleOf(item) = ty else {
            unreachable!("a `*args` parameter is a `tuple[T, ...]`")
        };
        let item = self.lowered_type(item);
        let tuple = self.list_display(&[], item, span);
        for value in values {
            let bits = self.bits(value, span);
            self.call_runtime(RuntimeFunction::ListAppend, vec![tuple.value, bits], span);
            self.release(value, span);
        }
        self.call_runtime(RuntimeFunction::TupleSeal, vec![tuple.value], span);
        tuple
    }

    /// Returns a new `dict[str, T]` of `entries`, whose values it releases.
    fn packed_dict<'call>(
        &mut self,
        ty: &Type,
        entries: impl Iterator<Item = (&'call str, Lowered)>,
        span: Span,
    ) -> Lowered {
        let Type::Dict(_, item) = ty else {
            unreachable!("a `**kwargs` parameter is a `dict[str, T]`")
        };
        let item = self.lowered_type(item);
        let dict = self.new_dict(TyirType::STR, item, 0, span);
        for (keyword, value) in entries {
            let key = self.typed_constant(&TypedConstant::Str(keyword.to_owned()), span);
            self.set_entry(dict.value, key, value, span);
            self.release(value, span);
            self.release(key, span);
        }
        dict
    }
}
