//! Finds, before any module is checked, the class variables a program assigns through a
//! subclass that only inherits them (CLS-25, ADR-0037). In CPython, `Square.sides = 4` gives
//! `Square` a variable of its own, which its instances and subclasses then read; a read through
//! an instance of the base may so read the variable of the instance's class, even in a module
//! checked before the one that assigns it.

use std::collections::{BTreeMap, BTreeSet};

use typy_source::Module;
use typy_source::syntax::{Expr, Stmt};

/// A class of the program by its module and its name, and the name of a variable.
pub(crate) type ClassVariable = (String, String, String);

/// The class variables of subclasses a program assigns.
#[derive(Clone, Debug, Default)]
pub(crate) struct SubclassVariables {
    /// Each class an assignment through it gives a variable of its own, with the variable.
    pub(crate) own: BTreeSet<ClassVariable>,
    /// Each class whose body gives a variable that some class derived from it gets one of its
    /// own of: a read through an instance of it, or of a class derived from it, reads the
    /// variable of the instance's class.
    pub(crate) shared: BTreeSet<ClassVariable>,
}

/// What the scan knows of a class: its bases and the names its body gives values.
struct ClassFacts<'tree> {
    bases: &'tree [Expr],
    gives: BTreeSet<String>,
}

/// The classes and the imported names of every module.
struct Index<'tree> {
    classes: BTreeMap<(String, String), ClassFacts<'tree>>,
    /// For each module, the names its `from` imports bind, each to a module and a name.
    imported: BTreeMap<String, BTreeMap<String, (String, String)>>,
    /// For each module, the names its `import` statements bind to modules.
    modules: BTreeMap<String, BTreeMap<String, String>>,
}

/// Returns the class variables `modules` assign through a subclass that inherits them.
pub(crate) fn subclass_variables(modules: &[Module]) -> SubclassVariables {
    let index = Index::of(modules);
    let mut found = SubclassVariables::default();
    for module in modules {
        let name = module.name.as_str();
        let mut assigned = Vec::new();
        assignments(&module.syntax.body, &mut assigned);
        for (target, variable) in assigned {
            let Some(class) = index.class_of(name, target) else {
                continue;
            };
            if let Some(holder) = index.holder(&class, variable) {
                if holder == class {
                    continue;
                }
                found.own.insert((class.0, class.1, variable.to_owned()));
                found
                    .shared
                    .insert((holder.0, holder.1, variable.to_owned()));
            }
        }
    }
    found
}

impl<'tree> Index<'tree> {
    fn of(modules: &'tree [Module]) -> Self {
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
                        ..
                    } => {
                        let gives = body.iter().filter_map(given_name).collect();
                        index.classes.insert(
                            (name.clone(), class.name.clone()),
                            ClassFacts { bases, gives },
                        );
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

    /// Returns the class `expression` names in `module`: a class of the module, one a `from`
    /// import binds, or `m.Class` of a module an `import` binds.
    fn class_of(&self, module: &str, expression: &Expr) -> Option<(String, String)> {
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

    /// Returns the class whose body gives `variable`: `class` itself or the nearest base.
    fn holder(&self, class: &(String, String), variable: &str) -> Option<(String, String)> {
        let mut current = class.clone();
        // A chain of bases is finite; the bound guards one the program writes in a cycle.
        for _ in 0..self.classes.len() {
            let facts = self.classes.get(&current)?;
            if facts.gives.contains(variable) {
                return Some(current);
            }
            current = self.class_of(&current.0, facts.bases.first()?)?;
        }
        None
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

/// Returns the name a statement of a class body gives a value: a class variable or the
/// default of an instance attribute.
fn given_name(statement: &Stmt) -> Option<String> {
    match statement {
        Stmt::Assign { targets, .. } => match targets.as_slice() {
            [Expr::Name { id, .. }] => Some(id.clone()),
            _ => None,
        },
        Stmt::AnnAssign {
            target: Expr::Name { id, .. },
            value: Some(_),
            ..
        } => Some(id.clone()),
        _ => None,
    }
}

/// Collects each `object.name = value` and `object.name op= value` of `statements`, at any
/// depth, with its object and name.
fn assignments<'tree>(statements: &'tree [Stmt], found: &mut Vec<(&'tree Expr, &'tree str)>) {
    for statement in statements {
        let targets: Vec<&Expr> = match statement {
            Stmt::Assign { targets, .. } => targets.iter().collect(),
            Stmt::AugAssign { target, .. } => vec![target],
            _ => Vec::new(),
        };
        for target in targets {
            if let Expr::Attribute {
                value, attribute, ..
            } = target
            {
                found.push((value.as_ref(), attribute.name.as_str()));
            }
        }
        for block in blocks(statement) {
            assignments(block, found);
        }
    }
}

/// Returns the blocks a statement holds: the bodies of a `def`, a `class`, a loop, an `if`,
/// a `with`, a `try` and the cases of a `match`.
fn blocks(statement: &Stmt) -> Vec<&[Stmt]> {
    match statement {
        Stmt::FunctionDef { body, .. } | Stmt::ClassDef { body, .. } | Stmt::With { body, .. } => {
            vec![body]
        }
        Stmt::If { body, orelse, .. }
        | Stmt::For { body, orelse, .. }
        | Stmt::While { body, orelse, .. } => vec![body, orelse],
        Stmt::Match { cases, .. } => cases.iter().map(|case| case.body.as_slice()).collect(),
        Stmt::Try { .. } => statement.try_blocks(),
        _ => Vec::new(),
    }
}
