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
    let Type::Tuple(items) = arguments else {
        return vec![arguments.clone()];
    };
    if !class.builtin().is_os_error() || !(3..=5).contains(&items.len()) {
        return vec![arguments.clone()];
    }
    let cut = Type::Tuple(items[..2].to_vec());
    match &items[2] {
        Type::NoneType => vec![arguments.clone()],
        Type::Optional(_) => vec![arguments.clone(), cut],
        _ => vec![cut],
    }
}

/// Returns `tuples` as one type: a tuple, or a union whose members have distinct layouts,
/// those of one layout joined item by item.
fn union_of(tuples: Vec<Type>) -> Type {
    let mut members: Vec<Vec<Type>> = Vec::new();
    for tuple in tuples {
        let Type::Tuple(items) = tuple else {
            continue;
        };
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
    let mut members: Vec<Type> = members.into_iter().map(Type::Tuple).collect();
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

/// Adds a creation unless it is there already or a type in it is an error, which was
/// reported where it arose.
fn push(creations: &mut Vec<(ExceptionClass, Type)>, creation: (ExceptionClass, Type)) {
    if !creation.1.has_error() && !creations.contains(&creation) {
        creations.push(creation);
    }
}

/// Returns true when two tuples are laid out alike: as many items, and at each position
/// one type, or two types held as objects.
fn same_layout(left: &[Type], right: &[Type]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| left == right || (held_as_object(left) && held_as_object(right)))
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
mod tests {
    use super::Creations;
    use crate::exception::{BuiltinException, ExceptionClass};
    use crate::types::Type;

    fn builtin(class: BuiltinException) -> ExceptionClass {
        ExceptionClass::Builtin(class)
    }

    fn tuple(items: &[Type]) -> Type {
        Type::Tuple(items.to_vec())
    }

    #[test]
    fn a_base_class_has_the_unicode_errors_five_arguments_when_the_program_can_cause_them() {
        let value_error = builtin(BuiltinException::ValueError);
        let mut creations = Creations::default();
        assert_eq!(creations.arguments_of(&value_error), tuple(&[Type::Str]));
        creations.codec(false, true);
        assert_eq!(
            creations.arguments_of(&value_error),
            Type::Union(vec![
                tuple(&[Type::Str, Type::Bytes, Type::Int, Type::Int, Type::Str]),
                tuple(&[Type::Str])
            ])
        );
        creations.codec(true, false);
        let Type::Union(members) = creations.arguments_of(&value_error) else {
            panic!("a union");
        };
        let object = Type::union(vec![Type::Str, Type::Bytes]);
        assert_eq!(
            members,
            [
                tuple(&[Type::Str, object, Type::Int, Type::Int, Type::Str]),
                tuple(&[Type::Str])
            ]
        );
    }

    #[test]
    fn a_class_the_program_never_creates_has_the_runtime_message() {
        let creations = Creations::default();
        assert_eq!(
            creations.arguments_of(&builtin(BuiltinException::TypeError)),
            tuple(&[Type::Str])
        );
        assert_eq!(
            creations.arguments_of(&builtin(BuiltinException::StopIteration)),
            tuple(&[])
        );
    }

    #[test]
    fn calls_with_other_layouts_become_members_of_a_union() {
        let mut creations = Creations::default();
        let type_error = builtin(BuiltinException::TypeError);
        creations.make(&type_error, &[Type::Str, Type::Int]);
        creations.make(&type_error, &[Type::Int]);
        let Type::Union(members) = creations.arguments_of(&type_error) else {
            panic!("a union");
        };
        assert_eq!(members.len(), 3, "{members:?}");
        assert!(members.contains(&tuple(&[Type::Str, Type::Int])));
        assert!(members.contains(&tuple(&[Type::Int])));
    }

    #[test]
    fn objects_at_one_position_share_a_member() {
        let mut creations = Creations::default();
        let type_error = builtin(BuiltinException::TypeError);
        creations.make(&type_error, &[Type::List(Box::new(Type::Int))]);
        assert_eq!(
            creations.arguments_of(&type_error),
            tuple(&[Type::union(vec![
                Type::Str,
                Type::List(Box::new(Type::Int))
            ])])
        );
    }

    #[test]
    fn an_int_and_a_float_stay_apart_since_their_layouts_differ() {
        let mut creations = Creations::default();
        let class = builtin(BuiltinException::AssertionError);
        creations.make(&class, &[Type::Int]);
        creations.make(&class, &[Type::Float]);
        assert_eq!(
            creations.arguments_of(&class),
            Type::Union(vec![tuple(&[Type::Float]), tuple(&[Type::Int])])
        );
    }

    #[test]
    fn a_base_class_has_the_arguments_of_its_derived_classes() {
        let mut creations = Creations::default();
        creations.key(&Type::Int);
        let lookup = creations.arguments_of(&builtin(BuiltinException::LookupError));
        assert_eq!(
            lookup,
            Type::Union(vec![tuple(&[Type::Int]), tuple(&[Type::Str])])
        );
        assert_eq!(
            creations.arguments_of(&builtin(BuiltinException::IndexError)),
            tuple(&[Type::Str])
        );
    }

    #[test]
    fn an_os_error_with_a_file_name_keeps_two_arguments() {
        let mut creations = Creations::default();
        let class = builtin(BuiltinException::FileNotFoundError);
        creations.make(&class, &[Type::Int, Type::Str, Type::Str]);
        creations.make(&class, &[Type::Int, Type::Str, Type::NoneType]);
        let Type::Union(members) = creations.arguments_of(&class) else {
            panic!("a union");
        };
        assert!(members.contains(&tuple(&[Type::Int, Type::Str])));
        assert!(members.contains(&tuple(&[Type::Int, Type::Str, Type::NoneType])));
        assert_eq!(members.len(), 3, "{members:?}");
    }

    #[test]
    fn an_initializer_applies_to_its_bases_and_derived_classes() {
        let mut creations = Creations::default();
        let base = ExceptionClass::program(
            crate::typed::ClassId(0),
            "Base".to_owned(),
            Vec::new(),
            BuiltinException::Exception,
        );
        let derived = ExceptionClass::program(
            crate::typed::ClassId(1),
            "Derived".to_owned(),
            vec![(crate::typed::ClassId(0), "Base".to_owned())],
            BuiltinException::Exception,
        );
        creations.initialize(&base, &[Type::Float]);
        creations.make(&derived, &[]);
        assert_eq!(
            creations.arguments_of(&derived),
            Type::Union(vec![tuple(&[Type::Float]), tuple(&[])])
        );
    }

    #[test]
    fn an_os_error_that_becomes_a_subclass_applies_to_the_subclasses() {
        let mut creations = Creations::default();
        let os_error = builtin(BuiltinException::OSError);
        creations.make(&os_error, &[Type::Int, Type::Str, Type::NoneType]);
        creations.make(&os_error, &[Type::Float]);
        let Type::Union(members) =
            creations.arguments_of(&builtin(BuiltinException::FileNotFoundError))
        else {
            panic!("a union");
        };
        assert!(members.contains(&tuple(&[Type::Int, Type::Str, Type::NoneType])));
        assert!(!members.contains(&tuple(&[Type::Float])));
    }

    #[test]
    fn covering_and_extending_compare_every_creation() {
        let mut first = Creations::default();
        let mut second = Creations::default();
        second.key(&Type::Str);
        second.codec(true, false);
        assert!(!first.covers(&second));
        first.extend(&second);
        assert!(first.covers(&second));
        assert_eq!(first, second);
    }
}
