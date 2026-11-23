//! Checks calls of `divmod`, `pow`, `bin`, `hex`, `oct` and `format` (LIB-28).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{BinaryOperator, Expr, Keyword};

use super::BodyChecker;
use crate::builtin::Builtin;
use crate::typed::{TypedConstant, TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Checks a call of one of the builtins of LIB-28 other than `bytearray`.
    pub(super) fn call_number_builtin(
        &mut self,
        builtin: Builtin,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        if builtin == Builtin::Pow {
            return self.call_pow(arguments, keywords, span);
        }
        if let Some(keyword) = keywords.first() {
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    keyword.span,
                    format!(
                        "{}() takes no keyword arguments: CPython raises `TypeError`",
                        builtin.name()
                    ),
                )
                .with_rule(RuleId::LIB_28),
                span,
            );
        }
        let allowed = match builtin {
            Builtin::Divmod => 2..=2,
            Builtin::Format => 1..=2,
            _ => 1..=1,
        };
        let values: Vec<TypedExpr> = arguments
            .iter()
            .map(|argument| self.expression(argument))
            .collect();
        if !self.arity(builtin, values.len(), allowed, span) {
            return Self::invalid_call(span);
        }
        if values.iter().any(|value| value.ty.has_error()) {
            return Self::invalid_call(span);
        }
        match builtin {
            Builtin::Divmod => self.divmod(values, span),
            Builtin::Format => self.format(values, span),
            _ => self.in_base(builtin, values, span),
        }
    }

    /// `divmod(a, b)` of two numbers: a tuple of `a // b` and `a % b`, of `int`s when both are
    /// integers and of `float`s otherwise.
    fn divmod(&mut self, values: Vec<TypedExpr>, span: Span) -> TypedExpr {
        let values: Vec<TypedExpr> = values.into_iter().map(Self::int_of_bool).collect();
        let item = match (&values[0].ty, &values[1].ty) {
            (Type::Int, Type::Int) => Type::Int,
            (Type::Int | Type::Float, Type::Int | Type::Float) => Type::Float,
            (left, right) => {
                let (left, right) = (left.clone(), right.clone());
                return self.operator_result_error("divmod", &left, &right, values[0].span, span);
            }
        };
        Self::typed(
            TypedExprKind::Builtin {
                builtin: Builtin::Divmod,
                arguments: values,
            },
            Type::Tuple(vec![item.clone(), item]),
            span,
        )
    }

    /// `bin`, `hex` or `oct` of an `int` or a `bool`: its digits with the prefix `0b`, `0x`
    /// or `0o`.
    fn in_base(&mut self, builtin: Builtin, values: Vec<TypedExpr>, span: Span) -> TypedExpr {
        if !matches!(values[0].ty, Type::Int | Type::Bool) {
            self.expect(
                &values[0],
                Type::Int,
                &format!("the argument of `{}`", builtin.name()),
            );
        }
        Self::typed(
            TypedExprKind::Builtin {
                builtin,
                arguments: values,
            },
            Type::Str,
            span,
        )
    }

    /// `format(value, spec)`: the value formatted as an f-string's field `{value:spec}`
    /// formats it.
    fn format(&mut self, values: Vec<TypedExpr>, span: Span) -> TypedExpr {
        if let Some(spec) = values.get(1) {
            self.expect(spec, Type::Str, "the format specification of `format`");
        }
        Self::typed(
            TypedExprKind::Builtin {
                builtin: Builtin::Format,
                arguments: values,
            },
            Type::Str,
            span,
        )
    }

    /// `pow(base, exp)` is `base ** exp`, checked by the rules of `**` (EXP-04), and
    /// `pow(base, exp, mod)` of three `int`s is `base ** exp % mod`, computed without the
    /// power, an `int` for any exponent.
    fn call_pow(&mut self, arguments: &[Expr], keywords: &[Keyword], span: Span) -> TypedExpr {
        let mut given: Vec<Option<&Expr>> = arguments.iter().map(Some).collect();
        for keyword in keywords {
            let position = match keyword.name.as_ref().map(|name| name.name.as_str()) {
                Some("base") => 0,
                Some("exp") => 1,
                Some("mod") => 2,
                Some(other) => {
                    return self.pow_error(
                        keyword.span,
                        &format!("pow() got an unexpected keyword argument '{other}'"),
                        span,
                    );
                }
                None => {
                    return self.pow_error(keyword.span, "`pow` takes no `**` arguments", span);
                }
            };
            if given.len() <= position {
                given.resize(position + 1, None);
            }
            if given[position].is_some() {
                return self.pow_error(
                    keyword.span,
                    &format!(
                        "argument for pow() given by name ('{}') and position ({})",
                        ["base", "exp", "mod"][position],
                        position + 1
                    ),
                    span,
                );
            }
            given[position] = Some(&keyword.value);
        }
        // `mod=None` is no modulus.
        if let Some(Some(Expr::Constant {
            value: typy_source::syntax::Constant::None,
            ..
        })) = given.get(2)
        {
            given.truncate(2);
        }
        if let Some(missing) =
            (0..2).find(|position| given.get(*position).is_none_or(Option::is_none))
        {
            return self.pow_error(
                span,
                &format!(
                    "pow() missing required argument '{}' (pos {})",
                    ["base", "exp"][missing],
                    missing + 1
                ),
                span,
            );
        }
        let Some(given) = given.into_iter().collect::<Option<Vec<&Expr>>>() else {
            return self.pow_error(span, "pow() missing required argument 'exp' (pos 2)", span);
        };
        if !self.arity(Builtin::Pow, given.len(), 2..=3, span) {
            return Self::invalid_call(span);
        }
        if let [base, exponent] = given.as_slice() {
            let power = Expr::BinOp {
                left: Box::new((*base).clone()),
                operator: BinaryOperator::Pow,
                right: Box::new((*exponent).clone()),
                span,
            };
            return self.expression(&power);
        }
        let values: Vec<TypedExpr> = given
            .iter()
            .map(|value| Self::int_of_bool(self.expression(value)))
            .collect();
        if values.iter().any(|value| value.ty.has_error()) {
            return Self::invalid_call(span);
        }
        for (value, what) in values.iter().zip(["base", "exponent", "modulus"]) {
            self.expect(
                value,
                Type::Int,
                &format!("the {what} of a `pow` with a modulus"),
            );
        }
        Self::typed(
            TypedExprKind::Builtin {
                builtin: Builtin::Pow,
                arguments: values,
            },
            Type::Int,
            span,
        )
    }

    /// Reports CPython's `TypeError` for a call of `pow` with its message.
    fn pow_error(&mut self, at: Span, message: &str, span: Span) -> TypedExpr {
        self.error(
            Diagnostic::new(
                DiagnosticCode::error(404),
                at,
                format!("{message}: CPython raises `TypeError`"),
            )
            .with_rule(RuleId::LIB_28),
            span,
        )
    }

    /// Reports E0401 for `name` of two operand types it does not take, and returns the call
    /// typed as an error.
    fn operator_result_error(
        &mut self,
        name: &str,
        left: &Type,
        right: &Type,
        at: Span,
        span: Span,
    ) -> TypedExpr {
        let ty = self.operator_error(name, left, right, at);
        Self::typed(TypedExprKind::Constant(TypedConstant::None), ty, span)
    }
}
