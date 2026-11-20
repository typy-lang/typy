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
    /// Whether the program has generator functions, so an exception keeps the variables of
    /// the functions it leaves, as CPython's traceback keeps their frames (ADR-0070).
    pub(crate) keeps_frames: bool,
}

/// The method table of each class of the program, and the class and name of each method.
pub(crate) struct MethodTables {
    /// For each class, by ID, its table: each method a subclass overrides, by name, at the
    /// position it has in the base's table, then those the class adds.
    tables: Vec<Vec<(String, FunctionId)>>,
    /// The class that defines each method, and the place its calls find it under: its name,
    /// or one of its own for an override held otherwise than its base's (CLS-04).
    owners: BTreeMap<FunctionId, (ClassId, String)>,
    /// For each class, by ID, its base when that is a class of the program.
    bases: Vec<Option<ClassId>>,
    /// The methods a class derived from their class defines again.
    overridden: BTreeSet<FunctionId>,
}

/// Returns the classes of the program, each after its base.
fn bases_first(program: &TypedProgram) -> Vec<ClassId> {
    let mut placed = vec![false; program.classes.len()];
    let mut order = Vec::with_capacity(program.classes.len());
    for class in &program.classes {
        let mut chain = Vec::new();
        let mut current = Some(class.id);
        while let Some(each) = current.filter(|each| !placed[each.0 as usize]) {
            placed[each.0 as usize] = true;
            chain.push(each);
            current = program.classes[each.0 as usize].base;
        }
        order.extend(chain.into_iter().rev());
    }
    order
}

impl MethodTables {
    /// Lays out the tables: a class starts from its base's table, puts its own method in
    /// place of each one it overrides, and adds each new method a class derived from it
    /// overrides. A method no class overrides has no entry, so a program without overrides
    /// has empty tables.
    pub(crate) fn new(program: &TypedProgram) -> Self {
        let bases: Vec<Option<ClassId>> = program.classes.iter().map(|class| class.base).collect();
        let mut layout = Self {
            tables: Vec::with_capacity(program.classes.len()),
            owners: BTreeMap::new(),
            bases,
            overridden: BTreeSet::new(),
        };
        // For each class, the place under which a call typed at the class finds each method:
        // its name, or for an override held otherwise than its base's, a place of its own
        // (CLS-04).
        let mut keys: Vec<BTreeMap<String, String>> = vec![BTreeMap::new(); program.classes.len()];
        layout.tables = vec![Vec::new(); program.classes.len()];
        // A base's table comes before the tables of the classes derived from it; a class
        // derived from a copy of a generic class has a lower ID than the copy (ADR-0040).
        for class in bases_first(program) {
            let class = &program.classes[class.0 as usize];
            let mut table = class
                .base
                .map(|base| layout.tables[base.0 as usize].clone())
                .unwrap_or_default();
            let mut class_keys = class
                .base
                .map(|base| keys[base.0 as usize].clone())
                .unwrap_or_default();
            for (name, own) in &class.methods {
                // A generic method is called through its copies, each held under a key of its
                // own, never itself (GEN-09).
                if program
                    .functions
                    .get(own.0 as usize)
                    .is_some_and(|function| function.generic)
                {
                    continue;
                }
                // A static method is held in the table as its bridge, which takes the
                // instance first (CLS-06).
                let id = class
                    .static_bridges
                    .iter()
                    .find(|(each, _)| each == name)
                    .map_or(own, |(_, bridge)| bridge);
                // A constructor runs on the class being made, whose parameters it takes, so it
                // is never called through a table.
                if name == "__init__" {
                    layout.owners.insert(*id, (class.id, name.clone()));
                    continue;
                }
                let inherited = class_keys
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| name.clone());
                let key = match class.adapters.iter().find(|(each, _)| each == name) {
                    // The adapter takes the base method's place, which a call through the base
                    // reaches, and the override a place of its own.
                    Some((_, adapter)) => {
                        if let Some(entry) = table.iter_mut().find(|(each, _)| *each == inherited) {
                            entry.1 = *adapter;
                        }
                        layout.owners.insert(*adapter, (class.id, inherited));
                        let own = format!("{name} of class {}", class.id.0);
                        class_keys.insert(name.clone(), own.clone());
                        own
                    }
                    None => inherited,
                };
                layout.owners.insert(*id, (class.id, key.clone()));
                let below = layout.defined_below(program, class.id, name);
                if below {
                    layout.overridden.insert(*id);
                }
                if let Some(entry) = table.iter_mut().find(|(each, _)| *each == key) {
                    entry.1 = *id;
                } else if below {
                    table.push((key, *id));
                }
            }
            layout.tables[class.id.0 as usize] = table;
            keys[class.id.0 as usize] = class_keys;
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
