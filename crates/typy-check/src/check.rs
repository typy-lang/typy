//! Checks a project and builds its typed program (detailed design section 7.2).

mod annotation;
mod class;
mod dataclass;

pub(crate) use annotation::{unhashable, unhashable_key};
mod definition;
mod definition_order;
mod generic_body;
mod module_imports;
mod module_scope;
mod overriding;
mod type_alias;
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
    /// For each name bound to a module of the program, that module's type aliases (SMT-10).
    pub(crate) module_aliases: BTreeMap<String, BTreeSet<String>>,
    pub(crate) names: BTreeMap<String, AnnotationName>,
    pub(crate) standard: BTreeMap<String, String>,
}
pub(crate) use annotation::{ClassVarAnnotation, final_type, other_module_alias};
pub(crate) use class::{ClassInfo, class_value_variable, class_values};
pub(crate) use definition::Definition;

use crate::body::{BodyChecker, Program};
use crate::subset;
use crate::typed::{ClassId, FunctionId, TypedProgram};
use class::{BaseScope, KnownClasses, class_types, collect_classes, collect_members};
use definition::collect_functions;
use definition_order::check_calls_follow_definitions;
pub(crate) use module_scope::declared_globals;
use module_scope::{check_module_names, module_variables};

impl ImportedNames {
    /// Returns the names the module's imports give its annotations.
    fn of(program: &Program, imports: &module_imports::ModuleImports) -> Self {
        Self {
            modules: program.module_classes_of(&imports.modules),
            module_aliases: program.module_aliases_of(&imports.modules),
            names: imports.annotations.clone(),
            standard: imports.standard.clone(),
        }
    }
}

/// Resolves names, checks types and the subset rules, and builds the typed program.
#[derive(Clone, Copy, Debug, Default)]
pub struct TypeChecker;

impl TypeChecker {
    /// Returns the typed program with the warnings of checking it, or every diagnostic found; a
    /// program with errors is never returned (TYP-11).
    pub fn check(project: &Project) -> Result<TypedProgram, Diagnostics> {
        let mut diagnostics = Diagnostics::new();
        // A dataclass gets its methods, and each annotation the types its aliases stand for,
        // before anything reads the module (CLS-05, SMT-10).
        let mut dataclasses = dataclass::Dataclasses::default();
        let modules: Vec<Module> = project
            .modules_in_order()
            .map(|module| {
                let mut module = dataclasses.expand(module, &mut diagnostics);
                type_alias::expand(&mut module.syntax.body, &mut diagnostics);
                module
            })
            .collect();
        for module in &modules {
            diagnostics.extend(subset::check(&module.syntax));
        }
        if diagnostics.has_errors() {
            return Err(diagnostics);
        }
        // From 3.14 CPython evaluates annotations only when they are read, so a signature may
        // name a class defined below it.
        let floor_defers = project.settings().python_floor.minor() >= 14;
        let mut program = Program::empty();
        program.set_floor(project.settings().python_floor.minor());
        let mut assigned = BTreeSet::new();
        for module in &modules {
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
        for module in &modules {
            let path = project.path(module.file).to_owned();
            let frozen = dataclasses
                .info
                .get(module.name.as_str())
                .cloned()
                .unwrap_or_default();
            check_module(
                &mut program,
                module,
                &path,
                floor_defers,
                &frozen,
                &mut diagnostics,
            );
        }
        if diagnostics.has_errors() {
            return Err(diagnostics);
        }
        let mut program = program.finish(ModuleName::main().as_str());
        program.warnings = diagnostics;
        Ok(program)
    }
}

/// Checks one module into the program, its functions and classes numbered after those of
/// the modules before it.
fn check_module(
    program: &mut Program,
    module: &Module,
    path: &str,
    floor_defers: bool,
    dataclasses: &BTreeMap<String, dataclass::DataclassInfo>,
    diagnostics: &mut Diagnostics,
) {
    let tree = &module.syntax;
    let first_class = program.class_count();
    let first_function = program.function_count();
    program.prepare_module(module);
    let imports = module_imports::module_imports(program, &tree.body);
    let annotations = ImportedNames::of(program, &imports);
    let classes = collect_classes(&tree.body, first_class, diagnostics);
    let mut class_ids: BTreeMap<String, ClassId> = imports.classes.clone();
    class_ids.extend(
        classes
            .iter()
            .map(|class| (class.name.name.clone(), class.id)),
    );
    let mut instances = class_types(
        &classes,
        &BaseScope {
            known: program.known_instances(),
            class_ids: &class_ids,
            modules: &annotations.modules,
        },
        diagnostics,
    );
    instances.extend(
        program
            .known_instances()
            .iter()
            .map(|(class, ty)| (*class, ty.clone())),
    );
    let mut definitions = collect_functions(
        &tree.body,
        first_class,
        &class_ids,
        &instances,
        &annotations,
        floor_defers,
        diagnostics,
    );
    let (class_infos, methods) = collect_members(
        &classes,
        &KnownClasses {
            ids: &class_ids,
            instances: &instances,
            earlier: program.classes(),
            signatures: program.signatures(),
        },
        &annotations,
        floor_defers,
        first_function + definitions.len(),
        diagnostics,
    );
    let mut class_infos = class_infos;
    mark_dataclasses(&mut class_infos, dataclasses);
    let mut variables = imports.variables.clone();
    module_variables(&tree.body, &mut variables);
    class_value_variables(&classes, &annotations, &mut variables);
    let module_variable_spans = variables.clone();
    check_module_names(
        &tree.body,
        &definitions,
        &class_ids,
        &variables,
        diagnostics,
    );
    check_calls_follow_definitions(&tree.body, &definitions, &methods, diagnostics);
    definitions.extend(methods);

    program.begin_module(
        module,
        path,
        &definitions,
        variables.into_keys().collect(),
        class_infos,
        class_ids,
        instances,
    );
    program.import_names(&imports.functions, annotations);
    program.import_module_functions(&imports.module_functions);
    program.declare_module_variables(&module_variable_spans, &definitions);
    let module_body: Vec<&Stmt> = tree
        .body
        .iter()
        .filter(|statement| !matches!(statement, Stmt::FunctionDef { .. }))
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
    check_bodies(program, &others, &definitions, first_function, diagnostics);
    program.end_module(body);
}

/// Checks the bodies of the module's functions after the module body, then the copies of
/// its generic functions that the calls need (GEN-03).
fn check_bodies(
    program: &mut Program,
    others: &[(usize, &Definition<'_>)],
    definitions: &[Definition<'_>],
    first_function: usize,
    diagnostics: &mut Diagnostics,
) {
    for (index, definition) in others {
        let id = FunctionId(u32::try_from(first_function + index).unwrap_or(u32::MAX));
        check_body(program, id, definition, diagnostics);
    }
    check_copies(program, definitions, first_function, diagnostics);
}

/// Adds the module variables that hold the values the module's class bodies give, each named
/// after its class (ADR-0024).
fn class_value_variables(
    classes: &[class::ClassStatement<'_>],
    annotations: &ImportedNames,
    variables: &mut BTreeMap<String, typy_diagnostics::Span>,
) {
    for class in classes {
        for value in class::class_values(
            &class.name.name,
            class.body,
            &annotations.names,
            &annotations.standard,
        ) {
            let name = class::class_value_variable(&class.name.name, &value.name);
            variables.entry(name).or_insert(value.assignment.span());
        }
    }
}

/// Checks the body of a function. A copy of an inherited class method reports only what its
/// own class causes, such as arguments its `__init__` does not take (ADR-0025).
fn check_body(
    program: &mut Program,
    id: FunctionId,
    definition: &Definition<'_>,
    diagnostics: &mut Diagnostics,
) {
    if !definition.type_parameters.is_empty() {
        generic_body::check(program, id, definition, diagnostics);
        return;
    }
    if definition.copy_of.is_none() {
        BodyChecker::check_function(program, id, definition, diagnostics);
        return;
    }
    let mut found = Diagnostics::new();
    BodyChecker::check_function(program, id, definition, &mut found);
    for diagnostic in found.iter() {
        if !diagnostics.iter().any(|reported| reported == diagnostic) {
            diagnostics.push(diagnostic.clone());
        }
    }
}

/// Checks each copy of a generic function of the module that a call needs, as an ordinary
/// function whose type parameters stand for its arguments; a copy may need further copies
/// (GEN-03, ADR-0023).
fn check_copies(
    program: &mut Program,
    definitions: &[Definition<'_>],
    first_function: usize,
    diagnostics: &mut Diagnostics,
) {
    while let Some(copy) = program.next_pending_copy() {
        // A copy is made only for a generic of the module being checked (GEN-07).
        let generic = &definitions[copy.generic.0 as usize - first_function];
        let given: Vec<String> = copy.arguments.iter().map(ToString::to_string).collect();
        let made = Definition {
            qualified_name: format!("{}[{}]", generic.qualified_name, given.join(", ")),
            copy_of: Some(copy.generic),
            signature: program.instantiated(copy.generic, &copy.arguments),
            type_parameters: Vec::new(),
            ..generic.clone()
        };
        program.type_arguments = generic
            .type_parameters
            .iter()
            .zip(&copy.arguments)
            .map(|(parameter, argument)| (parameter.name.clone(), argument.clone()))
            .collect();
        check_body(program, copy.copy, &made, diagnostics);
        program.type_arguments.clear();
    }
    program.end_copies();
}

/// Notes in each frozen dataclass's information what its expansion wrote (CLS-05).
fn mark_dataclasses(
    infos: &mut [ClassInfo],
    dataclasses: &BTreeMap<String, dataclass::DataclassInfo>,
) {
    for info in infos {
        if let Some(dataclass) = dataclasses.get(&info.name) {
            info.frozen = dataclass.frozen;
            info.generated_init = dataclass.generated_init;
        }
    }
}
