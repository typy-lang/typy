//! `print(**options)` (EXP-22, ADR-0083): the keywords and the `**` dicts bind as the keywords
//! of any call bind when it runs, against a description of `print`, with CPython 3.15's
//! errors; the merged dict's values are then checked as `print` checks them, and each keyword
//! no argument names takes its value from the dict, or its default.

use typy_check::{PrintCall, Type};
use typy_diagnostics::Span;
use typy_tyir::{Constant, RuntimeFunction, TyirType, ValueId};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

/// `print` as a description of a callee's parameters, which the runtime binds against.
const PRINT: &str = "print\nprint\nA-args\nK=sep\nK=end\nK=file\nK=flush";

/// The binding of a `print` with `**`: the description, the names the call gives, the merged
/// dict of the keywords `**` gives, and the type of its values.
pub(crate) struct PrintBinding {
    description: ValueId,
    pub(crate) names: Lowered,
    pub(crate) merged: Lowered,
    values: Type,
}

/// The values of the keywords of a `print`, each `None` for one that keeps its default.
type Keywords = (
    Option<Lowered>,
    Option<Lowered>,
    Option<Lowered>,
    Option<Lowered>,
);

impl FunctionLowerer<'_> {
    /// Starts the binding of a `print` that has `**`.
    pub(crate) fn print_binding(&mut self, print: &PrintCall, span: Span) -> PrintBinding {
        let options = &print.options[0].ty;
        let values = match options {
            Type::Dict(_, value) => value.as_ref().clone(),
            _ => Type::Error,
        };
        let (_, value) = self.dict_types(options);
        PrintBinding {
            description: self.string(PRINT, span),
            names: self.new_dict(TyirType::STR, TyirType::Int, 0, span),
            merged: self.new_dict(TyirType::STR, value, 0, span),
            values,
        }
    }

    /// Notes a keyword of a `print` with `**`, `name` with its value, or merges a `**` dict,
    /// whose name is `**`, raising CPython's error for a keyword given twice or unknown.
    pub(crate) fn print_keyword(
        &mut self,
        binding: &PrintBinding,
        (name, value): (&str, Lowered),
        span: Span,
    ) {
        if name == "**" {
            let kwargs = self.null_bits(span);
            self.call_runtime(
                RuntimeFunction::CallMerge,
                vec![
                    binding.description,
                    binding.names.value,
                    binding.merged.value,
                    value.value,
                    kwargs,
                ],
                span,
            );
            return;
        }
        let text = self.string(name, span);
        self.call_runtime(
            RuntimeFunction::CallName,
            vec![binding.description, binding.names.value, text],
            span,
        );
    }

    /// Returns the value of each keyword no argument gives, `given` saying which do, from the
    /// merged dict after checking its values as `print` checks them.
    pub(crate) fn print_options(
        &mut self,
        binding: &PrintBinding,
        (sep, end, file, flush): (bool, bool, bool, bool),
        span: Span,
    ) -> Keywords {
        let texts = matches!(&binding.values, Type::Str | Type::NoneType)
            || matches!(&binding.values, Type::Optional(inner) if **inner == Type::Str);
        let files = matches!(
            &binding.values,
            Type::TextFile | Type::DescriptorFile(None) | Type::NoneType
        ) || matches!(&binding.values, Type::Optional(inner) if **inner == Type::TextFile);
        // Each name must be a keyword of `print`; `*args` takes any number of values.
        let none = self.constant(Constant::Int("0".to_owned()), span);
        let names = self.bits(Self::borrowed(binding.names.value, TyirType::DICT), span);
        self.call_runtime(
            RuntimeFunction::CallCheck,
            vec![binding.description, none, names],
            span,
        );
        let usable = i64::from(texts) * 3 + i64::from(files) * 4;
        let mask = self.constant(Constant::Int(usable.to_string()), span);
        self.call_runtime(
            RuntimeFunction::PrintCheck,
            vec![binding.merged.value, mask],
            span,
        );
        let merged = binding.merged.value;
        let text = |this: &mut Self, given: bool, (name, default): (&str, &str)| {
            (texts && !given).then(|| {
                let (name, default) = (this.string(name, span), this.string(default, span));
                let found = this.call_runtime(
                    RuntimeFunction::PrintText,
                    vec![merged, name, default],
                    span,
                );
                this.borrowed_result(found)
            })
        };
        let separator = text(self, sep, ("sep", " "));
        let ending = text(self, end, ("end", "\n"));
        let target = (files && !file).then(|| {
            let found = self.call_runtime(RuntimeFunction::PrintFile, vec![merged], span);
            self.borrowed_result(found)
        });
        let flushed =
            (!flush).then(|| self.call_runtime(RuntimeFunction::PrintFlush, vec![merged], span));
        (separator, ending, target, flushed)
    }
}
