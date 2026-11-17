//! The classes of every module of a program, read from the syntax before any module is
//! checked, with the names each module's imports bind, so a scan can follow a class's chain
//! of bases across modules (CLS-21, CLS-25).

use std::collections::BTreeMap;

use typy_diagnostics::Span;
use typy_source::Module;
use typy_source::syntax::{Expr, Stmt};

/// A class of the program by its module and its name.
pub(crate) type ClassKey = (String, String);

/// What the index knows of a class: its bases, its body and the span of its statement.
struct ClassFacts<'tree> {
    bases: &'tree [Expr],
    body: &'tree [Stmt],
    span: Span,
}

/// The classes and the imported names of every module.
pub(crate) struct ClassIndex<'tree> {
    classes: BTreeMap<ClassKey, ClassFacts<'tree>>,
    /// For each module, the names its `from` imports bind, each to a module and a name.
    imported: BTreeMap<String, BTreeMap<String, ClassKey>>,
    /// For each module, the names its `import` statements bind to modules.
    modules: BTreeMap<String, BTreeMap<String, String>>,
}

impl<'tree> ClassIndex<'tree> {
    /// Returns the index of the classes at the top level of `modules`.
    pub(crate) fn of(modules: &'tree [Module]) -> Self {
        let mut index = Self {
            classes: BTreeMap::new(),
            imported: BTreeMap::new(),
            modules: BTreeMap::new(),
        };
        for module in modules {
            let name = module.name.as_str().to_owned();
            let mut imported = BTreeMap::new();
            let mut bound = BTreeMap::new();
            for statement in &module.syntax.body {
                match statement {
                    Stmt::ClassDef {
                        name: class,
                        bases,
                        body,
                        span,
                        ..
                    } => {
                        let facts = ClassFacts {
                            bases,
                            body,
                            span: *span,
                        };
                        index
                            .classes
                            .insert((name.clone(), class.name.clone()), facts);
                    }
                    Stmt::ImportFrom {
                        module: from,
                        names,
                        level,
                        ..
                    } => {
                        let Some(source) =
                            absolute(module, from.as_ref().map(|from| from.name.as_str()), *level)
                        else {
                            continue;
                        };
                        for alias in names {
                            let bound_name = alias.as_name.as_ref().unwrap_or(&alias.name);
                            imported.insert(
                                bound_name.name.clone(),
                                (source.clone(), alias.name.name.clone()),
                            );
                        }
                    }
                    Stmt::Import { names, .. } => {
                        for alias in names {
                            let bound_name = alias.as_name.as_ref().unwrap_or(&alias.name);
                            bound.insert(bound_name.name.clone(), alias.name.name.clone());
                        }
                    }
                    _ => {}
                }
            }
            index.imported.insert(name.clone(), imported);
            index.modules.insert(name, bound);
        }
        index
    }

    /// Returns each class of the index with its body and the span of its statement.
    pub(crate) fn classes(&self) -> impl Iterator<Item = (&ClassKey, &'tree [Stmt], Span)> {
        self.classes
            .iter()
            .map(|(key, facts)| (key, facts.body, facts.span))
    }

    /// Returns the body of `class`, or nothing for a class the index does not hold.
    pub(crate) fn body(&self, class: &ClassKey) -> &'tree [Stmt] {
        self.classes.get(class).map_or(&[], |facts| facts.body)
    }

    /// Returns the class `expression` names in `module`: a class of the module, one a `from`
    /// import binds, or `m.Class` of a module an `import` binds.
    pub(crate) fn class_of(&self, module: &str, expression: &Expr) -> Option<ClassKey> {
        let key = match expression {
            Expr::Name { id, .. } => self
                .imported
                .get(module)
                .and_then(|imported| imported.get(id))
                .cloned()
                .unwrap_or_else(|| (module.to_owned(), id.clone())),
            Expr::Attribute {
                value, attribute, ..
            } => match value.as_ref() {
                Expr::Name { id, .. } => (
                    self.modules.get(module)?.get(id)?.clone(),
                    attribute.name.clone(),
                ),
                _ => return None,
            },
            _ => return None,
        };
        self.classes.contains_key(&key).then_some(key)
    }

    /// Returns `class` and each base of it the index holds, nearest first.
    pub(crate) fn chain(&self, class: &ClassKey) -> Vec<ClassKey> {
        let mut chain = Vec::new();
        let mut current = Some(class.clone());
        // A chain of bases is finite; the bound guards one the program writes in a cycle.
        while let Some(class) = current.filter(|_| chain.len() <= self.classes.len()) {
            let Some(facts) = self.classes.get(&class) else {
                break;
            };
            current = facts
                .bases
                .first()
                .and_then(|base| self.class_of(&class.0, base));
            chain.push(class);
        }
        chain
    }
}

/// Returns the module a `from` import of `from` at `level` names, relative to `module`.
fn absolute(module: &Module, from: Option<&str>, level: u32) -> Option<String> {
    if level == 0 {
        return from.map(str::to_owned);
    }
    let mut base = module.package()?;
    for _ in 1..level {
        base = base.parent()?;
    }
    Some(match from {
        Some(from) => format!("{}.{from}", base.as_str()),
        None => base.as_str().to_owned(),
    })
}
