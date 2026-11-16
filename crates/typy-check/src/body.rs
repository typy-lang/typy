//! Checks one function body, or the module body: names, types and definite assignment.

mod accessor;
mod builtin_call;
mod builtin_module;
mod call;
mod class;
mod class_bridge;
mod class_copy;
mod class_value;
mod class_variable_chain;
mod comprehension;
mod container_method;
mod delete;
mod dict;
mod dict_call;
mod each_member;
mod generic_call;
mod generic_copy;
pub use each_member::member_class;
mod exception;
mod exception_creation;
mod exception_part;
mod expression;
mod flow;
mod fstring;
mod import;
mod in_place;
mod isinstance;
mod iterable_call;
mod iterator;
mod list;
mod list_call;
mod locals;
mod loop_source;
mod matching;
mod math_call;
mod math_iterable;
mod method_call;
mod module_context;
mod name;
mod named;
mod open_call;
mod open_options;
mod operator;
mod operator_method;
mod os_error;
mod pattern_class;
mod pattern_coverage;
mod pattern_mapping;
mod pattern_sequence;
mod percent;
mod program;
mod protocol;
mod rebound;
mod round_call;
mod set;
mod slice;
mod statement;
mod super_call;
mod sys_call;
mod type_alias_value;
mod type_object;
mod typing_class;
mod union;
mod union_loop;
mod version_info;
mod view_operator;

pub(crate) use builtin_module::ModuleFunction;
pub(crate) use class_copy::{ClassCopies, GenericClass};
pub(crate) use import::Imported;
pub(crate) use module_context::ModuleContext;
pub(crate) use program::Program;
pub(crate) use rebound::{Binding, Bound, binding_variable, rebound_planned};

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
    /// The variables an assignment expression in the body assigns (EXP-13).
    named_targets: Vec<SymbolId>,
    declared: BTreeMap<SymbolId, Type>,
    types: BTreeMap<SymbolId, Type>,
    assigned: Assigned,
    breaks: Vec<Vec<Assigned>>,
    reported_unassigned: BTreeSet<SymbolId>,
    returns: Option<Type>,
    /// The class and instance of the `__init__` being checked.
    initializing: Option<class::Initializing>,
    /// The types `isinstance` tests narrow variables to, which `Fact::Narrowed` names by
    /// position (TYP-07).
    narrowings: Vec<Type>,
    /// Statements that follow the statement being checked, such as the one that stores a
    /// lazy import's number in its binding variables (ADR-0036).
    following: Vec<crate::typed::TypedStmt>,
    /// The statements that assign the hidden variables an augmented assignment holds its
    /// target's object and key in, made while it is checked (CLS-20).
    pins: Vec<crate::typed::TypedStmt>,
    /// The variables, or `args` of the exceptions variables hold, an operation is being
    /// checked for each member of, innermost last, each with the member it is checked for now
    /// (TYP-14).
    splitting: Vec<(each_member::Split, Type)>,
    /// The class and instance of the method being checked, which `super()` reads.
    method: Option<class::Initializing>,
    /// Whether a use of the incomplete instance has been reported, so one cause gives one error.
    reported_incomplete: bool,
    /// How many `except` clauses enclose the statement being checked, in this function.
    handling: usize,
    /// For each `finally` block being checked, how many loops were open where it starts.
    finally_loops: Vec<usize>,
    /// The module variables the function declares `global`, which it assigns (SMT-08).
    global_names: BTreeSet<String>,
    /// The type aliases the body's `type` statements define, which annotations read
    /// (SMT-12).
    local_aliases: BTreeSet<String>,
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
            named_targets: Vec::new(),
            declared: BTreeMap::new(),
            types: BTreeMap::new(),
            assigned: Some(BTreeSet::new()),
            breaks: Vec::new(),
            reported_unassigned: BTreeSet::new(),
            returns,
            initializing: None,
            method: None,
            narrowings: Vec::new(),
            splitting: Vec::new(),
            following: Vec::new(),
            pins: Vec::new(),
            reported_incomplete: false,
            handling: 0,
            finally_loops: Vec::new(),
            global_names: BTreeSet::new(),
            local_aliases: BTreeSet::new(),
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
        if let (Some(class), true, Some(instance)) = (
            definition.class,
            definition.takes_instance,
            parameters.first(),
        ) {
            checker.method = Some(class::Initializing {
                class,
                instance: *instance,
            });
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
            // An attribute with a default holds it before `__init__` runs (ADR-0024).
            let defaults = checker.defaults(class);
            if let Some(facts) = &mut checker.assigned {
                facts.extend(defaults.into_iter().map(flow::Fact::Attribute));
            }
        }
        // A protocol's method only says what a class that satisfies the protocol has, so its
        // body, often `...`, is never run or checked (GEN-04).
        let protocol = definition
            .class
            .is_some_and(|class| checker.program.is_protocol(class));
        let checked_body: &[Stmt] = if protocol { &[] } else { definition.body };
        checker.declare_globals(checked_body);
        let saved = checker.import_names_of(checked_body);
        let statements: Vec<&Stmt> = checked_body.iter().collect();
        checker.collect_locals(&statements);
        let body = checker.statements(&statements);
        checker.restore_names(saved);
        checker.check_instance_complete(definition.span);
        if checker.assigned.is_some()
            && !protocol
            && returns != Type::NoneType
            && returns != Type::Error
        {
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
        let named_targets = std::mem::take(&mut checker.named_targets);
        let locals = checker.finish_locals();
        // A generic function is checked, never run: its copies run in its place, so it keeps
        // no body (ADR-0023). A protocol's method only says what a class that satisfies the
        // protocol has (GEN-04).
        let generic =
            program.generic(function).is_some() || program.is_opaque_copy(function) || protocol;
        program.functions[function.0 as usize] = Some(TypedFunction {
            id: function,
            name: definition.name.clone(),
            qualified_name: definition.qualified_name.clone(),
            module: program.module.clone(),
            signature: definition.signature.clone(),
            parameters,
            locals: if generic { Vec::new() } else { locals },
            body: if generic { Vec::new() } else { body },
            span: definition.span,
            generic,
            named_targets,
        });
    }

    /// Checks the module body and stores it in the program as its entry function.
    pub(crate) fn check_module(
        program: &mut Program,
        statements: &[&Stmt],
        diagnostics: &mut Diagnostics,
    ) -> FunctionId {
        let mut checker = BodyChecker::new(program, diagnostics, None);
        checker.collect_locals(statements);
        let first = statements.first().map(|statement| statement.span());
        let mut body = first.map_or_else(Vec::new, |span| checker.program.rebound_starts(span));
        body.extend(checker.statements(statements));
        let named_targets = std::mem::take(&mut checker.named_targets);
        let locals = checker.finish_locals();
        // A lazy import adds the function that runs it, so the body's ID comes after those.
        let entry = FunctionId(id(program.functions.len()));
        let span = statements.first().map_or_else(
            || Span::empty(typy_diagnostics::FileId(0), typy_diagnostics::TextOffset(0)),
            |first| first.span(),
        );
        // The module body takes an ID as a function does, so it has a signature too.
        program.signatures.push(FunctionSignature {
            parameters: Vec::new(),
            returns: Type::NoneType,
        });
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
            generic: false,
            named_targets,
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
