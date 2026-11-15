//! `args` of an exception and `code` of a `SystemExit` (EXC-11, EXC-13, EXC-15, ADR-0030,
//! ADR-0034). Printing, formatting, comparing with `==` and `!=`, testing and `len` of
//! `args` are one operation on the exception, which the runtime answers from its arguments.
//! Read as a value of its own, `args` has the type the creations of the exception's class
//! give it, and `code` the type that follows from that.

use typy_diagnostics::Span;
use typy_source::syntax::{CompareOperator, Expr, UnaryOperator};

use super::BodyChecker;
use super::each_member::Split;
use crate::builtin::Builtin;
use crate::exception::{BuiltinException, ExceptionClass};
use crate::typed::{
    EachMember, ExceptionPart, PartOperation, SymbolId, TypedConstant, TypedExpr, TypedExprKind,
};
use crate::types::{Type, TypeObjectOf};

impl BodyChecker<'_> {
    /// Returns the operation an expression makes of `args` or `code`, when it makes one; a
    /// use of either that is not one is reported.
    pub(super) fn exception_part_use(&mut self, expression: &Expr) -> Option<TypedExpr> {
        let span = expression.span();
        match expression {
            Expr::Attribute { value, .. } => {
                let (part, _) = self.part_access(expression)?;
                Some(match part {
                    ExceptionPart::Arguments => self.arguments_value(expression, value, span),
                    ExceptionPart::Code => self.code_value(expression, value, span),
                })
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
                let (part, _) = self.part_access(argument)?;
                let operation = match builtin {
                    Builtin::Str => PartOperation::Text { repr: false },
                    Builtin::Repr => PartOperation::Text { repr: true },
                    Builtin::Bool => PartOperation::Truth,
                    // `code` has a length only when it is a tuple, which its type says.
                    Builtin::Len if part == ExceptionPart::Arguments => PartOperation::Length,
                    _ => return None,
                };
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

    /// Returns `args` of the exception `value` holds, read as a value: a tuple, or a union of
    /// tuples of the layouts the program and the runtime create it with (ADR-0034). While an
    /// operation is checked for each member, it is the member.
    fn arguments_value(&mut self, accessed: &Expr, value: &Expr, span: Span) -> TypedExpr {
        let (symbol, class) = self
            .arguments_access(accessed)
            .expect("`args` of a variable that holds an exception");
        let split = Split {
            symbol,
            arguments: true,
        };
        let member = self
            .splitting
            .iter()
            .rev()
            .find(|(each, _)| *each == split)
            .map(|(_, member)| member.clone());
        let ty = self.program.arguments_type(&class, span);
        let exception = self.expression(value);
        Self::typed(
            TypedExprKind::ExceptionArguments(Box::new(exception)),
            member.unwrap_or(ty),
            span,
        )
    }

    /// Returns `code` of the `SystemExit` `value` holds, read as a value: for each member of
    /// the type of its `args`, `None` for no arguments, the argument for one, and the tuple
    /// for several, as CPython's `SystemExit.__init__` sets it (EXC-13, ADR-0034).
    fn code_value(&mut self, accessed: &Expr, value: &Expr, span: Span) -> TypedExpr {
        let (_, class) = self
            .exception_access(accessed)
            .expect("`code` of a variable that holds a `SystemExit`");
        let arguments = self.program.arguments_type(&class, span);
        let read = |this: &mut Self, ty: Type| {
            let exception = this.expression(value);
            Self::typed(
                TypedExprKind::ExceptionArguments(Box::new(exception)),
                ty,
                span,
            )
        };
        if arguments.has_error() {
            return read(self, Type::Error);
        }
        let members = arguments.members().to_vec();
        let ty = Type::union(members.iter().map(code_of).collect());
        let arm = |this: &mut Self, member: &Type| {
            let code = match member {
                Type::Tuple(items) if items.is_empty() => Self::typed(
                    TypedExprKind::Constant(TypedConstant::None),
                    Type::NoneType,
                    span,
                ),
                Type::Tuple(items) if items.len() == 1 => Self::typed(
                    TypedExprKind::TupleItem {
                        tuple: Box::new(read(this, member.clone())),
                        index: 0,
                    },
                    items[0].clone(),
                    span,
                ),
                _ => read(this, member.clone()),
            };
            Self::converted(code, Some(&ty))
        };
        if let [member] = members.as_slice() {
            return arm(self, member);
        }
        let value = read(self, arguments);
        let arms = members
            .iter()
            .map(|member| (TypeObjectOf::Tuple, arm(self, member)))
            .collect();
        Self::typed(
            TypedExprKind::EachMember(Box::new(EachMember { value, arms })),
            ty,
            span,
        )
    }

    /// Returns `args` of the exception a variable holds, with the members of the union of
    /// tuples it is, when no operation is already split on it.
    pub(super) fn union_arguments(&self, operand: &Expr) -> Option<(Split, Vec<Type>)> {
        let (symbol, class) = self.arguments_access(operand)?;
        let split = Split {
            symbol,
            arguments: true,
        };
        if self.splitting.iter().any(|(each, _)| *each == split) {
            return None;
        }
        match self.program.known_arguments_type(&class)? {
            Type::Union(members) => Some((split, members)),
            _ => None,
        }
    }

    /// Returns the variable and the class of the exception it holds, when `expression` is
    /// `name.args` of a variable that holds an exception.
    fn arguments_access(&self, expression: &Expr) -> Option<(SymbolId, ExceptionClass)> {
        match self.part_access(expression)? {
            (ExceptionPart::Arguments, _) => self.exception_access(expression),
            (ExceptionPart::Code, _) => None,
        }
    }

    /// Returns the variable and the class of the exception it holds, when `expression` is
    /// `name.args` or `name.code` of a variable that holds an exception.
    fn exception_access(&self, expression: &Expr) -> Option<(SymbolId, ExceptionClass)> {
        let (_, Expr::Name { id, .. }) = self.part_access(expression)? else {
            return None;
        };
        let symbol = *self
            .locals
            .get(id)
            .or_else(|| self.program.module_symbols.get(id))?;
        let ty = self
            .types
            .get(&symbol)
            .unwrap_or(&self.program.symbols[symbol.0 as usize].ty);
        let Type::Exception(class) = ty else {
            return None;
        };
        Some((symbol, class.clone()))
    }

    /// Returns `operation` on the part `accessed` reads.
    fn part_operation(
        &mut self,
        accessed: &Expr,
        operation: PartOperation,
        span: Span,
    ) -> Option<TypedExpr> {
        let (part, value) = self.part_access(accessed)?;
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

/// Returns the type of `code` of a `SystemExit` whose `args` is `arguments`.
fn code_of(arguments: &Type) -> Type {
    match arguments {
        Type::Tuple(items) if items.is_empty() => Type::NoneType,
        Type::Tuple(items) if items.len() == 1 => items[0].clone(),
        other => other.clone(),
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
