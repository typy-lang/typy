//! Module-level classes: their names, the attributes they declare or `__init__` assigns, and
//! their methods (CLS-01, CLS-02).

use std::collections::BTreeMap;

use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId, Span};
use typy_source::syntax::{Expr, Identifier, Stmt};

use super::annotation::{AnnotationScope, annotation_type};
use super::definition::{Definition, FunctionParts, Owner, defined_twice, definition};
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
    pub(crate) name: String,
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
            name, body, span, ..
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
            body,
            span: *span,
        });
    }
    classes
}

/// The methods a class may define in 0.2 besides its own: `__init__`, `__str__` and
/// `__repr__`. Any other name that starts and ends with two underscores changes how Python
/// treats the instances, and waits for the operator methods of 0.4 (CLS-07).
const SPECIAL_METHODS: [&str; 3] = ["__init__", "__str__", "__repr__"];

/// Collects each class's attributes and methods; the methods become definitions whose IDs
/// follow `first_function`.
pub(super) fn collect_members<'tree>(
    classes: &[ClassStatement<'tree>],
    class_ids: &BTreeMap<String, ClassId>,
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
            bound: if floor_defers { u32::MAX } else { class.id.0 },
        };
        let owner = Owner {
            id: class.id,
            name: &class.name.name,
        };
        let mut info = ClassInfo {
            name: class.name.name.clone(),
            attributes: Vec::new(),
            methods: Vec::new(),
            init: None,
            span: class.span,
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
                    if info.method(name).is_some() {
                        diagnostics.push(defined_twice(parts.name));
                        continue;
                    }
                    if name.starts_with("__")
                        && name.ends_with("__")
                        && !SPECIAL_METHODS.contains(&name.as_str())
                    {
                        diagnostics.push(planned_method(parts.name));
                        continue;
                    }
                    if parts.parameters.is_empty() {
                        diagnostics.push(
                            Diagnostic::new(
                                DiagnosticCode::error(201),
                                parts.name.span,
                                "a method without `self` is a static method, which is planned \
                                 for TyPy 0.4 (CLS-06)",
                            )
                            .with_rule(RuleId::CLS_06),
                        );
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

/// Reports a special method other than `__init__`, `__str__` and `__repr__`.
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
