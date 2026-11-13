//! Lowers list, set and dict comprehensions: a new collection, then one loop for each clause,
//! nested in order, which skips a value that fails a condition and adds what the
//! comprehension makes in the innermost loop (EXP-12).

use typy_check::{Generator, Type, TypedExpr};
use typy_diagnostics::Span;
use typy_tyir::RuntimeFunction;

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

impl FunctionLowerer<'_> {
    /// Returns the collection of type `ty` a comprehension makes: `element` for each value its
    /// clauses give, or the entry `element: value` for a dict.
    pub(crate) fn comprehension(
        &mut self,
        element: &TypedExpr,
        value: Option<&TypedExpr>,
        generators: &[Generator],
        ty: &Type,
        span: Span,
    ) -> Lowered {
        let result = match ty {
            Type::List(item) => {
                let item = self.lowered_type(item);
                self.list_display(&[], item, span)
            }
            Type::Set(item) => {
                let item = self.lowered_type(item);
                self.new_set(item, span)
            }
            Type::Dict(..) => {
                let (key, value) = self.dict_types(ty);
                self.new_dict(key, value, 0, span)
            }
            _ => unreachable!("a comprehension makes a list, a set or a dict"),
        };
        let (add, add_moved) = match ty {
            Type::List(_) => (
                RuntimeFunction::ListAppend,
                RuntimeFunction::ListAppendMoved,
            ),
            _ => (RuntimeFunction::SetAdd, RuntimeFunction::SetAddMoved),
        };
        self.generate(generators, span, &mut |lowerer| {
            let made = lowerer.expression(element);
            if let Some(value) = value {
                // CPython evaluates the key before the value.
                let entry = lowerer.expression(value);
                lowerer.set_entry(result.value, made, entry, span);
                lowerer.release(entry, span);
            } else {
                let bits = lowerer.bits(made, span);
                lowerer.store_call(add, add_moved, vec![result.value, bits], made, span);
            }
            lowerer.release(made, span);
        });
        result
    }
}
