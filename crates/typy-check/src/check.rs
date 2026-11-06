//! Checks a project and builds its typed program (detailed design section 7.2).

use std::collections::{BTreeMap, BTreeSet};

use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId, Span};
use typy_source::Project;
use typy_source::syntax::{Constant, Expr, Stmt};

use crate::body::{BodyChecker, Program};
use crate::subset;
use crate::typed::{
    FunctionId, FunctionSignature, SignatureParameter, TypedConstant, TypedProgram,
};
use crate::types::Type;

/// Resolves names, checks types and the subset rules, and builds the typed program.
#[derive(Clone, Copy, Debug, Default)]
pub struct TypeChecker;

impl TypeChecker {
    /// Returns the typed program, or every diagnostic found; a program with errors is never returned (TYP-11).
    pub fn check(project: &Project) -> Result<TypedProgram, Diagnostics> {
        let module = project.main_module();
        let tree = &module.syntax;
        let mut diagnostics = subset::check(tree);
        if diagnostics.has_errors() {
            return Err(diagnostics);
        }

        let functions = collect_functions(&tree.body, &mut diagnostics);
        let mut variables = BTreeMap::new();
        module_variables(&tree.body, &mut variables);
        check_names_are_functions_or_variables(&functions, &variables, &mut diagnostics);
        check_calls_follow_definitions(&tree.body, &functions, &mut diagnostics);

        let mut program = Program::new(
            module.name.as_str(),
            &functions,
            variables.into_keys().collect(),
        );
        for (index, definition) in functions.iter().enumerate() {
            let id = FunctionId(u32::try_from(index).unwrap_or(u32::MAX));
            BodyChecker::check_function(&mut program, id, definition, &mut diagnostics);
        }
        let module_body: Vec<&Stmt> = tree
            .body
            .iter()
            .filter(|statement| !matches!(statement, Stmt::FunctionDef { .. }))
            .collect();
        let entry = BodyChecker::check_module(&mut program, &module_body, &mut diagnostics);

        if diagnostics.has_errors() {
            return Err(diagnostics);
        }
        Ok(program.finish(entry))
    }
}

/// A module-level function definition with its signature.
#[derive(Clone, Debug)]
pub(crate) struct Definition<'tree> {
    pub(crate) name: String,
    pub(crate) signature: FunctionSignature,
    pub(crate) parameters: Vec<(String, Span)>,
    pub(crate) body: &'tree [Stmt],
    pub(crate) span: Span,
}

/// Collects the module's functions in source order, with their signatures (TYP-01).
fn collect_functions<'tree>(
    statements: &'tree [Stmt],
    diagnostics: &mut Diagnostics,
) -> Vec<Definition<'tree>> {
    let mut functions: Vec<Definition<'tree>> = Vec::new();
    for statement in statements {
        let Stmt::FunctionDef {
            name,
            parameters,
            returns,
            body,
            span,
            ..
        } = statement
        else {
            continue;
        };
        if functions.iter().any(|existing| existing.name == name.name) {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(202),
                    name.span,
                    format!(
                        "`{}` is defined more than once; TyPy keeps one definition per name",
                        name.name
                    ),
                )
                .with_rule(RuleId::STD_04),
            );
            continue;
        }
        let mut signature_parameters = Vec::new();
        let mut parameter_names = Vec::new();
        for parameter in parameters {
            let ty = if let Some(annotation) = &parameter.annotation {
                annotation_type(annotation, diagnostics)
            } else {
                diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::error(203),
                        parameter.span,
                        format!("parameter `{}` has no type annotation", parameter.name.name),
                    )
                    .with_rule(RuleId::TYP_01)
                    .with_help(format!(
                        "write `{}: int`, or the type it holds",
                        parameter.name.name
                    )),
                );
                Type::Error
            };
            let default = parameter
                .default
                .as_ref()
                .and_then(|default| default_value(default, ty, diagnostics));
            signature_parameters.push(SignatureParameter {
                name: parameter.name.name.clone(),
                ty,
                default,
            });
            parameter_names.push((parameter.name.name.clone(), parameter.name.span));
        }
        let returns = if let Some(annotation) = returns {
            annotation_type(annotation, diagnostics)
        } else {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(204),
                    name.span,
                    format!("function `{}` has no return annotation", name.name),
                )
                .with_rule(RuleId::TYP_01)
                .with_help("write `-> None` if it returns nothing"),
            );
            Type::Error
        };
        functions.push(Definition {
            name: name.name.clone(),
            signature: FunctionSignature {
                parameters: signature_parameters,
                returns,
            },
            parameters: parameter_names,
            body,
            span: *span,
        });
    }
    functions
}

/// Returns the type an annotation names; 0.1 knows `int`, `float`, `bool`, `str` and `None`.
pub(crate) fn annotation_type(annotation: &Expr, diagnostics: &mut Diagnostics) -> Type {
    match annotation {
        Expr::Constant {
            value: Constant::None,
            ..
        } => Type::NoneType,
        Expr::Name { id, span } => {
            match id.as_str() {
                "int" => Type::Int,
                "float" => Type::Float,
                "bool" => Type::Bool,
                "str" => Type::Str,
                "Any" | "object" => {
                    diagnostics.push(
                        Diagnostic::new(
                            DiagnosticCode::error(403),
                            *span,
                            format!("`{id}` does not give the value one type"),
                        )
                        .with_rule(RuleId::TYP_10),
                    );
                    Type::Error
                }
                "list" | "dict" | "set" | "tuple" | "bytes" => {
                    diagnostics.push(
                        Diagnostic::new(
                            DiagnosticCode::error(201),
                            *span,
                            format!("`{id}` annotations are planned for TyPy 0.2 (TYP-06)"),
                        )
                        .with_rule(RuleId::TYP_06),
                    );
                    Type::Error
                }
                _ => {
                    diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::error(205),
                        *span,
                        format!("`{id}` is not a type TyPy 0.1 knows; use int, float, bool, str or None"),
                    )
                    .with_rule(RuleId::STD_04),
                );
                    Type::Error
                }
            }
        }
        other => {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(205),
                    other.span(),
                    "this annotation does not name a type",
                )
                .with_rule(RuleId::STD_04),
            );
            Type::Error
        }
    }
}

/// Returns the constant of a default value, checked against the parameter's type (TYP-09).
fn default_value(default: &Expr, ty: Type, diagnostics: &mut Diagnostics) -> Option<TypedConstant> {
    let Expr::Constant { value, span } = default else {
        return None;
    };
    let constant = match value {
        Constant::None => TypedConstant::None,
        Constant::Bool(value) => TypedConstant::Bool(*value),
        Constant::Int(digits) => TypedConstant::Int(digits.clone()),
        Constant::Float(value) => TypedConstant::Float(*value),
        Constant::Str(value) => TypedConstant::Str(value.clone()),
        Constant::Complex { .. } | Constant::Bytes(_) | Constant::Ellipsis => return None,
    };
    if !constant.ty().is_assignable_to(ty) {
        let mut diagnostic = Diagnostic::new(
            DiagnosticCode::error(404),
            *span,
            format!(
                "the default value is {}, but the parameter is {}",
                constant.ty().with_article(),
                ty.with_article()
            ),
        )
        .with_rule(if ty == Type::Float {
            RuleId::TYP_09
        } else {
            RuleId::TYP_03
        });
        if let (TypedConstant::Int(digits), Type::Float) = (&constant, ty) {
            diagnostic = diagnostic.with_fix(*span, format!("{digits}.0"));
        }
        diagnostics.push(diagnostic);
    }
    Some(constant)
}

/// Names bound by module-level statements outside function definitions.
fn module_variables(statements: &[Stmt], names: &mut BTreeMap<String, Span>) {
    fn bind(names: &mut BTreeMap<String, Span>, target: &Expr) {
        if let Expr::Name { id, span } = target {
            names.entry(id.clone()).or_insert(*span);
        }
    }
    for statement in statements {
        match statement {
            Stmt::Assign { targets, .. } => targets.iter().for_each(|target| bind(names, target)),
            Stmt::AugAssign { target, .. } | Stmt::AnnAssign { target, .. } => bind(names, target),
            Stmt::For {
                target,
                body,
                orelse,
                ..
            } => {
                bind(names, target);
                module_variables(body, names);
                module_variables(orelse, names);
            }
            Stmt::While { body, orelse, .. } | Stmt::If { body, orelse, .. } => {
                module_variables(body, names);
                module_variables(orelse, names);
            }
            _ => {}
        }
    }
}

/// A name may be a module-level function or a module-level variable, not both: CPython would rebind it.
fn check_names_are_functions_or_variables(
    functions: &[Definition<'_>],
    variables: &BTreeMap<String, Span>,
    diagnostics: &mut Diagnostics,
) {
    for definition in functions {
        if let Some(span) = variables.get(&definition.name) {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(202),
                    *span,
                    format!(
                        "`{}` is a function and is also assigned as a variable",
                        definition.name
                    ),
                )
                .with_rule(RuleId::STD_04),
            );
        }
    }
}

/// Collects the module functions a statement list names, outside nested definitions.
fn referenced_functions(
    statements: &[Stmt],
    functions: &BTreeSet<String>,
    found: &mut BTreeSet<String>,
) {
    for statement in statements {
        visit_statement(statement, &mut |expression| {
            if let Expr::Name { id, .. } = expression
                && functions.contains(id)
            {
                found.insert(id.clone());
            }
        });
    }
}

fn visit_statement(statement: &Stmt, visit: &mut dyn FnMut(&Expr)) {
    match statement {
        Stmt::Return {
            value: Some(value), ..
        }
        | Stmt::Expr { value, .. }
        | Stmt::AnnAssign {
            value: Some(value), ..
        } => visit_expression(value, visit),
        Stmt::Assign { targets, value, .. } => {
            for target in targets {
                visit_expression(target, visit);
            }
            visit_expression(value, visit);
        }
        Stmt::AugAssign { target, value, .. } => {
            visit_expression(target, visit);
            visit_expression(value, visit);
        }
        Stmt::For {
            iter, body, orelse, ..
        } => {
            visit_expression(iter, visit);
            body.iter()
                .chain(orelse)
                .for_each(|inner| visit_statement(inner, visit));
        }
        Stmt::While {
            test, body, orelse, ..
        }
        | Stmt::If {
            test, body, orelse, ..
        } => {
            visit_expression(test, visit);
            body.iter()
                .chain(orelse)
                .for_each(|inner| visit_statement(inner, visit));
        }
        _ => {}
    }
}

fn visit_expression(expression: &Expr, visit: &mut dyn FnMut(&Expr)) {
    visit(expression);
    match expression {
        Expr::BoolOp { values, .. } => values
            .iter()
            .for_each(|value| visit_expression(value, visit)),
        Expr::BinOp { left, right, .. } => {
            visit_expression(left, visit);
            visit_expression(right, visit);
        }
        Expr::UnaryOp { operand, .. } => visit_expression(operand, visit),
        Expr::IfExp {
            test, body, orelse, ..
        } => {
            visit_expression(test, visit);
            visit_expression(body, visit);
            visit_expression(orelse, visit);
        }
        Expr::Compare {
            left, comparators, ..
        } => {
            visit_expression(left, visit);
            for comparator in comparators {
                visit_expression(comparator, visit);
            }
        }
        Expr::Call {
            function,
            arguments,
            keywords,
            ..
        } => {
            visit_expression(function, visit);
            for argument in arguments {
                visit_expression(argument, visit);
            }
            for keyword in keywords {
                visit_expression(&keyword.value, visit);
            }
        }
        Expr::Subscript { value, index, .. } => {
            visit_expression(value, visit);
            visit_expression(index, visit);
        }
        Expr::Constant { .. } | Expr::Name { .. } | Expr::Other { .. } => {}
    }
}

/// A module-level statement may only reach functions whose `def` has already run; otherwise CPython raises `NameError` (TYP-12).
fn check_calls_follow_definitions(
    statements: &[Stmt],
    functions: &[Definition<'_>],
    diagnostics: &mut Diagnostics,
) {
    let names: BTreeSet<String> = functions
        .iter()
        .map(|definition| definition.name.clone())
        .collect();
    let mut calls: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    for definition in functions {
        let mut found = BTreeSet::new();
        referenced_functions(definition.body, &names, &mut found);
        calls.insert(definition.name.as_str(), found);
    }
    let mut defined: BTreeSet<&str> = BTreeSet::new();
    for statement in statements {
        if let Stmt::FunctionDef { name, .. } = statement {
            defined.insert(name.name.as_str());
            continue;
        }
        let mut reached = BTreeSet::new();
        referenced_functions(std::slice::from_ref(statement), &names, &mut reached);
        let mut pending: Vec<String> = reached.iter().cloned().collect();
        while let Some(next) = pending.pop() {
            for callee in calls.get(next.as_str()).into_iter().flatten() {
                if reached.insert(callee.clone()) {
                    pending.push(callee.clone());
                }
            }
        }
        for name in reached
            .iter()
            .filter(|name| !defined.contains(name.as_str()))
        {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(306),
                    statement.span(),
                    format!("this statement can reach `{name}` before its `def` statement runs"),
                )
                .with_rule(RuleId::TYP_12)
                .with_help(format!(
                    "move the definition of `{name}` above this statement"
                )),
            );
        }
    }
}
