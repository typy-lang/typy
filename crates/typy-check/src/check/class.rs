//! Module-level classes: their names, their bases, the attributes they declare or `__init__`
//! assigns, and their methods (CLS-01, CLS-02, CLS-03, CLS-13).

use std::collections::BTreeMap;

use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId, Span};
use typy_source::syntax::{Expr, Identifier, Stmt};

use super::annotation::{AnnotationScope, annotation_type};
use super::definition::{Definition, FunctionParts, Owner, defined_twice, definition};
use crate::class_type::InstanceClass;
use crate::exception::{BuiltinException, ExceptionClass};
use crate::typed::{ClassId, FunctionId};
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

/// The methods an exception class may define again over its base's (CLS-13).
const REDEFINABLE: [&str; 3] = ["__init__", "__str__", "__repr__"];

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
                Stmt::AnnAssign {
                    target: Expr::Name { id, span },
                    annotation,
                    ..
                } => {
                    let ty = annotation_type(annotation, &scope, diagnostics);
                    declare(&mut info, id, Some(ty), *span, diagnostics);
                }
                Stmt::FunctionDef { .. } => {
                    let Some(parts) = FunctionParts::of(statement) else {
                        continue;
                    };
                    let name = &parts.name.name;
                    if let Some(refused) = refused_method(&parts, &info, classes) {
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
                    if matches!(name.as_str(), "__str__" | "__repr__") {
                        check_text_method(&method, parts.name, diagnostics);
                    }
                    if name == "__exit__" {
                        check_exit(&method, parts.name, diagnostics);
                    }
                    if matches!(name.as_str(), "__iter__" | "__next__" | "__enter__")
                        && method.signature.parameters.len() != 1
                    {
                        diagnostics.push(
                            Diagnostic::new(
                                DiagnosticCode::error(404),
                                parts.name.span,
                                format!("`{name}` takes only `self`"),
                            )
                            .with_rule(RuleId::CST_08),
                        );
                    }
                    methods.push(method);
                }
                _ => {}
            }
        }
        for (name, _) in &info.methods {
            if info.attribute(name).is_some() {
                diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::error(202),
                        class.name.span,
                        format!("`{}.{name}` is both an attribute and a method", info.name),
                    )
                    .with_rule(RuleId::STD_04),
                );
            }
        }
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
        span: class.span,
    }
}

/// Returns why a method cannot be defined, if it cannot: the class already defines it, it
/// redefines a method of a base other than those of CLS-13, it is a special method planned
/// with the operator methods, or it has no `self`.
fn refused_method(
    parts: &FunctionParts<'_>,
    info: &ClassInfo,
    classes: Classes<'_>,
) -> Option<Diagnostic> {
    let name = &parts.name.name;
    if info.method(name).is_some() {
        return Some(defined_twice(parts.name));
    }
    if !REDEFINABLE.contains(&name.as_str()) && inherited_method(classes, info.base, name) {
        return Some(
            Diagnostic::new(
                DiagnosticCode::error(201),
                parts.name.span,
                format!(
                    "redefining the method `{name}` of a base is planned for TyPy 0.4 (CLS-04)"
                ),
            )
            .with_rule(RuleId::CLS_13),
        );
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

/// Returns true when `base`, or a class it derives from, defines the method `name`.
fn inherited_method(classes: Classes<'_>, base: Option<ClassId>, name: &str) -> bool {
    let mut current = base;
    while let Some(info) = current.and_then(|class| classes.get(class)) {
        if info.method(name).is_some() {
            return true;
        }
        current = info.base;
    }
    false
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
