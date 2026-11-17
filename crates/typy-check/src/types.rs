//! The static types: the scalars of 0.1, and from 0.2 the types built from other types.

use std::fmt;
use std::sync::Arc;

use crate::class_type::InstanceClass;
use crate::exception::{BuiltinException, ExceptionClass};

/// The static type of a value (TYP-11). Types built from others, such as a tuple, hold their
/// parts, and two types are equal when their structure is.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Type {
    /// `int`, without a size limit (DAT-01).
    Int,
    /// `float`, an IEEE 754 double (DAT-02).
    Float,
    /// `bool` (DAT-03).
    Bool,
    /// `str` (DAT-05).
    Str,
    /// `bytes`, an immutable sequence of integers from 0 to 255 (DAT-06).
    Bytes,
    /// The type of `None` (DAT-04).
    NoneType,
    /// `tuple[A, B, ...]`: a fixed number of items, each with its own type (DAT-08).
    Tuple(Vec<Type>),
    /// `list[T]`: a mutable sequence shared by every name bound to it (DAT-07).
    List(Box<Type>),
    /// `tuple[T, ...]`: an immutable sequence of any number of items of one type (DAT-14).
    TupleOf(Box<Type>),
    /// `set[T]`: a mutable set of hashable items, shared by every name bound to it (DAT-10).
    Set(Box<Type>),
    /// `dict[K, V]`: a mutable mapping of hashable keys to values, in insertion order, shared by
    /// every name bound to it (DAT-09).
    Dict(Box<Type>, Box<Type>),
    /// A view of a dict's keys, values or items, which reads the dict as it changes (DAT-09).
    DictView(View, Box<Type>, Box<Type>),
    /// `T | None`: a value of `T`, or `None` (TYP-06, ADR-0011). `T` is never `None` or
    /// optional itself; `optional` builds the type and keeps that rule.
    Optional(Box<Type>),
    /// `A | B | ...`: a value of one of two or more types other than `None`, in the normal
    /// form `Type::union` builds (TYP-14, ADR-0027).
    Union(Vec<Type>),
    /// A type parameter of a generic function, as its body is checked: an opaque type that
    /// offers what its bound offers, and which no copy of the body holds (GEN-01, ADR-0023).
    Parameter(Arc<crate::generic::TypeParameter>),
    /// An instance of a class of the program, or of a class derived from it (CLS-01, CLS-04).
    Instance(Arc<InstanceClass>),
    /// An exception of a class (ADR-0013).
    Exception(ExceptionClass),
    /// An iterator that `iter()` makes, of the type of its items (ADR-0014).
    Iterator(Box<Type>),
    /// `Iterable[T]`: any built-in iterable or iterator of items of type `T`, held as the
    /// object itself, which a loop or `iter()` reads through an iterator (TYP-15).
    Iterable(Box<Type>),
    /// `types.ModuleType`: a module whose attributes are not known when the program is
    /// checked (MOD-10, ADR-0015).
    AnyModule,
    /// The traceback `__exit__` receives, which holds no frames in TyPy (CST-10).
    Traceback,
    /// A text file, `_io.TextIOWrapper`: `sys.stdout`, `sys.stderr` and the files `open`
    /// gives (LIB-04).
    TextFile,
    /// A binary file `open` gives, of the class CPython makes for its mode and buffering
    /// (LIB-14).
    BinaryFile(BinaryKind),
    /// A file `open` gives for a file descriptor, whose `name` is that `int`: a text file, or
    /// a binary one of this class (LIB-14).
    DescriptorFile(Option<BinaryKind>),
    /// `range` held as a value: a sequence of `int` (DAT-17).
    Range,
    /// A `slice`, of bounds that are each an `int` or `None` (CLS-23).
    Slice,
    /// A `typing.TypeAliasType`, the value a `type` statement binds (SMT-12).
    TypeAlias,
    /// `sys.version_info` held as a value, whose items are those of CPython 3.15.0 (LIB-16).
    VersionInfo,
    /// The type of `NotImplemented`, which an operator or comparison method returns to give
    /// way to the other operand's reflected method (CLS-21).
    NotImplemented,
    /// The module object of the module of this qualified name, whose attributes are known
    /// when the program is checked (MOD-10, ADR-0015).
    Module(String),
    /// A C library a `ctypes` declaration loads, by its index in the program's libraries; a
    /// program calls its declared functions and uses it no other way (FFI-01, ADR-0047).
    ForeignLibrary(u32),
    /// A type object, of the type it stands for, or of any type when that is known only
    /// when the program runs, as for `type(x)` of an optional `x` (CLS-12).
    TypeObject(Option<Box<TypeObjectOf>>),
    /// The type of an expression that already has a diagnostic; every check accepts it, so one cause gives one error (DIA-04).
    Error,
}

/// The type a type object stands for (CLS-12). Each is one type object of the program, so
/// two type objects of one `TypeObjectOf` are the same object.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TypeObjectOf {
    /// `int`.
    Int,
    /// `float`.
    Float,
    /// `bool`.
    Bool,
    /// `str`.
    Str,
    /// `bytes`.
    Bytes,
    /// The type of `None`, `NoneType`.
    NoneType,
    /// `list`, whatever its items.
    List,
    /// `tuple`, of a fixed or any number of items.
    Tuple,
    /// `set`.
    Set,
    /// `dict`.
    Dict,
    /// The type of one kind of view of a dict.
    View(View),
    /// `object`, the root of every type.
    Object,
    /// `type`, the type of every type object.
    Type,
    /// `slice` (CLS-23).
    Slice,
    /// `range`.
    Range,
    /// A class of the program, with its name.
    Instance(crate::typed::ClassId, String),
    /// An exception class.
    Exception(ExceptionClass),
    /// `collections.abc.Iterable` or `Iterator`, as the class of an `isinstance` test, with
    /// the classes of the program that define the methods it asks for (TYP-17).
    Abstract {
        /// Which abstract class.
        kind: AbstractClass,
        /// The classes that define the methods themselves, each with its bases; their
        /// subclasses inherit the methods.
        implementers: Vec<(crate::typed::ClassId, Vec<crate::typed::ClassId>)>,
    },
}

/// An abstract class of `collections.abc` that `isinstance` tests by the methods a class
/// defines, as CPython's `__subclasshook__` does (TYP-17).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AbstractClass {
    /// `Iterable`: a class with `__iter__`.
    Iterable,
    /// `Iterator`: a class with `__iter__` and `__next__`.
    Iterator,
}

impl TypeObjectOf {
    /// Returns the type of type objects of `of`, any type object when it is `None`.
    #[must_use]
    pub fn type_of(of: Option<Self>) -> Type {
        Type::TypeObject(of.map(Box::new))
    }

    /// Returns the type object of the values of `ty`, or `None` when it depends on the value,
    /// as it does for an optional type.
    #[must_use]
    pub fn of(ty: &Type) -> Option<Self> {
        Some(match ty {
            Type::Int => Self::Int,
            Type::Float => Self::Float,
            Type::Bool => Self::Bool,
            Type::Str => Self::Str,
            Type::Bytes => Self::Bytes,
            Type::NoneType => Self::NoneType,
            Type::List(_) => Self::List,
            Type::Tuple(_) | Type::TupleOf(_) => Self::Tuple,
            Type::Set(_) => Self::Set,
            Type::Dict(..) => Self::Dict,
            Type::DictView(view, ..) => Self::View(*view),
            // A copy of a generic class answers as the generic class (ADR-0028).
            Type::Instance(class) => match &class.generic {
                Some((root, _)) => Self::Instance(
                    *root,
                    class.name.split('[').next().unwrap_or_default().to_owned(),
                ),
                None => Self::Instance(class.id, class.name.clone()),
            },
            Type::TypeObject(_) => Self::Type,
            Type::Range => Self::Range,
            Type::Slice => Self::Slice,
            // An exception's class may derive from its static class, so it is read from the
            // exception when the program runs.
            Type::Exception(_)
            | Type::Iterator(_)
            | Type::Iterable(_)
            | Type::AnyModule
            | Type::TextFile
            | Type::BinaryFile(_)
            | Type::DescriptorFile(_)
            | Type::Traceback
            | Type::Module(_)
            | Type::ForeignLibrary(_)
            | Type::TypeAlias
            | Type::VersionInfo
            | Type::NotImplemented
            | Type::Optional(_)
            | Type::Union(_)
            | Type::Parameter(_)
            | Type::Error => {
                return None;
            }
        })
    }

    /// Returns the type object a built-in name gives as a value, such as `int`.
    #[must_use]
    pub fn named(name: &str) -> Option<Self> {
        Some(match name {
            "int" => Self::Int,
            "float" => Self::Float,
            "bool" => Self::Bool,
            "str" => Self::Str,
            "bytes" => Self::Bytes,
            "list" => Self::List,
            "tuple" => Self::Tuple,
            "set" => Self::Set,
            "dict" => Self::Dict,
            "object" => Self::Object,
            "type" => Self::Type,
            "range" => Self::Range,
            "slice" => Self::Slice,
            _ => Self::Exception(ExceptionClass::Builtin(BuiltinException::named(name)?)),
        })
    }

    /// Returns the type's `__name__`.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Int => "int",
            Self::Float => "float",
            Self::Bool => "bool",
            Self::Str => "str",
            Self::Bytes => "bytes",
            Self::NoneType => "NoneType",
            Self::List => "list",
            Self::Tuple => "tuple",
            Self::Set => "set",
            Self::Dict => "dict",
            Self::View(view) => view.name(),
            Self::Object => "object",
            Self::Type => "type",
            Self::Range => "range",
            Self::Slice => "slice",
            Self::Instance(_, name) => name,
            Self::Exception(class) => class.name(),
            Self::Abstract {
                kind: AbstractClass::Iterable,
                ..
            } => "Iterable",
            Self::Abstract {
                kind: AbstractClass::Iterator,
                ..
            } => "Iterator",
        }
    }
}

/// The class of a binary file, which its mode and buffering decide (LIB-14).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BinaryKind {
    /// `_io.BufferedReader`: a mode that only reads.
    Reader,
    /// `_io.BufferedWriter`: a mode that only writes.
    Writer,
    /// `_io.BufferedRandom`: a mode with `+`.
    Random,
    /// `_io.FileIO`: any binary mode with `buffering=0`.
    Raw,
}

impl BinaryKind {
    /// Returns the name of CPython's class.
    #[must_use]
    pub const fn class_name(self) -> &'static str {
        match self {
            Self::Reader => "BufferedReader",
            Self::Writer => "BufferedWriter",
            Self::Random => "BufferedRandom",
            Self::Raw => "FileIO",
        }
    }
}

/// What a view of a dict shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum View {
    /// `dict.keys()`.
    Keys,
    /// `dict.values()`.
    Values,
    /// `dict.items()`.
    Items,
}

impl View {
    /// Returns the name CPython gives the view's type.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Keys => "dict_keys",
            Self::Values => "dict_values",
            Self::Items => "dict_items",
        }
    }

    /// Returns the code the runtime takes for the view: 0, 1 or 2.
    #[must_use]
    pub const fn code(self) -> i64 {
        match self {
            Self::Keys => 0,
            Self::Values => 1,
            Self::Items => 2,
        }
    }

    /// Returns the type of what the view of a dict of `key` and `value` gives.
    #[must_use]
    pub fn item(self, key: &Type, value: &Type) -> Type {
        match self {
            Self::Keys => key.clone(),
            Self::Values => value.clone(),
            Self::Items => Type::Tuple(vec![key.clone(), value.clone()]),
        }
    }
}

impl Type {
    /// Returns the type of the instances of a class, shared so a type stays small.
    #[must_use]
    pub fn instance(class: InstanceClass) -> Self {
        Self::Instance(Arc::new(class))
    }

    /// Returns `T | None`: `None` for `None`, and `T | None` itself for an optional `T`, as
    /// Python's typing joins them.
    #[must_use]
    pub fn optional(inner: Self) -> Self {
        match inner {
            Self::NoneType | Self::Optional(_) | Self::Error => inner,
            other => Self::Optional(Box::new(other)),
        }
    }

    /// Returns true when a value of this type may be stored where `target` is declared: a
    /// value of the same type, `None` or a `T` where `T | None` is declared, which the checker
    /// wraps (TYP-03, TYP-06, TYP-09).
    #[must_use]
    pub fn is_assignable_to(&self, target: &Self) -> bool {
        match (self, target) {
            (Self::Error, _)
            | (_, Self::Error)
            | (Self::NoneType, Self::Optional(_))
            | (Self::Module(_), Self::AnyModule)
            | (Self::TypeObject(_), Self::TypeObject(None)) => true,
            // An exception of a class is one of each of its bases (EXC-01), and so is an
            // instance (CLS-04).
            (Self::Exception(class), Self::Exception(base)) => class.derives_from(base),
            (Self::Instance(class), Self::Instance(base)) => class.derives_from(base),
            // A bounded type parameter is held where its bound is (GEN-02).
            (Self::Parameter(parameter), _) if *self != *target => parameter
                .bound
                .as_ref()
                .is_some_and(|bound| bound.is_assignable_to(target)),
            // A union is held where each of its members is, and a member where its union is
            // (TYP-14).
            (Self::Union(members), _) => {
                members.iter().all(|member| member.is_assignable_to(target))
            }
            (_, Self::Union(members)) if !matches!(self, Self::Optional(_)) => {
                members.iter().any(|member| self.is_assignable_to(member))
            }
            (Self::Optional(_), _) => self.matches(target) || self.is_upcast_to(target),
            (_, Self::Optional(inner)) => self.is_assignable_to(inner),
            (_, Self::Iterable(item)) => self.iterable_item().is_some_and(|own| own.matches(item)),
            _ => self.matches(target),
        }
    }

    /// Returns true when a value of this type is an instance, or an optional one, of a class
    /// that derives from the class of `target`, which is of the same kind (CLS-04).
    #[must_use]
    pub fn is_upcast_to(&self, target: &Self) -> bool {
        match (self, target) {
            (Self::Instance(class), Self::Instance(base)) => class.derives_from(base),
            (Self::Optional(inner), Self::Optional(base)) => inner.is_upcast_to(base),
            (Self::Parameter(_), _) => *self != *target && self.is_assignable_to(target),
            // A union with `None` is an optional of the other members, so a union target
            // never holds `None`.
            (Self::Optional(_), Self::Union(_)) => false,
            // A value becomes a union, or a wider one, or the class every member derives
            // from, as the object it is (TYP-14).
            // An optional target is converted as an optional is: its base, then wrapped.
            (Self::Union(_), _) | (_, Self::Union(_)) if !matches!(target, Self::Optional(_)) => {
                !self.matches(target) && self.is_assignable_to(target)
            }
            _ => false,
        }
    }

    /// Returns true when the two types are the same, item by item, an error matching anything.
    /// The items of lists, sets, dicts and tuples must match, since the runtime stores each item
    /// by its type and a stored item is never converted.
    #[must_use]
    pub fn matches(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Error, _) | (_, Self::Error) => true,
            (Self::List(item), Self::List(other))
            | (Self::Iterator(item), Self::Iterator(other))
            | (Self::Iterable(item), Self::Iterable(other))
            | (Self::TupleOf(item), Self::TupleOf(other))
            | (Self::Set(item), Self::Set(other))
            | (Self::Optional(item), Self::Optional(other)) => item.matches(other),
            (Self::Dict(key, value), Self::Dict(other_key, other_value)) => {
                key.matches(other_key) && value.matches(other_value)
            }
            (
                Self::DictView(view, key, value),
                Self::DictView(other_view, other_key, other_value),
            ) => view == other_view && key.matches(other_key) && value.matches(other_value),
            (Self::Tuple(items), Self::Tuple(others)) => {
                items.len() == others.len()
                    && items
                        .iter()
                        .zip(others)
                        .all(|(item, other)| item.matches(other))
            }
            _ => self == other,
        }
    }

    /// Returns the class of an instance, or of an exception of a class of the program.
    #[must_use]
    pub fn class_id(&self) -> Option<crate::typed::ClassId> {
        match self {
            Self::Instance(class) => Some(class.id),
            Self::Exception(exception) => exception.class_id(),
            _ => None,
        }
    }

    /// Returns true when the type is `Error`, or holds it, so a check of it would repeat an error.
    #[must_use]
    pub fn has_error(&self) -> bool {
        match self {
            Self::Error => true,
            Self::Tuple(items) => items.iter().any(Self::has_error),
            Self::List(item)
            | Self::TupleOf(item)
            | Self::Set(item)
            | Self::Iterator(item)
            | Self::Iterable(item) => item.has_error(),
            Self::Dict(key, value) | Self::DictView(_, key, value) => {
                key.has_error() || value.has_error()
            }
            Self::Optional(inner) => inner.has_error(),
            _ => false,
        }
    }

    /// Returns true when `==` and `!=` accept a value of this type against one of `other`: the
    /// same type, two numbers, or tuples whose items accept each other (EXP-06).
    #[must_use]
    pub fn can_test_equality(&self, other: &Self) -> bool {
        match (self, other) {
            // Two type objects are equal when they are one object (CLS-12).
            (Self::Error, _) | (_, Self::Error) | (Self::TypeObject(_), Self::TypeObject(_)) => {
                true
            }
            // The runtime compares items by the list's own item kind, so two lists compare when
            // their items have one type.
            (Self::List(left), Self::List(right))
            | (Self::TupleOf(left), Self::TupleOf(right))
            | (Self::Set(left), Self::Set(right)) => left == right,
            (Self::Dict(..), Self::Dict(..)) => self == other,
            // An instance is equal only to itself, so a class compares with its bases and its
            // subclasses, whose instances it may be (CLS-04).
            (Self::Instance(left), Self::Instance(right)) => {
                left.derives_from(right) || right.derives_from(left)
            }
            // Views compare by the rules of `body/view_operator.rs` (DAT-16), an optional value
            // is compared with `None` by `is`, and iterators by `is`. A union in a variable is
            // compared member by member, and one computed in place is not compared (TYP-14).
            (
                Self::Union(_)
                | Self::DictView(..)
                | Self::Optional(_)
                | Self::Iterator(_)
                | Self::Iterable(_)
                | Self::Module(_)
                | Self::TypeAlias
                | Self::VersionInfo
                | Self::AnyModule
                | Self::Traceback,
                _,
            )
            | (
                _,
                Self::Union(_)
                | Self::DictView(..)
                | Self::Optional(_)
                | Self::Iterator(_)
                | Self::Iterable(_)
                | Self::Module(_)
                | Self::TypeAlias
                | Self::VersionInfo
                | Self::AnyModule
                | Self::Traceback,
            ) => false,
            (Self::Tuple(left), Self::Tuple(right)) => {
                left.len() == right.len()
                    && left
                        .iter()
                        .zip(right)
                        .all(|(left, right)| left.can_test_equality(right))
            }
            _ => self == other || (self.is_numeric() && other.is_numeric()),
        }
    }

    /// Returns true when `<`, `<=`, `>` and `>=` accept a value of this type against one of
    /// `other`: the same type other than `None`, two numbers, or tuples whose items order
    /// against each other (EXP-06). CPython orders tuples by the first pair of items that are
    /// not equal, so a pair of `None` items, which are always equal, is never ordered and is
    /// accepted inside a tuple. Two sets of one item type order as subsets and supersets.
    #[must_use]
    pub fn can_order(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Error, _) | (_, Self::Error) => true,
            // Two lists or tuples of any length order item by item, so their items must order.
            (Self::TupleOf(left), Self::TupleOf(right)) | (Self::List(left), Self::List(right)) => {
                left == right && left.can_order(right)
            }
            (Self::Set(left), Self::Set(right)) => left == right,
            // Instances order through `__lt__`, or a reflected `__gt__` (CLS-07).
            (Self::Instance(left), Self::Instance(right)) => {
                (left.derives_from(right) || right.derives_from(left))
                    && (left.operators.orders() || right.operators.orders())
            }
            (Self::Tuple(left), Self::Tuple(right)) => {
                left.len() == right.len()
                    && left.iter().zip(right).all(|(left, right)| {
                        (*left == Self::NoneType && *right == Self::NoneType)
                            || left.can_order(right)
                    })
            }
            // A class orders its instances with `__lt__` and the other operator methods, which
            // are planned for 0.4 (CLS-07).
            (
                Self::Parameter(_)
                | Self::Union(_)
                | Self::NoneType
                | Self::Dict(..)
                | Self::DictView(..)
                | Self::Optional(_)
                | Self::Instance(..)
                | Self::Exception(_)
                | Self::Iterator(_)
                | Self::Iterable(_)
                | Self::Module(_)
                | Self::TypeAlias
                | Self::VersionInfo
                | Self::AnyModule
                | Self::Traceback
                | Self::Range
                | Self::Slice
                | Self::TypeObject(_),
                _,
            )
            | (
                _,
                Self::Parameter(_)
                | Self::Union(_)
                | Self::NoneType
                | Self::Dict(..)
                | Self::DictView(..)
                | Self::Optional(_)
                | Self::Instance(..)
                | Self::Exception(_)
                | Self::Iterator(_)
                | Self::Iterable(_)
                | Self::Module(_)
                | Self::TypeAlias
                | Self::VersionInfo
                | Self::AnyModule
                | Self::Traceback
                | Self::Range
                | Self::Slice
                | Self::TypeObject(_),
            ) => false,
            _ => self == other || (self.is_numeric() && other.is_numeric()),
        }
    }

    /// Returns the item type of a sequence stored as a runtime list object: a `list[T]` or a
    /// `tuple[T, ...]`.
    #[must_use]
    pub fn sequence_item(&self) -> Option<&Self> {
        match self {
            Self::List(item) | Self::TupleOf(item) => Some(item),
            _ => None,
        }
    }

    /// Returns `tuple[T, ...]` for a fixed tuple whose items are all of type `item`, which it
    /// converts to where that type is expected (DAT-14).
    #[must_use]
    pub fn fits_tuple_of(&self, item: &Self) -> bool {
        matches!(self, Self::Tuple(items) if items.iter().all(|each| each.matches(item)))
    }

    /// Returns true when a value of this type has a hash, so it may be an item of a set or a key
    /// of a dict: not a list, a set, a dict or a view, which change, and not a tuple that holds
    /// one (DAT-09, DAT-10).
    #[must_use]
    pub fn is_hashable(&self) -> bool {
        match self {
            // An iterable may be a list, which has no hash.
            Self::List(_)
            | Self::Set(_)
            | Self::Dict(..)
            | Self::DictView(..)
            | Self::Iterable(_) => false,
            Self::Tuple(items) => items.iter().all(Self::is_hashable),
            Self::TupleOf(item) | Self::Optional(item) => item.is_hashable(),
            // A class that defines `__eq__` and not `__hash__` has no hash (CLS-07).
            Self::Instance(class) => class.operators.hashable(),
            _ => true,
        }
    }

    /// Returns true for `int` and `float`.
    #[must_use]
    pub fn is_numeric(&self) -> bool {
        matches!(self, Self::Int | Self::Float)
    }

    /// Returns true when a value of this type may be used in a truth test (EXP-10).
    #[must_use]
    pub fn is_truth_testable(&self) -> bool {
        // An iterable's truth depends on which collection it is, which it does not say.
        !matches!(self, Self::Iterable(_))
    }

    /// Returns the type of the items a loop over a value of this type reads, when it is one
    /// that `Iterable[T]` takes: a built-in iterable or an iterator (TYP-15).
    ///
    /// See also `file_line`, the line a file gives.
    #[must_use]
    pub fn iterable_item(&self) -> Option<Type> {
        if let Some(line) = self.file_line() {
            return Some(line);
        }
        match self {
            Self::List(item)
            | Self::TupleOf(item)
            | Self::Set(item)
            | Self::Dict(item, _)
            | Self::Iterator(item)
            | Self::Iterable(item) => Some(item.as_ref().clone()),
            Self::DictView(view, key, value) => Some(view.item(key, value)),
            Self::Str => Some(Self::Str),
            Self::Bytes | Self::Range => Some(Self::Int),
            // A fixed tuple whose items have one type is held as a `tuple[T, ...]` first.
            Self::Tuple(items) => {
                let (first, rest) = items.split_first()?;
                rest.iter().all(|item| item == first).then(|| first.clone())
            }
            _ => None,
        }
    }

    /// Returns the name CPython's messages give the class of a value of the type, such as
    /// `list` for `list[int]`, and `NoneType` for an optional value, which fails when it is
    /// `None`.
    #[must_use]
    pub fn class_name(&self) -> String {
        match self {
            Self::List(_) => "list".to_owned(),
            Self::Tuple(_) | Self::TupleOf(_) => "tuple".to_owned(),
            Self::Set(_) => "set".to_owned(),
            Self::Dict(..) => "dict".to_owned(),
            Self::Optional(_) | Self::NoneType => "NoneType".to_owned(),
            Self::Instance(class) => class.name.clone(),
            other => other.to_string(),
        }
    }

    /// Returns the type of a line of a file: `str` for a text file, `bytes` for a binary one,
    /// and `None` for any other type.
    #[must_use]
    pub const fn file_line(&self) -> Option<Type> {
        match self {
            Self::TextFile | Self::DescriptorFile(None) => Some(Self::Str),
            Self::BinaryFile(_) | Self::DescriptorFile(Some(_)) => Some(Self::Bytes),
            _ => None,
        }
    }

    /// Returns whether this is a file, text or binary.
    #[must_use]
    pub const fn is_file(&self) -> bool {
        matches!(
            self,
            Self::TextFile | Self::BinaryFile(_) | Self::DescriptorFile(_)
        )
    }

    /// Returns the class of a binary file, or `None` for a text file and any other type.
    #[must_use]
    pub const fn binary_kind(&self) -> Option<BinaryKind> {
        match self {
            Self::BinaryFile(kind) | Self::DescriptorFile(Some(kind)) => Some(*kind),
            _ => None,
        }
    }

    /// Returns the name of CPython's class of a file, or `None` for any other type.
    #[must_use]
    pub const fn file_class_name(&self) -> Option<&'static str> {
        match self {
            Self::TextFile | Self::DescriptorFile(None) => Some("TextIOWrapper"),
            Self::BinaryFile(kind) | Self::DescriptorFile(Some(kind)) => Some(kind.class_name()),
            _ => None,
        }
    }

    /// Returns the spelling with its article, for messages: `an int`, `a str`, `None`.
    #[must_use]
    pub fn with_article(&self) -> String {
        match self {
            Self::Int => "an int".to_owned(),
            Self::NoneType => "None".to_owned(),
            Self::Instance(class) if class.name.starts_with(['A', 'E', 'I', 'O', 'U']) => {
                format!("an {class}")
            }
            Self::Exception(class) if class.name().starts_with(['A', 'E', 'I', 'O', 'U']) => {
                format!("an {class}")
            }
            Self::Iterator(_) | Self::Iterable(_) => format!("an {self}"),
            Self::Parameter(parameter) => format!("a value of type {}", parameter.name),
            // The article reads with the first member, as in `an A | B`.
            Self::Union(members) => {
                let rest: Vec<String> = members[1..].iter().map(ToString::to_string).collect();
                format!("{} | {}", members[0].with_article(), rest.join(" | "))
            }
            // The article reads with the first member, as in `an int | None`.
            Self::Optional(inner) => {
                format!("{} | None", inner.with_article())
            }
            other => format!("a {other}"),
        }
    }

    /// Returns true when `print`, `str` and `repr` accept a value of this type.
    #[must_use]
    pub fn is_printable(&self) -> bool {
        true
    }
}

impl fmt::Display for Type {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int => formatter.write_str("int"),
            Self::Float => formatter.write_str("float"),
            Self::Bool => formatter.write_str("bool"),
            Self::Str => formatter.write_str("str"),
            Self::Bytes => formatter.write_str("bytes"),
            Self::NoneType => formatter.write_str("None"),
            Self::Tuple(items) if items.is_empty() => formatter.write_str("tuple[()]"),
            Self::Tuple(items) => {
                formatter.write_str("tuple[")?;
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        formatter.write_str(", ")?;
                    }
                    write!(formatter, "{item}")?;
                }
                formatter.write_str("]")
            }
            Self::List(item) => write!(formatter, "list[{item}]"),
            Self::TupleOf(item) => write!(formatter, "tuple[{item}, ...]"),
            Self::Set(item) => write!(formatter, "set[{item}]"),
            Self::Dict(key, value) => write!(formatter, "dict[{key}, {value}]"),
            Self::DictView(view, key, value) => {
                write!(formatter, "{}[{key}, {value}]", view.name())
            }
            Self::Optional(inner) => write!(formatter, "{inner} | None"),
            Self::Union(members) => {
                let text: Vec<String> = members.iter().map(ToString::to_string).collect();
                formatter.write_str(&text.join(" | "))
            }
            Self::Parameter(parameter) => formatter.write_str(&parameter.name),
            Self::Instance(class) => write!(formatter, "{class}"),
            Self::Exception(class) => write!(formatter, "{class}"),
            Self::Iterator(item) => write!(formatter, "Iterator[{item}]"),
            Self::Iterable(item) => write!(formatter, "Iterable[{item}]"),
            Self::AnyModule => formatter.write_str("ModuleType"),
            Self::TypeAlias => formatter.write_str("TypeAliasType"),
            Self::VersionInfo => formatter.write_str("sys.version_info"),
            Self::NotImplemented => formatter.write_str("NotImplementedType"),
            Self::TextFile => formatter.write_str("TextIOWrapper"),
            Self::BinaryFile(kind) => formatter.write_str(kind.class_name()),
            Self::DescriptorFile(kind) => write!(
                formatter,
                "{} of a file descriptor",
                kind.map_or("TextIOWrapper", BinaryKind::class_name)
            ),
            Self::Traceback => formatter.write_str("TracebackType"),
            Self::Range => formatter.write_str("range"),
            Self::Slice => formatter.write_str("slice"),
            Self::Module(name) => write!(formatter, "module `{name}`"),
            Self::ForeignLibrary(_) => formatter.write_str("CDLL"),
            Self::TypeObject(Some(of)) => write!(formatter, "type[{}]", of.name()),
            Self::TypeObject(None) => formatter.write_str("type"),
            Self::Error => formatter.write_str("<error>"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Type;

    #[test]
    fn an_int_is_not_assignable_to_a_float() {
        assert!(!Type::Int.is_assignable_to(&Type::Float));
        assert!(!Type::Float.is_assignable_to(&Type::Int));
        assert!(!Type::Bool.is_assignable_to(&Type::Int));
    }

    #[test]
    fn equal_types_and_error_are_assignable() {
        for ty in [
            Type::Int,
            Type::Float,
            Type::Bool,
            Type::Str,
            Type::NoneType,
        ] {
            assert!(ty.is_assignable_to(&ty));
            assert!(Type::Error.is_assignable_to(&ty));
            assert!(ty.is_assignable_to(&Type::Error));
            assert!(ty.is_truth_testable());
            assert!(ty.is_printable());
        }
    }

    #[test]
    fn instances_compare_by_identity_and_have_no_order() {
        use crate::class_type::InstanceClass;
        use crate::typed::ClassId;
        let point = Type::instance(InstanceClass::root(ClassId(0), "Point"));
        let item = Type::instance(InstanceClass::root(ClassId(1), "Item"));
        assert!(point.can_test_equality(&point));
        assert!(!point.can_test_equality(&item));
        assert!(!point.can_order(&point));
        assert!(!Type::Int.can_order(&point));
        assert!(point.is_hashable());
        assert!(point.is_assignable_to(&Type::optional(point.clone())));
        assert_eq!(
            (point.with_article(), item.with_article()),
            ("a Point".to_owned(), "an Item".to_owned())
        );
        assert_eq!(Type::optional(item).to_string(), "Item | None");
    }

    #[test]
    fn names_take_the_right_article() {
        assert_eq!(Type::Int.with_article(), "an int");
        assert_eq!(Type::Str.with_article(), "a str");
        assert_eq!(Type::NoneType.with_article(), "None");
    }

    #[test]
    fn only_int_and_float_are_numeric() {
        let numeric: Vec<bool> = [
            Type::Int,
            Type::Float,
            Type::Bool,
            Type::Str,
            Type::NoneType,
        ]
        .iter()
        .map(Type::is_numeric)
        .collect::<Vec<_>>();
        assert_eq!(numeric, [true, true, false, false, false]);
    }

    #[test]
    fn types_are_written_with_their_python_names() {
        let names: Vec<String> = [
            Type::Int,
            Type::Float,
            Type::Bool,
            Type::Str,
            Type::NoneType,
        ]
        .iter()
        .map(ToString::to_string)
        .collect();
        assert_eq!(names, ["int", "float", "bool", "str", "None"]);
    }

    #[test]
    fn lists_sets_and_tuples_that_hold_them_have_no_hash() {
        let list = Type::List(Box::new(Type::Int));
        let set = Type::Set(Box::new(Type::Int));
        let cases = [
            (Type::Int, true),
            (Type::Float, true),
            (Type::Bytes, true),
            (Type::NoneType, true),
            (Type::Tuple(vec![Type::Int, Type::Str]), true),
            (Type::TupleOf(Box::new(Type::Float)), true),
            (list.clone(), false),
            (set.clone(), false),
            (Type::Tuple(vec![Type::Int, list]), false),
            (Type::TupleOf(Box::new(set)), false),
        ];
        for (ty, hashable) in cases {
            assert_eq!(ty.is_hashable(), hashable, "{ty}");
        }
    }

    #[test]
    fn sets_compare_and_order_only_with_sets_of_their_item_type() {
        let ints = Type::Set(Box::new(Type::Int));
        let floats = Type::Set(Box::new(Type::Float));
        assert_eq!(ints.to_string(), "set[int]");
        assert!(ints.can_test_equality(&ints));
        assert!(ints.can_order(&ints));
        assert!(!ints.can_test_equality(&floats));
        assert!(!ints.can_order(&floats));
        assert!(ints.is_assignable_to(&ints));
        assert!(!ints.is_assignable_to(&floats));
        assert!(Type::Set(Box::new(Type::Error)).has_error());
    }

    #[test]
    fn dicts_and_views_follow_their_key_and_value_types() {
        use super::View;
        let dict = |key: Type, value: Type| Type::Dict(Box::new(key), Box::new(value));
        let ints = dict(Type::Int, Type::Int);
        let view = |kind| Type::DictView(kind, Box::new(Type::Int), Box::new(Type::Str));
        assert_eq!(ints.to_string(), "dict[int, int]");
        assert_eq!(view(View::Items).to_string(), "dict_items[int, str]");
        assert_eq!(view(View::Keys).to_string(), "dict_keys[int, str]");
        assert_eq!(view(View::Values).to_string(), "dict_values[int, str]");
        assert!(ints.is_assignable_to(&ints));
        assert!(!ints.is_assignable_to(&dict(Type::Int, Type::Float)));
        assert!(!ints.is_assignable_to(&dict(Type::Str, Type::Int)));
        assert!(dict(Type::Error, Type::Int).has_error());
        assert!(dict(Type::Int, Type::Error).has_error());
        assert!(ints.can_test_equality(&ints));
        assert!(!ints.can_test_equality(&dict(Type::Int, Type::Str)));
        assert!(!view(View::Keys).can_test_equality(&view(View::Keys)));
        assert!(!Type::Int.can_test_equality(&view(View::Keys)));
        assert!(!ints.can_order(&ints));
        assert!(!Type::Int.can_order(&view(View::Values)));
        assert!(!ints.is_hashable());
        assert!(!view(View::Keys).is_hashable());
        let lists = |item: Type| Type::List(Box::new(item));
        assert!(lists(Type::Int).can_order(&lists(Type::Int)));
        assert!(!lists(ints.clone()).can_order(&lists(ints)));
        assert_eq!(
            [View::Keys, View::Values, View::Items].map(View::code),
            [0, 1, 2]
        );
        assert_eq!(
            View::Items.item(&Type::Int, &Type::Str),
            Type::Tuple(vec![Type::Int, Type::Str])
        );
        assert_eq!(View::Values.item(&Type::Int, &Type::Str), Type::Str);
    }

    #[test]
    fn tuples_are_written_compared_and_ordered_item_by_item() {
        let pair = Type::Tuple(vec![Type::Int, Type::Str]);
        let nones = Type::Tuple(vec![Type::NoneType, Type::Int]);
        assert_eq!(Type::Tuple(Vec::new()).to_string(), "tuple[()]");
        assert_eq!(pair.to_string(), "tuple[int, str]");
        assert!(!pair.is_assignable_to(&Type::Tuple(vec![Type::Int])));
        assert!(!pair.can_test_equality(&Type::Tuple(vec![Type::Int])));
        assert!(pair.can_test_equality(&Type::Tuple(vec![Type::Float, Type::Str])));
        assert!(nones.can_order(&nones));
        assert!(!pair.can_order(&Type::Tuple(vec![Type::Int])));
        assert!(Type::Int.can_order(&Type::Float));
        assert!(!Type::Str.can_order(&Type::Int));
        assert!(Type::Str.can_test_equality(&Type::Str));
        assert!(!Type::Str.can_test_equality(&Type::Int));
    }

    #[test]
    fn optional_types_join_none_and_keep_containers_exact() {
        let optional = |inner: Type| Type::optional(inner);
        let int = optional(Type::Int);
        assert_eq!(int.to_string(), "int | None");
        assert_eq!(optional(Type::NoneType), Type::NoneType);
        assert_eq!(optional(int.clone()), int);
        assert_eq!(optional(Type::Error), Type::Error);
        assert!(Type::NoneType.is_assignable_to(&int));
        assert!(Type::Int.is_assignable_to(&int));
        assert!(!Type::Str.is_assignable_to(&int));
        assert!(int.is_assignable_to(&int));
        assert!(!int.is_assignable_to(&Type::Int));
        assert!(!int.is_assignable_to(&optional(Type::Float)));
        let list = |item: Type| Type::List(Box::new(item));
        assert!(!list(Type::Int).is_assignable_to(&list(int.clone())));
        assert!(list(int.clone()).matches(&list(int.clone())));
        let view = |view, key: Type| Type::DictView(view, Box::new(key), Box::new(Type::Int));
        assert!(view(super::View::Keys, Type::Str).matches(&view(super::View::Keys, Type::Str)));
        assert!(!view(super::View::Keys, Type::Str).matches(&view(super::View::Values, Type::Str)));
        assert!(!view(super::View::Keys, Type::Str).matches(&view(super::View::Keys, Type::Int)));
        let pair = Type::Tuple(vec![Type::Int, Type::Str]);
        assert!(!pair.matches(&Type::Tuple(vec![Type::Int])));
        assert!(pair.is_assignable_to(&optional(pair.clone())));
        assert!(optional(Type::Error).has_error());
        assert!(Type::Optional(Box::new(Type::Error)).has_error());
        assert!(int.is_hashable());
        assert!(!int.can_order(&int));
        assert!(!int.can_test_equality(&int));
        assert!(!Type::Int.can_test_equality(&int));
    }
}
