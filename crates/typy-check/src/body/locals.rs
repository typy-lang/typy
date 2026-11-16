//! Finds the local variables of a body and binds values to them (LR 4.2.2, TYP-02, TYP-03).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Expr, Stmt};

use super::BodyChecker;
use super::flow::Fact;
use crate::check::{AnnotationScope, annotation_type};
use crate::typed::{AssignTarget, SymbolId, SymbolKind, TypedConstant, TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Returns a new hidden variable of type `ty`, named `<kind N>`, which no program can
    /// name; an expression of the checker's own assigns and reads it by its symbol.
    pub(super) fn hidden_variable(&mut self, kind: &str, ty: Type, span: Span) -> SymbolId {
        let name = format!("<{kind} {}>", self.local_order.len());
        let symbol = self
            .program
            .add_symbol(&name, SymbolKind::Local, ty.clone(), span);
        self.locals.insert(name, symbol);
        self.local_order.push(symbol);
        self.types.insert(symbol, ty);
        symbol
    }

    /// Every name assigned anywhere in the body is a local of the body (LR 4.2.2).
    pub(super) fn collect_locals(&mut self, statements: &[&Stmt]) {
        for statement in statements {
            // An assignment expression binds its name in the body, also inside a
            // comprehension (EXP-13).
            let mut named = Vec::new();
            crate::check::visit_statement(statement, &mut |expression| {
                if let Expr::Named { target, .. } = expression {
                    named.push(Expr::Name {
                        id: target.name.clone(),
                        span: target.span,
                    });
                }
            });
            for target in &named {
                self.bind_local(target);
            }
            match statement {
                // An alias a function's `type` statement defines is read in annotations only
                // (SMT-12).
                Stmt::TypeAlias { name, .. } if self.in_function() => {
                    self.local_aliases.insert(name.name.clone());
                }
                // A class body's values are module variables, assigned where the class
                // statement runs (ADR-0024).
                Stmt::ClassDef {
                    name,
                    body,
                    type_parameters,
                    ..
                } => {
                    let values = crate::check::class_values(
                        (&name.name, type_parameters),
                        body,
                        &self.program.annotation_names,
                        &self.program.annotation_modules,
                    );
                    let assignments: Vec<&Stmt> =
                        values.iter().map(|value| &value.assignment).collect();
                    self.collect_locals(&assignments);
                }
                Stmt::Assign { targets, .. } => {
                    for target in targets {
                        self.bind_local(target);
                    }
                }
                Stmt::AugAssign { target, .. } => self.bind_local(target),
                Stmt::AnnAssign {
                    target,
                    annotation,
                    value,
                    ..
                } => {
                    self.bind_local(target);
                    self.declare(target, annotation, value.is_some());
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
                // The names a pattern binds are locals, as an assignment's are (CST-06).
                Stmt::Match { cases, .. } => {
                    for case in cases {
                        for name in case.pattern.bound_names() {
                            self.bind_local(&Expr::Name {
                                id: name.name.clone(),
                                span: name.span,
                            });
                        }
                        self.collect_locals(&case.body.iter().collect::<Vec<_>>());
                    }
                }
                Stmt::With { items, body, .. } => {
                    for target in items.iter().filter_map(|item| item.target.as_ref()) {
                        self.bind_local(target);
                    }
                    self.collect_locals(&body.iter().collect::<Vec<_>>());
                }
                Stmt::Try { .. } => {
                    let blocks = statement.try_blocks();
                    self.collect_locals(&blocks.into_iter().flatten().collect::<Vec<_>>());
                }
                Stmt::Import { .. } | Stmt::ImportFrom { .. } => self.bind_imported(statement),
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
        if self.locals.contains_key(id) {
            return;
        }
        // A name the module body binds is the module's variable, made before any body.
        let symbol = match self.program.module_symbols.get(id) {
            Some(symbol) if !self.in_function() => *symbol,
            _ => self
                .program
                .add_symbol(id, SymbolKind::Local, Type::Error, *span),
        };
        self.locals.insert(id.clone(), symbol);
        self.local_order.push(symbol);
    }

    /// Makes each name a function declares `global` the module's variable in it: its reads
    /// and assignments reach the module variable, with the module's type for it (SMT-08). A
    /// name the module never binds at module level is E0423.
    pub(super) fn declare_globals(&mut self, body: &[Stmt]) {
        for name in crate::check::declared_globals(body) {
            let Some(symbol) = self.program.module_symbols.get(&name.name).copied() else {
                self.diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::error(423),
                        name.span,
                        format!(
                            "`global {}` names a variable the module does not bind",
                            name.name
                        ),
                    )
                    .with_rule(RuleId::SMT_08)
                    .with_help(format!(
                        "assign `{}` at module level, or declare its type there, such as `{}: int`",
                        name.name, name.name
                    )),
                );
                continue;
            };
            let ty = self.program.symbols[symbol.0 as usize].ty.clone();
            self.global_names.insert(name.name.clone());
            self.locals.insert(name.name.clone(), symbol);
            self.declared.insert(symbol, ty.clone());
            self.types.insert(symbol, ty);
        }
    }

    /// Returns the type an annotation in a body names. CPython never evaluates the annotation
    /// of a variable in a function, and evaluates one in the module body after every class it
    /// can reach is defined (TYP-12), so it may name any class.
    pub(super) fn annotation(&mut self, annotation: &Expr) -> Type {
        let scope = AnnotationScope {
            classes: &self.program.class_ids,
            instances: &self.program.instances,
            modules: &self.program.module_classes,
            module_aliases: &self.program.module_aliases,
            imported_aliases: &self.program.imported_aliases,
            parameters: &self.program.type_arguments,
            copies: &self.program.class_copies,
            names: &self.program.annotation_names,
            standard: &self.program.annotation_modules,
            bound: u32::MAX,
        };
        let ty = annotation_type(annotation, &scope, self.diagnostics);
        // An annotation may name a copy of a generic class, which the program then holds.
        self.program.make_class_copies();
        crate::check::outside_signature(ty, annotation.span(), self.diagnostics)
    }

    pub(super) fn declare(&mut self, target: &Expr, annotation: &Expr, given: bool) {
        let Expr::Name { id, span } = target else {
            return;
        };
        let symbol = self.locals[id];
        // `Final[T]` declares `T`, and a bare `Final` takes the value's type (TYP-08).
        let annotation = match crate::check::final_type(
            annotation,
            &self.program.annotation_names,
            &self.program.annotation_modules,
        ) {
            Some(qualified) => {
                if !given {
                    self.diagnostics.push(
                        Diagnostic::new(
                            DiagnosticCode::error(408),
                            annotation.span(),
                            format!("`{id}` is declared `Final` and is given no value"),
                        )
                        .with_rule(RuleId::TYP_08)
                        .with_help(format!("give it its value, such as `{id}: Final = 0`")),
                    );
                }
                self.program.final_variables.insert(symbol, *span);
                match qualified {
                    crate::check::ClassVarAnnotation::Typed(inner) => inner,
                    crate::check::ClassVarAnnotation::Bare => return,
                }
            }
            None => annotation,
        };
        let ty = self.annotation(annotation);
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
        // A slice of a list takes a list of the same type, which an empty `[]` gets; so does
        // an item whose index is a slice object (CLS-23).
        if let Expr::Subscript { value, index, .. } = target
            && (matches!(index.as_ref(), Expr::Slice { .. })
                || self.peek_type(index) == Some(Type::Slice)
                || matches!(index.as_ref(), Expr::Call { function, .. }
                    if matches!(function.as_ref(), Expr::Name { id, .. } if id == "slice")))
        {
            return self
                .known_type(value)
                .filter(|ty| matches!(ty, Type::List(_)));
        }
        // An item of a list, or an entry of a dict, takes a value of its own type, which a
        // member of a union becomes (TYP-14).
        if let Expr::Subscript { value, .. } = target {
            return match self.known_type(value)? {
                Type::List(item) => Some(*item),
                Type::Dict(_, value) => Some(*value),
                _ => None,
            };
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
            if let Some(module) = self.module_of(object) {
                let target = self.module_attribute_target(&module, attribute, &ty, value, *span)?;
                // A module bound lazily is imported before the assignment (MOD-11).
                return Some(match self.lazy_module_function(object) {
                    Some(reify) => AssignTarget::Lazy {
                        reify,
                        target: Box::new(target),
                    },
                    None => target,
                });
            }
            if let Some(class) = self.class_named_by(object)
                && let Some(assignment) =
                    self.class_value_target(class, attribute, &ty, value, *span)
            {
                return match assignment {
                    super::class_value::ClassValueAssignment::Assigned(target) => Some(target),
                    super::class_value::ClassValueAssignment::Refused => None,
                };
            }
            if let Some(declared) = self.conversion_for(target, &ty) {
                let inner = self.attribute_target(object, attribute, &declared, None, *span)?;
                let (holder, converted) = self.conversion(&ty, &declared, *span);
                return Some(AssignTarget::Converted {
                    holder,
                    value: Box::new(converted),
                    target: Box::new(inner),
                });
            }
            return self.attribute_target(object, attribute, &ty, None, *span);
        }
        let (Expr::Tuple { elements, span } | Expr::List { elements, span }) = target else {
            if let Some(declared) = self.conversion_for(target, &ty) {
                let (holder, converted) = self.conversion(&ty, &declared, target.span());
                let symbol = self.assign(target, declared, &converted)?;
                return Some(AssignTarget::Converted {
                    holder,
                    value: Box::new(converted),
                    target: Box::new(AssignTarget::Name(symbol)),
                });
            }
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
        if let Expr::Slice {
            lower, upper, step, ..
        } = index
        {
            return self.slice_target(list, [lower, upper, step], ty, span);
        }
        let index = self.expression(index);
        // A slice object assigns the slice of its bounds (CLS-23).
        if index.ty == Type::Slice {
            let bounds = self.object_bounds(index, span);
            return self.slice_target_of(list, bounds, ty, span);
        }
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
                if object
                    .ty
                    .class_id()
                    .is_some_and(|class| self.refuse_final_attribute(class, index, span))
                {
                    return None;
                }
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
            TypedExprKind::ModuleAttribute { symbol, .. } => {
                let name = self.program.symbols[symbol.0 as usize].name.clone();
                if self.final_reassigned(symbol, &name, span) {
                    return None;
                }
                if !ty.is_assignable_to(&current.ty) {
                    self.diagnostics.push(
                        Diagnostic::new(
                            DiagnosticCode::error(402),
                            span,
                            format!(
                                "a module variable of type {} is assigned {}",
                                current.ty,
                                ty.with_article()
                            ),
                        )
                        .with_rule(RuleId::TYP_03),
                    );
                }
                Some(AssignTarget::Name(symbol))
            }
            // The read runs the lazy import, so the assignment goes to the variable it read.
            TypedExprKind::Lazy { value, .. } => self.augmented_item(*value, ty, span),
            TypedExprKind::TupleItem { tuple, .. } => {
                self.unassignable_item(&tuple.ty, span);
                None
            }
            TypedExprKind::Slice { .. } => {
                self.slice_assignment(span);
                None
            }
            // A target whose read failed was reported there; any other is reported here, so no
            // statement is left out without a diagnostic.
            _ if current.ty.has_error() => None,
            _ => {
                self.diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::error(401),
                        span,
                        format!(
                            "an augmented assignment cannot assign {} read this way",
                            current.ty.with_article()
                        ),
                    )
                    .with_rule(RuleId::SMT_01),
                );
                None
            }
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
        let target = AssignTarget::Item {
            list: Box::new(list),
            index: Box::new(index),
        };
        Some(self.converted_target(ty, &item, target, span))
    }

    pub(super) fn unassignable_item(&mut self, container: &Type, span: Span) {
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
                "an augmented assignment to a slice is planned for after TyPy 1.0",
            )
            .with_rule(RuleId::DAT_13),
        );
    }

    /// Checks `list[lower:upper:step] = items`: a slice of a list takes a list of its item
    /// type, and each bound is an `int` (DAT-13).
    fn slice_target(
        &mut self,
        list: TypedExpr,
        bounds: [&Option<Box<Expr>>; 3],
        ty: &Type,
        span: Span,
    ) -> Option<AssignTarget> {
        let bounds = bounds.map(|bound| {
            bound.as_deref().map(|bound| {
                let value = self.expression(bound);
                self.expect(&value, Type::optional(Type::Int), "a slice bound");
                Box::new(value)
            })
        });
        self.slice_target_of(list, bounds, ty, span)
    }

    /// Checks `list[lower:upper:step] = items` of checked bounds.
    fn slice_target_of(
        &mut self,
        list: TypedExpr,
        [lower, upper, step]: [Option<Box<TypedExpr>>; 3],
        ty: &Type,
        span: Span,
    ) -> Option<AssignTarget> {
        match &list.ty {
            Type::List(_) => {}
            Type::Error => return None,
            other => {
                let other = other.clone();
                self.unassignable_item(&other, span);
                return None;
            }
        }
        if !ty.is_assignable_to(&list.ty) {
            self.diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(402),
                    span,
                    format!(
                        "a slice of {} is assigned {}, and takes a list of its items",
                        list.ty,
                        ty.with_article()
                    ),
                )
                .with_rule(RuleId::DAT_13),
            );
        }
        Some(AssignTarget::Slice {
            list: Box::new(list),
            lower,
            upper,
            step,
        })
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
        if self.final_reassigned(symbol, id, *span) {
            return None;
        }
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
        // assigned again (TYP-13); a `for` loop's item of the base type shows it too.
        let not_none = matches!(self.types.get(&symbol), Some(Type::Optional(_)))
            && match &value.kind {
                TypedExprKind::Wrap(inner) => inner.ty != Type::NoneType,
                _ => !matches!(value.ty, Type::NoneType | Type::Optional(_) | Type::Error),
            };
        if let Some(facts) = &mut self.assigned {
            facts.insert(Fact::Assigned(symbol));
            if not_none {
                facts.insert(Fact::NotNone(symbol));
            } else {
                facts.remove(&Fact::NotNone(symbol));
            }
            // An assignment replaces what an `isinstance` test showed (TYP-07).
            facts.retain(|fact| !matches!(fact, Fact::Narrowed(each, _) if *each == symbol));
        }
        Some(symbol)
    }

    /// Reports an assignment to a variable declared `Final` other than its declaration;
    /// returns true when it reports one (TYP-08).
    pub(super) fn final_reassigned(&mut self, symbol: SymbolId, name: &str, span: Span) -> bool {
        let Some(declared) = self.program.final_variables.get(&symbol).copied() else {
            return false;
        };
        if declared == span {
            return false;
        }
        self.diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(414),
                span,
                format!("`{name}` is declared `Final`, so it is not assigned again"),
            )
            .with_rule(RuleId::TYP_08)
            .with_label(declared, "declared here"),
        );
        true
    }

    /// Returns `target`, which holds values of type `declared`, taking a value of type `ty`
    /// converted to `declared` when the two differ, as a member of a union or the base of an
    /// optional does. Only one value assigned to several targets, or a tuple unpacked into
    /// targets, gives such a value, since a single target's value is checked as the target's
    /// type already (TYP-16).
    pub(super) fn converted_target(
        &mut self,
        ty: &Type,
        declared: &Type,
        target: AssignTarget,
        span: Span,
    ) -> AssignTarget {
        if !Self::needs_conversion(ty, declared, span) {
            return target;
        }
        let (holder, value) = self.conversion(ty, declared, span);
        AssignTarget::Converted {
            holder,
            value: Box::new(value),
            target: Box::new(target),
        }
    }

    /// Returns a hidden variable for a value of type `ty`, and its read converted to
    /// `declared`.
    fn conversion(&mut self, ty: &Type, declared: &Type, span: Span) -> (SymbolId, TypedExpr) {
        let name = format!("<converted {}>", self.local_order.len());
        let holder = self
            .program
            .add_symbol(&name, SymbolKind::Local, ty.clone(), span);
        self.locals.insert(name, holder);
        self.local_order.push(holder);
        self.types.insert(holder, ty.clone());
        let stored = Self::typed(TypedExprKind::Local(holder), ty.clone(), span);
        (holder, Self::converted(stored, Some(declared)))
    }

    /// Returns the declared type of a variable or an attribute target, when a value of type
    /// `ty` changes its form to be held there.
    fn conversion_for(&self, target: &Expr, ty: &Type) -> Option<Type> {
        let declared = self.known_type(target)?;
        Self::needs_conversion(ty, &declared, target.span()).then_some(declared)
    }

    /// Returns whether a value of type `ty` changes its form to be held as `declared`.
    fn needs_conversion(ty: &Type, declared: &Type, span: Span) -> bool {
        if ty.matches(declared) || !ty.is_assignable_to(declared) {
            return false;
        }
        let probe = Self::typed(
            TypedExprKind::Constant(TypedConstant::None),
            ty.clone(),
            span,
        );
        Self::converted(probe, Some(declared)).ty != *ty
    }
}
