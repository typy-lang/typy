//! The type of `args` of each exception class, inferred from every way the program and the
//! runtime create an exception of the class or of a class derived from it (EXC-15,
//! ADR-0034).

use crate::exception::{BuiltinException, ExceptionClass};
use crate::types::Type;

/// The ways a check of the program saw exceptions created, with the types of their
/// arguments.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Creations {
    /// Each class the program calls or raises, with the tuple of the arguments the new
    /// exception holds.
    made: Vec<(ExceptionClass, Type)>,
    /// Each class whose `__init__` calls `super().__init__(...)`, with the tuple of that
    /// call's arguments, which replace those of an exception of the class or of a class
    /// derived from it (CLS-14).
    initialized: Vec<(ExceptionClass, Type)>,
    /// The type of each key the runtime can find missing, as the argument of its `KeyError`.
    keys: Vec<Type>,
    /// Whether the runtime can raise `UnicodeEncodeError`: the program encodes a `str`,
    /// opens a file, or makes a `str` with a surrogate standard output cannot write.
    encodes: bool,
    /// Whether the runtime can raise `UnicodeDecodeError`: the program decodes `bytes` or
    /// opens a file.
    decodes: bool,
    /// Whether the program opens a file from a descriptor (ADR-0035).
    descriptors: bool,
}

impl Creations {
    /// Records a call or a raise of `class` with arguments of `arguments`, all of them as the
    /// exception holds them.
    pub(crate) fn make(&mut self, class: &ExceptionClass, arguments: &[Type]) {
        // `OSError` itself of two to five arguments becomes the subclass for its `errno`, so
        // its arguments may be those of an exception of any related class.
        let mapped = *class == ExceptionClass::Builtin(BuiltinException::OSError)
            && (2..=5).contains(&arguments.len());
        let made = if mapped {
            &mut self.initialized
        } else {
            &mut self.made
        };
        push(made, (class.clone(), Type::Tuple(arguments.to_vec())));
    }

    /// Records a call of `class` with `*`, whose arguments are any number of values of type
    /// `item` (EXP-22).
    pub(crate) fn make_variadic(&mut self, class: &ExceptionClass, item: &Type) {
        push(
            &mut self.made,
            (class.clone(), Type::TupleOf(Box::new(item.clone()))),
        );
    }

    /// Records that the runtime may raise an `OSError` about a file opened from a
    /// descriptor, which holds the descriptor as its third argument.
    pub(crate) fn descriptors(&mut self) {
        self.descriptors = true;
    }

    /// Records that the runtime may raise `UnicodeEncodeError` when `encodes`, and
    /// `UnicodeDecodeError` when `decodes`.
    pub(crate) fn codec(&mut self, encodes: bool, decodes: bool) {
        self.encodes |= encodes;
        self.decodes |= decodes;
    }

    /// Records `super().__init__(arguments)` in the `__init__` of `class`.
    pub(crate) fn initialize(&mut self, class: &ExceptionClass, arguments: &[Type]) {
        push(
            &mut self.initialized,
            (class.clone(), Type::Tuple(arguments.to_vec())),
        );
    }

    /// Records a key the runtime looks up and may find missing.
    pub(crate) fn key(&mut self, key: &Type) {
        if !key.has_error() && !self.keys.contains(key) {
            self.keys.push(key.clone());
        }
    }

    /// Returns true when every creation of `other` is one of these.
    pub(crate) fn covers(&self, other: &Self) -> bool {
        other.made.iter().all(|each| self.made.contains(each))
            && other
                .initialized
                .iter()
                .all(|each| self.initialized.contains(each))
            && other.keys.iter().all(|each| self.keys.contains(each))
            && (self.encodes || !other.encodes)
            && (self.decodes || !other.decodes)
            && (self.descriptors || !other.descriptors)
    }

    /// Adds the creations of `other` that these lack.
    pub(crate) fn extend(&mut self, other: &Self) {
        for each in &other.made {
            push(&mut self.made, each.clone());
        }
        for each in &other.initialized {
            push(&mut self.initialized, each.clone());
        }
        for key in &other.keys {
            self.key(key);
        }
        self.codec(other.encodes, other.decodes);
        self.descriptors |= other.descriptors;
    }

    /// Returns the type of `args` of an exception of `class`: a tuple, or a union of tuples
    /// no two of which have the same layout, so the layout of an exception's arguments
    /// names the member it holds.
    pub(crate) fn arguments_of(&self, class: &ExceptionClass) -> Type {
        union_of(self.tuples(class, false))
    }

    /// Returns the type of the arguments an exception of `class` holds, file names included,
    /// which the attributes of an `OSError` read (ADR-0035).
    pub(crate) fn raw_arguments_of(&self, class: &ExceptionClass) -> Type {
        union_of(self.tuples(class, true))
    }

    /// Returns the argument tuples of every creation of an exception of `class`, as the
    /// exception holds them when `raw`, and as `args` shows them otherwise.
    fn tuples(&self, class: &ExceptionClass, raw: bool) -> Vec<Type> {
        let mut tuples = Vec::new();
        for builtin in BuiltinException::ALL {
            if ExceptionClass::Builtin(builtin).derives_from(class) {
                tuples.extend(self.runtime_arguments(builtin, raw));
            }
        }
        let mut add = |made: &ExceptionClass, arguments: &Type| {
            if raw {
                tuples.push(arguments.clone());
            } else {
                tuples.extend(shown(made, arguments));
            }
        };
        for (made, arguments) in &self.made {
            if made.derives_from(class) {
                add(made, arguments);
            }
        }
        // An `__init__` runs for the classes derived from its own, so its arguments may be
        // those of an exception of a base or of a derived class.
        for (initialized, arguments) in &self.initialized {
            if initialized.derives_from(class) || class.derives_from(initialized) {
                add(initialized, arguments);
            }
        }
        tuples
    }

    /// Returns the argument tuples of the exceptions of `class` the runtime raises: its
    /// message for most classes, the missing key for `KeyError`, the code and the text for
    /// `OSError`, the five of a Unicode error when the program can cause one, and none for
    /// `StopIteration`. It raises no `SystemExit`, `AssertionError` or `UnicodeError` itself.
    /// An `OSError` keeps both, since reading or writing a standard file the wrong way raises
    /// `io.UnsupportedOperation` with a message in any program; when `raw`, it also holds the
    /// file name, or the descriptor, that `args` leaves out.
    fn runtime_arguments(&self, class: BuiltinException, raw: bool) -> Vec<Type> {
        let message = Type::Tuple(vec![Type::Str]);
        match class {
            BuiltinException::SystemExit
            | BuiltinException::AssertionError
            | BuiltinException::UnicodeError => Vec::new(),
            BuiltinException::StopIteration => vec![Type::Tuple(Vec::new())],
            BuiltinException::UnicodeEncodeError | BuiltinException::UnicodeDecodeError => {
                let (object, raised) = if class == BuiltinException::UnicodeEncodeError {
                    (Type::Str, self.encodes)
                } else {
                    (Type::Bytes, self.decodes)
                };
                if !raised {
                    return Vec::new();
                }
                vec![Type::Tuple(vec![
                    Type::Str,
                    object,
                    Type::Int,
                    Type::Int,
                    Type::Str,
                ])]
            }
            BuiltinException::KeyError => std::iter::once(message)
                .chain(self.keys.iter().map(|key| Type::Tuple(vec![key.clone()])))
                .collect(),
            other if other.is_os_error() => {
                let mut tuples = vec![message, Type::Tuple(vec![Type::Int, Type::Str])];
                if raw {
                    tuples.push(Type::Tuple(vec![Type::Int, Type::Str, Type::Str]));
                    if self.descriptors {
                        tuples.push(Type::Tuple(vec![Type::Int, Type::Str, Type::Int]));
                    }
                }
                tuples
            }
            _ => vec![message],
        }
    }
}

/// Returns the tuples `args` shows for an exception of `class` that holds `arguments`: an
/// `OSError` of three to five whose file name is not `None` shows `errno` and `strerror`
/// alone, as CPython's `OSError.__init__` cuts them (EXC-14), and a file name that may be
/// `None` gives both.
fn shown(class: &ExceptionClass, arguments: &Type) -> Vec<Type> {
    match arguments {
        Type::Tuple(items) if class.builtin().is_os_error() && (3..=5).contains(&items.len()) => {
            let cut = Type::Tuple(items[..2].to_vec());
            match &items[2] {
                Type::NoneType => vec![arguments.clone()],
                Type::Optional(_) => vec![arguments.clone(), cut],
                _ => vec![cut],
            }
        }
        _ => vec![arguments.clone()],
    }
}

/// Returns `tuples` as one type: a tuple, or a union whose members have distinct layouts,
/// those of one layout joined item by item. A `tuple[T, ...]` takes in each fixed tuple whose
/// items are all held as `T` is, so the runtime tells the members apart by their layout alone
/// (EXP-22).
fn union_of(tuples: Vec<Type>) -> Type {
    let mut variadic: Vec<Type> = Vec::new();
    let mut fixed: Vec<Vec<Type>> = Vec::new();
    for tuple in tuples {
        match tuple {
            Type::TupleOf(item) => join_variadic(&mut variadic, &[*item]),
            Type::Tuple(items) => fixed.push(items),
            _ => {}
        }
    }
    let mut members: Vec<Vec<Type>> = Vec::new();
    for items in fixed {
        if variadic
            .iter()
            .any(|item| items.iter().all(|each| same_item_layout(item, each)))
        {
            join_variadic(&mut variadic, &items);
            continue;
        }
        match members
            .iter_mut()
            .find(|member| same_layout(member, &items))
        {
            Some(member) => {
                for (held, item) in member.iter_mut().zip(items) {
                    if *held != item {
                        *held = Type::union(vec![held.clone(), item]);
                    }
                }
            }
            None => members.push(items),
        }
    }
    let mut members: Vec<Type> = members
        .into_iter()
        .map(Type::Tuple)
        .chain(
            variadic
                .into_iter()
                .map(|item| Type::TupleOf(Box::new(item))),
        )
        .collect();
    // A union's members stand in one order, as `Type::union` puts them; it is not used,
    // since it drops a tuple another accepts, such as `tuple[int]` for `tuple[float]`, whose
    // layout differs.
    members.sort_by_cached_key(|member| format!("{member:?}"));
    match members.len() {
        0 => Type::Tuple(Vec::new()),
        1 => members.remove(0),
        _ => Type::Union(members),
    }
}

/// Joins `items` into the item type of the `tuple[T, ...]` of `variadic` that holds them
/// alike, or adds one for the first of them; `items` all share one layout.
fn join_variadic(variadic: &mut Vec<Type>, items: &[Type]) {
    let Some(first) = items.first() else {
        return;
    };
    let index = variadic
        .iter()
        .position(|item| same_item_layout(item, first))
        .unwrap_or_else(|| {
            variadic.push(first.clone());
            variadic.len() - 1
        });
    for item in items {
        let held = &mut variadic[index];
        if held != item {
            *held = Type::union(vec![held.clone(), item.clone()]);
        }
    }
}

/// Adds a creation unless it is there already or a type in it is an error, which was
/// reported where it arose.
fn push(creations: &mut Vec<(ExceptionClass, Type)>, creation: (ExceptionClass, Type)) {
    if !creation.1.has_error() && !creations.contains(&creation) {
        creations.push(creation);
    }
}

/// Returns true when two tuples are laid out alike: as many items, and at each position
/// items held alike.
fn same_layout(left: &[Type], right: &[Type]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| same_item_layout(left, right))
}

/// Returns true when items of two types are held alike: one type, or two types held as
/// objects.
fn same_item_layout(left: &Type, right: &Type) -> bool {
    left == right || (held_as_object(left) && held_as_object(right))
}

/// Returns true when a value of the type is the address of an object: a union only when
/// every member is one.
fn held_as_object(ty: &Type) -> bool {
    match ty {
        Type::Union(members) => members.iter().all(Type::is_object),
        other => other.is_object(),
    }
}

#[cfg(test)]
mod tests;
