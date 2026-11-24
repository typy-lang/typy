//! A class of the program as the type of its instances, with the chain of its bases (CLS-01,
//! CLS-04, ADR-0021).

use std::fmt;

use crate::typed::ClassId;

/// The class of an instance: its ID, its name for messages, and the classes of the program it
/// derives from, nearest first. Two classes with the same ID have the same chain, so equality
/// is that of the IDs.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct InstanceClass {
    /// The class.
    pub id: ClassId,
    /// Its name.
    pub name: String,
    /// Its bases that are classes of the program, nearest first, each with its name.
    pub ancestors: Vec<(ClassId, String)>,
    /// The operator methods the class defines or inherits (CLS-07, ADR-0026).
    pub operators: Operators,
    /// For a copy of a generic class, the generic class and the copy's type arguments
    /// (GEN-03, ADR-0028).
    pub generic: Option<(ClassId, Vec<crate::types::Type>)>,
    /// Whether the class is a protocol, which any class with its members satisfies (GEN-04,
    /// ADR-0023).
    pub protocol: bool,
}

/// The hidden method the compiler writes for a named tuple, which returns its fields as a
/// tuple; a name with a space is no Python name, so no program defines or calls it (CLS-27,
/// ADR-0088).
pub(crate) const NAMED_TUPLE_FIELDS: &str = " (fields)";

/// The hidden method the compiler writes for a named tuple and each class derived from it,
/// which `_replace` calls: it takes each field as `tuple[F] | None`, `None` where the call
/// leaves the field as it is, and returns an instance of its own class (CLS-27, ADR-0088).
pub(crate) const NAMED_TUPLE_REPLACE: &str = " (replace)";

/// The hidden static method the compiler writes for an enum class, which returns the member
/// whose value equals its argument or raises `ValueError`, as `Color(value)` does (CLS-44,
/// ADR-0116).
pub(crate) const ENUM_LOOKUP: &str = " (lookup)";

/// The hidden static method of an enum class that returns the member a name names, an alias
/// too, or raises `KeyError`, as `Color[name]` does.
pub(crate) const ENUM_NAMED: &str = " (named)";

/// The hidden static method of an enum class that returns a new list of its members, aliases
/// left out, which a loop over the class, `len` and `in` read.
pub(crate) const ENUM_MEMBERS: &str = " (members)";

/// The hidden method of an `IntEnum` or a `StrEnum` that returns a member's value, which marks
/// the class as one whose members the checker reads as their values where an `int` or a `str`
/// is wanted.
pub(crate) const ENUM_VALUE: &str = " (value)";

/// The hidden static method of a `Flag` or an `IntFlag` that returns the member or composite
/// of a value, made once and kept, or raises CPython's `ValueError` (CLS-44, ADR-0118).
pub(crate) const ENUM_FLAG: &str = " (flag)";

/// The hidden method of a flag that returns a new list of the members whose bits it holds, in
/// definition order, which a loop over it, `len` and `in` read; it also marks the class as a
/// flag.
pub(crate) const ENUM_BITS: &str = " (bits)";

/// The hidden static method of a flag that returns how many bits a value has, as
/// `int.bit_length` does.
pub(crate) const ENUM_FLAG_WIDTH: &str = " (width)";

/// The hidden static method of a flag that writes a value in CPython's two's complement binary
/// form, as its `ValueError` shows it.
pub(crate) const ENUM_FLAG_BINARY: &str = " (binary)";

/// The start of the hidden module variable, followed by the class's name, that keeps the
/// composites of a flag by value.
pub(crate) const ENUM_FLAGS: &str = " (flags) ";

/// The start of the hidden name an enum class's member variables call to make each member,
/// followed by the class's name: a new instance through `__init__`, where `Color(...)` finds a
/// member instead.
pub(crate) const ENUM_MEMBER: &str = " (member) ";

/// The comparison and hash methods a class defines or inherits, which decide whether its
/// instances can be ordered and hashed (CLS-07, ADR-0026), and whether it is a named tuple,
/// whose instances are tuples (CLS-27).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Operators(u8);

impl Operators {
    const EQ: u8 = 1;
    const LT: u8 = 2;
    const GT: u8 = 4;
    const LE: u8 = 8;
    const GE: u8 = 16;
    const HASH: u8 = 32;
    const UNHASHABLE: u8 = 64;
    const NAMED_TUPLE: u8 = 128;

    /// Returns what a class offers whose body defines the methods `names` and whose base
    /// offers `base`. A class that defines `__eq__` and not `__hash__` has no hash, as in
    /// CPython.
    #[must_use]
    pub fn of<'name>(names: impl IntoIterator<Item = &'name str>, base: Self) -> Self {
        let mut own = 0;
        for name in names {
            own |= match name {
                "__eq__" => Self::EQ,
                "__lt__" => Self::LT,
                "__gt__" => Self::GT,
                "__le__" => Self::LE,
                "__ge__" => Self::GE,
                "__hash__" => Self::HASH,
                NAMED_TUPLE_FIELDS => Self::NAMED_TUPLE,
                _ => 0,
            };
        }
        let inherited = base.0 & !(Self::UNHASHABLE | Self::HASH);
        let hash = if own & Self::HASH != 0 {
            Self::HASH
        } else if own & Self::EQ != 0 {
            Self::UNHASHABLE
        } else {
            base.0 & (Self::UNHASHABLE | Self::HASH)
        };
        Self(own | inherited | hash)
    }

    /// Returns true when instances are ordered by `<`, which CPython also reaches through a
    /// reflected `__gt__`.
    #[must_use]
    pub fn orders(self) -> bool {
        self.0 & (Self::LT | Self::GT) != 0
    }

    /// Returns true when instances have a hash: their identity's, or `__hash__`'s.
    #[must_use]
    pub fn hashable(self) -> bool {
        self.0 & Self::UNHASHABLE == 0
    }

    /// Returns true when the class defines or inherits `__eq__`.
    #[must_use]
    pub fn compares(self) -> bool {
        self.0 & Self::EQ != 0
    }

    /// Returns true when the class is a named tuple or derives from one (CLS-27).
    #[must_use]
    pub fn named_tuple(self) -> bool {
        self.0 & Self::NAMED_TUPLE != 0
    }
}

impl InstanceClass {
    /// Returns a class that derives from `object` alone.
    #[must_use]
    pub fn root(id: ClassId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            ancestors: Vec::new(),
            operators: Operators::default(),
            generic: None,
            protocol: false,
        }
    }

    /// Returns true when an instance of this class is an instance of `base`: the class itself,
    /// one it derives from, or a protocol it satisfies in the open table of fits.
    #[must_use]
    pub fn derives_from(&self, base: &Self) -> bool {
        self.id == base.id
            || self.ancestors.iter().any(|(id, _)| *id == base.id)
            // A class that satisfies a protocol takes its place as a base's (ADR-0041).
            || (base.protocol && !self.protocol && crate::protocol_fits::fits(self.id, base.id))
    }

    /// Returns the class's base, when it is a class of the program.
    #[must_use]
    pub fn base(&self) -> Option<Self> {
        let ((id, name), rest) = self.ancestors.split_first()?;
        Some(Self {
            id: *id,
            name: name.clone(),
            ancestors: rest.to_vec(),
            operators: Operators::default(),
            generic: None,
            protocol: false,
        })
    }

    /// Returns the nearest class both derive from, if any.
    #[must_use]
    pub fn common_base(&self, other: &Self) -> Option<Self> {
        let mut current = Some(self.clone());
        while let Some(class) = current {
            if other.derives_from(&class) {
                return Some(class);
            }
            current = class.base();
        }
        None
    }
}

impl fmt::Display for InstanceClass {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // A class written for `map` or `filter` is shown as the built-in, with the type
        // arguments of a copy (ADR-0089).
        match self
            .name
            .split_once(crate::check::builtin_iterators::BUILT_IN)
        {
            Some((name, rest)) => {
                let arguments = rest.split_once(')').map_or("", |(_, arguments)| arguments);
                write!(formatter, "{name}{arguments}")
            }
            None => formatter.write_str(&self.name),
        }
    }
}

#[cfg(test)]
mod tests;
