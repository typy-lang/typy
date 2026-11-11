//! Checks a project and builds its typed program (detailed design section 7.2).

mod annotation;
mod class;

pub(crate) use annotation::{unhashable, unhashable_key};
mod definition;
mod definition_order;
mod module_scope;
mod visit;

use std::collections::BTreeMap;

use typy_diagnostics::Diagnostics;
use typy_source::Project;
use typy_source::syntax::Stmt;

pub(crate) use annotation::{AnnotationScope, annotation_type};
pub(crate) use class::ClassInfo;
pub(crate) use definition::Definition;

use crate::body::{BodyChecker, Program};
use crate::subset;
use crate::typed::{ClassId, FunctionId, TypedProgram};
use class::{collect_classes, collect_members, exception_classes};
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

        // From 3.14 CPython evaluates annotations only when they are read, so a signature may
        // name a class defined below it.
        let floor_defers = project.settings().python_floor.minor() >= 14;
        let classes = collect_classes(&tree.body, &mut diagnostics);
        let class_ids: BTreeMap<String, ClassId> = classes
            .iter()
            .map(|class| (class.name.name.clone(), class.id))
            .collect();
        let exceptions = exception_classes(&classes, &mut diagnostics);
        let mut definitions = collect_functions(
            &tree.body,
            &class_ids,
            &exceptions,
            floor_defers,
            &mut diagnostics,
        );
        let (class_infos, methods) = collect_members(
            &classes,
            &class_ids,
            &exceptions,
            floor_defers,
            definitions.len(),
            &mut diagnostics,
        );
        let mut variables = BTreeMap::new();
        module_variables(&tree.body, &mut variables);
        check_names_are_functions_or_variables(
            &definitions,
            &class_ids,
            &variables,
            &mut diagnostics,
        );
        check_calls_follow_definitions(&tree.body, &definitions, &methods, &mut diagnostics);
        definitions.extend(methods);

        let mut program = Program::new(
            module.name.as_str(),
            &definitions,
            variables.into_keys().collect(),
            class_infos,
            class_ids,
            exceptions,
        );
        // Each `__init__` is checked first, since it gives the attributes it assigns their types.
        let (initializers, others): (Vec<_>, Vec<_>) =
            definitions.iter().enumerate().partition(|(_, definition)| {
                definition.class.is_some() && definition.name == "__init__"
            });
        for (index, definition) in initializers.into_iter().chain(others) {
            let id = FunctionId(u32::try_from(index).unwrap_or(u32::MAX));
            BodyChecker::check_function(&mut program, id, definition, &mut diagnostics);
        }
        let module_body: Vec<&Stmt> = tree
            .body
            .iter()
            .filter(|statement| {
                !matches!(statement, Stmt::FunctionDef { .. } | Stmt::ClassDef { .. })
            })
            .collect();
        let entry = BodyChecker::check_module(&mut program, &module_body, &mut diagnostics);

        if diagnostics.has_errors() {
            return Err(diagnostics);
        }
        Ok(program.finish(entry))
    }
}
