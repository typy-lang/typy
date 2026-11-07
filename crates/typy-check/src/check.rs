//! Checks a project and builds its typed program (detailed design section 7.2).

mod annotation;
mod definition;
mod definition_order;
mod module_scope;
mod visit;

use std::collections::BTreeMap;

use typy_diagnostics::Diagnostics;
use typy_source::Project;
use typy_source::syntax::Stmt;

pub(crate) use annotation::annotation_type;
pub(crate) use definition::Definition;

use crate::body::{BodyChecker, Program};
use crate::subset;
use crate::typed::{FunctionId, TypedProgram};
use definition::collect_functions;
use definition_order::check_calls_follow_definitions;
use module_scope::{check_names_are_functions_or_variables, module_variables};

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
