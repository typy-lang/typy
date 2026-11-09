//! Checks list, set and dict comprehensions, each with its own scope (EXP-12, LR 6.2.4).

use typy_source::syntax::{Comprehension, Expr};

use super::BodyChecker;
use crate::check::{unhashable, unhashable_key};
use crate::typed::{Generator, SymbolKind, TypedConstant, TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Checks a comprehension. Its first source is read in the enclosing scope and each later
    /// one with the targets before it bound, as CPython does; its targets are variables of its
    /// own, so a name it binds hides an outer one inside it and leaves it as it was.
    pub(super) fn comprehension(
        &mut self,
        expression: &Expr,
        expected: Option<&Type>,
    ) -> TypedExpr {
        let span = expression.span();
        let (made, generators): (Vec<&Expr>, &[Comprehension]) = match expression {
            Expr::ListComp {
                element,
                generators,
                ..
            }
            | Expr::SetComp {
                element,
                generators,
                ..
            } => (vec![element], generators),
            Expr::DictComp {
                key,
                value,
                generators,
                ..
            } => (vec![key, value], generators),
            _ => unreachable!("a comprehension expression"),
        };
        let outer_locals = self.locals.clone();
        let outer_assigned = self.assigned.clone();
        let mut checked = Vec::with_capacity(generators.len());
        for generator in generators {
            let (source, item) = self.loop_source(&generator.iter, "a comprehension");
            self.bind_comprehension_target(&generator.target);
            let binding = TypedExpr {
                kind: TypedExprKind::Constant(TypedConstant::None),
                ty: item.clone(),
                span: generator.iter.span(),
            };
            let target = self.assign_target(&generator.target, item, &binding);
            // Each condition, and what comes after it, is read only where those before held.
            let conditions = generator
                .ifs
                .iter()
                .map(|condition| {
                    let condition = self.truth_test(condition);
                    self.add_facts(super::flow::test_facts(&condition).0);
                    condition
                })
                .collect();
            if let Some(target) = target {
                checked.push(Generator {
                    target,
                    source,
                    conditions,
                });
            }
        }
        let result = self.comprehension_result(expression, &made, expected);
        self.locals = outer_locals;
        self.assigned = outer_assigned;
        let Some((element, value, ty)) = result else {
            return Self::invalid_comprehension(span);
        };
        if checked.len() < generators.len() {
            return Self::invalid_comprehension(span);
        }
        Self::typed(
            TypedExprKind::Comprehension {
                element: Box::new(element),
                value: value.map(Box::new),
                generators: checked,
            },
            ty,
            span,
        )
    }

    /// Checks what a comprehension makes, with the expected item types, and returns it with
    /// the comprehension's type, or `None` after reporting why it has none.
    fn comprehension_result(
        &mut self,
        expression: &Expr,
        made: &[&Expr],
        expected: Option<&Type>,
    ) -> Option<(TypedExpr, Option<TypedExpr>, Type)> {
        let span = expression.span();
        match (expression, made) {
            (Expr::DictComp { .. }, [key, value]) => {
                let (key_type, value_type) = match expected {
                    Some(Type::Dict(key, value)) => (Some(key.as_ref()), Some(value.as_ref())),
                    _ => (None, None),
                };
                let key = self.expression_expecting(key, key_type);
                let value = self.expression_expecting(value, value_type);
                if key.ty.has_error() || value.ty.has_error() {
                    return None;
                }
                if !key.ty.is_hashable() {
                    self.diagnostics.push(unhashable_key(&key.ty, key.span));
                    return None;
                }
                let ty = Type::Dict(Box::new(key.ty.clone()), Box::new(value.ty.clone()));
                Some((key, Some(value), ty))
            }
            (_, [element]) => {
                let set = matches!(expression, Expr::SetComp { .. });
                let item = match expected {
                    Some(Type::List(item)) if !set => Some(item.as_ref()),
                    Some(Type::Set(item)) if set => Some(item.as_ref()),
                    _ => None,
                };
                let element = self.expression_expecting(element, item);
                if element.ty.has_error() {
                    return None;
                }
                if set && !element.ty.is_hashable() {
                    self.diagnostics.push(unhashable(&element.ty, span));
                    return None;
                }
                let item = Box::new(element.ty.clone());
                let ty = if set {
                    Type::Set(item)
                } else {
                    Type::List(item)
                };
                Some((element, None, ty))
            }
            _ => unreachable!("a comprehension makes an element, or a key and a value"),
        }
    }

    /// Gives every name a comprehension target binds a new variable of the comprehension.
    fn bind_comprehension_target(&mut self, target: &Expr) {
        match target {
            Expr::Tuple { elements, .. } | Expr::List { elements, .. } => {
                for element in elements {
                    self.bind_comprehension_target(element);
                }
            }
            Expr::Name { id, span } => {
                let symbol = self
                    .program
                    .add_symbol(id, SymbolKind::Local, Type::Error, *span);
                self.locals.insert(id.clone(), symbol);
                self.local_order.push(symbol);
            }
            _ => {}
        }
    }

    fn invalid_comprehension(span: typy_diagnostics::Span) -> TypedExpr {
        Self::typed(
            TypedExprKind::Constant(TypedConstant::None),
            Type::Error,
            span,
        )
    }
}
