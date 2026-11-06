//! Checks one function body, or the module body: names, types and definite assignment.

use std::collections::{BTreeMap, BTreeSet};

use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId, Span};
use typy_source::syntax::{
    BinaryOperator, BoolOperator, CompareOperator, Constant, Expr, Keyword, Stmt, UnaryOperator,
};

use crate::builtins::{Builtin, Unprovided, unprovided};
use crate::check::{Definition, annotation_type};
use crate::typed::{
    Argument, FunctionId, FunctionSignature, Symbol, SymbolId, SymbolKind, TypedConstant,
    TypedExpr, TypedExprKind, TypedFunction, TypedProgram, TypedStmt,
};
use crate::types::Type;

/// Module-level names that CPython defines and TyPy 0.1 does not provide.
const MODULE_DUNDERS: &[&str] = &[
    "__annotations__",
    "__builtins__",
    "__doc__",
    "__file__",
    "__loader__",
    "__package__",
    "__spec__",
];

/// The program being built: symbols, finished functions, and what every body may refer to.
pub(crate) struct Program {
    module: String,
    symbols: Vec<Symbol>,
    functions: Vec<Option<TypedFunction>>,
    function_ids: BTreeMap<String, FunctionId>,
    signatures: Vec<FunctionSignature>,
    module_variables: BTreeSet<String>,
}

impl Program {
    pub(crate) fn new(
        module: &str,
        definitions: &[Definition<'_>],
        module_variables: BTreeSet<String>,
    ) -> Self {
        let function_ids = definitions
            .iter()
            .enumerate()
            .map(|(index, definition)| (definition.name.clone(), FunctionId(id(index))))
            .collect();
        Self {
            module: module.to_owned(),
            symbols: Vec::new(),
            functions: vec![None; definitions.len()],
            function_ids,
            signatures: definitions
                .iter()
                .map(|definition| definition.signature.clone())
                .collect(),
            module_variables,
        }
    }

    fn add_symbol(&mut self, name: &str, kind: SymbolKind, ty: Type, span: Span) -> SymbolId {
        let symbol = SymbolId(id(self.symbols.len()));
        self.symbols.push(Symbol {
            id: symbol,
            name: name.to_owned(),
            kind,
            ty,
            span,
        });
        symbol
    }

    pub(crate) fn finish(self, entry: FunctionId) -> TypedProgram {
        TypedProgram {
            symbols: self.symbols,
            functions: self.functions.into_iter().flatten().collect(),
            entry,
        }
    }
}

fn id(index: usize) -> u32 {
    u32::try_from(index).unwrap_or(u32::MAX)
}

/// Which variables are assigned on every path; `None` when the point cannot be reached.
type Assigned = Option<BTreeSet<SymbolId>>;

fn meet(left: Assigned, right: Assigned) -> Assigned {
    match (left, right) {
        (None, other) | (other, None) => other,
        (Some(left), Some(right)) => Some(left.intersection(&right).copied().collect()),
    }
}

/// Checks the statements of one body.
pub(crate) struct BodyChecker<'program> {
    program: &'program mut Program,
    diagnostics: &'program mut Diagnostics,
    locals: BTreeMap<String, SymbolId>,
    local_order: Vec<SymbolId>,
    declared: BTreeMap<SymbolId, Type>,
    types: BTreeMap<SymbolId, Type>,
    assigned: Assigned,
    breaks: Vec<Vec<Assigned>>,
    reported_unassigned: BTreeSet<SymbolId>,
    returns: Option<Type>,
}

impl<'program> BodyChecker<'program> {
    fn new(
        program: &'program mut Program,
        diagnostics: &'program mut Diagnostics,
        returns: Option<Type>,
    ) -> Self {
        Self {
            program,
            diagnostics,
            locals: BTreeMap::new(),
            local_order: Vec::new(),
            declared: BTreeMap::new(),
            types: BTreeMap::new(),
            assigned: Some(BTreeSet::new()),
            breaks: Vec::new(),
            reported_unassigned: BTreeSet::new(),
            returns,
        }
    }

    /// Checks a module-level function and stores it in the program.
    pub(crate) fn check_function(
        program: &mut Program,
        function: FunctionId,
        definition: &Definition<'_>,
        diagnostics: &mut Diagnostics,
    ) {
        let returns = definition.signature.returns;
        let mut checker = BodyChecker::new(program, diagnostics, Some(returns));
        let mut parameters = Vec::new();
        for ((name, span), parameter) in definition
            .parameters
            .iter()
            .zip(&definition.signature.parameters)
        {
            let symbol =
                checker
                    .program
                    .add_symbol(name, SymbolKind::Parameter, parameter.ty, *span);
            checker.locals.insert(name.clone(), symbol);
            checker.types.insert(symbol, parameter.ty);
            checker
                .assigned
                .get_or_insert_with(BTreeSet::new)
                .insert(symbol);
            parameters.push(symbol);
        }
        let statements: Vec<&Stmt> = definition.body.iter().collect();
        checker.collect_locals(&statements);
        let body = checker.statements(&statements);
        if checker.assigned.is_some() && returns != Type::NoneType && returns != Type::Error {
            checker.diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(405),
                    definition.span,
                    format!(
                        "`{}` can reach its end without returning {}",
                        definition.name,
                        returns.with_article()
                    ),
                )
                .with_rule(RuleId::TYP_01),
            );
        }
        let locals = checker.finish_locals();
        program.functions[function.0 as usize] = Some(TypedFunction {
            id: function,
            name: definition.name.clone(),
            module: program.module.clone(),
            signature: definition.signature.clone(),
            parameters,
            locals,
            body,
            span: definition.span,
        });
    }

    /// Checks the module body and stores it in the program as its entry function.
    pub(crate) fn check_module(
        program: &mut Program,
        statements: &[&Stmt],
        diagnostics: &mut Diagnostics,
    ) -> FunctionId {
        let entry = FunctionId(id(program.functions.len()));
        let mut checker = BodyChecker::new(program, diagnostics, None);
        checker.collect_locals(statements);
        let body = checker.statements(statements);
        let locals = checker.finish_locals();
        let span = statements.first().map_or_else(
            || Span::empty(typy_diagnostics::FileId(0), typy_diagnostics::TextOffset(0)),
            |first| first.span(),
        );
        program.functions.push(Some(TypedFunction {
            id: entry,
            name: "<module>".to_owned(),
            module: program.module.clone(),
            signature: FunctionSignature {
                parameters: Vec::new(),
                returns: Type::NoneType,
            },
            parameters: Vec::new(),
            locals,
            body,
            span,
        }));
        entry
    }

    fn in_function(&self) -> bool {
        self.returns.is_some()
    }

    /// Every name assigned anywhere in the body is a local of the body (LR 4.2.2).
    fn collect_locals(&mut self, statements: &[&Stmt]) {
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

    fn bind_local(&mut self, target: &Expr) {
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

    fn declare(&mut self, target: &Expr, annotation: &Expr) {
        let Expr::Name { id, span } = target else {
            return;
        };
        let ty = annotation_type(annotation, self.diagnostics);
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
                self.declared.insert(symbol, ty);
                self.types.insert(symbol, ty);
            }
        }
    }

    fn finish_locals(&mut self) -> Vec<SymbolId> {
        for symbol in &self.local_order {
            let ty = self.types.get(symbol).copied().unwrap_or(Type::Error);
            self.program.symbols[symbol.0 as usize].ty = ty;
        }
        std::mem::take(&mut self.local_order)
    }

    fn statements(&mut self, statements: &[&Stmt]) -> Vec<TypedStmt> {
        statements
            .iter()
            .filter_map(|statement| self.statement(statement))
            .collect()
    }

    fn block(&mut self, statements: &[Stmt]) -> Vec<TypedStmt> {
        let statements: Vec<&Stmt> = statements.iter().collect();
        self.statements(&statements)
    }

    #[allow(clippy::too_many_lines, reason = "one arm per statement kind")]
    fn statement(&mut self, statement: &Stmt) -> Option<TypedStmt> {
        Some(match statement {
            Stmt::Assign {
                targets,
                value,
                span,
            } => {
                let value = self.expression(value);
                let targets = targets
                    .iter()
                    .filter_map(|target| self.assign(target, value.ty, &value))
                    .collect();
                TypedStmt::Assign {
                    targets,
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
                let value = self.expression(value);
                let target = self.assign(target, value.ty, &value)?;
                TypedStmt::Assign {
                    targets: vec![target],
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
                let current = self.expression(target);
                let value = self.expression(value);
                let result = self.binary_type(*operator, &current, &value, *span);
                let symbol = self.assign(target, result, &value)?;
                TypedStmt::AugAssign {
                    target: symbol,
                    operator: *operator,
                    value,
                    span: *span,
                }
            }
            Stmt::Expr { value, span } => TypedStmt::Expr {
                value: self.expression(value),
                span: *span,
            },
            Stmt::Return { value, span } => self.return_statement(value.as_ref(), *span),
            Stmt::If {
                test,
                body,
                orelse,
                span,
            } => {
                let test = self.truth_test(test);
                let before = self.assigned.clone();
                let body = self.block(body);
                let after_body = std::mem::replace(&mut self.assigned, before);
                let orelse = self.block(orelse);
                self.assigned = meet(after_body, self.assigned.take());
                TypedStmt::If {
                    test,
                    body,
                    orelse,
                    span: *span,
                }
            }
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
                let test = self.truth_test(test);
                let (body, orelse) = self.loop_statement(body, orelse, always);
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
            } => self.for_range(target, iter, body, orelse, *span)?,
            Stmt::Pass { span } => TypedStmt::Pass { span: *span },
            Stmt::Break { span } => {
                let state = self.assigned.take();
                if let Some(breaks) = self.breaks.last_mut() {
                    breaks.push(state);
                }
                TypedStmt::Break { span: *span }
            }
            Stmt::Continue { span } => {
                self.assigned = None;
                TypedStmt::Continue { span: *span }
            }
            Stmt::AnnAssign { value: None, .. }
            | Stmt::FunctionDef { .. }
            | Stmt::Import { .. }
            | Stmt::ImportFrom { .. }
            | Stmt::Other { .. } => return None,
        })
    }

    /// Checks a loop body and its `else` clause; the body may run zero times, and `else` runs only when no `break` does.
    fn loop_statement(
        &mut self,
        body: &[Stmt],
        orelse: &[Stmt],
        always: bool,
    ) -> (Vec<TypedStmt>, Vec<TypedStmt>) {
        let entry = self.assigned.clone();
        self.breaks.push(Vec::new());
        let body = self.block(body);
        let breaks = self.breaks.pop().unwrap_or_default();
        self.assigned = if always { None } else { entry };
        let orelse = self.block(orelse);
        let mut exit = self.assigned.take();
        for state in breaks {
            exit = meet(exit, state);
        }
        self.assigned = exit;
        (body, orelse)
    }

    fn for_range(
        &mut self,
        target: &Expr,
        iter: &Expr,
        body: &[Stmt],
        orelse: &[Stmt],
        span: Span,
    ) -> Option<TypedStmt> {
        let Expr::Call {
            arguments,
            span: call_span,
            ..
        } = iter
        else {
            return None;
        };
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
            span: *call_span,
        };
        let mut given = values.into_iter();
        let (first, limit, stride) = match (given.next(), given.next(), given.next()) {
            (Some(only), None, _) => (constant("0"), only, constant("1")),
            (Some(begin), Some(end), None) => (begin, end, constant("1")),
            (Some(begin), Some(end), Some(increment)) => (begin, end, increment),
            (None, _, _) => return None,
        };
        let entry = self.assigned.clone();
        let variable = first.clone();
        let target = self.assign(target, Type::Int, &variable)?;
        self.assigned = entry;
        let mut inside = self.assigned.clone().unwrap_or_default();
        inside.insert(target);
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

    fn return_statement(&mut self, value: Option<&Expr>, span: Span) -> TypedStmt {
        let expected = self.returns.unwrap_or(Type::NoneType);
        let value = value.map(|value| self.expression(value));
        let actual = value.as_ref().map_or(Type::NoneType, |value| value.ty);
        if !actual.is_assignable_to(expected) {
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

    /// Binds a name target to a value of type `ty`; the first binding fixes the variable's type (TYP-02, TYP-03).
    fn assign(&mut self, target: &Expr, ty: Type, value: &TypedExpr) -> Option<SymbolId> {
        let Expr::Name { id, span } = target else {
            return None;
        };
        let symbol = *self.locals.get(id)?;
        match self.types.get(&symbol).copied() {
            None => {
                self.types.insert(symbol, ty);
            }
            Some(existing) if !ty.is_assignable_to(existing) => {
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
                    (&value.kind, existing)
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
        if let Some(assigned) = &mut self.assigned {
            assigned.insert(symbol);
        }
        Some(symbol)
    }

    fn error(&mut self, diagnostic: Diagnostic, span: Span) -> TypedExpr {
        self.diagnostics.push(diagnostic);
        TypedExpr {
            kind: TypedExprKind::Constant(TypedConstant::None),
            ty: Type::Error,
            span,
        }
    }

    fn typed(kind: TypedExprKind, ty: Type, span: Span) -> TypedExpr {
        TypedExpr { kind, ty, span }
    }

    fn truth_test(&mut self, test: &Expr) -> TypedExpr {
        let test = self.expression(test);
        if !test.ty.is_truth_testable() {
            self.diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(304),
                    test.span,
                    format!("{} cannot be used as a condition", test.ty.with_article()),
                )
                .with_rule(RuleId::EXP_10),
            );
        }
        test
    }

    /// Reports a value whose type is not `expected`.
    fn expect(&mut self, value: &TypedExpr, expected: Type, what: &str) {
        if value.ty.is_assignable_to(expected) {
            return;
        }
        let rule = if expected == Type::Float && value.ty == Type::Int {
            RuleId::TYP_09
        } else {
            RuleId::TYP_01
        };
        let mut diagnostic = Diagnostic::new(
            DiagnosticCode::error(404),
            value.span,
            format!(
                "{what} must be {}, and this is {}",
                expected.with_article(),
                value.ty.with_article()
            ),
        )
        .with_rule(rule);
        if let (TypedExprKind::Constant(TypedConstant::Int(digits)), Type::Float) =
            (&value.kind, expected)
        {
            diagnostic = diagnostic.with_fix(value.span, format!("{digits}.0"));
        }
        self.diagnostics.push(diagnostic);
    }

    #[allow(clippy::too_many_lines, reason = "one arm per expression kind")]
    fn expression(&mut self, expression: &Expr) -> TypedExpr {
        let span = expression.span();
        match expression {
            Expr::Constant { value, .. } => {
                let constant = match value {
                    Constant::None => TypedConstant::None,
                    Constant::Bool(value) => TypedConstant::Bool(*value),
                    Constant::Int(digits) => TypedConstant::Int(digits.clone()),
                    Constant::Float(value) => TypedConstant::Float(*value),
                    Constant::Str(value) => TypedConstant::Str(value.clone()),
                    Constant::Complex { .. } | Constant::Bytes(_) | Constant::Ellipsis => {
                        return Self::typed(
                            TypedExprKind::Constant(TypedConstant::None),
                            Type::Error,
                            span,
                        );
                    }
                };
                let ty = constant.ty();
                Self::typed(TypedExprKind::Constant(constant), ty, span)
            }
            Expr::Name { id, .. } => self.name(id, span),
            Expr::BinOp {
                left,
                operator,
                right,
                ..
            } => {
                let left = self.expression(left);
                let right = self.expression(right);
                let ty = self.binary_type(*operator, &left, &right, span);
                Self::typed(
                    TypedExprKind::Binary {
                        left: Box::new(left),
                        operator: *operator,
                        right: Box::new(right),
                    },
                    ty,
                    span,
                )
            }
            Expr::UnaryOp {
                operator, operand, ..
            } => {
                let operand = self.expression(operand);
                let ty = self.unary_type(*operator, &operand, span);
                Self::typed(
                    TypedExprKind::Unary {
                        operator: *operator,
                        operand: Box::new(operand),
                    },
                    ty,
                    span,
                )
            }
            Expr::BoolOp {
                operator, values, ..
            } => {
                let values: Vec<TypedExpr> =
                    values.iter().map(|value| self.truth_test(value)).collect();
                let ty = values[0].ty;
                if let Some(other) = values.iter().find(|value| !value.ty.is_assignable_to(ty)) {
                    let word = if *operator == BoolOperator::And {
                        "and"
                    } else {
                        "or"
                    };
                    return self.error(
                        Diagnostic::new(
                            DiagnosticCode::error(303),
                            other.span,
                            format!(
                                "the operands of `{word}` must have one type; they are {ty} and {}",
                                other.ty
                            ),
                        )
                        .with_rule(RuleId::EXP_09),
                        span,
                    );
                }
                Self::typed(
                    TypedExprKind::BoolOp {
                        operator: *operator,
                        values,
                    },
                    ty,
                    span,
                )
            }
            Expr::Compare {
                left,
                operators,
                comparators,
                ..
            } => {
                let left = self.expression(left);
                let comparators: Vec<TypedExpr> = comparators
                    .iter()
                    .map(|value| self.expression(value))
                    .collect();
                let mut previous = &left;
                for (operator, right) in operators.iter().zip(&comparators) {
                    self.compare_types(*operator, previous, right);
                    previous = right;
                }
                Self::typed(
                    TypedExprKind::Compare {
                        left: Box::new(left),
                        operators: operators.clone(),
                        comparators,
                    },
                    Type::Bool,
                    span,
                )
            }
            Expr::IfExp {
                test, body, orelse, ..
            } => {
                let test = self.truth_test(test);
                let body = self.expression(body);
                let orelse = self.expression(orelse);
                if !orelse.ty.is_assignable_to(body.ty) {
                    return self.error(
                        Diagnostic::new(
                            DiagnosticCode::error(305),
                            orelse.span,
                            format!(
                                "both branches must have one type; they are {} and {}",
                                body.ty, orelse.ty
                            ),
                        )
                        .with_rule(RuleId::EXP_11),
                        span,
                    );
                }
                let ty = body.ty;
                Self::typed(
                    TypedExprKind::IfExp {
                        test: Box::new(test),
                        body: Box::new(body),
                        orelse: Box::new(orelse),
                    },
                    ty,
                    span,
                )
            }
            Expr::Subscript { value, index, .. } => {
                let value = self.expression(value);
                let index = self.expression(index);
                if value.ty == Type::Error || index.ty == Type::Error {
                    return Self::typed(
                        TypedExprKind::Constant(TypedConstant::None),
                        Type::Error,
                        span,
                    );
                }
                if value.ty != Type::Str {
                    return self.error(
                        Diagnostic::new(
                            DiagnosticCode::error(401),
                            value.span,
                            format!("{} cannot be indexed", value.ty.with_article()),
                        )
                        .with_rule(RuleId::EXP_02),
                        span,
                    );
                }
                self.expect(&index, Type::Int, "an index");
                Self::typed(
                    TypedExprKind::Index {
                        value: Box::new(value),
                        index: Box::new(index),
                    },
                    Type::Str,
                    span,
                )
            }
            Expr::Call {
                function,
                arguments,
                keywords,
                ..
            } => {
                let Expr::Name {
                    id,
                    span: name_span,
                } = function.as_ref()
                else {
                    return Self::typed(
                        TypedExprKind::Constant(TypedConstant::None),
                        Type::Error,
                        span,
                    );
                };
                self.call(id, *name_span, arguments, keywords, span)
            }
            Expr::Other { .. } => Self::typed(
                TypedExprKind::Constant(TypedConstant::None),
                Type::Error,
                span,
            ),
        }
    }

    /// Resolves a name that is read as a value (LR 4.2.2).
    fn name(&mut self, id: &str, span: Span) -> TypedExpr {
        if let Some(symbol) = self.locals.get(id).copied() {
            if let Some(assigned) = &self.assigned
                && !assigned.contains(&symbol)
                && self.reported_unassigned.insert(symbol)
            {
                self.diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::error(306),
                        span,
                        format!("`{id}` may be used before it is assigned"),
                    )
                    .with_rule(RuleId::TYP_12)
                    .with_help(format!(
                        "assign `{id}` on every path that reaches this line"
                    )),
                );
            }
            let ty = self.types.get(&symbol).copied().unwrap_or(Type::Error);
            return Self::typed(TypedExprKind::Local(symbol), ty, span);
        }
        if id == "__name__" {
            return Self::typed(TypedExprKind::ModuleName, Type::Str, span);
        }
        if self.in_function() && self.program.module_variables.contains(id) {
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(201),
                    span,
                    format!("reading the module variable `{id}` from a function is planned for TyPy 0.3 (SMT-08)"),
                )
                .with_rule(RuleId::SMT_08),
                span,
            );
        }
        if self.program.function_ids.contains_key(id) || Builtin::named(id).is_some() {
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(201),
                    span,
                    format!("using `{id}` as a value is planned for TyPy 1.0 (FUN-05)"),
                )
                .with_rule(RuleId::FUN_05),
                span,
            );
        }
        self.unknown_name(id, span)
    }

    fn unknown_name(&mut self, id: &str, span: Span) -> TypedExpr {
        let diagnostic = match unprovided(id) {
            Some(Unprovided::Planned { release, rule }) => Diagnostic::new(
                DiagnosticCode::error(201),
                span,
                format!("`{id}` is planned for TyPy {release} ({rule})"),
            )
            .with_rule(rule),
            Some(Unprovided::Excluded) => Diagnostic::new(
                DiagnosticCode::error(202),
                span,
                format!("`{id}` is a Python builtin that TyPy does not provide"),
            )
            .with_rule(RuleId::STD_04),
            None if MODULE_DUNDERS.contains(&id) => Diagnostic::new(
                DiagnosticCode::error(202),
                span,
                format!("`{id}` is a module attribute that TyPy does not provide"),
            )
            .with_rule(RuleId::STD_04),
            None => Diagnostic::new(
                DiagnosticCode::error(205),
                span,
                format!("`{id}` is not defined"),
            )
            .with_rule(RuleId::STD_04),
        };
        self.error(diagnostic, span)
    }

    fn operator_error(&mut self, symbol: &str, left: Type, right: Type, span: Span) -> Type {
        self.diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(401),
                span,
                format!("`{symbol}` is not defined for {left} and {right}"),
            )
            .with_rule(RuleId::EXP_02),
        );
        Type::Error
    }

    fn bool_arithmetic(&mut self, span: Span) -> Type {
        self.diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(406),
                span,
                "arithmetic on bool values is not supported",
            )
            .with_rule(RuleId::DAT_03)
            .with_help("convert with `int(...)` first"),
        );
        Type::Error
    }

    /// Returns the type of a binary operation, with Python's results (EXP-02 to EXP-05).
    fn binary_type(
        &mut self,
        operator: BinaryOperator,
        left: &TypedExpr,
        right: &TypedExpr,
        span: Span,
    ) -> Type {
        use BinaryOperator as Op;
        use Type::{Bool, Error, Float, Int, Str};
        let (l, r) = (left.ty, right.ty);
        if l == Error || r == Error {
            return Error;
        }
        let symbol = binary_symbol(operator);
        match operator {
            Op::BitAnd | Op::BitOr | Op::BitXor if l == Bool && r == Bool => Bool,
            _ if l == Bool || r == Bool => self.bool_arithmetic(span),
            Op::Add if l == Str && r == Str => Str,
            Op::Mult if (l == Str && r == Int) || (l == Int && r == Str) => Str,
            Op::Add | Op::Sub | Op::Mult | Op::FloorDiv | Op::Mod
                if l.is_numeric() && r.is_numeric() =>
            {
                if l == Int && r == Int {
                    Int
                } else {
                    Float
                }
            }
            Op::Div if l.is_numeric() && r.is_numeric() => Float,
            Op::Pow if l.is_numeric() && r.is_numeric() => {
                if l == Int && r == Int {
                    if !matches!(&right.kind, TypedExprKind::Constant(TypedConstant::Int(_))) {
                        self.diagnostics.push(
                            Diagnostic::new(
                                DiagnosticCode::error(302),
                                right.span,
                                "the exponent of `int ** int` must be a non-negative integer literal, because CPython returns a float for a negative exponent",
                            )
                            .with_rule(RuleId::EXP_04)
                            .with_help("write the base as a float, such as `2.0 ** n`"),
                        );
                        return Error;
                    }
                    Int
                } else {
                    Float
                }
            }
            Op::LShift | Op::RShift | Op::BitAnd | Op::BitOr | Op::BitXor
                if l == Int && r == Int =>
            {
                Int
            }
            Op::Mod if l == Str => {
                self.diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::error(201),
                        span,
                        "formatting strings with `%` is planned for TyPy 0.2 (LIB-03)",
                    )
                    .with_rule(RuleId::LIB_03),
                );
                Error
            }
            _ => self.operator_error(symbol, l, r, span),
        }
    }

    fn unary_type(&mut self, operator: UnaryOperator, operand: &TypedExpr, span: Span) -> Type {
        let ty = operand.ty;
        match operator {
            _ if ty == Type::Error => Type::Error,
            UnaryOperator::Not => Type::Bool,
            _ if ty == Type::Bool => self.bool_arithmetic(span),
            UnaryOperator::USub | UnaryOperator::UAdd if ty.is_numeric() => ty,
            UnaryOperator::Invert if ty == Type::Int => Type::Int,
            _ => {
                let symbol = match operator {
                    UnaryOperator::USub => "-",
                    UnaryOperator::UAdd => "+",
                    UnaryOperator::Invert | UnaryOperator::Not => "~",
                };
                self.diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::error(401),
                        span,
                        format!("unary `{symbol}` is not defined for {ty}"),
                    )
                    .with_rule(RuleId::EXP_02),
                );
                Type::Error
            }
        }
    }

    /// Checks one link of a comparison chain (EXP-06, EXP-08).
    fn compare_types(&mut self, operator: CompareOperator, left: &TypedExpr, right: &TypedExpr) {
        let (l, r) = (left.ty, right.ty);
        if l == Type::Error || r == Type::Error {
            return;
        }
        let span = right.span;
        match operator {
            CompareOperator::Is | CompareOperator::IsNot => {
                if l != Type::NoneType && r != Type::NoneType {
                    self.diagnostics.push(
                        Diagnostic::new(
                            DiagnosticCode::error(301),
                            span,
                            format!("`is` compares identity, which is defined only against None here; these are {l} and {r}"),
                        )
                        .with_rule(RuleId::EXP_08)
                        .with_help("use `==` to compare values"),
                    );
                }
            }
            CompareOperator::Eq | CompareOperator::NotEq => {
                if !(l == r || (l.is_numeric() && r.is_numeric())) {
                    self.comparison_error(l, r, span);
                }
            }
            CompareOperator::Lt
            | CompareOperator::LtE
            | CompareOperator::Gt
            | CompareOperator::GtE => {
                let ordered = (l == r && l != Type::NoneType) || (l.is_numeric() && r.is_numeric());
                if !ordered {
                    self.comparison_error(l, r, span);
                }
            }
            CompareOperator::In | CompareOperator::NotIn => {}
        }
    }

    fn comparison_error(&mut self, left: Type, right: Type, span: Span) {
        self.diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(407),
                span,
                format!("{left} and {right} cannot be compared"),
            )
            .with_rule(RuleId::EXP_06),
        );
    }

    fn call(
        &mut self,
        name: &str,
        name_span: Span,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        if let Some(symbol) = self.locals.get(name) {
            let ty = self.types.get(symbol).copied().unwrap_or(Type::Error);
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(401),
                    name_span,
                    format!("`{name}` is {} and cannot be called", ty.with_article()),
                )
                .with_rule(RuleId::EXP_02),
                span,
            );
        }
        if let Some(function) = self.program.function_ids.get(name).copied() {
            return self.call_function(function, name, arguments, keywords, span);
        }
        if let Some(builtin) = Builtin::named(name) {
            return self.call_builtin(builtin, arguments, keywords, span);
        }
        if self.in_function() && self.program.module_variables.contains(name) {
            return self.name(name, name_span);
        }
        let result = self.unknown_name(name, name_span);
        for argument in arguments {
            self.expression(argument);
        }
        TypedExpr { span, ..result }
    }

    fn call_function(
        &mut self,
        function: FunctionId,
        name: &str,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        let signature = self.program.signatures[function.0 as usize].clone();
        let mut bound: Vec<Argument> = Vec::new();
        let mut filled = vec![false; signature.parameters.len()];
        for (index, argument) in arguments.iter().enumerate() {
            let value = self.expression(argument);
            let Some(parameter) = signature.parameters.get(index) else {
                return self.error(
                    Diagnostic::new(
                        DiagnosticCode::error(404),
                        value.span,
                        format!(
                            "`{name}` takes {} arguments, and more are given",
                            signature.parameters.len()
                        ),
                    )
                    .with_rule(RuleId::TYP_01),
                    span,
                );
            };
            self.expect(
                &value,
                parameter.ty,
                &format!("argument `{}`", parameter.name),
            );
            filled[index] = true;
            bound.push(Argument {
                parameter: index,
                value,
            });
        }
        for keyword in keywords {
            let value = self.expression(&keyword.value);
            let Some(keyword_name) = &keyword.name else {
                continue;
            };
            let Some(index) = signature
                .parameters
                .iter()
                .position(|parameter| parameter.name == keyword_name.name)
            else {
                return self.error(
                    Diagnostic::new(
                        DiagnosticCode::error(404),
                        keyword_name.span,
                        format!("`{name}` has no parameter named `{}`", keyword_name.name),
                    )
                    .with_rule(RuleId::TYP_01),
                    span,
                );
            };
            if filled[index] {
                return self.error(
                    Diagnostic::new(
                        DiagnosticCode::error(404),
                        keyword_name.span,
                        format!("argument `{}` is given more than once", keyword_name.name),
                    )
                    .with_rule(RuleId::TYP_01),
                    span,
                );
            }
            self.expect(
                &value,
                signature.parameters[index].ty,
                &format!("argument `{}`", keyword_name.name),
            );
            filled[index] = true;
            bound.push(Argument {
                parameter: index,
                value,
            });
        }
        for (parameter, is_filled) in signature.parameters.iter().zip(&filled) {
            if !is_filled && parameter.default.is_none() {
                return self.error(
                    Diagnostic::new(
                        DiagnosticCode::error(404),
                        span,
                        format!("`{name}` is missing the argument `{}`", parameter.name),
                    )
                    .with_rule(RuleId::TYP_01),
                    span,
                );
            }
        }
        Self::typed(
            TypedExprKind::Call {
                function,
                arguments: bound,
            },
            signature.returns,
            span,
        )
    }

    fn arity(
        &mut self,
        builtin: Builtin,
        given: usize,
        allowed: std::ops::RangeInclusive<usize>,
        span: Span,
    ) -> bool {
        if allowed.contains(&given) {
            return true;
        }
        let expected = if allowed.start() == allowed.end() {
            format!("{}", allowed.start())
        } else {
            format!("{} to {}", allowed.start(), allowed.end())
        };
        self.diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(404),
                span,
                format!(
                    "`{}` takes {expected} arguments in TyPy 0.1, and {given} are given",
                    builtin.name()
                ),
            )
            .with_rule(RuleId::LIB_01),
        );
        false
    }

    #[allow(clippy::too_many_lines, reason = "one arm per builtin")]
    fn call_builtin(
        &mut self,
        builtin: Builtin,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        if builtin == Builtin::Print {
            return self.print(arguments, keywords, span);
        }
        let values: Vec<TypedExpr> = arguments
            .iter()
            .map(|argument| self.expression(argument))
            .collect();
        if let Some(keyword) = keywords.first() {
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    keyword.span,
                    format!(
                        "`{}` takes no keyword arguments in TyPy 0.1",
                        builtin.name()
                    ),
                )
                .with_rule(RuleId::LIB_01),
                span,
            );
        }
        if values.iter().any(|value| value.ty == Type::Error) {
            return Self::typed(
                TypedExprKind::Constant(TypedConstant::None),
                Type::Error,
                span,
            );
        }
        let types: Vec<Type> = values.iter().map(|value| value.ty).collect();
        let result = match builtin {
            Builtin::Print => unreachable!("print is checked above"),
            Builtin::Range => {
                return self.error(
                    Diagnostic::new(
                        DiagnosticCode::error(201),
                        span,
                        "`range` outside a `for` loop is planned for TyPy 0.2 (CST-03)",
                    )
                    .with_rule(RuleId::CST_03),
                    span,
                );
            }
            Builtin::Len => {
                if !self.arity(builtin, values.len(), 1..=1, span) {
                    return Self::typed(
                        TypedExprKind::Constant(TypedConstant::None),
                        Type::Error,
                        span,
                    );
                }
                self.expect(&values[0], Type::Str, "the argument of `len`");
                Type::Int
            }
            Builtin::Abs => {
                if !self.arity(builtin, values.len(), 1..=1, span) {
                    return Self::typed(
                        TypedExprKind::Constant(TypedConstant::None),
                        Type::Error,
                        span,
                    );
                }
                match types[0] {
                    Type::Int | Type::Float => types[0],
                    Type::Bool => self.bool_arithmetic(values[0].span),
                    other => self.operator_error("abs", other, other, values[0].span),
                }
            }
            Builtin::Min | Builtin::Max => {
                if values.len() == 1 {
                    return self.error(
                        Diagnostic::new(
                            DiagnosticCode::error(201),
                            span,
                            format!(
                                "`{}` of one iterable is planned for TyPy 0.2 (LIB-02)",
                                builtin.name()
                            ),
                        )
                        .with_rule(RuleId::LIB_02),
                        span,
                    );
                }
                if !self.arity(builtin, values.len(), 2..=usize::MAX, span) {
                    return Self::typed(
                        TypedExprKind::Constant(TypedConstant::None),
                        Type::Error,
                        span,
                    );
                }
                let first = types[0];
                if let Some(other) = values.iter().find(|value| value.ty != first) {
                    return self.error(
                        Diagnostic::new(
                            DiagnosticCode::error(404),
                            other.span,
                            format!(
                                "the arguments of `{}` must have one type, because CPython returns one of them unchanged; they are {first} and {}",
                                builtin.name(),
                                other.ty
                            ),
                        )
                        .with_rule(RuleId::TYP_09),
                        span,
                    );
                }
                if !matches!(first, Type::Int | Type::Float | Type::Str) {
                    self.comparison_error(first, first, span);
                    return Self::typed(
                        TypedExprKind::Constant(TypedConstant::None),
                        Type::Error,
                        span,
                    );
                }
                first
            }
            Builtin::Int | Builtin::Float => {
                let target = if builtin == Builtin::Int {
                    Type::Int
                } else {
                    Type::Float
                };
                if !self.arity(
                    builtin,
                    values.len(),
                    0..=usize::from(builtin == Builtin::Int) + 1,
                    span,
                ) {
                    return Self::typed(
                        TypedExprKind::Constant(TypedConstant::None),
                        Type::Error,
                        span,
                    );
                }
                if let Some(value) = values.first() {
                    if !matches!(value.ty, Type::Int | Type::Float | Type::Str | Type::Bool) {
                        self.expect(
                            value,
                            target,
                            &format!("the argument of `{}`", builtin.name()),
                        );
                    }
                    if values.len() == 2 {
                        self.expect(
                            &values[0],
                            Type::Str,
                            "the first argument of `int` with a base",
                        );
                        self.expect(&values[1], Type::Int, "the base");
                    }
                }
                target
            }
            Builtin::Str | Builtin::Repr => {
                let allowed = if builtin == Builtin::Str {
                    0..=1
                } else {
                    1..=1
                };
                if !self.arity(builtin, values.len(), allowed, span) {
                    return Self::typed(
                        TypedExprKind::Constant(TypedConstant::None),
                        Type::Error,
                        span,
                    );
                }
                Type::Str
            }
            Builtin::Bool => {
                if !self.arity(builtin, values.len(), 0..=1, span) {
                    return Self::typed(
                        TypedExprKind::Constant(TypedConstant::None),
                        Type::Error,
                        span,
                    );
                }
                Type::Bool
            }
            Builtin::Ord => {
                if !self.arity(builtin, values.len(), 1..=1, span) {
                    return Self::typed(
                        TypedExprKind::Constant(TypedConstant::None),
                        Type::Error,
                        span,
                    );
                }
                self.expect(&values[0], Type::Str, "the argument of `ord`");
                Type::Int
            }
            Builtin::Chr => {
                if !self.arity(builtin, values.len(), 1..=1, span) {
                    return Self::typed(
                        TypedExprKind::Constant(TypedConstant::None),
                        Type::Error,
                        span,
                    );
                }
                self.expect(&values[0], Type::Int, "the argument of `chr`");
                Type::Str
            }
        };
        Self::typed(
            TypedExprKind::Builtin {
                builtin,
                arguments: values,
            },
            result,
            span,
        )
    }

    fn print(&mut self, arguments: &[Expr], keywords: &[Keyword], span: Span) -> TypedExpr {
        let values: Vec<TypedExpr> = arguments
            .iter()
            .map(|argument| self.expression(argument))
            .collect();
        let mut sep = None;
        let mut end = None;
        for keyword in keywords {
            let value = self.expression(&keyword.value);
            let Some(name) = &keyword.name else {
                continue;
            };
            match name.name.as_str() {
                "sep" => {
                    self.expect(&value, Type::Str, "`sep`");
                    sep = Some(Box::new(value));
                }
                "end" => {
                    self.expect(&value, Type::Str, "`end`");
                    end = Some(Box::new(value));
                }
                "file" | "flush" => {
                    return self.error(
                        Diagnostic::new(
                            DiagnosticCode::error(201),
                            name.span,
                            format!(
                                "the `{}` argument of `print` is planned for TyPy 0.3 (LIB-04)",
                                name.name
                            ),
                        )
                        .with_rule(RuleId::LIB_04),
                        span,
                    );
                }
                other => {
                    return self.error(
                        Diagnostic::new(
                            DiagnosticCode::error(404),
                            name.span,
                            format!("`print` has no parameter named `{other}`"),
                        )
                        .with_rule(RuleId::LIB_01),
                        span,
                    );
                }
            }
        }
        Self::typed(
            TypedExprKind::Print { values, sep, end },
            Type::NoneType,
            span,
        )
    }
}

fn binary_symbol(operator: BinaryOperator) -> &'static str {
    match operator {
        BinaryOperator::Add => "+",
        BinaryOperator::Sub => "-",
        BinaryOperator::Mult => "*",
        BinaryOperator::MatMult => "@",
        BinaryOperator::Div => "/",
        BinaryOperator::Mod => "%",
        BinaryOperator::Pow => "**",
        BinaryOperator::LShift => "<<",
        BinaryOperator::RShift => ">>",
        BinaryOperator::BitOr => "|",
        BinaryOperator::BitXor => "^",
        BinaryOperator::BitAnd => "&",
        BinaryOperator::FloorDiv => "//",
    }
}
