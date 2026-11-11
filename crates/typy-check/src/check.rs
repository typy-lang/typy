//! Checks a project and builds its typed program (detailed design section 7.2).

mod annotation;
mod class;

pub(crate) use annotation::{unhashable, unhashable_key};
mod definition;
mod definition_order;
mod module_imports;
mod module_scope;
mod visit;

use std::collections::{BTreeMap, BTreeSet};

use typy_diagnostics::{Diagnostics, SourceLookup};
use typy_source::syntax::Stmt;
use typy_source::{Module, ModuleName, Project};

pub(crate) use annotation::{
    ANNOTATION_MODULES, AnnotationName, AnnotationScope, annotation_name, annotation_type,
};

/// The names a module's imports give its annotations: each module's classes by the name
/// bound to it, and the names of `typing`, `collections.abc` and `types` (TYP-15).
#[derive(Clone, Debug, Default)]
pub(crate) struct ImportedNames {
    pub(crate) modules: BTreeMap<String, BTreeMap<String, ClassId>>,
    pub(crate) names: BTreeMap<String, AnnotationName>,
    pub(crate) standard: BTreeMap<String, String>,
}
pub(crate) use class::ClassInfo;
pub(crate) use definition::Definition;

use crate::body::{BodyChecker, Program};
use crate::subset;
use crate::typed::{ClassId, FunctionId, TypedProgram};
use class::{collect_classes, collect_members, exception_classes};
use definition::collect_functions;
use definition_order::check_calls_follow_definitions;
pub(crate) use module_scope::declared_globals;
use module_scope::{check_names_are_functions_or_variables, module_variables};

/// Resolves names, checks types and the subset rules, and builds the typed program.
#[derive(Clone, Copy, Debug, Default)]
pub struct TypeChecker;

impl TypeChecker {
    /// Returns the typed program, or every diagnostic found; a program with errors is never returned (TYP-11).
    pub fn check(project: &Project) -> Result<TypedProgram, Diagnostics> {
        let mut diagnostics = Diagnostics::new();
        for module in project.modules_in_order() {
            diagnostics.extend(subset::check(&module.syntax));
        }
        if diagnostics.has_errors() {
            return Err(diagnostics);
        }
        // From 3.14 CPython evaluates annotations only when they are read, so a signature may
        // name a class defined below it.
        let floor_defers = project.settings().python_floor.minor() >= 14;
        let mut program = Program::empty();
        let mut assigned = BTreeSet::new();
        for module in project.modules_in_order() {
            assigned.extend(module_imports::assigned_through_modules(
                &module.syntax.body,
            ));
        }
        program.set_project(
            project
                .modules()
                .map(|module| module.name.as_str().to_owned())
                .collect(),
            project
                .standard_modules()
                .map(|module| module.as_str().to_owned())
                .collect(),
            assigned,
        );
        // Each module is checked after the modules its module-level imports need.
        for module in project.modules_in_order() {
            let path = project.path(module.file).to_owned();
            check_module(&mut program, module, &path, floor_defers, &mut diagnostics);
        }
        if diagnostics.has_errors() {
            return Err(diagnostics);
        }
        Ok(program.finish(ModuleName::main().as_str()))
    }
}

/// Checks one module into the program, its functions and classes numbered after those of
/// the modules before it.
fn check_module(
    program: &mut Program,
    module: &Module,
    path: &str,
    floor_defers: bool,
    diagnostics: &mut Diagnostics,
) {
    let tree = &module.syntax;
    let first_class = program.class_count();
    let first_function = program.function_count();
    program.prepare_module(module);
    let imports = module_imports::module_imports(program, &tree.body);
    let annotations = ImportedNames {
        modules: program.module_classes_of(&imports.modules),
        names: imports.annotations.clone(),
        standard: imports.standard.clone(),
    };
    let classes = collect_classes(&tree.body, first_class, diagnostics);
    let mut class_ids: BTreeMap<String, ClassId> = imports.classes.clone();
    class_ids.extend(
        classes
            .iter()
            .map(|class| (class.name.name.clone(), class.id)),
    );
    let mut exceptions = program.known_exceptions();
    exceptions.extend(exception_classes(&classes, diagnostics));
    let mut definitions = collect_functions(
        &tree.body,
        first_class,
        &class_ids,
        &exceptions,
        &annotations,
        floor_defers,
        diagnostics,
    );
    let (class_infos, methods) = collect_members(
        &classes,
        &class_ids,
        &exceptions,
        &annotations,
        floor_defers,
        first_function + definitions.len(),
        diagnostics,
    );
    let mut variables = imports.variables.clone();
    module_variables(&tree.body, &mut variables);
    let module_variable_spans = variables.clone();
    check_names_are_functions_or_variables(&definitions, &class_ids, &variables, diagnostics);
    check_calls_follow_definitions(&tree.body, &definitions, &methods, diagnostics);
    definitions.extend(methods);

    program.begin_module(
        module,
        path,
        &definitions,
        variables.into_keys().collect(),
        class_infos,
        class_ids,
        exceptions,
    );
    program.import_names(&imports.functions, annotations);
    program.declare_module_variables(&module_variable_spans, &definitions);
    let module_body: Vec<&Stmt> = tree
        .body
        .iter()
        .filter(|statement| !matches!(statement, Stmt::FunctionDef { .. } | Stmt::ClassDef { .. }))
        .collect();
    // Each `__init__` is checked first, since it gives the attributes it assigns their
    // types; the module body comes next, since it gives the module variables theirs, and
    // the other functions last. An `__init__` reads a module variable with the type a
    // first look at the module body gives it, whose diagnostics are dropped and whose other
    // effects are undone, so the cost of a module does not grow with the modules before it.
    let snapshot = program.snapshot(first_class);
    BodyChecker::check_module(program, &module_body, &mut Diagnostics::new());
    program.keep_module_types(snapshot);
    let (initializers, others): (Vec<_>, Vec<_>) = definitions
        .iter()
        .enumerate()
        .partition(|(_, definition)| definition.class.is_some() && definition.name == "__init__");
    let function_id =
        |index: usize| FunctionId(u32::try_from(first_function + index).unwrap_or(u32::MAX));
    for (index, definition) in initializers {
        BodyChecker::check_function(program, function_id(index), definition, diagnostics);
    }
    program.end_first_look();
    let body = BodyChecker::check_module(program, &module_body, diagnostics);
    for (index, definition) in others {
        BodyChecker::check_function(program, function_id(index), definition, diagnostics);
    }
    program.end_module(body);
}
