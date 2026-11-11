//! Checks calls of the builtins of 0.1 (LIB-01).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Expr, Keyword};

use super::BodyChecker;
use super::iterable_call::takes_one_iterable;
use crate::builtin::Builtin;
use crate::typed::{TypedConstant, TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    #[allow(clippy::too_many_lines, reason = "one arm per builtin")]
    pub(super) fn call_builtin(
        &mut self,
        builtin: Builtin,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        if builtin == Builtin::Print {
            return self.print(arguments, keywords, span);
        }
        if builtin == Builtin::List {
            return self.call_list(arguments, keywords, None, span);
        }
        if builtin == Builtin::Bytes {
            return self.call_bytes(arguments, keywords, span);
        }
        if builtin == Builtin::Tuple {
            return self.call_tuple(arguments, keywords, None, span);
        }
        if builtin == Builtin::Set {
            return self.call_set(arguments, keywords, None, span);
        }
        if builtin == Builtin::Dict {
            return self.call_dict(arguments, keywords, None, span);
        }
        if builtin == Builtin::Type {
            return self.call_type(arguments, keywords, span);
        }
        if builtin == Builtin::Round {
            return self.call_round(arguments, keywords, span);
        }
        if builtin == Builtin::Iter {
            return self.call_iter(arguments, keywords, span);
        }
        if builtin == Builtin::Next {
            return self.call_next(arguments, keywords, span);
        }
        if takes_one_iterable(builtin, arguments.len()) {
            return self.call_over_iterable(builtin, arguments, keywords, span);
        }
        let values: Vec<TypedExpr> = arguments
            .iter()
            .map(|argument| self.expression(argument))
            .collect();
        if let Some(keyword) = keywords.first() {
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    keyword.span,
                    format!(
                        "`{}` takes no keyword arguments in TyPy 0.1",
                        builtin.name()
                    ),
                )
                .with_rule(RuleId::LIB_01),
                span,
            );
        }
        if values.iter().any(|value| value.ty == Type::Error) {
            return Self::typed(
                TypedExprKind::Constant(TypedConstant::None),
                Type::Error,
                span,
            );
        }
        let types: Vec<Type> = values.iter().map(|value| value.ty.clone()).collect();
        let result = match builtin {
            Builtin::Print
            | Builtin::Bytes
            | Builtin::List
            | Builtin::Tuple
            | Builtin::Set
            | Builtin::Dict
            | Builtin::Round
            | Builtin::Iter
            | Builtin::Next
            | Builtin::Type
            | Builtin::Sorted
            | Builtin::Sum
            | Builtin::Any
            | Builtin::All => {
                unreachable!(
                    "print, round, type and the builtins of one iterable are checked above"
                )
            }
            Builtin::Enumerate | Builtin::Zip | Builtin::Reversed => {
                return self.error(
                    Diagnostic::new(
                        DiagnosticCode::error(201),
                        span,
                        format!(
                            "`{}` outside a `for` loop, a comprehension or an iterable argument \
                             is planned for after TyPy 1.0",
                            builtin.name()
                        ),
                    )
                    .with_rule(RuleId::CST_09),
                    span,
                );
            }
            Builtin::Range => {
                if !self.arity(builtin, values.len(), 1..=3, span) {
                    return Self::typed(
                        TypedExprKind::Constant(TypedConstant::None),
                        Type::Error,
                        span,
                    );
                }
                for value in &values {
                    self.expect(value, Type::Int, "an argument of `range`");
                }
                Type::Range
            }
            Builtin::Len => {
                if !self.arity(builtin, values.len(), 1..=1, span) {
                    return Self::typed(
                        TypedExprKind::Constant(TypedConstant::None),
                        Type::Error,
                        span,
                    );
                }
                if !matches!(
                    values[0].ty,
                    Type::Tuple(_)
                        | Type::Range
                        | Type::List(_)
                        | Type::TupleOf(_)
                        | Type::Set(_)
                        | Type::Dict(..)
                        | Type::DictView(..)
                        | Type::Bytes
                ) {
                    self.expect(&values[0], Type::Str, "the argument of `len`");
                }
                Type::Int
            }
            Builtin::Abs => {
                if !self.arity(builtin, values.len(), 1..=1, span) {
                    return Self::typed(
                        TypedExprKind::Constant(TypedConstant::None),
                        Type::Error,
                        span,
                    );
                }
                match &types[0] {
                    Type::Int | Type::Float => types[0].clone(),
                    Type::Bool => self.bool_arithmetic(values[0].span),
                    other => self.operator_error("abs", other, other, values[0].span),
                }
            }
            Builtin::Min | Builtin::Max => {
                if !self.arity(builtin, values.len(), 2..=usize::MAX, span) {
                    return Self::typed(
                        TypedExprKind::Constant(TypedConstant::None),
                        Type::Error,
                        span,
                    );
                }
                let first = types[0].clone();
                if let Some(other) = values.iter().find(|value| value.ty != first) {
                    return self.error(
                        Diagnostic::new(
                            DiagnosticCode::error(404),
                            other.span,
                            format!(
                                "the arguments of `{}` must have one type, because CPython returns one of them unchanged; they are {first} and {}",
                                builtin.name(),
                                other.ty
                            ),
                        )
                        .with_rule(RuleId::TYP_09),
                        span,
                    );
                }
                if !matches!(first, Type::Int | Type::Float | Type::Str) {
                    self.comparison_error(&first, &first, span);
                    return Self::typed(
                        TypedExprKind::Constant(TypedConstant::None),
                        Type::Error,
                        span,
                    );
                }
                first
            }
            Builtin::Int | Builtin::Float => {
                let target = if builtin == Builtin::Int {
                    Type::Int
                } else {
                    Type::Float
                };
                if !self.arity(
                    builtin,
                    values.len(),
                    0..=usize::from(builtin == Builtin::Int) + 1,
                    span,
                ) {
                    return Self::typed(
                        TypedExprKind::Constant(TypedConstant::None),
                        Type::Error,
                        span,
                    );
                }
                if let Some(value) = values.first() {
                    if !matches!(value.ty, Type::Int | Type::Float | Type::Str | Type::Bool) {
                        self.expect(
                            value,
                            target.clone(),
                            &format!("the argument of `{}`", builtin.name()),
                        );
                    }
                    if values.len() == 2 {
                        self.expect(
                            &values[0],
                            Type::Str,
                            "the first argument of `int` with a base",
                        );
                        self.expect(&values[1], Type::Int, "the base");
                    }
                }
                target
            }
            Builtin::Str | Builtin::Repr => {
                let allowed = if builtin == Builtin::Str {
                    0..=1
                } else {
                    1..=1
                };
                if !self.arity(builtin, values.len(), allowed, span) {
                    return Self::typed(
                        TypedExprKind::Constant(TypedConstant::None),
                        Type::Error,
                        span,
                    );
                }
                Type::Str
            }
            Builtin::Bool => {
                if !self.arity(builtin, values.len(), 0..=1, span) {
                    return Self::typed(
                        TypedExprKind::Constant(TypedConstant::None),
                        Type::Error,
                        span,
                    );
                }
                Type::Bool
            }
            Builtin::Ord => {
                if !self.arity(builtin, values.len(), 1..=1, span) {
                    return Self::typed(
                        TypedExprKind::Constant(TypedConstant::None),
                        Type::Error,
                        span,
                    );
                }
                self.expect(&values[0], Type::Str, "the argument of `ord`");
                Type::Int
            }
            Builtin::Chr => {
                if !self.arity(builtin, values.len(), 1..=1, span) {
                    return Self::typed(
                        TypedExprKind::Constant(TypedConstant::None),
                        Type::Error,
                        span,
                    );
                }
                self.expect(&values[0], Type::Int, "the argument of `chr`");
                Type::Str
            }
        };
        Self::typed(
            TypedExprKind::Builtin {
                builtin,
                arguments: values,
            },
            result,
            span,
        )
    }

    pub(super) fn print(
        &mut self,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        let values: Vec<TypedExpr> = arguments
            .iter()
            .map(|argument| self.expression(argument))
            .collect();
        let mut sep = None;
        let mut end = None;
        for keyword in keywords {
            let value = self.expression(&keyword.value);
            let Some(name) = &keyword.name else {
                continue;
            };
            match name.name.as_str() {
                "sep" => {
                    self.expect(&value, Type::Str, "`sep`");
                    sep = Some(Box::new(value));
                }
                "end" => {
                    self.expect(&value, Type::Str, "`end`");
                    end = Some(Box::new(value));
                }
                "file" | "flush" => {
                    return self.error(
                        Diagnostic::new(
                            DiagnosticCode::error(201),
                            name.span,
                            format!(
                                "the `{}` argument of `print` is planned for TyPy 0.3 (LIB-04)",
                                name.name
                            ),
                        )
                        .with_rule(RuleId::LIB_04),
                        span,
                    );
                }
                other => {
                    return self.error(
                        Diagnostic::new(
                            DiagnosticCode::error(404),
                            name.span,
                            format!("`print` has no parameter named `{other}`"),
                        )
                        .with_rule(RuleId::LIB_01),
                        span,
                    );
                }
            }
        }
        Self::typed(
            TypedExprKind::Print { values, sep, end },
            Type::NoneType,
            span,
        )
    }

    pub(super) fn arity(
        &mut self,
        builtin: Builtin,
        given: usize,
        allowed: std::ops::RangeInclusive<usize>,
        span: Span,
    ) -> bool {
        if allowed.contains(&given) {
            return true;
        }
        let expected = if allowed.start() == allowed.end() {
            format!("{}", allowed.start())
        } else {
            format!("{} to {}", allowed.start(), allowed.end())
        };
        self.diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(404),
                span,
                format!(
                    "`{}` takes {expected} arguments in TyPy 0.1, and {given} are given",
                    builtin.name()
                ),
            )
            .with_rule(RuleId::LIB_01),
        );
        false
    }
}
