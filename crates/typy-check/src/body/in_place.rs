//! Augmented assignments that call a class's methods (CLS-20): `x += y` calls `x.__iadd__(y)`
//! when the class of `x` defines it, and `x + y`'s method otherwise, and rebinds the target to
//! the result. A target that is an item or an attribute of an expression is evaluated once:
//! its object, and its key, are held in hidden variables first, as CPython evaluates them once.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{BinaryOperator, Expr, Stmt};

use super::BodyChecker;
use crate::typed::{
    Argument, AssignTarget, OperatorCall, OperatorKind, SymbolKind, TypedExpr, TypedExprKind,
    TypedStmt,
};
use crate::types::Type;

/// What `pinned_augmented` makes of an augmented assignment.
pub(super) enum Pinned {
    /// The target is a variable, or an item or attribute of one, which needs no pins.
    Unpinned,
    /// The checked statement that holds the target's parts first, or `None` after a report.
    Checked(Box<Option<TypedStmt>>),
}

impl BodyChecker<'_> {
    /// Returns `target.__iop__(value)` when the class of the target defines the in-place
    /// method and it takes the value, and when the method may return `NotImplemented`, the
    /// call that gives way to `target op value`; a class derived from it that defines the
    /// method alone is reported, since the call would depend on the class of the object.
    pub(super) fn in_place_call(
        &mut self,
        operator: BinaryOperator,
        current: &TypedExpr,
        value: &TypedExpr,
        span: Span,
    ) -> Option<TypedExpr> {
        let Type::Instance(class) = &current.ty else {
            return None;
        };
        let (forward, _) = super::operator_method::binary_names(operator);
        let name = format!("__i{}", &forward[2..]);
        let Some(method) = self.operator_method(&current.ty, &name, &value.ty) else {
            if self.program.method(class.id, &name).is_none() && self.defined_below(class.id, &name)
            {
                self.diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::error(201),
                        span,
                        format!(
                            "an augmented assignment whose target's class has no `{name}` while \
                             a class derived from it defines one is planned for TyPy 1.0 \
                             (CLS-32)"
                        ),
                    )
                    .with_rule(RuleId::CLS_32)
                    .with_help(format!("define `{name}` in `{}`", class.name)),
                );
            }
            return None;
        };
        let signature = self.program.signatures[method.0 as usize].clone();
        if signature.returns.may_give_way() {
            return Some(self.giving_way_in_place(operator, method, current, value, span));
        }
        let parameter = |index: usize| signature.parameters.get(index).map(|each| &each.ty);
        Some(Self::typed(
            TypedExprKind::Call {
                function: method,
                arguments: vec![
                    Argument {
                        parameter: 0,
                        value: Self::converted(current.clone(), parameter(0)),
                    },
                    Argument {
                        parameter: 1,
                        value: Self::converted(value.clone(), parameter(1)),
                    },
                ],
                dispatch: true,
            },
            signature.returns,
            span,
        ))
    }

    /// Returns `target op= value` whose in-place method may give way (CLS-21): the method runs
    /// first, then the operator's methods, and then the augmented assignment raises
    /// `TypeError`.
    fn giving_way_in_place(
        &mut self,
        operator: BinaryOperator,
        method: crate::typed::FunctionId,
        current: &TypedExpr,
        value: &TypedExpr,
        span: Span,
    ) -> TypedExpr {
        let (name, reflected) = super::operator_method::binary_names(operator);
        let kind = OperatorKind::Arithmetic(super::operator_method::binary_symbol(operator, true));
        let own = self.program.signatures[method.0 as usize]
            .returns
            .implemented();
        let binary = self.operator_method_call((name, reflected), current, value, kind, span);
        let (mut call, ty) = match binary {
            Some(TypedExpr {
                kind: TypedExprKind::OperatorCall(call),
                ty,
                ..
            }) => (call, Type::union(vec![own, ty])),
            _ => (
                Box::new(OperatorCall {
                    left: current.clone(),
                    right: value.clone(),
                    forward: None,
                    reflected: None,
                    left_reflected: None,
                    negated: false,
                    in_place: None,
                    kind,
                }),
                own,
            ),
        };
        call.in_place = Some(method);
        Self::typed(TypedExprKind::OperatorCall(call), ty, span)
    }

    /// Returns `target op= value` for an item or an attribute of an expression whose operator
    /// calls a method: the object, and the key, are held in hidden variables, then the
    /// assignment reads and assigns through them.
    pub(super) fn pinned_augmented(
        &mut self,
        target: &Expr,
        operator: BinaryOperator,
        value: &Expr,
        span: Span,
    ) -> Pinned {
        let rewritten = match target {
            Expr::Subscript {
                value: container,
                index,
                span: item_span,
            } if !is_name(container) || !is_name(index) => {
                let container = self.pinned(container);
                // A slice is made once, and passed to `__getitem__` and `__setitem__` (CLS-23).
                let index = if let Expr::Slice { .. } = index.as_ref() {
                    let slice = self.index_value(index, *item_span);
                    self.pinned_value(slice)
                } else {
                    self.pinned(index)
                };
                Expr::Subscript {
                    value: Box::new(container),
                    index: Box::new(index),
                    span: *item_span,
                }
            }
            Expr::Attribute {
                value: object,
                attribute,
                span: attribute_span,
            } if !is_name(object) => Expr::Attribute {
                value: Box::new(self.pinned(object)),
                attribute: attribute.clone(),
                span: *attribute_span,
            },
            _ => return Pinned::Unpinned,
        };
        let statement = Stmt::AugAssign {
            target: rewritten,
            operator,
            value: value.clone(),
            span,
        };
        let checked = self.statement(&statement);
        // The hidden variables are assigned first, in the order they were made.
        let mut pins = std::mem::take(&mut self.pins);
        pins.extend(checked);
        let first = (!pins.is_empty()).then(|| pins.remove(0));
        self.following.splice(0..0, pins);
        Pinned::Checked(Box::new(first))
    }

    /// Returns a name for `expression`, after noting the statement that assigns its value to
    /// a hidden variable, which `pinned_augmented` runs first.
    fn pinned(&mut self, expression: &Expr) -> Expr {
        let value = self.expression(expression);
        self.pinned_value(value)
    }

    /// Returns a name for a checked value, after noting the statement that assigns it to a
    /// hidden variable, as `pinned` does.
    fn pinned_value(&mut self, value: TypedExpr) -> Expr {
        let span = value.span;
        let name = format!("<pinned {}>", self.local_order.len());
        let symbol = self
            .program
            .add_symbol(&name, SymbolKind::Local, value.ty.clone(), span);
        self.locals.insert(name.clone(), symbol);
        self.local_order.push(symbol);
        let target = Expr::Name {
            id: name.clone(),
            span,
        };
        self.assign(&target, value.ty.clone(), &value);
        self.pins.push(TypedStmt::Assign {
            targets: vec![AssignTarget::Name(symbol)],
            value,
            span,
        });
        Expr::Name { id: name, span }
    }
}

/// Returns true for a variable, which reading again has no effect.
pub(super) fn is_name(expression: &Expr) -> bool {
    matches!(expression, Expr::Name { .. })
}
