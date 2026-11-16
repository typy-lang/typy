//! Lays out each class's method table and decides which method calls go through it
//! (ADR-0021).

use std::collections::{BTreeMap, BTreeSet};

use typy_check::{ClassId, FunctionId, TypedProgram};

/// What lowering knows of every call: for each function, whether a call of it is an
/// invoke, and the method tables that decide which calls go through a table.
pub(crate) struct Calls<'program> {
    pub(crate) may_raise: &'program [bool],
    pub(crate) methods: &'program MethodTables,
    /// Whether a method a type object's slot calls may raise (CLS-22, ADR-0039).
    pub(crate) raising_slots: bool,
}

/// The method table of each class of the program, and the class and name of each method.
pub(crate) struct MethodTables {
    /// For each class, by ID, its table: each method a subclass overrides, by name, at the
    /// position it has in the base's table, then those the class adds.
    tables: Vec<Vec<(String, FunctionId)>>,
    /// The class that defines each method, and the method's name.
    owners: BTreeMap<FunctionId, (ClassId, String)>,
    /// For each class, by ID, its base when that is a class of the program.
    bases: Vec<Option<ClassId>>,
    /// The methods a class derived from their class defines again.
    overridden: BTreeSet<FunctionId>,
}

impl MethodTables {
    /// Lays out the tables: a class starts from its base's table, puts its own method in
    /// place of each one it overrides, and adds each new method a class derived from it
    /// overrides. A method no class overrides has no entry, so a program without overrides
    /// has empty tables.
    pub(crate) fn new(program: &TypedProgram) -> Self {
        let bases: Vec<Option<ClassId>> = program.classes.iter().map(|class| class.base).collect();
        let owners = program
            .classes
            .iter()
            .flat_map(|class| {
                class
                    .methods
                    .iter()
                    .map(move |(name, id)| (*id, (class.id, name.clone())))
            })
            .collect();
        let mut layout = Self {
            tables: Vec::with_capacity(program.classes.len()),
            owners,
            bases,
            overridden: BTreeSet::new(),
        };
        // A base has a lower ID than the classes derived from it, so its table comes first.
        for class in &program.classes {
            let mut table = class
                .base
                .map(|base| layout.tables[base.0 as usize].clone())
                .unwrap_or_default();
            // A constructor runs on the class being made, whose parameters it takes, so it is
            // never called through a table.
            for (name, id) in class.methods.iter().filter(|(name, _)| name != "__init__") {
                let below = layout.defined_below(program, class.id, name);
                if below {
                    layout.overridden.insert(*id);
                }
                if let Some(entry) = table.iter_mut().find(|(each, _)| each == name) {
                    entry.1 = *id;
                } else if below {
                    table.push((name.clone(), *id));
                }
            }
            layout.tables.push(table);
        }
        layout
    }

    /// Returns the table of a class, as TyIR numbers its functions.
    pub(crate) fn table(&self, class: ClassId) -> Vec<typy_tyir::FunctionId> {
        self.tables[class.0 as usize]
            .iter()
            .map(|(_, id)| typy_tyir::FunctionId(id.0))
            .collect()
    }

    /// Returns the class and position through which a call of `function` on an instance
    /// goes, or `None` for a call that is direct: one whose method no class derived from its
    /// class defines again, and any function that is not a method.
    pub(crate) fn dispatch(&self, function: FunctionId) -> Option<(u32, u32)> {
        if !self.overridden.contains(&function) {
            return None;
        }
        let (class, name) = self.owners.get(&function)?;
        let slot = self.tables[class.0 as usize]
            .iter()
            .position(|(each, _)| each == name)?;
        Some((class.0, u32::try_from(slot).unwrap_or(u32::MAX)))
    }

    /// Returns the methods that share a position in the tables, so a call through one of
    /// them may run any: the table's first class and the position name each group.
    pub(crate) fn families(&self) -> Vec<Vec<FunctionId>> {
        let mut families: BTreeMap<(usize, usize), Vec<FunctionId>> = BTreeMap::new();
        for (class, table) in self.tables.iter().enumerate() {
            for (slot, (_, id)) in table.iter().enumerate() {
                let mut first = class;
                while let Some(base) =
                    self.bases[first].filter(|base| self.tables[base.0 as usize].len() > slot)
                {
                    first = base.0 as usize;
                }
                let family = families.entry((first, slot)).or_default();
                if !family.contains(id) {
                    family.push(*id);
                }
            }
        }
        families.into_values().collect()
    }

    /// Returns each method that overrides `function` in a class derived from its class, with
    /// that class, the classes nearest the end of a chain of bases first.
    pub(crate) fn overrides(&self, function: FunctionId) -> Vec<(ClassId, FunctionId)> {
        let Some((class, name)) = self.owners.get(&function) else {
            return Vec::new();
        };
        let depth = |class: ClassId| {
            std::iter::successors(Some(class), |each| {
                self.bases.get(each.0 as usize).copied().flatten()
            })
            .count()
        };
        let mut found: Vec<(ClassId, FunctionId)> = self
            .owners
            .iter()
            .filter(|(method, (owner, each))| {
                **method != function
                    && each == name
                    && *owner != *class
                    && self.derives_from(*owner, *class)
            })
            .map(|(method, (owner, _))| (*owner, *method))
            .collect();
        found.sort_by_key(|(owner, _)| std::cmp::Reverse(depth(*owner)));
        found
    }

    /// Returns true when a class derived from `class`, other than `class`, defines `name`.
    fn defined_below(&self, program: &TypedProgram, class: ClassId, name: &str) -> bool {
        program.classes.iter().any(|other| {
            other.id != class
                && self.derives_from(other.id, class)
                && other.methods.iter().any(|(each, _)| each == name)
        })
    }

    /// Returns true when `class` is `base` or derives from it.
    fn derives_from(&self, class: ClassId, base: ClassId) -> bool {
        let mut current = Some(class);
        while let Some(each) = current {
            if each == base {
                return true;
            }
            current = self.bases.get(each.0 as usize).copied().flatten();
        }
        false
    }
}
