//! Augmented assignments that call a class's methods (CLS-20): `x += y` calls `x.__iadd__(y)`
//! when the class of `x` defines it, and `x + y`'s method otherwise, and rebinds the target to
//! the result. A target that is an item or an attribute of an expression is evaluated once:
//! its object, and its key, are held in hidden variables first, as CPython evaluates them once.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{BinaryOperator, Expr, Stmt};

use super::BodyChecker;
use crate::typed::{Argument, AssignTarget, SymbolKind, TypedExpr, TypedExprKind, TypedStmt};
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
    /// method and it takes the value; a class derived from it that defines the method alone
    /// is reported, since the call would depend on the class of the object.
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
                if let Expr::Slice { .. } = index.as_ref() {
                    self.diagnostics.push(
                        Diagnostic::new(
                            DiagnosticCode::error(201),
                            span,
                            "an augmented assignment to a slice that calls an operator method is \
                             planned for TyPy 0.5 (CLS-23)",
                        )
                        .with_rule(RuleId::CLS_23)
                        .with_help("write `part = part + value`"),
                    );
                    return Pinned::Checked(Box::new(None));
                }
                let container = self.pinned(container);
                let index = self.pinned(index);
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
        let span = expression.span();
        let value = self.expression(expression);
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
