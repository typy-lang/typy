//! Container methods: `len(x)` calls `__len__`, `x[k]` `__getitem__`, `x[k] = v`
//! `__setitem__`, `del x[k]` `__delitem__`, and `v in x` `__contains__` (CLS-07).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{BinaryOperator, BoolOperator, CompareOperator, Expr, UnaryOperator};

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
            passed: crate::typed::Passed::Position,
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
                passed: crate::typed::Passed::Position,
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

    /// Returns `value[index]` through `__getitem__`, which takes a slice object for
    /// `value[lower:upper:step]` (CLS-23).
    pub(super) fn item_call(
        &mut self,
        value: TypedExpr,
        index: &Expr,
        span: Span,
    ) -> Result<TypedExpr, TypedExpr> {
        let Some(method) = self.instance_method(&value.ty, "__getitem__") else {
            return Err(value);
        };
        let index = self.index_value(index, span);
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
        let value = self.expression(value);
        let target = self.expression(target);
        let index = self.index_value(index, span);
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
        let target = self.expression(target);
        let index = self.index_value(index, span);
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
    /// `__contains__`, or for a `ctypes` array: each item the container gives is compared with
    /// the value, the item on the left, until one is the value or equal to it, as CPython's
    /// `PySequence_Contains` does (CLS-23, FFI-03).
    fn iterated_contains(
        &mut self,
        item: &TypedExpr,
        container: &TypedExpr,
        negated: bool,
        span: Span,
    ) -> Option<TypedExpr> {
        if let Type::CData(crate::ctypes::CData::Array(each, _)) = &container.ty {
            let source = crate::typed::LoopSource::Items(container.clone());
            return self.contains_by_equality(item, (source, each.result_type()), negated, span);
        }
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
        let source = self.protocol_source(container.clone(), "`in`");
        self.contains_by_equality(item, source, negated, span)
    }

    /// Returns `item in` the items `source` gives, each of type `each_type`, compared with the
    /// value for equality; `None` when they cannot be compared with it.
    fn contains_by_equality(
        &mut self,
        item: &TypedExpr,
        (source, each_type): (crate::typed::LoopSource, Type),
        negated: bool,
        span: Span,
    ) -> Option<TypedExpr> {
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

    /// Returns true when `current`, what the augmented assignment to `target` reads, is a call
    /// of the getter of a property, whose setter the result goes to (CLS-06).
    pub(super) fn is_property_read(&self, target: &Expr, current: &TypedExpr) -> bool {
        let (
            Expr::Attribute { attribute, .. },
            TypedExprKind::Call {
                function,
                arguments,
                ..
            },
        ) = (target, &current.kind)
        else {
            return false;
        };
        arguments.first().is_some_and(|object| {
            object.value.ty.class_id().is_some_and(|class| {
                self.property_getter(class, &attribute.name) == Some(*function)
            })
        })
    }

    /// Returns true when `current`, the item an augmented assignment reads, is a call of
    /// `__getitem__` of an instance.
    pub(super) fn is_instance_item(&self, current: &TypedExpr) -> bool {
        let TypedExprKind::Call {
            function,
            arguments,
            ..
        } = &current.kind
        else {
            return false;
        };
        arguments.first().is_some_and(|container| {
            self.instance_method(&container.value.ty, "__getitem__") == Some(*function)
        })
    }

    /// Returns `container[index] op= value` of an instance: the item `__getitem__` gave,
    /// combined with the value as the operator combines them, given to `__setitem__`, which
    /// CPython calls in that order. The container and the index are names here, or hidden
    /// variables `pinned_augmented` made, so each is evaluated once (CLS-07, CLS-23).
    pub(super) fn instance_item_augmented(
        &mut self,
        target: &Expr,
        operator: BinaryOperator,
        (current, value): (TypedExpr, TypedExpr),
        span: Span,
    ) -> Option<TypedStmt> {
        let Expr::Subscript {
            value: container,
            index,
            ..
        } = target
        else {
            return None;
        };
        let container = self.expression(container);
        let Some(setter) = self.instance_method(&container.ty, "__setitem__") else {
            self.unassignable_item(&container.ty, span);
            return None;
        };
        let result = self.augmented_result(operator, current, value, span);
        let index = self.index_value(index, span);
        let call = self.container_call("__setitem__", setter, container, vec![index, result], span);
        Some(TypedStmt::Expr { value: call, span })
    }

    /// Returns `current op value` as an augmented assignment computes it: the in-place
    /// method, the operator's method, or the built-in operator.
    pub(super) fn augmented_result(
        &mut self,
        operator: BinaryOperator,
        current: TypedExpr,
        value: TypedExpr,
        span: Span,
    ) -> TypedExpr {
        if let Some(call) = self.in_place_call(operator, &current, &value, span) {
            return call;
        }
        let (current, value) = match self.operator_binary(operator, current, value, span) {
            super::operator_method::Comparison::Called(call) => return call,
            super::operator_method::Comparison::Builtin(current, value) => (current, value),
        };
        let (current, value) = Self::tuple_operands(operator, current, value);
        if let Some(formatted) = self.percent_dynamic(operator, &current, &value, span) {
            return formatted;
        }
        let ty = self.binary_type(operator, &current, &value, span);
        Self::typed(
            TypedExprKind::Binary {
                left: Box::new(current),
                operator,
                right: Box::new(value),
            },
            ty,
            span,
        )
    }

    /// Returns the index an instance's container method takes: the expression, or the slice
    /// object `x[lower:upper:step]` passes (CLS-23).
    pub(super) fn index_value(&mut self, index: &Expr, span: Span) -> TypedExpr {
        match index {
            Expr::Slice {
                lower, upper, step, ..
            } => self.slice_object((lower.as_deref(), upper.as_deref(), step.as_deref()), span),
            other => self.expression(other),
        }
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
