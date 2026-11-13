//! Module-level classes: their names, their bases, the attributes they declare or `__init__`
//! assigns, and their methods (CLS-01, CLS-02, CLS-03, CLS-13).

use std::collections::BTreeMap;

use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId, Span};
use typy_source::syntax::{Expr, Identifier, Stmt};

use super::annotation::{AnnotationScope, annotation_type};
use super::definition::{Definition, FunctionParts, Owner, defined_twice, definition};
use super::overriding::Override;
use crate::class_type::InstanceClass;
use crate::exception::{BuiltinException, ExceptionClass};
use crate::typed::{ClassId, FunctionId, FunctionSignature};
use crate::types::Type;

/// An attribute of a class: declared in the class body, or assigned in `__init__`, whose
/// check gives it its type.
#[derive(Clone, Debug)]
pub(crate) struct AttributeInfo {
    pub(crate) name: String,
    /// The type; unknown until `__init__` assigns an attribute the class body does not declare.
    pub(crate) ty: Option<Type>,
}

/// A class while the program is checked.
#[derive(Clone, Debug)]
pub(crate) struct ClassInfo {
    pub(crate) id: ClassId,
    pub(crate) name: String,
    /// The base when it is a class of the program, whose attributes come first.
    pub(crate) base: Option<ClassId>,
    /// How many of the attributes come from the base.
    pub(crate) inherited: usize,
    /// The class as an exception class, when it derives from one (CLS-03).
    pub(crate) exception: Option<ExceptionClass>,
    pub(crate) attributes: Vec<AttributeInfo>,
    pub(crate) methods: Vec<(String, FunctionId)>,
    pub(crate) init: Option<FunctionId>,
    /// The class variables the class body gives a value, each held as the module variable
    /// `Class.name` (CLS-11, ADR-0024).
    pub(crate) class_variables: Vec<String>,
    /// The positions of the class's own attributes the class body gives a default, each held
    /// as the module variable `Class.name`.
    pub(crate) defaults: Vec<usize>,
    /// Whether the class's `__init__` uses the instance other than through its attributes,
    /// once checked.
    pub(crate) init_uses_instance: bool,
    pub(crate) span: Span,
}

impl ClassInfo {
    /// Returns the position of an attribute.
    pub(crate) fn attribute(&self, name: &str) -> Option<usize> {
        self.attributes
            .iter()
            .position(|attribute| attribute.name == name)
    }

    /// Returns a method by its name.
    pub(crate) fn method(&self, name: &str) -> Option<FunctionId> {
        self.methods
            .iter()
            .find(|(each, _)| each == name)
            .map(|(_, id)| *id)
    }
}

/// A `class` statement, with its ID in source order.
pub(super) struct ClassStatement<'tree> {
    pub(super) id: ClassId,
    pub(super) name: &'tree Identifier,
    pub(super) base: Option<&'tree Expr>,
    pub(super) body: &'tree [Stmt],
    pub(super) span: Span,
}

/// Collects the module's classes in source order, numbered from `first`, the number of
/// classes of the modules checked before it.
pub(super) fn collect_classes<'tree>(
    statements: &'tree [Stmt],
    first: usize,
    diagnostics: &mut Diagnostics,
) -> Vec<ClassStatement<'tree>> {
    let mut classes: Vec<ClassStatement<'tree>> = Vec::new();
    for statement in statements {
        let Stmt::ClassDef {
            name,
            bases,
            body,
            span,
            ..
        } = statement
        else {
            continue;
        };
        if classes.iter().any(|class| class.name.name == name.name) {
            diagnostics.push(defined_twice(name));
            continue;
        }
        classes.push(ClassStatement {
            id: ClassId(u32::try_from(first + classes.len()).unwrap_or(u32::MAX)),
            name,
            base: bases.first(),
            body,
            span: *span,
        });
    }
    classes
}

/// The classes a base may name: those of the modules checked before, by ID, with the type of
/// their instances, and the names an annotation may use for them.
pub(super) struct BaseScope<'names> {
    pub(super) known: &'names BTreeMap<ClassId, Type>,
    pub(super) class_ids: &'names BTreeMap<String, ClassId>,
    pub(super) modules: &'names BTreeMap<String, BTreeMap<String, ClassId>>,
}

/// Resolves each class's base, in source order, and returns the type of each class's
/// instances: an exception for a class derived from a built-in exception class or from a
/// class of the program that is one (CLS-03), otherwise an instance whose chain holds the
/// classes of the program it derives from (CLS-04, ADR-0021). A base that cannot be resolved
/// is reported, and the class derives from `object`, so its uses are still checked.
pub(super) fn class_types(
    classes: &[ClassStatement<'_>],
    scope: &BaseScope<'_>,
    diagnostics: &mut Diagnostics,
) -> BTreeMap<ClassId, Type> {
    let mut found: BTreeMap<ClassId, Type> = BTreeMap::new();
    for class in classes {
        let name = class.name.name.clone();
        let ty = class
            .base
            .and_then(|base| derived_type(class, base, scope, &found, diagnostics))
            .unwrap_or_else(|| Type::instance(InstanceClass::root(class.id, name)));
        found.insert(class.id, ty);
    }
    found
}

/// Returns the type of the instances of a class with a base other than `object`, or `None`
/// for `object` and for a base that is reported.
fn derived_type(
    class: &ClassStatement<'_>,
    base: &Expr,
    scope: &BaseScope<'_>,
    found: &BTreeMap<ClassId, Type>,
    diagnostics: &mut Diagnostics,
) -> Option<Type> {
    let name = class.name.name.clone();
    let span = base.span();
    let (written, base_id) = match base {
        Expr::Name { id, .. } if id == "object" => return None,
        Expr::Name { id, .. } => {
            if let Some(builtin) = BuiltinException::named(id) {
                if !builtin.can_be_called() {
                    diagnostics.push(
                        Diagnostic::new(
                            DiagnosticCode::error(417),
                            span,
                            format!("a class cannot derive from `{id}` in TyPy"),
                        )
                        .with_rule(RuleId::EXC_12)
                        .with_help("derive from `ValueError` instead"),
                    );
                    return None;
                }
                return Some(Type::Exception(ExceptionClass::program(
                    class.id,
                    name,
                    Vec::new(),
                    builtin,
                )));
            }
            (id.clone(), scope.class_ids.get(id).copied())
        }
        Expr::Attribute {
            value, attribute, ..
        } => match value.as_ref() {
            Expr::Name { id, .. } => (
                format!("{id}.{}", attribute.name),
                scope
                    .modules
                    .get(id)
                    .and_then(|classes| classes.get(&attribute.name))
                    .copied(),
            ),
            _ => (String::new(), None),
        },
        _ => (String::new(), None),
    };
    let Some(base_id) = base_id else {
        diagnostics.push(unsupported_base(span));
        return None;
    };
    if base_id >= class.id {
        diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(306),
                span,
                format!("`{written}` is defined after the class that derives from it"),
            )
            .with_rule(RuleId::TYP_12),
        );
        return None;
    }
    match found.get(&base_id).or_else(|| scope.known.get(&base_id))? {
        Type::Exception(ExceptionClass::Class(parent)) => {
            let mut chain = vec![(base_id, parent.name.clone())];
            chain.extend(parent.ancestors.iter().cloned());
            Some(Type::Exception(ExceptionClass::program(
                class.id,
                name,
                chain,
                parent.builtin,
            )))
        }
        Type::Instance(parent) => {
            let mut ancestors = vec![(base_id, parent.name.clone())];
            ancestors.extend(parent.ancestors.iter().cloned());
            Some(Type::instance(InstanceClass {
                id: class.id,
                name,
                ancestors,
            }))
        }
        _ => None,
    }
}

/// Returns the error for a base other than `object`, an exception class or a class of the
/// program: a built-in type such as `list`, or an expression that names no class.
fn unsupported_base(span: Span) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::error(201),
        span,
        "a class derives from `object`, an exception class or a class of the program in TyPy \
         (CLS-04)",
    )
    .with_rule(RuleId::CLS_04)
}

/// The special methods a class may define: `__init__`, `__str__` and `__repr__`, and from 0.3
/// `__iter__` and `__next__` (CST-08) and `__enter__` and `__exit__` (CST-05). Any other name that starts and ends with two
/// underscores changes how Python treats the instances, and waits for the operator methods of
/// 0.4 (CLS-07).
const SPECIAL_METHODS: [&str; 7] = [
    "__init__",
    "__str__",
    "__repr__",
    "__iter__",
    "__next__",
    "__enter__",
    "__exit__",
];

/// The classes a module's class bodies may name: each by its name, the type of each one's
/// instances, and the classes of the modules checked before, indexed by ID.
pub(super) struct KnownClasses<'known> {
    pub(super) ids: &'known BTreeMap<String, ClassId>,
    pub(super) instances: &'known BTreeMap<ClassId, Type>,
    pub(super) earlier: &'known [ClassInfo],
    /// The signatures of the functions of the modules checked before, indexed by ID.
    pub(super) signatures: &'known [FunctionSignature],
}

/// Collects each class's attributes and methods; the methods become definitions whose IDs
/// follow `first_function`.
pub(super) fn collect_members<'tree>(
    classes: &[ClassStatement<'tree>],
    known: &KnownClasses<'_>,
    names: &crate::check::ImportedNames,
    floor_defers: bool,
    first_function: usize,
    diagnostics: &mut Diagnostics,
) -> (Vec<ClassInfo>, Vec<Definition<'tree>>) {
    let mut infos = Vec::new();
    let mut methods: Vec<Definition<'tree>> = Vec::new();
    for class in classes {
        // The class's own name is bound only after its body runs.
        let scope = AnnotationScope {
            classes: known.ids,
            instances: known.instances,
            modules: &names.modules,
            names: &names.names,
            standard: &names.standard,
            bound: if floor_defers { u32::MAX } else { class.id.0 },
        };
        let instance = known
            .instances
            .get(&class.id)
            .cloned()
            .unwrap_or_else(|| Type::instance(InstanceClass::root(class.id, &class.name.name)));
        let classes = Classes {
            earlier: known.earlier,
            local: &infos,
        };
        let mut info = new_info(class, &instance, classes);
        let owner = Owner {
            id: class.id,
            instance,
        };
        for statement in class.body {
            match statement {
                Stmt::AnnAssign { .. } | Stmt::Assign { .. } => {
                    class_body_name(statement, &mut info, &scope, names, diagnostics);
                }
                Stmt::FunctionDef { .. } => {
                    let Some(parts) = FunctionParts::of(statement) else {
                        continue;
                    };
                    let name = &parts.name.name;
                    if let Some(refused) = refused_method(&parts, &info) {
                        diagnostics.push(refused);
                        continue;
                    }
                    let id = FunctionId(
                        u32::try_from(first_function + methods.len()).unwrap_or(u32::MAX),
                    );
                    if name == "__init__" {
                        info.init = Some(id);
                        for attribute in initialized_attributes(&parts) {
                            if info.attribute(&attribute).is_none() {
                                info.attributes.push(AttributeInfo {
                                    name: attribute,
                                    ty: None,
                                });
                            }
                        }
                    }
                    info.methods.push((name.clone(), id));
                    let method = definition(&parts, Some(&owner), &scope, diagnostics);
                    let signatures = Signatures {
                        earlier: known.signatures,
                        local: &methods,
                        first_function,
                    };
                    if let Some(diagnostic) =
                        check_override(&info, classes, signatures, &method, parts.name)
                    {
                        diagnostics.push(diagnostic);
                    }
                    check_special_method(&method, parts.name, diagnostics);
                    methods.push(method);
                }
                _ => {}
            }
        }
        report_name_conflicts(&info, class.name.span, diagnostics);
        infos.push(info);
    }
    (infos, methods)
}

/// The classes collected so far: those of the modules checked before, indexed by ID, and the
/// module's own classes before the one being collected.
#[derive(Clone, Copy)]
struct Classes<'infos> {
    earlier: &'infos [ClassInfo],
    local: &'infos [ClassInfo],
}

impl<'infos> Classes<'infos> {
    /// Returns a class collected before; a base is defined before the class, so it is one.
    fn get(self, class: ClassId) -> Option<&'infos ClassInfo> {
        self.earlier
            .get(class.0 as usize)
            .or_else(|| self.local.iter().find(|info| info.id == class))
    }
}

/// Returns the class's information before its body is read: a class whose base is a class of
/// the program holds its base's attributes first, so each keeps its place (ADR-0021).
fn new_info(class: &ClassStatement<'_>, instance: &Type, classes: Classes<'_>) -> ClassInfo {
    let (base, exception) = match instance {
        Type::Exception(exception) => (
            match exception {
                ExceptionClass::Class(program) => program.ancestors.first().map(|(base, _)| *base),
                ExceptionClass::Builtin(_) => None,
            },
            Some(exception.clone()),
        ),
        Type::Instance(own) => (own.ancestors.first().map(|(base, _)| *base), None),
        _ => (None, None),
    };
    let attributes: Vec<AttributeInfo> = base
        .and_then(|base| classes.get(base))
        .map_or_else(Vec::new, |info| info.attributes.clone());
    ClassInfo {
        id: class.id,
        name: class.name.name.clone(),
        base,
        inherited: attributes.len(),
        exception,
        attributes,
        methods: Vec::new(),
        init: None,
        class_variables: Vec::new(),
        defaults: Vec::new(),
        init_uses_instance: false,
        span: class.span,
    }
}

/// Returns why a method cannot be defined, if it cannot: the class already defines it, it is
/// a special method planned with the operator methods, or it has no `self`.
fn refused_method(parts: &FunctionParts<'_>, info: &ClassInfo) -> Option<Diagnostic> {
    let name = &parts.name.name;
    if info.method(name).is_some() {
        return Some(defined_twice(parts.name));
    }
    if name.starts_with("__") && name.ends_with("__") && !SPECIAL_METHODS.contains(&name.as_str()) {
        return Some(planned_method(parts.name));
    }
    if parts.parameters.is_empty() {
        return Some(
            Diagnostic::new(
                DiagnosticCode::error(201),
                parts.name.span,
                "a method without `self` is a static method, which is planned for TyPy 0.4 \
                 (CLS-06)",
            )
            .with_rule(RuleId::CLS_06),
        );
    }
    None
}

/// The signatures of the functions collected so far: those of the modules checked before,
/// indexed by ID, and the module's methods, whose IDs follow `first_function`.
#[derive(Clone, Copy)]
struct Signatures<'definitions, 'tree> {
    earlier: &'definitions [FunctionSignature],
    local: &'definitions [Definition<'tree>],
    first_function: usize,
}

impl<'definitions> Signatures<'definitions, '_> {
    fn get(self, id: FunctionId) -> Option<&'definitions FunctionSignature> {
        (id.0 as usize)
            .checked_sub(self.first_function)
            .map_or_else(
                || self.earlier.get(id.0 as usize),
                |local| self.local.get(local).map(|base| &base.signature),
            )
    }
}

/// Returns why a method does not fit the base method it overrides, if it overrides one and
/// does not fit; a constructor runs on its own class only, so `__init__` is not compared.
fn check_override(
    info: &ClassInfo,
    classes: Classes<'_>,
    signatures: Signatures<'_, '_>,
    method: &Definition<'_>,
    name: &Identifier,
) -> Option<Diagnostic> {
    if name.name == "__init__" {
        return None;
    }
    let (base_id, base_class) = inherited_method(classes, info.base, &name.name)?;
    let base = signatures.get(base_id)?;
    Override {
        name: &format!("{}.{}", info.name, name.name),
        signature: &method.signature,
        base_name: &format!("{base_class}.{}", name.name),
        base,
        span: name.span,
    }
    .check()
}

/// Returns the method `name` of `base` or of the nearest class it derives from that defines
/// it, with that class's name.
fn inherited_method<'infos>(
    classes: Classes<'infos>,
    base: Option<ClassId>,
    name: &str,
) -> Option<(FunctionId, &'infos str)> {
    let mut current = base;
    while let Some(info) = current.and_then(|class| classes.get(class)) {
        if let Some(method) = info.method(name) {
            return Some((method, &info.name));
        }
        current = info.base;
    }
    None
}

/// Reports a name a class gives two meanings: a class variable that is also an attribute or a
/// method, or an attribute that is also a method.
fn report_name_conflicts(info: &ClassInfo, span: Span, diagnostics: &mut Diagnostics) {
    for name in &info.class_variables {
        if info.attribute(name).is_some() || info.method(name).is_some() {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(202),
                    span,
                    format!(
                        "`{}.{name}` is a class variable and also an attribute or a method",
                        info.name
                    ),
                )
                .with_rule(RuleId::STD_04),
            );
        }
    }
    for (name, _) in &info.methods {
        if info.attribute(name).is_some() {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(202),
                    span,
                    format!("`{}.{name}` is both an attribute and a method", info.name),
                )
                .with_rule(RuleId::STD_04),
            );
        }
    }
}

/// Declares what an annotation or an assignment in a class body names: an attribute, with
/// or without a default, or a class variable (CLS-01, CLS-11, ADR-0024).
fn class_body_name(
    statement: &Stmt,
    info: &mut ClassInfo,
    scope: &AnnotationScope<'_>,
    names: &crate::check::ImportedNames,
    diagnostics: &mut Diagnostics,
) {
    match statement {
        Stmt::AnnAssign {
            target: Expr::Name { id, span },
            annotation,
            value,
            ..
        } => {
            let class_var =
                super::annotation::class_var_type(annotation, &names.names, &names.standard)
                    .is_some();
            if class_var && value.is_some() {
                info.class_variables.push(id.clone());
                return;
            }
            let ty = annotation_type(annotation, scope, diagnostics);
            declare(info, id, Some(ty), *span, diagnostics);
            if value.is_some() {
                default_value(info, id, *span, diagnostics);
            }
        }
        Stmt::Assign { targets, .. } => {
            if let [Expr::Name { id, .. }] = targets.as_slice() {
                info.class_variables.push(id.clone());
            }
        }
        _ => {}
    }
}

/// Notes that the class body gives the attribute `name` a default. An exception is made by
/// the runtime's exception path, which copies no defaults, so an exception class's are
/// planned (ADR-0024).
fn default_value(info: &mut ClassInfo, name: &str, span: Span, diagnostics: &mut Diagnostics) {
    if info.exception.is_some() {
        diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(201),
                span,
                "a default for an attribute of an exception class is planned for TyPy 0.5 \
                 (CLS-11)",
            )
            .with_rule(RuleId::CLS_11)
            .with_help("assign the attribute in `__init__`"),
        );
        return;
    }
    // A subclass may give an inherited attribute a default of its own, which its instances
    // hold in place of the base's.
    if let Some(index) = info.attribute(name) {
        info.defaults.push(index);
    }
}

/// Declares an attribute in the class body; a second declaration is an error.
fn declare(
    info: &mut ClassInfo,
    name: &str,
    ty: Option<Type>,
    span: Span,
    diagnostics: &mut Diagnostics,
) {
    if let Some(index) = info.attribute(name).filter(|index| *index < info.inherited) {
        // A subclass may declare an attribute of its base again with the same type, so code
        // that knows only the base reads it at the same place with the same type (ADR-0021).
        let base = info.attributes[index].ty.clone();
        if base.as_ref() == ty.as_ref() {
            return;
        }
        let message = match base {
            Some(base) => format!(
                "`{}.{name}` is declared as {}, but its base declares it as {base}",
                info.name,
                ty.map_or_else(|| "something else".to_owned(), |ty| ty.to_string()),
            ),
            None => format!(
                "`{}.{name}` is declared again, but its base assigns it without declaring it",
                info.name
            ),
        };
        let diagnostic =
            Diagnostic::new(DiagnosticCode::error(202), span, message).with_rule(RuleId::CLS_04);
        diagnostics.push(if info.attributes[index].ty.is_none() {
            diagnostic.with_help(format!("declare `{name}` in the body of the base class"))
        } else {
            diagnostic
        });
        return;
    }
    if info.attribute(name).is_some() {
        diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(202),
                span,
                format!("`{}.{name}` is declared more than once", info.name),
            )
            .with_rule(RuleId::STD_04),
        );
        return;
    }
    info.attributes.push(AttributeInfo {
        name: name.to_owned(),
        ty,
    });
}

/// Checks the signature of a special method the runtime or a statement calls: `__str__`,
/// `__repr__`, `__exit__`, `__iter__`, `__next__` and `__enter__`.
fn check_special_method(method: &Definition<'_>, name: &Identifier, diagnostics: &mut Diagnostics) {
    match name.name.as_str() {
        "__str__" | "__repr__" => check_text_method(method, name, diagnostics),
        "__exit__" => check_exit(method, name, diagnostics),
        "__iter__" | "__next__" | "__enter__" if method.signature.parameters.len() != 1 => {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    name.span,
                    format!("`{}` takes only `self`", name.name),
                )
                .with_rule(RuleId::CST_08),
            );
        }
        _ => {}
    }
}

/// `__str__` and `__repr__` take only the instance and return a `str`, as CPython requires
/// (CLS-08).
fn check_text_method(method: &Definition<'_>, name: &Identifier, diagnostics: &mut Diagnostics) {
    let signature = &method.signature;
    if signature.parameters.len() != 1 || !signature.returns.matches(&Type::Str) {
        diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(404),
                name.span,
                format!("`{}` takes only `self` and returns a str", name.name),
            )
            .with_rule(RuleId::CLS_08),
        );
    }
}

/// `__exit__` takes `self` and the three parameters of CST-10, which accept the types CPython
/// passes, and returns `None`, a `bool` or `bool | None`, a true value suppressing the
/// exception (CST-05).
fn check_exit(method: &Definition<'_>, name: &Identifier, diagnostics: &mut Diagnostics) {
    let signature = &method.signature;
    let parameters_fit = signature.parameters.len() == 4
        && signature.parameters[1..]
            .iter()
            .enumerate()
            .all(|(index, parameter)| {
                super::definition::exit_parameter(index + 1).is_assignable_to(&parameter.ty)
            });
    let returns_fit = matches!(signature.returns, Type::NoneType | Type::Bool | Type::Error)
        || signature.returns == Type::optional(Type::Bool);
    if !parameters_fit || !returns_fit {
        diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(404),
                name.span,
                "`__exit__` takes `self`, the class of the exception, the exception and its \
                 traceback, and returns `None` or a `bool`",
            )
            .with_rule(RuleId::CST_05),
        );
    }
}

/// Reports a special method a class may not define yet.
fn planned_method(name: &Identifier) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::error(201),
        name.span,
        format!(
            "the special method `{}` is planned for TyPy 0.4 (CLS-07)",
            name.name
        ),
    )
    .with_rule(RuleId::CLS_07)
}

/// Returns the attributes `__init__` assigns on its first parameter, in source order, at any
/// depth of its blocks; an augmented assignment reads the attribute first, so it assigns none.
fn initialized_attributes(parts: &FunctionParts<'_>) -> Vec<String> {
    fn assigned(target: &Expr, instance: &str, found: &mut Vec<String>) {
        match target {
            Expr::Attribute {
                value, attribute, ..
            } if matches!(value.as_ref(), Expr::Name { id, .. } if id == instance) => {
                if !found.contains(&attribute.name) {
                    found.push(attribute.name.clone());
                }
            }
            Expr::Tuple { elements, .. } | Expr::List { elements, .. } => {
                for element in elements {
                    assigned(element, instance, found);
                }
            }
            _ => {}
        }
    }
    fn statements(body: &[Stmt], instance: &str, found: &mut Vec<String>) {
        for statement in body {
            match statement {
                Stmt::Assign { targets, .. } => {
                    for each in targets {
                        assigned(each, instance, found);
                    }
                }
                // An annotation without a value assigns nothing.
                Stmt::AnnAssign {
                    target: each,
                    value: Some(_),
                    ..
                } => assigned(each, instance, found),
                Stmt::For {
                    target: each,
                    body,
                    orelse,
                    ..
                } => {
                    assigned(each, instance, found);
                    statements(body, instance, found);
                    statements(orelse, instance, found);
                }
                Stmt::While { body, orelse, .. } | Stmt::If { body, orelse, .. } => {
                    statements(body, instance, found);
                    statements(orelse, instance, found);
                }
                Stmt::Try { .. } => {
                    for block in statement.try_blocks() {
                        statements(block, instance, found);
                    }
                }
                Stmt::With { body, .. } => statements(body, instance, found),
                _ => {}
            }
        }
    }
    let mut found = Vec::new();
    if let Some(instance) = parts.parameters.first() {
        statements(parts.body, &instance.name.name, &mut found);
    }
    found
}

/// A value a class body gives a name: a class variable, or the default of an instance
/// attribute, with the assignment that gives it, to the module variable `Class.name` that
/// holds it, which runs where the `class` statement runs (CLS-11, ADR-0024).
pub(crate) struct ClassValue {
    pub(crate) name: String,
    pub(crate) assignment: Stmt,
}

/// Returns the name of the module variable that holds a class's value `name`.
pub(crate) fn class_value_variable(class: &str, name: &str) -> String {
    format!("{class}.{name}")
}

/// Returns the values a class body gives, in source order: `name = value`, and
/// `name: ClassVar[T] = value`, for a class variable, and `name: T = value` for the default
/// of an instance attribute.
pub(crate) fn class_values(
    class: &str,
    body: &[Stmt],
    names: &BTreeMap<String, super::annotation::AnnotationName>,
    standard: &BTreeMap<String, String>,
) -> Vec<ClassValue> {
    let renamed = |id: &str, span: Span| Expr::Name {
        id: class_value_variable(class, id),
        span,
    };
    body.iter()
        .filter_map(|statement| match statement {
            Stmt::AnnAssign {
                target: Expr::Name { id, span },
                annotation,
                value: Some(value),
                simple,
                span: whole,
            } => {
                let target = renamed(id, *span);
                let assignment =
                    match super::annotation::class_var_type(annotation, names, standard) {
                        Some(super::annotation::ClassVarAnnotation::Typed(inner)) => {
                            Stmt::AnnAssign {
                                target,
                                annotation: inner.clone(),
                                value: Some(value.clone()),
                                simple: *simple,
                                span: *whole,
                            }
                        }
                        Some(super::annotation::ClassVarAnnotation::Bare) => Stmt::Assign {
                            targets: vec![target],
                            value: value.clone(),
                            span: *whole,
                        },
                        None => Stmt::AnnAssign {
                            target,
                            annotation: annotation.clone(),
                            value: Some(value.clone()),
                            simple: *simple,
                            span: *whole,
                        },
                    };
                Some(ClassValue {
                    name: id.clone(),
                    assignment,
                })
            }
            Stmt::Assign {
                targets,
                value,
                span,
            } => match targets.as_slice() {
                [Expr::Name { id, span: at }] => Some(ClassValue {
                    name: id.clone(),
                    assignment: Stmt::Assign {
                        targets: vec![renamed(id, *at)],
                        value: value.clone(),
                        span: *span,
                    },
                }),
                _ => None,
            },
            _ => None,
        })
        .collect()
}
