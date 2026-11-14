//! `args` of an exception and `code` of a `SystemExit` (EXC-11, EXC-13, ADR-0030). Each may
//! hold any value, which TyPy has no type for yet, so a program reads them where it prints,
//! formats, compares or tests them: `print`, `str`, `repr`, `bool`, `len` of `args`, an
//! f-string field, `==` and `!=`, a condition and `not`. Each such use is one operation on
//! the exception; any other use is planned (EXC-15).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{CompareOperator, Expr, UnaryOperator};

use super::BodyChecker;
use crate::builtin::Builtin;
use crate::exception::{BuiltinException, ExceptionClass};
use crate::typed::{ExceptionPart, PartOperation, TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Returns the operation an expression makes of `args` or `code`, when it makes one; a
    /// use of either that is not one is reported.
    pub(super) fn exception_part_use(&mut self, expression: &Expr) -> Option<TypedExpr> {
        let span = expression.span();
        match expression {
            Expr::Attribute { value, .. } => {
                let (part, _) = self.part_access(expression)?;
                self.expression(value);
                Some(
                    self.error(
                        Diagnostic::new(
                            DiagnosticCode::error(201),
                            span,
                            format!(
                                "using `{}` as a value of its own is planned for TyPy 0.5 (EXC-15)",
                                part.name()
                            ),
                        )
                        .with_rule(RuleId::EXC_15)
                        .with_help("print it, format it, compare it with `==` or test it"),
                        span,
                    ),
                )
            }
            Expr::Compare {
                left,
                operators,
                comparators,
                ..
            } => {
                let ([operator @ (CompareOperator::Eq | CompareOperator::NotEq)], [right]) =
                    (operators.as_slice(), comparators.as_slice())
                else {
                    return None;
                };
                let (accessed, other) = if self.part_access(left).is_some() {
                    (left.as_ref(), right)
                } else if self.part_access(right).is_some() {
                    (right, left.as_ref())
                } else {
                    return None;
                };
                let other = self.expression(other);
                let equal =
                    self.part_operation(accessed, PartOperation::Equal(Box::new(other)), span)?;
                Some(if *operator == CompareOperator::NotEq {
                    negated(equal, span)
                } else {
                    equal
                })
            }
            Expr::UnaryOp {
                operator: UnaryOperator::Not,
                operand,
                ..
            } if self.part_access(operand).is_some() => {
                let truth = self.part_operation(operand, PartOperation::Truth, span)?;
                Some(negated(truth, span))
            }
            Expr::Call { .. } => {
                let (builtin, arguments, keywords) = self.called_builtin(expression)?;
                let ([argument], []) = (arguments, keywords) else {
                    return None;
                };
                let operation = match builtin {
                    Builtin::Str => PartOperation::Text { repr: false },
                    Builtin::Repr => PartOperation::Text { repr: true },
                    Builtin::Bool => PartOperation::Truth,
                    Builtin::Len => PartOperation::Length,
                    _ => return None,
                };
                self.part_access(argument)?;
                self.part_operation(argument, operation, span)
            }
            _ => None,
        }
    }

    /// Returns `str` of `args` or `code` when `expression` is one, as `print` and an
    /// f-string field write it, or `repr` when `repr`.
    pub(super) fn written_part(&mut self, expression: &Expr, repr: bool) -> Option<TypedExpr> {
        self.part_access(expression)?;
        self.part_operation(expression, PartOperation::Text { repr }, expression.span())
    }

    /// Returns the truth of `args` or `code` when `expression` is one, as a condition tests it.
    pub(super) fn tested_part(&mut self, expression: &Expr) -> Option<TypedExpr> {
        self.part_access(expression)?;
        self.part_operation(expression, PartOperation::Truth, expression.span())
    }

    /// Returns the part an expression reads, with the exception it reads it from:
    /// `name.args` of an exception, or `name.code` of a `SystemExit`, where `name` is a
    /// variable.
    fn part_access<'expression>(
        &self,
        expression: &'expression Expr,
    ) -> Option<(ExceptionPart, &'expression Expr)> {
        let Expr::Attribute {
            value, attribute, ..
        } = expression
        else {
            return None;
        };
        let Expr::Name { id, .. } = value.as_ref() else {
            return None;
        };
        let symbol = self
            .locals
            .get(id)
            .or_else(|| self.program.module_symbols.get(id))?;
        let ty = self
            .types
            .get(symbol)
            .unwrap_or(&self.program.symbols[symbol.0 as usize].ty);
        let Type::Exception(class) = ty else {
            return None;
        };
        match attribute.name.as_str() {
            "args" => Some((ExceptionPart::Arguments, value.as_ref())),
            "code" if is_system_exit(class) => Some((ExceptionPart::Code, value.as_ref())),
            _ => None,
        }
    }

    /// Returns `operation` on the part `accessed` reads, or `None` after reporting `len` of
    /// `code`, which has no length.
    fn part_operation(
        &mut self,
        accessed: &Expr,
        operation: PartOperation,
        span: Span,
    ) -> Option<TypedExpr> {
        let (part, value) = self.part_access(accessed)?;
        if part == ExceptionPart::Code && operation == PartOperation::Length {
            self.diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(201),
                    span,
                    "`len` of `code` is planned for TyPy 0.5 (EXC-15)",
                )
                .with_rule(RuleId::EXC_15),
            );
            return Some(Self::invalid_call(span));
        }
        let exception = self.expression(value);
        let ty = match operation {
            PartOperation::Text { .. } => Type::Str,
            PartOperation::Length => Type::Int,
            PartOperation::Truth | PartOperation::Equal(_) => Type::Bool,
        };
        Some(Self::typed(
            TypedExprKind::ExceptionPart {
                exception: Box::new(exception),
                part,
                operation,
            },
            ty,
            span,
        ))
    }
}

impl ExceptionPart {
    /// Returns the attribute's name.
    const fn name(self) -> &'static str {
        match self {
            Self::Arguments => "args",
            Self::Code => "code",
        }
    }
}

/// Returns true for `SystemExit` and the classes derived from it.
fn is_system_exit(class: &ExceptionClass) -> bool {
    class.builtin() == BuiltinException::SystemExit
}

/// Returns `not value` of a `bool`.
fn negated(value: TypedExpr, span: Span) -> TypedExpr {
    TypedExpr {
        kind: TypedExprKind::Unary {
            operator: UnaryOperator::Not,
            operand: Box::new(value),
        },
        ty: Type::Bool,
        span,
    }
}
