//! Finds the local variables of a body and binds values to them (LR 4.2.2, TYP-02, TYP-03).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Expr, Stmt};

use super::BodyChecker;
use super::flow::Fact;
use crate::check::{AnnotationScope, annotation_type};
use crate::typed::{AssignTarget, SymbolId, SymbolKind, TypedConstant, TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Every name assigned anywhere in the body is a local of the body (LR 4.2.2).
    pub(super) fn collect_locals(&mut self, statements: &[&Stmt]) {
        for statement in statements {
            match statement {
                Stmt::Assign { targets, .. } => {
                    for target in targets {
                        self.bind_local(target);
                    }
                }
                Stmt::AugAssign { target, .. } => self.bind_local(target),
                Stmt::AnnAssign {
                    target, annotation, ..
                } => {
                    self.bind_local(target);
                    self.declare(target, annotation);
                }
                Stmt::For {
                    target,
                    body,
                    orelse,
                    ..
                } => {
                    self.bind_local(target);
                    self.collect_locals(&body.iter().chain(orelse).collect::<Vec<_>>());
                }
                Stmt::While { body, orelse, .. } | Stmt::If { body, orelse, .. } => {
                    self.collect_locals(&body.iter().chain(orelse).collect::<Vec<_>>());
                }
                _ => {}
            }
        }
    }

    pub(super) fn bind_local(&mut self, target: &Expr) {
        if let Expr::Tuple { elements, .. } | Expr::List { elements, .. } = target {
            for element in elements {
                self.bind_local(element);
            }
            return;
        }
        let Expr::Name { id, span } = target else {
            return;
        };
        if !self.locals.contains_key(id) {
            let symbol = self
                .program
                .add_symbol(id, SymbolKind::Local, Type::Error, *span);
            self.locals.insert(id.clone(), symbol);
            self.local_order.push(symbol);
        }
    }

    /// Returns the type an annotation in a body names. CPython never evaluates the annotation
    /// of a variable in a function, and evaluates one in the module body after every class it
    /// can reach is defined (TYP-12), so it may name any class.
    pub(super) fn annotation(&mut self, annotation: &Expr) -> Type {
        let scope = AnnotationScope {
            classes: &self.program.class_ids,
            bound: u32::MAX,
        };
        annotation_type(annotation, &scope, self.diagnostics)
    }

    pub(super) fn declare(&mut self, target: &Expr, annotation: &Expr) {
        let Expr::Name { id, span } = target else {
            return;
        };
        let ty = self.annotation(annotation);
        let symbol = self.locals[id];
        match self.declared.get(&symbol) {
            Some(existing) if *existing != ty => self.diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(402),
                    *span,
                    format!("`{id}` was declared as {existing} and is declared again as {ty}"),
                )
                .with_rule(RuleId::TYP_03),
            ),
            Some(_) => {}
            None => {
                self.declared.insert(symbol, ty.clone());
                self.types.insert(symbol, ty);
            }
        }
    }

    pub(super) fn finish_locals(&mut self) -> Vec<SymbolId> {
        for symbol in &self.local_order {
            let ty = self.types.get(symbol).cloned().unwrap_or(Type::Error);
            self.program.symbols[symbol.0 as usize].ty = ty;
        }
        std::mem::take(&mut self.local_order)
    }

    /// Returns the type a name target already has, declared by an annotation or fixed by an
    /// earlier assignment, which a value without a type of its own, such as `[]`, takes.
    pub(super) fn known_type(&self, target: &Expr) -> Option<Type> {
        if let Expr::Attribute {
            value, attribute, ..
        } = target
        {
            return self.attribute_type(value, attribute);
        }
        let Expr::Name { id, .. } = target else {
            return None;
        };
        let symbol = self.locals.get(id)?;
        self.types.get(symbol).cloned()
    }

    /// Binds a target, a name or a tuple of targets, to a value of type `ty`; a tuple target
    /// takes a tuple of as many items, one for each of its targets (SMT-02).
    pub(super) fn assign_target(
        &mut self,
        target: &Expr,
        ty: Type,
        value: &TypedExpr,
    ) -> Option<AssignTarget> {
        if let Expr::Subscript {
            value: list,
            index,
            span,
        } = target
        {
            return self.item_target(list, index, &ty, *span);
        }
        if let Expr::Attribute {
            value: object,
            attribute,
            span,
        } = target
        {
            return self.attribute_target(object, attribute, &ty, None, *span);
        }
        let (Expr::Tuple { elements, span } | Expr::List { elements, span }) = target else {
            return self.assign(target, ty, value).map(AssignTarget::Name);
        };
        let items = match ty {
            Type::Tuple(items) if items.len() == elements.len() => items,
            Type::Error => vec![Type::Error; elements.len()],
            other => {
                let counted = match &other {
                    Type::Tuple(items) => format!("{} values", items.len()),
                    other => other.with_article(),
                };
                self.diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::error(308),
                        value.span,
                        format!("this unpacks {counted} into {} targets", elements.len()),
                    )
                    .with_label(*span, "the targets")
                    .with_rule(RuleId::SMT_02),
                );
                vec![Type::Error; elements.len()]
            }
        };
        let targets: Option<Vec<AssignTarget>> = elements
            .iter()
            .zip(items)
            .map(|(element, item)| self.assign_target(element, item, value))
            .collect();
        targets.map(AssignTarget::Tuple)
    }

    /// Checks `list[index] = value`: the target is an item of a list, and the value has the
    /// list's item type (DAT-07).
    fn item_target(
        &mut self,
        list: &Expr,
        index: &Expr,
        ty: &Type,
        span: Span,
    ) -> Option<AssignTarget> {
        let list = self.expression(list);
        if let Type::Dict(key, _) = &list.ty {
            let key = key.as_ref().clone();
            let key = self.expression_expecting(index, Some(&key));
            return self.entry_target(list, key, ty, span);
        }
        if let Expr::Slice { .. } = index {
            self.slice_assignment(span);
            return None;
        }
        let index = self.expression(index);
        self.checked_item_target(list, index, ty, span)
    }

    /// Checks the target of `target[index] op= value`, already checked as the expression
    /// `current`, which the operation gives a value of type `ty`.
    pub(super) fn augmented_item(
        &mut self,
        current: TypedExpr,
        ty: &Type,
        span: Span,
    ) -> Option<AssignTarget> {
        match current.kind {
            TypedExprKind::ListItem { list, index }
            | TypedExprKind::Index { value: list, index } => {
                self.checked_item_target(*list, *index, ty, span)
            }
            TypedExprKind::DictItem { dict, key } => self.entry_target(*dict, *key, ty, span),
            TypedExprKind::Attribute { object, index } => {
                if !ty.is_assignable_to(&current.ty) {
                    self.diagnostics.push(
                        Diagnostic::new(
                            DiagnosticCode::error(402),
                            span,
                            format!(
                                "an attribute of type {} is assigned {}",
                                current.ty,
                                ty.with_article()
                            ),
                        )
                        .with_rule(RuleId::TYP_03),
                    );
                }
                Some(AssignTarget::Attribute { object, index })
            }
            TypedExprKind::TupleItem { tuple, .. } => {
                self.unassignable_item(&tuple.ty, span);
                None
            }
            TypedExprKind::Slice { .. } => {
                self.slice_assignment(span);
                None
            }
            _ => None,
        }
    }

    fn checked_item_target(
        &mut self,
        list: TypedExpr,
        index: TypedExpr,
        ty: &Type,
        span: Span,
    ) -> Option<AssignTarget> {
        let item = match &list.ty {
            Type::List(item) => item.as_ref().clone(),
            Type::Error => return None,
            other => {
                let other = other.clone();
                self.unassignable_item(&other, span);
                return None;
            }
        };
        self.expect(&index, Type::Int, "an index");
        if !ty.is_assignable_to(&item) {
            self.diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(402),
                    span,
                    format!("an item of {} is assigned {}", list.ty, ty.with_article()),
                )
                .with_rule(RuleId::TYP_03),
            );
        }
        Some(AssignTarget::Item {
            list: Box::new(list),
            index: Box::new(index),
        })
    }

    fn unassignable_item(&mut self, container: &Type, span: Span) {
        let rule = match container {
            Type::Str => RuleId::DAT_05,
            Type::Tuple(_) => RuleId::DAT_08,
            _ => RuleId::DAT_07,
        };
        self.diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(401),
                span,
                format!("an item of {} cannot be assigned", container.with_article()),
            )
            .with_rule(rule),
        );
    }

    fn slice_assignment(&mut self, span: Span) {
        self.diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(201),
                span,
                "assigning to a slice is planned for TyPy 0.3 (DAT-13)",
            )
            .with_rule(RuleId::DAT_13),
        );
    }

    /// Binds a name target to a value of type `ty`; the first binding fixes the variable's type (TYP-02, TYP-03).
    pub(super) fn assign(
        &mut self,
        target: &Expr,
        ty: Type,
        value: &TypedExpr,
    ) -> Option<SymbolId> {
        let Expr::Name { id, span } = target else {
            return None;
        };
        let symbol = *self.locals.get(id)?;
        match self.types.get(&symbol).cloned() {
            None => {
                self.types.insert(symbol, ty);
            }
            Some(existing) if !ty.is_assignable_to(&existing) => {
                let mut diagnostic = Diagnostic::new(
                    DiagnosticCode::error(402),
                    value.span,
                    format!(
                        "`{id}` was declared as {existing} and is assigned {}",
                        ty.with_article()
                    ),
                )
                .with_label(*span, "the variable")
                .with_rule(if existing == Type::Float && ty == Type::Int {
                    RuleId::TYP_09
                } else {
                    RuleId::TYP_03
                });
                if let (TypedExprKind::Constant(TypedConstant::Int(digits)), Type::Float) =
                    (&value.kind, &existing)
                {
                    diagnostic = diagnostic.with_fix(value.span, format!("{digits}.0"));
                } else {
                    diagnostic = diagnostic.with_help(format!(
                        "assign {}, or use a new variable",
                        existing.with_article()
                    ));
                }
                self.diagnostics.push(diagnostic);
            }
            Some(_) => {}
        }
        // A value that is not `None` shows the optional variable is not `None` until it is
        // assigned again (TYP-13).
        let not_none = matches!(self.types.get(&symbol), Some(Type::Optional(_)))
            && matches!(&value.kind, TypedExprKind::Wrap(inner) if inner.ty != Type::NoneType);
        if let Some(facts) = &mut self.assigned {
            facts.insert(Fact::Assigned(symbol));
            if not_none {
                facts.insert(Fact::NotNone(symbol));
            } else {
                facts.remove(&Fact::NotNone(symbol));
            }
        }
        Some(symbol)
    }
}
