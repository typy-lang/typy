//! Reads the abstract classes of `abc` (CLS-45, ADR-0093). `ABC` leaves the bases of each
//! class that names it, and `@abstractmethod` leaves each function it decorates, before the
//! checker reads the module, so it sees plain classes and methods; what they marked is kept
//! by the span of each name. CPython's `ABCMeta` gives the rules the classes then follow.

use std::collections::{BTreeMap, BTreeSet};

use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId, Span};
use typy_source::Module;
use typy_source::syntax::{Constant, Expr, Stmt};

use super::class::{ClassInfo, ClassStatement};
use super::imported_name::ImportedName;
use crate::typed::ClassId;

/// What `abc` marks in one module: each class that names `ABC` among its bases, and each
/// method `@abstractmethod` decorates, by the span of its name.
#[derive(Clone, Debug, Default)]
pub(crate) struct AbstractMarks {
    derived: BTreeSet<Span>,
    methods: BTreeSet<Span>,
}

/// Returns a copy of the module without `ABC` among its classes' bases and without
/// `@abstractmethod` among its functions' decorators, or `None` when it names neither, with
/// what they marked. A misplaced `@abstractmethod` is reported and left in place.
pub(crate) fn read(
    module: &Module,
    diagnostics: &mut Diagnostics,
) -> (Option<Module>, AbstractMarks) {
    let body = &module.syntax.body;
    let base = ImportedName::of(body, "abc", "ABC");
    let decorator = ImportedName::of(body, "abc", "abstractmethod");
    let mut marks = AbstractMarks::default();
    if base.is_empty() && decorator.is_empty() {
        return (None, marks);
    }
    let mut read = module.clone();
    for statement in &mut read.syntax.body {
        match statement {
            Stmt::ClassDef {
                name, bases, body, ..
            } => {
                let count = bases.len();
                bases.retain(|each| !base.names(each));
                if bases.len() < count {
                    marks.derived.insert(name.span);
                }
                for member in body.iter_mut() {
                    if let Some(method) = take_decorator(member, &decorator, diagnostics) {
                        marks.methods.insert(method);
                    }
                }
            }
            // `@abstractmethod` on a module function only sets an attribute CPython reads
            // through a class.
            other => {
                take_decorator(other, &decorator, diagnostics);
            }
        }
    }
    (Some(read), marks)
}

/// Takes `@abstractmethod` out of a `def`'s decorators when it is the last, the first CPython
/// applies, and returns the span of the function's name. Above `@property`,
/// `@staticmethod`, `@classmethod` or a setter it is reported and taken out, as CPython raises
/// `AttributeError` when the class is defined; above another decorator it stays, which the
/// rules for that decorator refuse.
fn take_decorator(
    statement: &mut Stmt,
    names: &ImportedName,
    diagnostics: &mut Diagnostics,
) -> Option<Span> {
    let Stmt::FunctionDef {
        name, decorators, ..
    } = statement
    else {
        return None;
    };
    let position = decorators.iter().position(|each| names.names(each))?;
    if position + 1 == decorators.len() {
        decorators.pop();
        return Some(name.span);
    }
    if let Some(under) = decorators.get(position + 1).and_then(method_decorator) {
        diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(434),
                decorators[position].span(),
                format!(
                    "`@abstractmethod` is applied to a {under} object here, and CPython raises \
                     `AttributeError` when the class is defined"
                ),
            )
            .with_rule(RuleId::CLS_45)
            .with_help("write `@abstractmethod` last, just above `def`"),
        );
        decorators.remove(position);
    }
    None
}

/// Returns the kind of object a decorator of CLS-06 makes, or `None` for another decorator.
fn method_decorator(decorator: &Expr) -> Option<&'static str> {
    match decorator {
        Expr::Name { id, .. } => match id.as_str() {
            "property" => Some("`property`"),
            "staticmethod" => Some("`staticmethod`"),
            "classmethod" => Some("`classmethod`"),
            _ => None,
        },
        Expr::Attribute { attribute, .. } if attribute.name == "setter" => Some("`property`"),
        _ => None,
    }
}

/// Notes, for each class of a module, whether `ABC` is among its ancestors, the abstract
/// methods it has without an override, and its own abstract methods that take the instance
/// and whose bodies give no value. `earlier` holds the classes of the modules checked before,
/// which a base may be, and `root_of` gives the generic class a copy is made from, whose
/// methods a base that is a copy has. An exception class is left as it is: `BaseException`
/// makes its instances without the test of abstract methods.
pub(super) fn mark_abstract(
    infos: &mut [ClassInfo],
    classes: &[ClassStatement<'_>],
    (earlier, root_of): (&[ClassInfo], &dyn Fn(ClassId) -> Option<ClassId>),
    marks: &AbstractMarks,
) {
    for index in 0..infos.len() {
        let Some(class) = classes.iter().find(|class| class.id == infos[index].id) else {
            continue;
        };
        if infos[index].exception.is_some() {
            continue;
        }
        let find = |class: ClassId| {
            earlier
                .get(class.0 as usize)
                .filter(|info| info.id == class)
                .or_else(|| infos[..index].iter().find(|info| info.id == class))
        };
        let base = infos[index]
            .base
            .and_then(|base| find(base).or_else(|| find(root_of(base)?)));
        let derived = marks.derived.contains(&class.name.span);
        if !derived && !base.is_some_and(|base| base.abstract_base) {
            continue;
        }
        let defined = defined_names(class.body);
        let own: BTreeMap<&str, &Stmt> = class
            .body
            .iter()
            .filter_map(|member| match member {
                Stmt::FunctionDef { name, .. } if marks.methods.contains(&name.span) => {
                    Some((name.name.as_str(), member))
                }
                _ => None,
            })
            .collect();
        // CPython's `ABCMeta` takes the class's own abstract methods, and each abstract method
        // of a base that the class does not define again.
        let mut abstract_methods: BTreeSet<String> =
            own.keys().map(|name| (*name).to_owned()).collect();
        if let Some(base) = base.filter(|base| base.abstract_base) {
            abstract_methods.extend(
                base.abstract_methods
                    .iter()
                    .filter(|name| !defined.contains(name.as_str()))
                    .cloned(),
            );
        }
        let empty: Vec<String> = own
            .iter()
            .filter(|(_, member)| takes_instance(member) && gives_no_value(member))
            .map(|(name, _)| (*name).to_owned())
            .collect();
        let info = &mut infos[index];
        info.abstract_base = true;
        info.abstract_methods = abstract_methods.into_iter().collect();
        info.empty_abstract = empty;
    }
}

/// Returns the names a class body defines: its functions, and the names it assigns.
fn defined_names(body: &[Stmt]) -> BTreeSet<&str> {
    let mut names = BTreeSet::new();
    for statement in body {
        match statement {
            Stmt::FunctionDef { name, .. } => {
                names.insert(name.name.as_str());
            }
            Stmt::Assign { targets, .. } => {
                names.extend(targets.iter().filter_map(|target| match target {
                    Expr::Name { id, .. } => Some(id.as_str()),
                    _ => None,
                }));
            }
            Stmt::AnnAssign {
                target: Expr::Name { id, .. },
                ..
            } => {
                names.insert(id.as_str());
            }
            _ => {}
        }
    }
    names
}

/// Returns whether a method takes the instance: it is not a static or a class method.
fn takes_instance(method: &Stmt) -> bool {
    !matches!(method, Stmt::FunctionDef { decorators, .. }
        if decorators.iter().any(|decorator| matches!(decorator, Expr::Name { id, .. }
            if id == "staticmethod" || id == "classmethod")))
}

/// Returns whether a function's body only holds a docstring, `pass` or `...`, so it gives no
/// value when it runs.
fn gives_no_value(function: &Stmt) -> bool {
    let Stmt::FunctionDef { body, .. } = function else {
        return false;
    };
    body.iter().all(|statement| match statement {
        Stmt::Pass { .. } => true,
        Stmt::Expr {
            value: Expr::Constant { value, .. },
            ..
        } => matches!(value, Constant::Ellipsis | Constant::Str(_)),
        _ => false,
    })
}

#[cfg(test)]
mod tests;
