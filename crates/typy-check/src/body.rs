//! Checks one function body, or the module body: names, types and definite assignment.

mod builtin_call;
mod call;
mod class;
mod comprehension;
mod dict;
mod expression;
mod flow;
mod iterable_call;
mod list;
mod list_call;
mod locals;
mod loop_source;
mod method_call;
mod name;
mod operator;
mod program;
mod round_call;
mod set;
mod slice;
mod statement;
mod type_object;

pub(crate) use program::Program;

use std::collections::{BTreeMap, BTreeSet};

use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId, Span};
use typy_source::syntax::Stmt;

use crate::check::Definition;
use crate::typed::{
    FunctionId, FunctionSignature, SymbolId, SymbolKind, TypedConstant, TypedExpr, TypedExprKind,
    TypedFunction,
};
use crate::types::Type;
use flow::Assigned;
use program::id;

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
    /// The class and instance of the `__init__` being checked.
    initializing: Option<class::Initializing>,
    /// Whether a use of the incomplete instance has been reported, so one cause gives one error.
    reported_incomplete: bool,
}

impl<'program> BodyChecker<'program> {
    pub(super) fn new(
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
            initializing: None,
            reported_incomplete: false,
        }
    }

    /// Checks a module-level function and stores it in the program.
    pub(crate) fn check_function(
        program: &mut Program,
        function: FunctionId,
        definition: &Definition<'_>,
        diagnostics: &mut Diagnostics,
    ) {
        let returns = definition.signature.returns.clone();
        let mut checker = BodyChecker::new(program, diagnostics, Some(returns.clone()));
        let mut parameters = Vec::new();
        for ((name, span), parameter) in definition
            .parameters
            .iter()
            .zip(&definition.signature.parameters)
        {
            let symbol = checker.program.add_symbol(
                name,
                SymbolKind::Parameter,
                parameter.ty.clone(),
                *span,
            );
            checker.locals.insert(name.clone(), symbol);
            checker.types.insert(symbol, parameter.ty.clone());
            checker
                .assigned
                .get_or_insert_with(BTreeSet::new)
                .insert(flow::Fact::Assigned(symbol));
            parameters.push(symbol);
        }
        if let (Some(class), "__init__", Some(instance)) = (
            definition.class,
            definition.name.as_str(),
            parameters.first(),
        ) {
            checker.initializing = Some(class::Initializing {
                class,
                instance: *instance,
            });
        }
        let statements: Vec<&Stmt> = definition.body.iter().collect();
        checker.collect_locals(&statements);
        let body = checker.statements(&statements);
        checker.check_instance_complete(definition.span);
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
            qualified_name: definition.qualified_name.clone(),
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
            qualified_name: "<module>".to_owned(),
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

    /// Reports an `__init__` that can end, here, without assigning every attribute.
    pub(super) fn check_instance_complete(&mut self, span: Span) {
        if let Some(initializing) = self.initializing
            && !self.instance_is_complete(initializing)
        {
            self.incomplete_instance(initializing, span, true);
        }
    }

    pub(super) fn in_function(&self) -> bool {
        self.returns.is_some()
    }

    pub(super) fn error(&mut self, diagnostic: Diagnostic, span: Span) -> TypedExpr {
        self.diagnostics.push(diagnostic);
        TypedExpr {
            kind: TypedExprKind::Constant(TypedConstant::None),
            ty: Type::Error,
            span,
        }
    }

    pub(super) fn typed(kind: TypedExprKind, ty: Type, span: Span) -> TypedExpr {
        TypedExpr { kind, ty, span }
    }
}
