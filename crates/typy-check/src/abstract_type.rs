//! The abstract collections of `collections.abc` and `typing` as types (LIB-20, ADR-0086): a
//! `Sequence[int]` is held as the built-in types CPython registers with it, and keeps its own
//! name for messages. A value of one is the built-in collection it holds, so an expression's
//! type is never abstract; a declared type is, and takes a collection of instances of a class
//! derived from the item class when the collection only reads its items.

use std::fmt;

use crate::types::{Type, View};

/// An abstract collection of `typing` and `collections.abc`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AbstractCollection {
    /// `Sequence[T]`.
    Sequence,
    /// `MutableSequence[T]`.
    MutableSequence,
    /// `Mapping[K, V]`.
    Mapping,
    /// `MutableMapping[K, V]`.
    MutableMapping,
    /// `collections.abc.Set[T]`, `typing.AbstractSet[T]`.
    Set,
    /// `MutableSet[T]`.
    MutableSet,
    /// `Collection[T]`.
    Collection,
    /// `Container[T]`.
    Container,
    /// `Reversible[T]`.
    Reversible,
    /// `KeysView[K]`.
    KeysView,
    /// `ValuesView[V]`.
    ValuesView,
    /// `ItemsView[K, V]`.
    ItemsView,
}

/// A declared abstract collection: which one, its type arguments, and the built-in types it
/// holds, one type or their union.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AbstractType {
    /// The abstract collection.
    pub collection: AbstractCollection,
    /// The types it is given, such as `int` for `Sequence[int]`.
    pub arguments: Vec<Type>,
    /// The built-in types it holds, as a value of it is held.
    pub members: Type,
}

impl AbstractCollection {
    /// Returns the abstract collection `name` of `module` names, `typing.AbstractSet` being
    /// `Set`.
    pub(crate) fn named(module: &str, name: &str) -> Option<Self> {
        Some(match (module, name) {
            (_, "Sequence") => Self::Sequence,
            (_, "MutableSequence") => Self::MutableSequence,
            (_, "Mapping") => Self::Mapping,
            (_, "MutableMapping") => Self::MutableMapping,
            ("collections.abc", "Set") | ("typing", "AbstractSet") => Self::Set,
            (_, "MutableSet") => Self::MutableSet,
            (_, "Collection") => Self::Collection,
            (_, "Container") => Self::Container,
            (_, "Reversible") => Self::Reversible,
            (_, "KeysView") => Self::KeysView,
            (_, "ValuesView") => Self::ValuesView,
            (_, "ItemsView") => Self::ItemsView,
            _ => return None,
        })
    }

    /// Returns the collection's name, as an annotation writes it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Sequence => "Sequence",
            Self::MutableSequence => "MutableSequence",
            Self::Mapping => "Mapping",
            Self::MutableMapping => "MutableMapping",
            Self::Set => "Set",
            Self::MutableSet => "MutableSet",
            Self::Collection => "Collection",
            Self::Container => "Container",
            Self::Reversible => "Reversible",
            Self::KeysView => "KeysView",
            Self::ValuesView => "ValuesView",
            Self::ItemsView => "ItemsView",
        }
    }

    /// Returns how many type arguments the collection takes.
    #[must_use]
    pub const fn arity(self) -> usize {
        match self {
            Self::Mapping | Self::MutableMapping | Self::ItemsView => 2,
            _ => 1,
        }
    }

    /// Returns true when the collection only reads its items, so a collection of instances of
    /// a class derived from the item class is one, as `typing` reads it (covariance). The
    /// mutable collections write items, and keep their item type.
    #[must_use]
    pub const fn reads_only(self) -> bool {
        !matches!(
            self,
            Self::MutableSequence | Self::MutableMapping | Self::MutableSet
        )
    }

    /// Returns the type of the collection of items `arguments` names: each built-in type
    /// CPython registers with it, with those items (ADR-0086); `None` for arguments of
    /// another number than it takes.
    pub(crate) fn of(self, arguments: &[Type]) -> Option<Type> {
        if arguments.len() != self.arity() {
            return None;
        }
        let members = self.members(arguments);
        Some(Type::Abstract(Box::new(AbstractType {
            collection: self,
            arguments: arguments.to_vec(),
            members,
        })))
    }

    /// Returns the built-in types the collection of `arguments` holds, which are as many as
    /// it takes.
    fn members(self, arguments: &[Type]) -> Type {
        let first = Box::new(arguments[0].clone());
        let second = || Box::new(arguments.get(1).cloned().unwrap_or(Type::Error));
        // A view of a dict reads only its keys, or only its values, so a dict of any other
        // part fits; the part it does not read is held as `None` here.
        let keys = || Type::DictView(View::Keys, first.clone(), Box::new(Type::NoneType));
        let values = || Type::DictView(View::Values, Box::new(Type::NoneType), first.clone());
        let items = |key: Box<Type>, value: Box<Type>| Type::DictView(View::Items, key, value);
        let pair = match &arguments[0] {
            Type::Tuple(parts) if parts.len() == 2 => {
                Some((Box::new(parts[0].clone()), Box::new(parts[1].clone())))
            }
            _ => None,
        };
        // `str` is a sequence of `str`, and `bytes` and `range` of `int`.
        let mut sequences = vec![Type::List(first.clone()), Type::TupleOf(first.clone())];
        match &arguments[0] {
            Type::Str => sequences.push(Type::Str),
            Type::Int => sequences.extend([Type::Bytes, Type::Range]),
            _ => {}
        }
        let mut sets = vec![Type::Set(first.clone()), keys()];
        sets.extend(pair.clone().map(|(key, value)| items(key, value)));
        match self {
            Self::MutableSequence => Type::List(first),
            Self::Mapping | Self::MutableMapping => Type::Dict(first, second()),
            Self::MutableSet => Type::Set(first),
            Self::Set => Type::union(sets),
            Self::Sequence => Type::union(sequences),
            Self::Reversible => {
                sequences.extend([
                    Type::Dict(first.clone(), Box::new(Type::NoneType)),
                    keys(),
                    values(),
                ]);
                sequences.extend(pair.map(|(key, value)| items(key, value)));
                Type::union(sequences)
            }
            Self::Collection | Self::Container => {
                sequences.extend(sets);
                sequences.push(values());
                Type::union(sequences)
            }
            Self::KeysView => keys(),
            Self::ValuesView => values(),
            Self::ItemsView => items(first, second()),
        }
    }
}

impl AbstractType {
    /// Returns true when a value of type `value`, which is not abstract, is one of this
    /// collection: of a member's type, or for a collection that only reads its items, of a
    /// member's kind whose items are instances of classes derived from the member's.
    #[must_use]
    pub fn takes(&self, value: &Type) -> bool {
        value.is_assignable_to(&self.members) || self.member_for(value).is_some()
    }

    /// Returns the built-in types a display builds a value of the collection as: its members
    /// but those that read one part of a dict, whose other part only the value gives.
    #[must_use]
    pub fn display_members(&self) -> Type {
        if !self.reads_parts() {
            return self.members.clone();
        }
        let members = match &self.members {
            Type::Union(members) => members.as_slice(),
            single => std::slice::from_ref(single),
        };
        let unread = |part: &Type| *part == Type::NoneType;
        Type::union(
            members
                .iter()
                .filter(|member| match member {
                    Type::Dict(_, part)
                    | Type::DictView(View::Keys, _, part)
                    | Type::DictView(View::Values, part, _) => !unread(part),
                    _ => true,
                })
                .cloned()
                .collect(),
        )
    }

    /// Returns true when the collection holds members that read one part of a dict.
    const fn reads_parts(&self) -> bool {
        !matches!(
            self.collection,
            AbstractCollection::Mapping
                | AbstractCollection::MutableMapping
                | AbstractCollection::ItemsView
        )
    }

    /// Returns the member a value of type `value` is held as when it is not of a member's
    /// type itself: one of its kind whose items, or whose part a view reads, it passes for.
    #[must_use]
    pub fn member_for(&self, value: &Type) -> Option<&Type> {
        let members = match &self.members {
            Type::Union(members) => members.as_slice(),
            single => std::slice::from_ref(single),
        };
        let parts = self.reads_parts();
        members
            .iter()
            .find(|member| fits_member(value, member, (self.collection.reads_only(), parts)))
    }
}

/// Returns true when a value of type `value` is held as `member`: of its kind, with items
/// that pass where the member's do. With `covariant`, an instance of a derived class passes
/// for an item of the base, as the same pointer; with `parts`, a member that reads one part
/// of a dict, a view or a dict read for its keys, takes a dict of any other part, which the
/// member holds as `None`.
fn fits_member(value: &Type, member: &Type, (covariant, parts): (bool, bool)) -> bool {
    let item = |given: &Type, wanted: &Type| {
        given.matches(wanted) || (covariant && given.is_upcast_to(wanted))
    };
    match (value, member) {
        (Type::List(given), Type::List(wanted))
        | (Type::TupleOf(given), Type::TupleOf(wanted))
        | (Type::Set(given), Type::Set(wanted)) => item(given, wanted),
        (Type::Tuple(items), Type::TupleOf(wanted)) => {
            items.iter().all(|given| item(given, wanted))
        }
        (Type::Dict(key, _), Type::Dict(wanted, unread))
        | (Type::DictView(View::Keys, key, _), Type::DictView(View::Keys, wanted, unread))
            if parts && **unread == Type::NoneType =>
        {
            item(key, wanted)
        }
        (Type::DictView(View::Values, _, value), Type::DictView(View::Values, unread, wanted))
            if parts && **unread == Type::NoneType =>
        {
            item(value, wanted)
        }
        (Type::Dict(key, given), Type::Dict(wanted_key, wanted)) => {
            key.matches(wanted_key) && item(given, wanted)
        }
        (
            Type::DictView(View::Items, key, given),
            Type::DictView(View::Items, wanted_key, wanted),
        ) => item(key, wanted_key) && item(given, wanted),
        _ => false,
    }
}

impl fmt::Display for AbstractType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let arguments: Vec<String> = self.arguments.iter().map(ToString::to_string).collect();
        write!(
            formatter,
            "{}[{}]",
            self.collection.name(),
            arguments.join(", ")
        )
    }
}

impl Type {
    /// Returns true when the type is, or holds, an abstract collection.
    #[must_use]
    pub fn has_abstract(&self) -> bool {
        let mut found = false;
        self.visit(&mut |ty| found |= matches!(ty, Self::Abstract(_)));
        found
    }

    /// Returns the type with each abstract collection in it replaced by the built-in types it
    /// holds, the type of a value of it.
    #[must_use]
    pub fn without_abstract(&self) -> Self {
        if !self.has_abstract() {
            return self.clone();
        }
        let inner = |item: &Self| Box::new(item.without_abstract());
        match self {
            Self::Abstract(collection) => collection.members.without_abstract(),
            Self::List(item) => Self::List(inner(item)),
            Self::TupleOf(item) => Self::TupleOf(inner(item)),
            Self::Set(item) => Self::Set(inner(item)),
            Self::Iterator(item) => Self::Iterator(inner(item)),
            Self::Iterable(item) => Self::Iterable(inner(item)),
            Self::Optional(item) => Self::optional(*inner(item)),
            Self::Dict(key, value) => Self::Dict(inner(key), inner(value)),
            Self::DictView(view, key, value) => Self::DictView(*view, inner(key), inner(value)),
            Self::Tuple(items) => Self::Tuple(items.iter().map(|item| *inner(item)).collect()),
            Self::Union(items) => Self::union(items.iter().map(|item| *inner(item)).collect()),
            Self::Callable {
                parameters,
                returns,
            } => Self::Callable {
                parameters: parameters.iter().map(|item| *inner(item)).collect(),
                returns: inner(returns),
            },
            Self::Generator {
                yields,
                sends,
                returns,
            } => Self::Generator {
                yields: inner(yields),
                sends: inner(sends),
                returns: inner(returns),
            },
            other => other.clone(),
        }
    }
}
