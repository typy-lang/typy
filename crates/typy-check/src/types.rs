//! The static types: the scalars of 0.1, and from 0.2 the types built from other types.

use std::fmt;

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
    /// An instance of a class of the program, with the class's name for messages (CLS-01).
    Instance(crate::typed::ClassId, String),
    /// An exception of a class (ADR-0013).
    Exception(ExceptionClass),
    /// An iterator that `iter()` makes, of the type of its items (ADR-0014).
    Iterator(Box<Type>),
    /// The traceback `__exit__` receives, which holds no frames in TyPy (CST-10).
    Traceback,
    /// `range` held as a value: a sequence of `int` (DAT-17).
    Range,
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
    /// `range`.
    Range,
    /// A class of the program, with its name.
    Instance(crate::typed::ClassId, String),
    /// An exception class.
    Exception(ExceptionClass),
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
            Type::Instance(class, name) => Self::Instance(*class, name.clone()),
            Type::TypeObject(_) => Self::Type,
            Type::Range => Self::Range,
            // An exception's class may derive from its static class, so it is read from the
            // exception when the program runs.
            Type::Exception(_)
            | Type::Iterator(_)
            | Type::Traceback
            | Type::Optional(_)
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
            Self::Instance(_, name) => name,
            Self::Exception(class) => class.name(),
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
            | (Self::TypeObject(_), Self::TypeObject(None)) => true,
            // An exception of a class is one of each of its bases (EXC-01).
            (Self::Exception(class), Self::Exception(base)) => class.derives_from(base),
            (Self::Optional(_), _) => self.matches(target),
            (_, Self::Optional(inner)) => self.is_assignable_to(inner),
            _ => self.matches(target),
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
            Self::Instance(class, _) => Some(*class),
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
            Self::List(item) | Self::TupleOf(item) | Self::Set(item) | Self::Iterator(item) => {
                item.has_error()
            }
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
            // Comparing views is planned with their set operations (DAT-16), an optional value
            // is compared with `None` by `is`, and iterators by `is`.
            (Self::DictView(..) | Self::Optional(_) | Self::Iterator(_) | Self::Traceback, _)
            | (_, Self::DictView(..) | Self::Optional(_) | Self::Iterator(_) | Self::Traceback) => {
                false
            }
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
                Self::NoneType
                | Self::Dict(..)
                | Self::DictView(..)
                | Self::Optional(_)
                | Self::Instance(..)
                | Self::Exception(_)
                | Self::Iterator(_)
                | Self::Traceback
                | Self::Range
                | Self::TypeObject(_),
                _,
            )
            | (
                _,
                Self::NoneType
                | Self::Dict(..)
                | Self::DictView(..)
                | Self::Optional(_)
                | Self::Instance(..)
                | Self::Exception(_)
                | Self::Iterator(_)
                | Self::Traceback
                | Self::Range
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
            Self::List(_) | Self::Set(_) | Self::Dict(..) | Self::DictView(..) => false,
            Self::Tuple(items) => items.iter().all(Self::is_hashable),
            Self::TupleOf(item) | Self::Optional(item) => item.is_hashable(),
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
        true
    }

    /// Returns the type's name with its article, for messages: `an int`, `a str`, `None`.
    #[must_use]
    pub fn with_article(&self) -> String {
        match self {
            Self::Int => "an int".to_owned(),
            Self::NoneType => "None".to_owned(),
            Self::Instance(_, name) if name.starts_with(['A', 'E', 'I', 'O', 'U']) => {
                format!("an {name}")
            }
            Self::Exception(class) if class.name().starts_with(['A', 'E', 'I', 'O', 'U']) => {
                format!("an {class}")
            }
            Self::Iterator(_) => format!("an {self}"),
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
            Self::Instance(_, name) => formatter.write_str(name),
            Self::Exception(class) => write!(formatter, "{class}"),
            Self::Iterator(item) => write!(formatter, "Iterator[{item}]"),
            Self::Traceback => formatter.write_str("TracebackType"),
            Self::Range => formatter.write_str("range"),
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
        use crate::typed::ClassId;
        let point = Type::Instance(ClassId(0), "Point".to_owned());
        let item = Type::Instance(ClassId(1), "Item".to_owned());
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
