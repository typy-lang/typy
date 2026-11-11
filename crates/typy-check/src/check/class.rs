//! Module-level classes: their names, their bases, the attributes they declare or `__init__`
//! assigns, and their methods (CLS-01, CLS-02, CLS-03, CLS-13).

use std::collections::BTreeMap;

use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId, Span};
use typy_source::syntax::{Expr, Identifier, Stmt};

use super::annotation::{AnnotationScope, annotation_type};
use super::definition::{Definition, FunctionParts, Owner, defined_twice, definition};
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

/// Collects the module's classes in source order.
pub(super) fn collect_classes<'tree>(
    statements: &'tree [Stmt],
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
            id: ClassId(u32::try_from(classes.len()).unwrap_or(u32::MAX)),
            name,
            base: bases.first(),
            body,
            span: *span,
        });
    }
    classes
}

/// Resolves each class's base, in source order: `object`, a built-in exception class, or a
/// class defined before it that is an exception class. Returns the exception classes, each
/// with its chain of bases (CLS-03); any other base is planned with inheritance (CLS-04).
pub(super) fn exception_classes(
    classes: &[ClassStatement<'_>],
    diagnostics: &mut Diagnostics,
) -> BTreeMap<ClassId, ExceptionClass> {
    let mut found: BTreeMap<ClassId, ExceptionClass> = BTreeMap::new();
    for class in classes {
        let Some(Expr::Name { id, span }) = class.base else {
            continue;
        };
        let name = class.name.name.clone();
        let exception = if id == "object" {
            continue;
        } else if let Some(builtin) = BuiltinException::named(id) {
            if !builtin.can_be_called() {
                diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::error(417),
                        *span,
                        format!("a class cannot derive from `{id}` in TyPy"),
                    )
                    .with_rule(RuleId::EXC_12)
                    .with_help("derive from `ValueError` instead"),
                );
                continue;
            }
            ExceptionClass::program(class.id, name, Vec::new(), builtin)
        } else if let Some(base) = classes.iter().find(|other| other.name.name == *id) {
            if base.id >= class.id {
                diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::error(306),
                        *span,
                        format!("`{id}` is defined after the class that derives from it"),
                    )
                    .with_rule(RuleId::TYP_12),
                );
                continue;
            }
            let Some(ExceptionClass::Class(parent)) = found.get(&base.id) else {
                diagnostics.push(base_planned(*span));
                continue;
            };
            let mut chain = vec![(base.id, id.clone())];
            chain.extend(parent.ancestors.iter().cloned());
            ExceptionClass::program(class.id, name, chain, parent.builtin)
        } else {
            diagnostics.push(base_planned(*span));
            continue;
        };
        found.insert(class.id, exception);
    }
    found
}

/// Returns the error for a base other than `object` or an exception class.
fn base_planned(span: Span) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::error(201),
        span,
        "base classes other than `object` and exception classes are planned for TyPy 0.4 (CLS-04)",
    )
    .with_rule(RuleId::CLS_04)
}

/// The methods an exception class may define again over its base's (CLS-13).
const REDEFINABLE: [&str; 3] = ["__init__", "__str__", "__repr__"];

/// The special methods a class may define: `__init__`, `__str__` and `__repr__`, and from 0.3
/// `__iter__` and `__next__` (CST-08). Any other name that starts and ends with two
/// underscores changes how Python treats the instances, and waits for the operator methods of
/// 0.4 (CLS-07).
const SPECIAL_METHODS: [&str; 5] = ["__init__", "__str__", "__repr__", "__iter__", "__next__"];

/// Collects each class's attributes and methods; the methods become definitions whose IDs
/// follow `first_function`.
pub(super) fn collect_members<'tree>(
    classes: &[ClassStatement<'tree>],
    class_ids: &BTreeMap<String, ClassId>,
    exceptions: &BTreeMap<ClassId, ExceptionClass>,
    floor_defers: bool,
    first_function: usize,
    diagnostics: &mut Diagnostics,
) -> (Vec<ClassInfo>, Vec<Definition<'tree>>) {
    let mut infos = Vec::new();
    let mut methods: Vec<Definition<'tree>> = Vec::new();
    for class in classes {
        // The class's own name is bound only after its body runs.
        let scope = AnnotationScope {
            classes: class_ids,
            exceptions,
            bound: if floor_defers { u32::MAX } else { class.id.0 },
        };
        let mut info = new_info(class, exceptions.get(&class.id).cloned(), &infos);
        let owner = Owner {
            id: class.id,
            instance: info.exception.clone().map_or_else(
                || Type::Instance(class.id, class.name.name.clone()),
                Type::Exception,
            ),
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
                    if let Some(refused) = refused_method(&parts, &info, &infos) {
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
                    if matches!(name.as_str(), "__iter__" | "__next__")
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

/// Returns the class's information before its body is read: an exception class whose base is
/// a class of the program holds its base's attributes first, so each keeps its place.
fn new_info(
    class: &ClassStatement<'_>,
    exception: Option<ExceptionClass>,
    infos: &[ClassInfo],
) -> ClassInfo {
    let base = match &exception {
        Some(ExceptionClass::Class(program)) => program.ancestors.first().map(|(base, _)| *base),
        _ => None,
    };
    let attributes: Vec<AttributeInfo> = base
        .and_then(|base| infos.iter().find(|info| info.id == base))
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
    infos: &[ClassInfo],
) -> Option<Diagnostic> {
    let name = &parts.name.name;
    if info.method(name).is_some() {
        return Some(defined_twice(parts.name));
    }
    if !REDEFINABLE.contains(&name.as_str()) && inherited_method(infos, info.base, name) {
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
fn inherited_method(infos: &[ClassInfo], base: Option<ClassId>, name: &str) -> bool {
    let mut current = base;
    while let Some(class) = current {
        // A base is defined before the class, so its information is already collected.
        let info = &infos[class.0 as usize];
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
