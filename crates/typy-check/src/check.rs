//! Checks a project and builds its typed program (detailed design section 7.2).

mod annotation;
mod class;
mod dataclass;

pub(crate) use annotation::{unhashable, unhashable_key};
mod definition;
mod definition_order;
mod generic_body;
mod generic_class;
#[cfg(test)]
mod generic_tests;
mod module_imports;
mod module_scope;
mod overriding;
mod protocol;
pub(crate) mod type_alias;
mod visit;
pub(crate) use visit::visit_statement;

use std::collections::{BTreeMap, BTreeSet};

use typy_diagnostics::{Diagnostics, SourceLookup};
use typy_source::syntax::Stmt;
use typy_source::{Module, ModuleName, Project};

pub(crate) use annotation::{
    ANNOTATION_MODULES, AnnotationName, AnnotationScope, annotation_name, annotation_type,
};

/// The names a module's imports give its annotations: each module's classes by the name
/// bound to it, and the names of `typing`, `collections.abc` and `types` (TYP-15).
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct ImportedNames {
    pub(crate) modules: BTreeMap<String, BTreeMap<String, ClassId>>,
    /// For each name bound to a module of the program, that module's type aliases (SMT-10).
    pub(crate) module_aliases: BTreeMap<String, AliasTable>,
    /// The type aliases of other modules imported by name (SMT-12).
    pub(crate) aliases: AliasTable,
    pub(crate) names: BTreeMap<String, AnnotationName>,
    pub(crate) standard: BTreeMap<String, String>,
}
pub(crate) use annotation::{AliasTable, ClassVarAnnotation, ExportedAlias, final_type};
pub(crate) use class::{ClassInfo, class_value_variable, class_values};
pub(crate) use definition::{Definition, factory_call};
pub(crate) use protocol::outside_signature;

use crate::body::{BodyChecker, Program};
use crate::exception_arguments::Creations;
use crate::subset;
use crate::typed::{ClassId, FunctionId, TypedProgram};
use crate::types::Type;
use class::{BaseScope, KnownClasses, class_types, collect_classes, collect_members};
use definition::{FunctionScope, collect_functions};
use definition_order::check_calls_follow_definitions;
pub(crate) use module_scope::declared_globals;
use module_scope::{check_module_names, module_variables};

impl ImportedNames {
    /// Returns the names the module's imports give its annotations.
    fn of(program: &Program, imports: &module_imports::ModuleImports) -> Self {
        Self {
            modules: program.module_classes_of(&imports.modules),
            module_aliases: program.module_aliases_of(&imports.modules),
            aliases: imports.aliases.clone(),
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
        let expanded: Vec<(Module, BTreeMap<String, type_alias::Alias>)> = project
            .modules_in_order()
            .map(|module| {
                let mut module = dataclasses.expand(module, &mut diagnostics);
                let aliases = type_alias::expand(&mut module.syntax.body, &mut diagnostics);
                (module, aliases)
            })
            .collect();
        let (modules, aliases): (Vec<Module>, Vec<_>) = expanded.into_iter().unzip();
        for module in &modules {
            diagnostics.extend(subset::check(&module.syntax));
        }
        if diagnostics.has_errors() {
            return Err(diagnostics);
        }
        // From 3.14 CPython evaluates annotations only when they are read, so a signature may
        // name a class defined below it.
        let floor_defers = project.settings().python_floor.minor() >= 14;
        let checked = Checked {
            project,
            modules: &modules,
            aliases: &aliases,
            dataclasses: &dataclasses,
            floor_defers,
        };
        // A program that reads `args` as a value is checked again with the creations of
        // exceptions the check before found, until a check finds no new one (ADR-0034).
        let mut known: Option<Creations> = None;
        let mut checks = 1;
        let (program, diagnostics) = loop {
            let mut found = diagnostics.clone();
            let program = checked.run(known.as_ref(), &mut found);
            let Some(read) = program.first_arguments_read() else {
                break (program, found);
            };
            // An error found before the creations settle may come from a type of `args` that
            // lacked one, so only a settled check's errors are reported.
            let recorded = program.creations();
            if known.as_ref().is_some_and(|known| known.covers(&recorded)) {
                break (program, found);
            }
            if checks == MAXIMUM_CHECKS {
                if !found.has_errors() {
                    found.push(unsettled_arguments(read));
                }
                break (program, found);
            }
            match &mut known {
                Some(known) => known.extend(&recorded),
                None => known = Some(recorded),
            }
            checks += 1;
        };
        if diagnostics.has_errors() {
            return Err(diagnostics);
        }
        let mut program = program.finish(ModuleName::main().as_str());
        program.warnings = diagnostics;
        Ok(program)
    }
}

/// The checks a program that reads `args` as a value may take before the creations of its
/// exceptions settle; one more would only find an `args` held in the arguments of an
/// exception of a related class, whose type grows with each check (EXC-17).
const MAXIMUM_CHECKS: u32 = 8;

/// Returns the error for a program whose checks keep finding new types of arguments, read
/// as `args` at `read`.
fn unsettled_arguments(read: typy_diagnostics::Span) -> typy_diagnostics::Diagnostic {
    use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId};
    Diagnostic::new(
        DiagnosticCode::error(201),
        read,
        "`args` of an exception whose arguments hold the `args` of an exception of a \
         related class, a type without end, is planned for TyPy 1.0 (EXC-17)",
    )
    .with_rule(RuleId::EXC_17)
    .with_help("pass the items of `args` to the new exception, not `args` itself")
}

/// What each check of the modules reads: the modules in the order they are checked, with
/// their type aliases and dataclasses.
struct Checked<'project> {
    project: &'project Project,
    modules: &'project [Module],
    aliases: &'project [BTreeMap<String, type_alias::Alias>],
    dataclasses: &'project dataclass::Dataclasses,
    floor_defers: bool,
}

impl Checked<'_> {
    /// Checks every module into a new program, with `args` typed by the creations of
    /// exceptions `known` records, or as errors in a first check, which knows none.
    fn run(&self, known: Option<&Creations>, diagnostics: &mut Diagnostics) -> Program {
        let project = self.project;
        let mut program = Program::empty();
        program.set_floor(project.settings().python_floor.minor());
        program.set_known_creations(known.cloned());
        let mut assigned = BTreeSet::new();
        for module in self.modules {
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
        let mut deferred = Vec::new();
        let mut definitions = ModuleDefinitions::new();
        let mut copies = class::ClassMethodCopies::default();
        for (module, aliases) in self.modules.iter().zip(self.aliases) {
            let path = project.path(module.file).to_owned();
            let frozen = self
                .dataclasses
                .info
                .get(module.name.as_str())
                .cloned()
                .unwrap_or_default();
            copies.begin_module(module.name.as_str());
            let (waiting, checked) = check_module(
                &mut program,
                (module, aliases, &mut copies),
                &path,
                self.floor_defers,
                &frozen,
                diagnostics,
            );
            deferred.extend(waiting);
            definitions.insert(module.name.as_str().to_owned(), checked);
        }
        for bodies in deferred {
            bodies.check(&mut program, &definitions, diagnostics);
        }
        program
    }
}

/// Bodies checked once every module is, in the names of `module`: the functions of a module
/// that import a module checked after it (MOD-13), and the copies of a class method of
/// `module` that classes of other modules inherit (ADR-0037).
struct Deferred<'tree> {
    module: String,
    bodies: Vec<(FunctionId, Definition<'tree>)>,
}

/// The definitions of each module checked, with the ID of the first, which the copies of a
/// module's generic functions are made from (GEN-03).
type ModuleDefinitions<'tree> = BTreeMap<String, (Vec<Definition<'tree>>, usize)>;

impl Deferred<'_> {
    /// Checks the bodies, then the copies of the module's generic functions they need.
    fn check(
        self,
        program: &mut Program,
        definitions: &ModuleDefinitions<'_>,
        diagnostics: &mut Diagnostics,
    ) {
        let Some(context) = program.context_of(&self.module) else {
            return;
        };
        let current = program.swap_module_context(context);
        for (id, definition) in &self.bodies {
            check_body(program, *id, definition, diagnostics);
        }
        if let Some((definitions, first_function)) = definitions.get(&self.module) {
            check_copies(program, definitions, *first_function, diagnostics);
        }
        program.swap_module_context(current);
    }
}

/// Returns the classes a module's annotations and calls name: those it imports by name, then
/// its own.
fn class_table(
    imports: &module_imports::ModuleImports,
    classes: &[class::ClassStatement<'_>],
) -> BTreeMap<String, ClassId> {
    let mut table = imports.classes.clone();
    table.extend(
        classes
            .iter()
            .map(|class| (class.name.name.clone(), class.id)),
    );
    table
}

/// Reports the names imports bind that clash with another binding, and a module-level
/// statement that can reach a function or a class before its `def`, `class` or import runs.
fn check_bindings(
    body: &[Stmt],
    imports: &module_imports::ModuleImports,
    (classes, definitions, methods): (
        &[class::ClassStatement<'_>],
        &[Definition<'_>],
        &[Definition<'_>],
    ),
    variables: &BTreeMap<String, typy_diagnostics::Span>,
    diagnostics: &mut Diagnostics,
) {
    let defined: Vec<String> = classes
        .iter()
        .map(|class| class.name.name.clone())
        .chain(definitions.iter().map(|definition| definition.name.clone()))
        .collect();
    imports.report_clashes(&defined, variables, diagnostics);
    check_calls_follow_definitions(
        body,
        (definitions, methods),
        &imports.bound_at_top(),
        diagnostics,
    );
}

/// Checks one module into the program, its functions and classes numbered after those of
/// the modules before it.
fn check_module<'tree>(
    program: &mut Program,
    (module, aliases, copies): (
        &'tree Module,
        &BTreeMap<String, type_alias::Alias>,
        &mut class::ClassMethodCopies<'tree>,
    ),
    path: &str,
    floor_defers: bool,
    dataclasses: &BTreeMap<String, dataclass::DataclassInfo>,
    diagnostics: &mut Diagnostics,
) -> (Vec<Deferred<'tree>>, (Vec<Definition<'tree>>, usize)) {
    let tree = &module.syntax;
    let first_class = program.class_count();
    let first_function = program.function_count();
    program.prepare_module(module);
    let mut imports = module_imports::module_imports(program, &tree.body);
    imports.add_binding_variables();
    let annotations = ImportedNames::of(program, &imports);
    let classes = collect_classes(&tree.body, first_class, diagnostics);
    let class_ids = class_table(&imports, &classes);
    program.export_aliases(aliases, &class_ids, &annotations);
    let instances = module_instances(program, &classes, &class_ids, &annotations, diagnostics);
    let known_scope = (&class_ids, &instances, &annotations);
    note_module_generics(program, &classes, first_class, known_scope, diagnostics);
    let mut definitions = collect_functions(
        &tree.body,
        first_class,
        &FunctionScope {
            classes: &class_ids,
            instances: &instances,
            names: &annotations,
            copies: &program.class_copies,
        },
        floor_defers,
        diagnostics,
    );
    let (mut class_infos, methods) = collect_members(
        &classes,
        &KnownClasses {
            ids: &class_ids,
            instances: &instances,
            earlier: program.classes(),
            signatures: program.signatures(),
            copies: &program.class_copies,
        },
        &annotations,
        floor_defers,
        (first_function + definitions.len(), copies),
        diagnostics,
    );
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
    let defined = (
        classes.as_slice(),
        definitions.as_slice(),
        methods.as_slice(),
    );
    check_bindings(&tree.body, &imports, defined, &variables, diagnostics);
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
    program.make_class_copies();
    program.import_names(&imports.functions, annotations);
    program.import_module_functions(&imports.module_functions);
    program.declare_module_variables(&module_variable_spans, &definitions);
    program.set_rebound(imports.rebound());
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
    // The `__init__` of a generic class is checked once, with its type parameters opaque,
    // since the types it gives the attributes hold for every copy.
    for (index, definition) in initializers {
        program.type_arguments = definition
            .type_parameters
            .iter()
            .map(|parameter| (parameter.name.clone(), Type::parameter(parameter.clone())))
            .collect();
        BodyChecker::check_function(program, function_id(index), definition, diagnostics);
        program.type_arguments.clear();
    }
    program.end_first_look();
    let body = BodyChecker::check_module(program, &module_body, diagnostics);
    let deferred = check_bodies(program, others, &definitions, first_function, diagnostics);
    program.end_module(body);
    (deferred, (definitions, first_function))
}

/// Checks the bodies of the module's functions after the module body, then the copies of
/// its generic functions that the calls need (GEN-03). A function that imports a module
/// checked after this one waits for it (MOD-13), and a copy of a class method of another
/// module waits to be checked in that module's names (ADR-0037).
fn check_bodies<'tree>(
    program: &mut Program,
    others: Vec<(usize, &Definition<'tree>)>,
    definitions: &[Definition<'tree>],
    first_function: usize,
    diagnostics: &mut Diagnostics,
) -> Vec<Deferred<'tree>> {
    let id = |index: usize| FunctionId(u32::try_from(first_function + index).unwrap_or(u32::MAX));
    let mut waiting: BTreeMap<String, Vec<(FunctionId, Definition<'tree>)>> = BTreeMap::new();
    let own = program.module_name().to_owned();
    for (index, definition) in others {
        let module = match &definition.home {
            Some(home) => home.clone(),
            None if program.imports_later_modules(definition.body) => own.clone(),
            None => {
                check_body(program, id(index), definition, diagnostics);
                continue;
            }
        };
        waiting
            .entry(module)
            .or_default()
            .push((id(index), definition.clone()));
    }
    check_copies(program, definitions, first_function, diagnostics);
    waiting
        .into_iter()
        .map(|(module, bodies)| Deferred { module, bodies })
        .collect()
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

/// Returns the type of the instances of each class: the module's, then those of the modules
/// checked before it.
fn module_instances(
    program: &Program,
    classes: &[class::ClassStatement<'_>],
    class_ids: &BTreeMap<String, ClassId>,
    annotations: &ImportedNames,
    diagnostics: &mut Diagnostics,
) -> BTreeMap<ClassId, crate::types::Type> {
    let mut instances = class_types(
        classes,
        &BaseScope {
            known: program.known_instances(),
            class_ids,
            modules: &annotations.modules,
            names: &annotations.names,
            standard: &annotations.standard,
        },
        diagnostics,
    );
    instances.extend(
        program
            .known_instances()
            .iter()
            .map(|(class, ty)| (*class, ty.clone())),
    );
    instances
}

/// Notes the module's generic classes, whose copies an annotation of the module may ask for;
/// a copy's class ID follows the module's classes (ADR-0028).
fn note_module_generics(
    program: &Program,
    classes: &[class::ClassStatement<'_>],
    first_class: usize,
    (class_ids, instances, annotations): (
        &BTreeMap<String, ClassId>,
        &BTreeMap<ClassId, crate::types::Type>,
        &ImportedNames,
    ),
    diagnostics: &mut Diagnostics,
) {
    {
        let mut copies = program.class_copies.borrow_mut();
        copies.next = copies.next.max(first_class + classes.len());
    }
    generic_class::note_generic_classes(
        classes,
        &annotation::AnnotationScope {
            classes: class_ids,
            instances,
            modules: &annotations.modules,
            module_aliases: &annotations.module_aliases,
            imported_aliases: &annotations.aliases,
            names: &annotations.names,
            standard: &annotations.standard,
            parameters: &annotation::NO_PARAMETERS,
            copies: &program.class_copies,
            bound: u32::MAX,
        },
        &program.class_copies,
        diagnostics,
    );
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
        // A copy of a method belongs to its copy of the class, whose name it takes.
        let qualified_name = match copy.class {
            Some(class) => format!(
                "{}[{}].{}",
                program.class_name(class),
                given.join(", "),
                generic.name
            ),
            None => format!("{}[{}]", generic.qualified_name, given.join(", ")),
        };
        let made = Definition {
            qualified_name,
            class: copy.class.or(generic.class),
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

/// Notes in each dataclass's information what its expansion wrote (CLS-05, CST-06).
fn mark_dataclasses(
    infos: &mut [ClassInfo],
    dataclasses: &BTreeMap<String, dataclass::DataclassInfo>,
) {
    for info in infos {
        if let Some(dataclass) = dataclasses.get(&info.name) {
            info.frozen = dataclass.frozen;
            info.generated_init = dataclass.generated_init;
            if info.match_args.is_none() {
                info.match_args = dataclass.match_args.clone().map(Ok);
            }
        }
    }
}
