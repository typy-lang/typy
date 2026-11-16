//! Container methods: `len(x)` calls `__len__`, `x[k]` `__getitem__`, `x[k] = v`
//! `__setitem__`, `del x[k]` `__delitem__`, and `v in x` `__contains__` (CLS-07).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{BoolOperator, CompareOperator, Expr, UnaryOperator};

use super::BodyChecker;
use crate::typed::{Argument, FunctionId, TypedConstant, TypedExpr, TypedExprKind, TypedStmt};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Returns the method `name` of the class of an instance of type `ty`, own or inherited.
    pub(super) fn instance_method(&self, ty: &Type, name: &str) -> Option<FunctionId> {
        let Type::Instance(class) = ty else {
            return None;
        };
        self.program.method(class.id, name)
    }

    /// Returns the call of `method` on `receiver` with `values`, each checked against its
    /// parameter, the instance converted to the class that defines the method.
    fn container_call(
        &mut self,
        name: &str,
        method: FunctionId,
        receiver: TypedExpr,
        values: Vec<TypedExpr>,
        span: Span,
    ) -> TypedExpr {
        let signature = self.program.signatures[method.0 as usize].clone();
        let mut arguments = vec![Argument {
            parameter: 0,
            value: Self::converted(receiver, signature.parameters.first().map(|p| &p.ty)),
        }];
        // The checked signature gives the method one parameter for each value.
        for ((position, value), parameter) in values
            .into_iter()
            .enumerate()
            .zip(signature.parameters.iter().skip(1))
        {
            if !value.ty.is_assignable_to(&parameter.ty) {
                self.diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::error(404),
                        value.span,
                        format!(
                            "`{name}` takes {} for `{}`, not {}",
                            parameter.ty.with_article(),
                            parameter.name,
                            value.ty.with_article()
                        ),
                    )
                    .with_rule(RuleId::CLS_07),
                );
            }
            arguments.push(Argument {
                parameter: position + 1,
                value: Self::converted(value, Some(&parameter.ty)),
            });
        }
        Self::typed(
            TypedExprKind::Call {
                function: method,
                arguments,
                dispatch: true,
            },
            signature.returns,
            span,
        )
    }

    /// Returns `len(value)` through `__len__`, its result checked to be a length, as CPython
    /// checks it.
    pub(super) fn length_call(&mut self, value: &TypedExpr, span: Span) -> Option<TypedExpr> {
        let method = self.instance_method(&value.ty, "__len__")?;
        let call = self.container_call("__len__", method, value.clone(), Vec::new(), span);
        Some(Self::typed(
            TypedExprKind::CheckedLength(Box::new(call)),
            Type::Int,
            span,
        ))
    }

    /// Returns `value[index]` through `__getitem__`; a slice is planned, since TyPy has no
    /// slice object.
    pub(super) fn item_call(
        &mut self,
        value: TypedExpr,
        index: &Expr,
        span: Span,
    ) -> Result<TypedExpr, TypedExpr> {
        let Some(method) = self.instance_method(&value.ty, "__getitem__") else {
            return Err(value);
        };
        if let Expr::Slice { .. } = index {
            return Ok(self.slice_planned(index.span(), span));
        }
        let index = self.expression(index);
        Ok(self.container_call("__getitem__", method, value, vec![index], span))
    }

    /// Checks `target[index] = value` through `__setitem__`, when the target reads a variable
    /// that holds an instance of a class that defines it. The value is evaluated first, then
    /// the target and the index, as in CPython.
    pub(super) fn item_assignment(
        &mut self,
        target: &Expr,
        index: &Expr,
        value: &Expr,
        span: Span,
    ) -> Option<TypedStmt> {
        let ty = self.peek_type(target)?;
        let method = self.instance_method(&ty, "__setitem__")?;
        if let Expr::Slice { .. } = index {
            self.expression(value);
            self.slice_planned(index.span(), span);
            return Some(TypedStmt::Pass { span });
        }
        let value = self.expression(value);
        let target = self.expression(target);
        let index = self.expression(index);
        let call = self.container_call("__setitem__", method, target, vec![index, value], span);
        Some(TypedStmt::Expr { value: call, span })
    }

    /// Returns `del target[index]` through `__delitem__`, when the target reads a variable that
    /// holds an instance of a class that defines it.
    pub(super) fn item_deletion(
        &mut self,
        target: &Expr,
        index: &Expr,
        span: Span,
    ) -> Option<TypedExpr> {
        let ty = self.peek_type(target)?;
        let method = self.instance_method(&ty, "__delitem__")?;
        if let Expr::Slice { .. } = index {
            return Some(self.slice_planned(index.span(), span));
        }
        let target = self.expression(target);
        let index = self.expression(index);
        Some(self.container_call("__delitem__", method, target, vec![index], span))
    }

    /// Returns `item in container`, or `not in`, through the container's `__contains__`.
    pub(super) fn contains_call(
        &mut self,
        item: &TypedExpr,
        container: &TypedExpr,
        negated: bool,
        span: Span,
    ) -> Option<TypedExpr> {
        let Some(method) = self.instance_method(&container.ty, "__contains__") else {
            return self.iterated_contains(item, container, negated, span);
        };
        let call = self.container_call(
            "__contains__",
            method,
            container.clone(),
            vec![item.clone()],
            span,
        );
        Some(if negated {
            Self::typed(
                TypedExprKind::Unary {
                    operator: UnaryOperator::Not,
                    operand: Box::new(call),
                },
                Type::Bool,
                span,
            )
        } else {
            call
        })
    }

    /// Returns `item in container` for an instance whose class has `__iter__` and no
    /// `__contains__`: each item the instance gives is compared with the value, the item on the
    /// left, until one is the value or equal to it, as CPython's `PySequence_Contains` does
    /// (CLS-23).
    fn iterated_contains(
        &mut self,
        item: &TypedExpr,
        container: &TypedExpr,
        negated: bool,
        span: Span,
    ) -> Option<TypedExpr> {
        let Type::Instance(class) = &container.ty else {
            return None;
        };
        if self.program.method(class.id, "__iter__").is_none() {
            // A class with `__getitem__` alone is iterated by index, planned for 1.0.
            if self.program.method(class.id, "__getitem__").is_some() {
                self.diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::error(201),
                        span,
                        format!(
                            "`in` over {}, whose class has `__getitem__` and no `__iter__` or \
                             `__contains__`, is planned for TyPy 1.0 (CST-11)",
                            container.ty.with_article()
                        ),
                    )
                    .with_rule(RuleId::CST_11)
                    .with_help("define `__contains__` or `__iter__` in the class"),
                );
                return Some(Self::typed(
                    TypedExprKind::Constant(TypedConstant::Bool(false)),
                    Type::Bool,
                    span,
                ));
            }
            return None;
        }
        let (source, each_type) = self.protocol_source(container.clone(), "`in`");
        if !each_type.can_test_equality(&item.ty) {
            return None;
        }
        let held = self.hidden_variable("sought", item.ty.clone(), span);
        let each = self.hidden_variable("item", each_type.clone(), span);
        let read = |symbol, ty: &Type| Self::typed(TypedExprKind::Local(symbol), ty.clone(), span);
        let equal = self.single_comparison(
            CompareOperator::Eq,
            read(each, &each_type),
            read(held, &item.ty),
            span,
        );
        // An item that is the value is found without calling `__eq__`.
        let test = if matches!(each_type, Type::Instance(_)) && matches!(item.ty, Type::Instance(_))
        {
            let same = Self::typed(
                TypedExprKind::Compare {
                    left: Box::new(read(each, &each_type)),
                    operators: vec![CompareOperator::Is],
                    comparators: vec![read(held, &item.ty)],
                },
                Type::Bool,
                span,
            );
            Self::typed(
                TypedExprKind::BoolOp {
                    operator: BoolOperator::Or,
                    values: vec![same, equal],
                },
                Type::Bool,
                span,
            )
        } else {
            equal
        };
        Some(Self::typed(
            TypedExprKind::IterContains {
                value: Box::new(item.clone()),
                held,
                source: Box::new(source),
                each,
                test: Box::new(test),
                negated,
            },
            Type::Bool,
            span,
        ))
    }

    /// Reports a slice of an instance, whose slice object TyPy does not make yet.
    fn slice_planned(&mut self, at: Span, span: Span) -> TypedExpr {
        self.error(
            Diagnostic::new(
                DiagnosticCode::error(201),
                at,
                "a slice of an instance, which passes a slice object to its method, is planned \
                 for TyPy 0.5 (CLS-07)",
            )
            .with_rule(RuleId::CLS_07),
            span,
        )
    }

    /// Returns the type of an expression that reads a variable or an attribute of one, without
    /// checking it, so a statement can choose how to check it.
    pub(super) fn peek_type(&self, expression: &Expr) -> Option<Type> {
        match expression {
            Expr::Name { id, .. } => {
                let symbol = self.locals.get(id)?;
                self.types.get(symbol).cloned()
            }
            Expr::Attribute {
                value, attribute, ..
            } => {
                let owner = self.peek_type(value)?.class_id()?;
                let index = self.program.classes[owner.0 as usize].attribute(&attribute.name)?;
                self.program.attribute_ty(owner, index)
            }
            _ => None,
        }
    }
}
