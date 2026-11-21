//! A named tuple read as the tuple of its fields where a tuple operation applies (CLS-27,
//! ADR-0088): an index, a slice, `len`, a loop, unpacking, `*` in a call, a sequence pattern,
//! `in`, `+` and a comparison with a tuple. The hidden method written for its class returns
//! the tuple, so the value is evaluated once. `_replace` calls the other hidden method, and the
//! members planned for 0.14 are refused (CLS-43).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{BinaryOperator, CompareOperator, Expr, Identifier, Keyword};

use super::BodyChecker;
use crate::class_type::{NAMED_TUPLE_FIELDS, NAMED_TUPLE_REPLACE};
use crate::typed::{Argument, ClassId, Passed, TypedConstant, TypedExpr, TypedExprKind};
use crate::types::{Type, TypeObjectOf};

impl BodyChecker<'_> {
    /// Checks `expression` and returns it as `tuple_view` does.
    pub(super) fn tuple_expression(&mut self, expression: &Expr) -> TypedExpr {
        let value = self.expression(expression);
        self.tuple_view(value)
    }

    /// Returns `value` as the tuple of its items when it is `sys.version_info` or a named
    /// tuple; any other value as it is.
    pub(super) fn tuple_view(&self, value: TypedExpr) -> TypedExpr {
        self.fields_tuple(Self::version_tuple(value))
    }

    /// Returns a named tuple as the tuple of its fields; any other value as it is.
    pub(super) fn fields_tuple(&self, value: TypedExpr) -> TypedExpr {
        let Type::Instance(class) = &value.ty else {
            return value;
        };
        if !class.operators.named_tuple() {
            return value;
        }
        let Some(method) = self.program.method(class.id, NAMED_TUPLE_FIELDS) else {
            return value;
        };
        let signature = &self.program.signatures[method.0 as usize];
        let ty = signature.returns.clone();
        let span = value.span;
        let receiver = Self::converted(
            value,
            signature.parameters.first().map(|parameter| &parameter.ty),
        );
        Self::typed(
            TypedExprKind::Call {
                function: method,
                arguments: vec![Argument {
                    passed: Passed::Position,
                    parameter: 0,
                    value: receiver,
                }],
                dispatch: false,
            },
            ty,
            span,
        )
    }

    /// Returns the operands of a comparison that no method of their classes takes: for `in`,
    /// a named tuple on the right read as its fields; for another comparison, each named tuple
    /// read so when both operands are tuples or named tuples.
    pub(super) fn compared_tuples(
        &self,
        operator: CompareOperator,
        left: TypedExpr,
        right: TypedExpr,
    ) -> (TypedExpr, TypedExpr) {
        match operator {
            CompareOperator::In | CompareOperator::NotIn => (left, self.fields_tuple(right)),
            CompareOperator::Is | CompareOperator::IsNot => (left, right),
            _ if [&left.ty, &right.ty].iter().all(|ty| is_tuple(ty))
                && [&left.ty, &right.ty].iter().any(|ty| is_named_tuple(ty)) =>
            {
                (self.fields_tuple(left), self.fields_tuple(right))
            }
            _ => (left, right),
        }
    }

    /// Returns the operands of `+` and `*` that no method of their classes takes, each named
    /// tuple read as its fields, so they follow the rules of a fixed tuple: `+` joins two.
    pub(super) fn arithmetic_tuples(
        &self,
        operator: BinaryOperator,
        left: TypedExpr,
        right: TypedExpr,
    ) -> (TypedExpr, TypedExpr) {
        if matches!(operator, BinaryOperator::Add | BinaryOperator::Mult) {
            (self.fields_tuple(left), self.fields_tuple(right))
        } else {
            (left, right)
        }
    }
}

impl BodyChecker<'_> {
    /// Checks a call of a member of a named tuple the checker gives itself: `_replace` on an
    /// instance, and the members planned for 0.14; gives back the receiver of any other call.
    pub(super) fn named_tuple_call(
        &mut self,
        receiver: TypedExpr,
        name: &Identifier,
        (arguments, keywords): (&[Expr], &[Keyword]),
        span: Span,
    ) -> Result<TypedExpr, TypedExpr> {
        if let Some(refused) = self.planned_member(&receiver.ty, name, true) {
            for argument in arguments {
                self.expression(argument);
            }
            return Ok(refused);
        }
        match &receiver.ty {
            Type::Instance(class) if name.name == "_replace" && class.operators.named_tuple() => {
                let class = class.id;
                Ok(self.replace_call(receiver, class, (arguments, keywords), span))
            }
            _ => Err(receiver),
        }
    }

    /// Checks `receiver._replace(name=value, ...)` of a named tuple: a call of the hidden
    /// method of the receiver's class, which takes each field named as `(value,)` and each
    /// other as `None`, so the instance it makes is of the receiver's own class.
    fn replace_call(
        &mut self,
        receiver: TypedExpr,
        class: ClassId,
        (arguments, keywords): (&[Expr], &[Keyword]),
        span: Span,
    ) -> TypedExpr {
        let class_name = self.program.classes[class.0 as usize].name.clone();
        let Some(method) = self.program.method(class, NAMED_TUPLE_REPLACE) else {
            return Self::invalid_call(span);
        };
        if !arguments.is_empty() {
            return self.replace_refused(
                (arguments, keywords),
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    span,
                    format!(
                        "{class_name}._replace() takes 1 positional argument but {} were given, \
                         as CPython raises `TypeError`",
                        arguments.len() + 1
                    ),
                )
                .with_rule(RuleId::CLS_27),
                span,
            );
        }
        if let Some(spread) = keywords.iter().find(|keyword| keyword.name.is_none()) {
            // The call takes `**` in a later release, so its refusal stands for the general one.
            self.unpacking_calls.push(span);
            return self.replace_refused(
                (arguments, keywords),
                planned(spread.span, "`**` in a call of `_replace`"),
                span,
            );
        }
        let signature = self.program.signatures[method.0 as usize].clone();
        let fields = &signature.parameters[1..];
        let unknown: Vec<String> = keywords
            .iter()
            .filter_map(|keyword| keyword.name.as_ref())
            .filter(|name| fields.iter().all(|field| field.name != name.name))
            .map(|name| format!("'{}'", name.name))
            .collect();
        if !unknown.is_empty() {
            return self.replace_refused(
                (arguments, keywords),
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    span,
                    format!(
                        "Got unexpected field names: [{}], as CPython raises `ValueError` up to \
                         3.12 and `TypeError` from 3.13",
                        unknown.join(", ")
                    ),
                )
                .with_rule(RuleId::CLS_27),
                span,
            );
        }
        let receiver = Self::converted(
            receiver,
            signature.parameters.first().map(|parameter| &parameter.ty),
        );
        let mut passed = vec![Argument {
            passed: Passed::Position,
            parameter: 0,
            value: receiver,
        }];
        for (position, field) in fields.iter().enumerate() {
            let given = keywords.iter().find(|keyword| {
                keyword
                    .name
                    .as_ref()
                    .is_some_and(|name| name.name == field.name)
            });
            let value = match given {
                Some(keyword) => self.replaced_value(&keyword.value, field),
                None => Self::typed(
                    TypedExprKind::Constant(TypedConstant::None),
                    Type::NoneType,
                    span,
                ),
            };
            passed.push(Argument {
                passed: Passed::Position,
                parameter: position + 1,
                value: Self::converted(value, Some(&field.ty)),
            });
        }
        Self::typed(
            TypedExprKind::Call {
                function: method,
                arguments: passed,
                dispatch: true,
            },
            signature.returns,
            span,
        )
    }

    /// Returns the value `_replace` gives a field, checked against the field's type and held
    /// as `(value,)`.
    fn replaced_value(
        &mut self,
        value: &Expr,
        field: &crate::typed::SignatureParameter,
    ) -> TypedExpr {
        let ty = match &field.ty {
            Type::Optional(inner) => match inner.as_ref() {
                Type::Tuple(items) => items.first().cloned().unwrap_or(Type::Error),
                _ => Type::Error,
            },
            _ => Type::Error,
        };
        let value = self.expression_expecting(value, Some(&ty));
        self.expect(&value, ty.clone(), &format!("the field `{}`", field.name));
        let span = value.span;
        let value = Self::converted(value, Some(&ty));
        Self::typed(
            TypedExprKind::Tuple(vec![value]),
            Type::Tuple(vec![ty]),
            span,
        )
    }

    /// Checks the arguments of a refused call of `_replace` for their own errors, and returns
    /// the call after reporting `diagnostic`.
    fn replace_refused(
        &mut self,
        (arguments, keywords): (&[Expr], &[Keyword]),
        diagnostic: Diagnostic,
        span: Span,
    ) -> TypedExpr {
        for argument in arguments {
            self.expression(argument);
        }
        for keyword in keywords {
            self.expression(&keyword.value);
        }
        self.diagnostics.push(diagnostic);
        Self::invalid_call(span)
    }

    /// Reports a member of a named tuple, or of its class, that TyPy gives from 0.14:
    /// `_make`, `_field_defaults`, and `_replace` other than called on an instance (CLS-43).
    /// `called` says whether the member is called; returns the refused read.
    pub(super) fn planned_member(
        &mut self,
        ty: &Type,
        member: &Identifier,
        called: bool,
    ) -> Option<TypedExpr> {
        let (class, instance) = match ty {
            Type::Instance(class) if class.operators.named_tuple() => (class.id, true),
            Type::TypeObject(Some(of)) => match of.as_ref() {
                TypeObjectOf::Instance(class, _) if self.is_named_tuple_class(*class) => {
                    (*class, false)
                }
                _ => return None,
            },
            _ => return None,
        };
        let refused = match member.name.as_str() {
            "_make" | "_field_defaults" => true,
            "_replace" => !(called && instance),
            _ => false,
        };
        if !refused {
            return None;
        }
        let name = &self.program.classes[class.0 as usize].name;
        let what = if member.name == "_replace" {
            format!("`{name}._replace` other than called on an instance")
        } else {
            format!("`{name}.{}`", member.name)
        };
        self.diagnostics.push(planned(member.span, &what));
        Some(Self::invalid_call(member.span))
    }

    /// Returns true when `class` is a named tuple or derives from one.
    fn is_named_tuple_class(&self, class: ClassId) -> bool {
        self.program.classes[class.0 as usize].frozen == Some(crate::check::Frozen::NamedTuple)
    }
}

/// Returns the error for a member of a named tuple TyPy gives from 0.14 (CLS-43).
fn planned(span: Span, what: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::error(201),
        span,
        format!("{what} is planned for TyPy 0.14 (CLS-43)"),
    )
    .with_rule(RuleId::CLS_43)
}

/// Returns true for a named tuple.
fn is_named_tuple(ty: &Type) -> bool {
    matches!(ty, Type::Instance(class) if class.operators.named_tuple())
}

/// Returns true for a tuple of any form or a named tuple.
fn is_tuple(ty: &Type) -> bool {
    matches!(ty, Type::Tuple(_) | Type::TupleOf(_)) || is_named_tuple(ty)
}
