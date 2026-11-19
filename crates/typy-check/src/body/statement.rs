//! Checks statements, and follows definite assignment through branches and loops.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Constant, Expr, Stmt};

use super::BodyChecker;
use std::collections::BTreeSet;

use super::flow::{Fact, meet, test_facts};
use crate::builtin::Builtin;
use crate::typed::{AssignTarget, TypedConstant, TypedExpr, TypedExprKind, TypedStmt};
use crate::types::Type;

impl BodyChecker<'_> {
    pub(super) fn statements(&mut self, statements: &[&Stmt]) -> Vec<TypedStmt> {
        let mut typed = Vec::new();
        for statement in statements {
            if let Stmt::ClassDef {
                name,
                body,
                type_parameters,
                ..
            } = statement
            {
                typed.extend(self.class_body_values((&name.name, type_parameters), body));
            } else {
                typed.extend(self.statement(statement));
                typed.append(&mut self.following);
            }
        }
        typed
    }

    /// Checks the values a class body gives, as assignments to the module variables that hold
    /// them, where the `class` statement runs (CLS-11, ADR-0024).
    fn class_body_values(
        &mut self,
        class: (&str, &[typy_source::syntax::TypeParameter]),
        body: &[Stmt],
    ) -> Vec<TypedStmt> {
        let values = crate::check::class_values(
            class,
            body,
            &self.program.annotation_names,
            &self.program.annotation_modules,
        );
        values
            .iter()
            .filter_map(|value| self.statement(&value.assignment))
            .collect()
    }

    pub(super) fn block(&mut self, statements: &[Stmt]) -> Vec<TypedStmt> {
        let statements: Vec<&Stmt> = statements.iter().collect();
        self.statements(&statements)
    }

    #[allow(clippy::too_many_lines, reason = "one arm per statement kind")]
    pub(super) fn statement(&mut self, statement: &Stmt) -> Option<TypedStmt> {
        Some(match statement {
            Stmt::Expr {
                value: Expr::Yield { value, .. },
                span,
            } => self.yield_statement(value.as_deref(), *span),
            Stmt::Assign {
                value: value @ Expr::Yield { .. },
                span,
                ..
            }
            | Stmt::AnnAssign {
                value: Some(value @ Expr::Yield { .. }),
                span,
                ..
            } if self.received.is_none() => self.yield_assignment(statement, value, *span),
            Stmt::Expr {
                value: value @ Expr::YieldFrom { .. },
                span,
            }
            | Stmt::Assign {
                value: value @ Expr::YieldFrom { .. },
                span,
                ..
            }
            | Stmt::AnnAssign {
                value: Some(value @ Expr::YieldFrom { .. }),
                span,
                ..
            } => TypedStmt::Expr {
                value: self.generator_form_planned("`yield from`", value.span()),
                span: *span,
            },
            Stmt::Assign {
                targets,
                value,
                span,
            } => {
                if let [
                    Expr::Attribute {
                        value: object,
                        attribute,
                        ..
                    },
                ] = targets.as_slice()
                    && let Some(call) = self.property_assignment(object, attribute, value, *span)
                {
                    return Some(call);
                }
                if let [
                    Expr::Subscript {
                        value: object,
                        index,
                        ..
                    },
                ] = targets.as_slice()
                    && let Some(call) = self.item_assignment(object, index, value, *span)
                {
                    return Some(call);
                }
                let expected = match targets.as_slice() {
                    [single] => self.known_type(single),
                    _ => None,
                };
                // A name may be bound to a `ctypes` class, which the program then calls or
                // names in a declaration (FFI-03).
                self.class_allowed = targets
                    .iter()
                    .all(|target| matches!(target, Expr::Name { .. }))
                    && matches!(
                        value,
                        Expr::Name { .. }
                            | Expr::Attribute { .. }
                            | Expr::Call { .. }
                            | Expr::BinOp { .. }
                    );
                let value = self.expression_expecting(value, expected.as_ref());
                self.class_allowed = false;
                // Unpacking reads `sys.version_info` as the tuple of its items (LIB-16).
                let value = if targets
                    .iter()
                    .any(|target| matches!(target, Expr::Tuple { .. } | Expr::List { .. }))
                {
                    Self::version_tuple(value)
                } else {
                    value
                };
                let targets = targets
                    .iter()
                    .filter_map(|target| self.assign_target(target, value.ty.clone(), &value))
                    .collect();
                TypedStmt::Assign {
                    targets,
                    value,
                    span: *span,
                }
            }
            Stmt::AnnAssign {
                target:
                    Expr::Attribute {
                        value: object,
                        attribute,
                        ..
                    },
                annotation,
                value: Some(value),
                span,
                ..
            } => {
                // `Final[T]` declares `T`, and a bare `Final` takes the value's type (TYP-08).
                let declared = match crate::check::final_type(
                    annotation,
                    &self.program.annotation_names,
                    &self.program.annotation_modules,
                ) {
                    Some(qualified) => {
                        if !self.final_attribute_declaration(object, *span) {
                            self.diagnostics.push(
                                Diagnostic::new(
                                    DiagnosticCode::error(408),
                                    annotation.span(),
                                    "`Final` declares an attribute in the body of `__init__`, \
                                     as `self.name: Final = value`",
                                )
                                .with_rule(RuleId::TYP_08),
                            );
                        }
                        match qualified {
                            crate::check::ClassVarAnnotation::Typed(inner) => {
                                Some(self.annotation(inner))
                            }
                            crate::check::ClassVarAnnotation::Bare => None,
                        }
                    }
                    None => Some(self.annotation(annotation)),
                };
                let value = self.expression_expecting(value, declared.as_ref());
                let target =
                    self.attribute_target(object, attribute, &value.ty, declared, *span)?;
                TypedStmt::Assign {
                    targets: vec![target],
                    value,
                    span: *span,
                }
            }
            Stmt::AnnAssign {
                target,
                value: Some(value),
                span,
                ..
            } => {
                let expected = self.known_type(target);
                let value = self.expression_expecting(value, expected.as_ref());
                let target = self.assign(target, value.ty.clone(), &value)?;
                TypedStmt::Assign {
                    targets: vec![AssignTarget::Name(target)],
                    value,
                    span: *span,
                }
            }
            Stmt::AugAssign {
                target,
                operator,
                value,
                span,
            } => {
                let checked_value = self.expression(value);
                let current = self.expression(target);
                let instance_item = self.is_instance_item(&current);
                let property = self.is_property_read(target, &current);
                // A `ctypes` object's value, item or field is read, combined and written back, as an
                // assignment of the result (FFI-03).
                let cdata = matches!(
                    current.kind,
                    TypedExprKind::CData(
                        crate::typed::CDataExpr::Value(_)
                            | crate::typed::CDataExpr::Item { .. }
                            | crate::typed::CDataExpr::Field { .. }
                    )
                );
                // An operator that calls a method of an instance reads and assigns an item or
                // an attribute of an expression through hidden variables (CLS-20), and so does
                // an item of an instance, which `__setitem__` is given (CLS-07).
                if matches!(current.ty, Type::Instance(_))
                    || matches!(checked_value.ty, Type::Instance(_))
                    || instance_item
                    || property
                    || cdata
                {
                    if let super::in_place::Pinned::Checked(pinned) =
                        self.pinned_augmented(target, *operator, value, *span)
                    {
                        return *pinned;
                    }
                    if instance_item {
                        return self.instance_item_augmented(
                            target,
                            *operator,
                            (current, checked_value),
                            *span,
                        );
                    }
                    // The getter's value, combined with the value, goes to the setter (CLS-06).
                    if property || cdata {
                        let result =
                            self.augmented_result(*operator, current, checked_value, *span);
                        return self.rebound(target, result, *span);
                    }
                    if let Some(call) =
                        self.in_place_call(*operator, &current, &checked_value, *span)
                    {
                        return self.rebound(target, call, *span);
                    }
                }
                let written = value;
                let value = checked_value;
                // Without an in-place method, `x += y` on an instance is `x = x + y` (CLS-20).
                let (current, value) = match self.operator_binary(*operator, current, value, *span)
                {
                    super::operator_method::Comparison::Called(call) => {
                        return self.rebound(target, call, *span);
                    }
                    super::operator_method::Comparison::Builtin(current, value) => (current, value),
                };
                let (current, value) = Self::tuple_operands(*operator, current, value);
                if let Some(formatted) = self.percent_dynamic(*operator, &current, &value, *span) {
                    // The item or attribute of an expression is read and assigned through
                    // hidden variables, as an operator method's result is (CLS-20).
                    if let super::in_place::Pinned::Checked(pinned) =
                        self.pinned_augmented(target, *operator, written, *span)
                    {
                        return *pinned;
                    }
                    return self.rebound(target, formatted, *span);
                }
                let result = self.binary_type(*operator, &current, &value, *span);
                let class_value = match target {
                    Expr::Attribute {
                        value: object,
                        attribute,
                        ..
                    } => self.augmented_class_value(object, attribute, &current, &result, *span),
                    _ => None,
                };
                let target = if let Some(assignment) = class_value {
                    match assignment {
                        super::class_value::ClassValueAssignment::Assigned(target) => target,
                        super::class_value::ClassValueAssignment::Refused => return None,
                    }
                } else if let Expr::Subscript { .. } | Expr::Attribute { .. } = target {
                    self.augmented_item(current, &result, *span)?
                } else {
                    AssignTarget::Name(self.assign(target, result, &value)?)
                };
                TypedStmt::AugAssign {
                    target,
                    operator: *operator,
                    value,
                    span: *span,
                }
            }
            // `...` alone does nothing, as `pass` does (GEN-04).
            Stmt::Pass { span }
            | Stmt::Expr {
                value:
                    Expr::Constant {
                        value: typy_source::syntax::Constant::Ellipsis,
                        ..
                    },
                span,
            } => TypedStmt::Pass { span: *span },
            Stmt::Expr { value, span } => TypedStmt::Expr {
                value: self.expression(value),
                span: *span,
            },
            Stmt::Return { value, span } => self.return_statement(value.as_ref(), *span),
            Stmt::Raise {
                exception,
                cause,
                span,
            } => self.raise_statement(exception.as_ref(), cause.as_ref(), *span),
            Stmt::Assert {
                test,
                message,
                span,
            } => self.assert_statement(test, message.as_ref(), *span),
            Stmt::With { items, body, span } => self.with_statement(items, body, *span)?,
            Stmt::Delete { targets, span } => self.delete_statement(targets, *span),
            Stmt::Try {
                body,
                handlers,
                orelse,
                finalbody,
                span,
            } => self.try_statement(body, handlers, orelse, finalbody, *span),
            Stmt::If {
                test,
                body,
                orelse,
                span,
            } => {
                let test = self.truth_test(test);
                let (holds, fails) = test_facts(&test);
                let before = self.assigned.clone();
                self.add_facts(holds);
                let body = self.block(body);
                let after_body = std::mem::replace(&mut self.assigned, before);
                self.add_facts(fails);
                let orelse = self.block(orelse);
                self.assigned = meet(after_body, self.assigned.take());
                TypedStmt::If {
                    test,
                    body,
                    orelse,
                    span: *span,
                }
            }
            Stmt::Match {
                subject,
                cases,
                span,
            } => self.match_statement(subject, cases, *span),
            Stmt::While {
                test,
                body,
                orelse,
                span,
            } => {
                let always = matches!(
                    test,
                    Expr::Constant {
                        value: Constant::Bool(true),
                        ..
                    }
                );
                // A variable the body assigns may be `None` again when the test is next read.
                self.forget_not_none(body);
                let test = self.truth_test(test);
                let facts = test_facts(&test);
                let (body, orelse) = self.loop_statement(body, orelse, always, facts);
                TypedStmt::While {
                    test,
                    body,
                    orelse,
                    span: *span,
                }
            }
            Stmt::For {
                target,
                iter,
                body,
                orelse,
                span,
            } => self.for_statement(target, iter, body, orelse, *span)?,
            Stmt::Break { span } => {
                self.check_stays_in_finally("break", *span);
                let state = self.assigned.take();
                if let Some(breaks) = self.breaks.last_mut() {
                    breaks.push(state);
                }
                TypedStmt::Break { span: *span }
            }
            Stmt::Continue { span } => {
                self.check_stays_in_finally("continue", *span);
                self.assigned = None;
                TypedStmt::Continue { span: *span }
            }
            Stmt::Import { span, .. } | Stmt::ImportFrom { span, .. } => {
                return self.import_statement(statement, *span);
            }
            Stmt::FunctionDef { .. } if self.in_function() => return self.nested_def(statement),
            // `global` changes where names resolve, which `declare_globals` did.
            // The aliases are expanded in the annotations, and a program reads none as a value.
            // A module's functions are checked on their own.
            Stmt::AnnAssign { value: None, .. }
            | Stmt::Global { .. }
            | Stmt::Nonlocal { .. }
            | Stmt::TypeAlias { .. }
            | Stmt::ClassDef { .. }
            | Stmt::FunctionDef { .. }
            | Stmt::Other { .. } => return None,
        })
    }

    /// Returns `target = value` for an augmented assignment whose operator called a method:
    /// a variable, or an attribute of one, which reading again has no effect.
    fn rebound(&mut self, target: &Expr, value: TypedExpr, span: Span) -> Option<TypedStmt> {
        let simple = match target {
            Expr::Name { .. } => true,
            Expr::Attribute { value: object, .. } => super::in_place::is_name(object),
            Expr::Subscript {
                value: object,
                index,
                ..
            } => super::in_place::is_name(object) && super::in_place::is_name(index),
            _ => false,
        };
        // Each caller holds the parts of another target in hidden variables first, so the
        // target is evaluated once; one that does not is a defect of the checker.
        if !simple {
            self.diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(9001),
                    span,
                    "internal compiler error: the target of an augmented assignment would be \
                     evaluated twice",
                )
                .with_rule(RuleId::NFR_09)
                .with_help(
                    "this is a defect in TyPy; please report it with the program that caused it",
                ),
            );
            return None;
        }
        let ty = value.ty.clone();
        let target = self.assign_target(target, ty, &value)?;
        Some(TypedStmt::Assign {
            targets: vec![target],
            value,
            span,
        })
    }

    /// Adds what a test shows to what holds here, when this point can be reached.
    pub(super) fn add_facts(&mut self, facts: BTreeSet<Fact>) {
        if let Some(known) = &mut self.assigned {
            known.extend(facts);
        }
    }

    /// Forgets that a variable is not `None` for each variable `body` assigns, since a loop
    /// reads its body again with what the body left (TYP-13).
    pub(super) fn forget_not_none(&mut self, body: &[Stmt]) {
        let mut names = BTreeSet::new();
        assigned_names(body, &mut names);
        if let Some(known) = &mut self.assigned {
            for name in names {
                if let Some(symbol) = self.locals.get(name) {
                    known.remove(&Fact::NotNone(*symbol));
                    known.retain(|fact| !matches!(fact, Fact::Narrowed(each, _) if each == symbol));
                }
            }
        }
    }

    /// Checks a loop body and its `else` clause; the body may run zero times, and `else` runs only when no `break` does.
    pub(super) fn loop_statement(
        &mut self,
        body: &[Stmt],
        orelse: &[Stmt],
        always: bool,
        (holds, fails): (BTreeSet<Fact>, BTreeSet<Fact>),
    ) -> (Vec<TypedStmt>, Vec<TypedStmt>) {
        let entry = self.assigned.clone();
        self.add_facts(holds);
        self.breaks.push(Vec::new());
        let body = self.block(body);
        let breaks = self.breaks.pop().unwrap_or_default();
        self.assigned = if always { None } else { entry };
        self.add_facts(fails);
        let orelse = self.block(orelse);
        let mut exit = self.assigned.take();
        for state in breaks {
            exit = meet(exit, state);
        }
        self.assigned = exit;
        (body, orelse)
    }

    /// Checks a `for` statement: over a call of `range`, or over the items of a list (CST-02,
    /// CST-03).
    pub(super) fn for_statement(
        &mut self,
        target: &Expr,
        iter: &Expr,
        body: &[Stmt],
        orelse: &[Stmt],
        span: Span,
    ) -> Option<TypedStmt> {
        if let Some(arguments) = self.range_arguments(iter) {
            return self.for_range(target, (arguments, iter.span()), body, orelse, span);
        }
        let (source, item) = self.loop_source(iter, "`for`");
        // What each step binds has no expression of its own; diagnostics about it point at the
        // loop's source.
        let binding = TypedExpr {
            kind: TypedExprKind::Constant(TypedConstant::None),
            ty: item.clone(),
            span: iter.span(),
        };
        // Binding the target marks its names as assigned: the body runs with them, while the
        // `else` clause and the code after the loop start from the state before it, because the
        // list may be empty.
        self.forget_not_none(body);
        let before_loop = self.assigned.clone();
        let target = self.assign_target(target, item, &binding)?;
        self.breaks.push(Vec::new());
        let body = self.block(body);
        let breaks = self.breaks.pop().unwrap_or_default();
        self.assigned = before_loop;
        let orelse = self.block(orelse);
        let mut exit = self.assigned.take();
        for state in breaks {
            exit = meet(exit, state);
        }
        self.assigned = exit;
        Some(TypedStmt::ForEach {
            target,
            source,
            body,
            orelse,
            span,
        })
    }

    /// Checks `for target in range(...)`, given the arguments of the call and its span.
    pub(super) fn for_range(
        &mut self,
        target: &Expr,
        (arguments, call_span): (&[Expr], Span),
        body: &[Stmt],
        orelse: &[Stmt],
        span: Span,
    ) -> Option<TypedStmt> {
        let values: Vec<TypedExpr> = arguments
            .iter()
            .map(|argument| self.expression(argument))
            .collect();
        for value in &values {
            self.expect(value, Type::Int, "an argument of `range`");
        }
        let constant = |digits: &str| TypedExpr {
            kind: TypedExprKind::Constant(TypedConstant::Int(digits.to_owned())),
            ty: Type::Int,
            span: call_span,
        };
        let mut given = values.into_iter();
        let (first, limit, stride) = match (given.next(), given.next(), given.next()) {
            (Some(only), None, _) => (constant("0"), only, constant("1")),
            (Some(begin), Some(end), None) => (begin, end, constant("1")),
            (Some(begin), Some(end), Some(increment)) => (begin, end, increment),
            (None, _, _) => return None,
        };
        self.forget_not_none(body);
        let entry = self.assigned.clone();
        let variable = first.clone();
        let target = self.assign(target, Type::Int, &variable)?;
        self.assigned = entry;
        let mut inside = self.assigned.clone().unwrap_or_default();
        inside.insert(Fact::Assigned(target));
        // The counter is an `int`, so a `T | None` variable is not `None` in the body (TYP-13).
        if matches!(self.types.get(&target), Some(Type::Optional(_))) {
            inside.insert(Fact::NotNone(target));
        }
        let before_loop = self.assigned.replace(inside);
        self.breaks.push(Vec::new());
        let body = self.block(body);
        let breaks = self.breaks.pop().unwrap_or_default();
        self.assigned = before_loop;
        let orelse = self.block(orelse);
        let mut exit = self.assigned.take();
        for state in breaks {
            exit = meet(exit, state);
        }
        self.assigned = exit;
        Some(TypedStmt::ForRange {
            target,
            start: first,
            stop: limit,
            step: stride,
            body,
            orelse,
            span,
        })
    }

    /// Returns the arguments of a call of the builtin `range` with one to three positional
    /// arguments, the form `for` lowers as a counting loop. A program function or variable
    /// named `range` hides the builtin, as it does in a call.
    fn range_arguments<'call>(&self, iter: &'call Expr) -> Option<&'call [Expr]> {
        match self.called_builtin(iter) {
            Some((Builtin::Range, arguments, keywords))
                if keywords.is_empty() && (1..=3).contains(&arguments.len()) =>
            {
                Some(arguments)
            }
            _ => None,
        }
    }

    pub(super) fn return_statement(&mut self, value: Option<&Expr>, span: Span) -> TypedStmt {
        self.check_instance_complete(span);
        self.check_stays_in_finally("return", span);
        if self.generator.is_some() {
            if let Some(value) = value {
                let refused =
                    self.generator_form_planned("a generator's return value", value.span());
                return TypedStmt::Expr {
                    value: refused,
                    span,
                };
            }
            self.assigned = None;
            return TypedStmt::Return { value: None, span };
        }
        let expected = self.returns.clone().unwrap_or(Type::NoneType);
        // A result of no known type, such as that of a `lambda` whose context gives none,
        // takes the value as it is (EXP-14).
        let known = (expected != Type::Error).then_some(&expected);
        let value = value.map(|value| self.expression_expecting(value, known));
        let actual = value
            .as_ref()
            .map_or(Type::NoneType, |value| value.ty.clone());
        if !actual.is_assignable_to(&expected) {
            let mut diagnostic = Diagnostic::new(
                DiagnosticCode::error(405),
                value.as_ref().map_or(span, |value| value.span),
                format!(
                    "this returns {}, but the function returns {expected}",
                    actual.with_article()
                ),
            )
            .with_rule(if expected == Type::Float && actual == Type::Int {
                RuleId::TYP_09
            } else {
                RuleId::TYP_01
            });
            if let Some(help) = self.protocol_help(&actual, &expected) {
                diagnostic = diagnostic.with_help(help);
            }
            if actual == Type::NotImplemented {
                diagnostic = diagnostic.with_help(
                    "only an operator or comparison method returns `NotImplemented`, to give way \
                     to the other operand's method (CLS-21)",
                );
            }
            if let Some(TypedExpr {
                kind: TypedExprKind::Constant(TypedConstant::Int(digits)),
                span,
                ..
            }) = &value
                && expected == Type::Float
            {
                diagnostic = diagnostic.with_fix(*span, format!("{digits}.0"));
            }
            self.diagnostics.push(diagnostic);
        }
        self.assigned = None;
        TypedStmt::Return { value, span }
    }
}

/// Collects the names the statements assign, in nested blocks as well.
fn assigned_names<'tree>(statements: &'tree [Stmt], names: &mut BTreeSet<&'tree str>) {
    for statement in statements {
        match statement {
            Stmt::Assign { targets, .. } => {
                for target in targets {
                    target_names(target, names);
                }
            }
            Stmt::AugAssign { target, .. } | Stmt::AnnAssign { target, .. } => {
                target_names(target, names);
            }
            Stmt::For {
                target,
                body,
                orelse,
                ..
            } => {
                target_names(target, names);
                assigned_names(body, names);
                assigned_names(orelse, names);
            }
            Stmt::While { body, orelse, .. } | Stmt::If { body, orelse, .. } => {
                assigned_names(body, names);
                assigned_names(orelse, names);
            }
            Stmt::Match { cases, .. } => {
                for case in cases {
                    names.extend(
                        case.pattern
                            .bound_names()
                            .into_iter()
                            .map(|name| name.name.as_str()),
                    );
                    assigned_names(&case.body, names);
                }
            }
            Stmt::Try { .. } => {
                for block in statement.try_blocks() {
                    assigned_names(block, names);
                }
            }
            Stmt::With { items, body, .. } => {
                for target in items.iter().filter_map(|item| item.target.as_ref()) {
                    target_names(target, names);
                }
                assigned_names(body, names);
            }
            _ => {}
        }
    }
}

/// Collects the names an assignment target binds.
fn target_names<'tree>(target: &'tree Expr, names: &mut BTreeSet<&'tree str>) {
    match target {
        Expr::Name { id, .. } => {
            names.insert(id.as_str());
        }
        Expr::Tuple { elements, .. } | Expr::List { elements, .. } => {
            for element in elements {
                target_names(element, names);
            }
        }
        _ => {}
    }
}
