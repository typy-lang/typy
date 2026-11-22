//! Checks a call expression: it goes to the program's function, method or class it names,
//! to a builtin, or through a function value. A call that unpacks with `*` or `**` binds only
//! to the program's functions, methods and classes and to `print`; any other takes only
//! unpacking written out in full, and the rest is planned for 0.11 (EXP-17, EXP-19, EXP-22,
//! ADR-0073).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Constant, Expr, Identifier, Keyword};

use super::BodyChecker;
use crate::builtin::Builtin;
use crate::exception::BuiltinException;
use crate::typed::TypedExpr;
use crate::types::Type;

impl BodyChecker<'_> {
    /// Checks `function(arguments, keywords)`.
    pub(super) fn checked_call(
        &mut self,
        function: &Expr,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        let Some(at) = unpacking(arguments, keywords) else {
            return self.call_expression(function, arguments, keywords, span);
        };
        // Unpacking the program shows in full binds as the values it gives, for a callee that
        // does not bind what it unpacks when the program runs (EXP-19).
        if !self.binds_when_it_runs(function)
            && let Some((arguments, keywords)) = self.written_out(arguments, keywords)
        {
            return self.call_expression(function, &arguments, &keywords, span);
        }
        // A built-in function is checked for each number of arguments `*` may give (EXP-22).
        if let Expr::Name { id, .. } = function
            && self.peek_type(function).is_none()
            && let Some(builtin) = Builtin::named(id)
            && let Some(call) =
                self.builtin_unpacked((function, builtin), arguments, keywords, span)
        {
            return call;
        }
        // A callee that takes the unpacked arguments binds them and notes the call; any other
        // saw each `*value` as an error it does not report, so its diagnostics give way, but
        // for an error in the callee itself, such as a name that is not defined.
        let reported = self.diagnostics.len();
        let call = self.call_expression(function, arguments, keywords, span);
        let callee = function.span();
        let callee_refused = self.diagnostics.iter().skip(reported).any(|diagnostic| {
            diagnostic.span().is_some_and(|at| {
                at.file() == callee.file()
                    && at.start() >= callee.start()
                    && at.end() <= callee.end()
            })
        });
        if self.unpacking_calls.contains(&span) || callee_refused {
            return call;
        }
        self.diagnostics.truncate(reported);
        self.error(
            Diagnostic::new(
                DiagnosticCode::error(201),
                at,
                "`*` of a value only the run knows in a call of `max`, `min`, `zip`, a method of \
                 a built-in type or a function of a built-in module, and `**` of one in a call \
                 of a builtin other than `print`, are planned for TyPy 0.11 (EXP-22)",
            )
            .with_rule(RuleId::EXP_22)
            .with_help("unpack the values into variables first, and pass them one by one"),
            span,
        )
    }

    pub(super) fn call_expression(
        &mut self,
        function: &Expr,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        match function {
            // A module's declaration of a library holds nothing to evaluate (FFI-01).
            _ if let Some(library) = self.foreign_library(span) => library,
            // `typing.cast`, imported by name or read through the module (TYP-24).
            Expr::Name { id, .. }
                if self.program.annotation_names.get(id)
                    == Some(&crate::check::AnnotationName::Cast) =>
            {
                self.cast_call(arguments, keywords, span)
            }
            Expr::Attribute {
                value, attribute, ..
            } if attribute.name == "cast"
                && matches!(value.as_ref(), Expr::Name { id, .. }
                    if self.program.annotation_modules.get(id).map(String::as_str) == Some("typing")) =>
            {
                self.cast_call(arguments, keywords, span)
            }
            Expr::Name {
                id,
                span: name_span,
            } => self.call(id, *name_span, arguments, keywords, span),
            Expr::Attribute {
                value, attribute, ..
            } => self.method_call(value, attribute, arguments, keywords, span),
            // A generic class given its type arguments, such as `Box[int](1)`.
            Expr::Subscript { value, .. }
                if matches!(value.as_ref(), Expr::Name { id, .. }
                if self.program.class_ids.contains_key(id)) =>
            {
                self.applied_class_call(function, arguments, keywords, span)
            }
            // A class `POINTER(T)` or `T * length` called where it is made (FFI-03), or a
            // function value (FUN-05).
            _ => {
                let class = self.expression_or_class(function);
                match class.ty {
                    Type::CDataClass(class) => self.cdata_new(&class, arguments, keywords, span),
                    Type::Error => {
                        for argument in arguments {
                            self.expression(argument);
                        }
                        TypedExpr { span, ..class }
                    }
                    Type::Callable { .. } => self.call_value(class, arguments, keywords, span),
                    _ => {
                        for argument in arguments {
                            self.expression(argument);
                        }
                        self.not_callable((None, function.span()), &class.ty, span)
                    }
                }
            }
        }
    }
}

impl BodyChecker<'_> {
    /// Returns true when the callee binds what a call unpacks when the program runs, with
    /// CPython's errors: a function, a method or a class of the program, or a class held in a
    /// variable (ADR-0073), or a method through a protocol value (EXP-22). A built-in, a
    /// function value and an exception class first take unpacking written out in full.
    fn binds_when_it_runs(&self, function: &Expr) -> bool {
        match function {
            // An exception class without an `__init__` of the program's takes unpacking
            // written out as fixed arguments, which `args` shows as they are.
            Expr::Name { id, .. }
                if self.program.class_ids.get(id).is_some_and(|class| {
                    matches!(self.program.instance_type(*class), Type::Exception(_))
                        && self.program.method(*class, "__init__").is_none()
                }) =>
            {
                false
            }
            Expr::Name { id, .. } => match self.peek_type(function) {
                Some(Type::Callable { .. }) => false,
                Some(_) => true,
                None => {
                    self.program.function_ids.contains_key(id)
                        || self.program.class_ids.contains_key(id)
                        || !(Builtin::named(id).is_some() || BuiltinException::named(id).is_some())
                }
            },
            Expr::Attribute { value, .. } => {
                if let Some(module) = self.module_of(value) {
                    return !super::builtin_module::is_builtin_module(&module);
                }
                // A method through a protocol value binds as the protocol's method, whose
                // parameters its dispatcher takes (EXP-22, ADR-0041).
                match self.peek_type(value) {
                    Some(Type::Instance(_) | Type::TypeObject(_)) | None => true,
                    Some(_) => false,
                }
            }
            _ => true,
        }
    }

    /// Returns the arguments and keywords of a call with each `*` of a tuple display or of a
    /// variable that holds a fixed tuple, and each `**` of a dict display whose keys are
    /// `str` literals, written out as the values they give, which bind as CPython binds
    /// them; `None` when one of them gives values only the run knows, or a dict display
    /// names a key twice, whose first value would still be evaluated.
    fn written_out(
        &self,
        arguments: &[Expr],
        keywords: &[Keyword],
    ) -> Option<(Vec<Expr>, Vec<Keyword>)> {
        let mut written = Vec::new();
        for argument in arguments {
            let Expr::Starred { value, .. } = argument else {
                written.push(argument.clone());
                continue;
            };
            match value.as_ref() {
                Expr::Tuple { elements, .. }
                    if !elements
                        .iter()
                        .any(|element| matches!(element, Expr::Starred { .. })) =>
                {
                    written.extend(elements.iter().cloned());
                }
                Expr::Name { span, .. } => {
                    let Some(Type::Tuple(items)) = self.peek_type(value) else {
                        return None;
                    };
                    written.extend((0..items.len()).map(|position| Expr::Subscript {
                        value: value.clone(),
                        index: Box::new(Expr::Constant {
                            value: Constant::Int(position.to_string()),
                            span: *span,
                        }),
                        span: *span,
                    }));
                }
                _ => return None,
            }
        }
        let mut named = Vec::new();
        for keyword in keywords {
            if keyword.name.is_some() {
                named.push(keyword.clone());
                continue;
            }
            let Expr::Dict { keys, values, .. } = &keyword.value else {
                return None;
            };
            let mut seen = Vec::new();
            for (key, value) in keys.iter().zip(values) {
                let Some(Expr::Constant {
                    value: Constant::Str(name),
                    span,
                }) = key
                else {
                    return None;
                };
                if seen.contains(name) {
                    return None;
                }
                seen.push(name.clone());
                named.push(Keyword {
                    name: Some(Identifier {
                        name: name.clone(),
                        span: *span,
                    }),
                    value: value.clone(),
                    span: *span,
                });
            }
        }
        Some((written, named))
    }
}

/// Returns where the first `*value` or `**value` of a call is, if it has one.
pub(super) fn unpacking(arguments: &[Expr], keywords: &[Keyword]) -> Option<Span> {
    arguments
        .iter()
        .find_map(|argument| match argument {
            Expr::Starred { span, .. } => Some(*span),
            _ => None,
        })
        .or_else(|| {
            keywords
                .iter()
                .find(|keyword| keyword.name.is_none())
                .map(|keyword| keyword.span)
        })
}
