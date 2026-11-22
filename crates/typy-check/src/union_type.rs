//! Unions of two or more types other than `None`, such as `str | list[int]` (TYP-14,
//! ADR-0027).

use crate::class_type::InstanceClass;
use crate::types::Type;

impl Type {
    /// Returns the union of `members` in normal form: a nested union is flattened, `None`
    /// makes the union optional, a member another member accepts is dropped, and the members
    /// stand in one order, so `A | B` and `B | A` are one type. A union of one member is that
    /// member, and an error member makes the whole an error.
    #[must_use]
    pub fn union(members: Vec<Self>) -> Self {
        let mut flat = Vec::new();
        let mut optional = false;
        for member in members {
            match member {
                Self::Error => return Self::Error,
                Self::NoneType => optional = true,
                Self::Optional(inner) => {
                    optional = true;
                    flat.extend(inner.members().iter().cloned());
                }
                Self::Union(inner) => flat.extend(inner),
                // An abstract collection in a union is the types it holds (ADR-0086).
                Self::Abstract(collection) => match collection.members {
                    Self::Union(inner) => flat.extend(inner),
                    other => flat.push(other),
                },
                other => flat.push(other),
            }
        }
        let mut kept: Vec<Self> = Vec::new();
        for (position, member) in flat.iter().enumerate() {
            // A member is dropped for a wider one, or for an equal one written before it.
            let covered = flat.iter().enumerate().any(|(other, wider)| {
                other != position
                    && member.is_assignable_to(wider)
                    && (!wider.is_assignable_to(member) || other < position)
            });
            if !covered {
                kept.push(member.clone());
            }
        }
        // The text of a type names its class with its ID, so the order is one for one set.
        kept.sort_by_cached_key(|member| format!("{member:?}"));
        let union = match kept.len() {
            0 => Self::NoneType,
            1 => kept.remove(0),
            _ => Self::Union(kept),
        };
        if optional {
            Self::optional(union)
        } else {
            union
        }
    }

    /// Returns true when a value of the type may be `NotImplemented`, which an operator or
    /// comparison method returns to give way to the other operand's method (CLS-21).
    #[must_use]
    pub fn may_give_way(&self) -> bool {
        match self {
            Self::Optional(inner) => inner.may_give_way(),
            other => other.members().contains(&Self::NotImplemented),
        }
    }

    /// Returns the type without `NotImplementedType`: what an operator gives once the methods
    /// that gave way are passed over (CLS-21).
    #[must_use]
    pub fn implemented(&self) -> Self {
        match self {
            _ if !self.may_give_way() => self.clone(),
            Self::Optional(inner) => Self::union(vec![Self::NoneType, inner.implemented()]),
            other => Self::union(
                other
                    .members()
                    .iter()
                    .filter(|member| **member != Self::NotImplemented)
                    .cloned()
                    .collect(),
            ),
        }
    }

    /// Returns the members of a union, or the type itself as its only member.
    #[must_use]
    pub fn members(&self) -> &[Self] {
        match self {
            Self::Union(members) => members,
            other => std::slice::from_ref(other),
        }
    }

    /// Returns true when a value of the type is held as the address of an object whose header
    /// says what it is, as each member of a union of objects is (ADR-0027).
    #[must_use]
    pub const fn is_object(&self) -> bool {
        matches!(
            self,
            Self::Str
                | Self::Bytes
                | Self::List(_)
                | Self::TupleOf(_)
                | Self::Set(_)
                | Self::Dict(..)
                | Self::DictView(..)
                | Self::Instance(_)
                | Self::Exception(_)
                | Self::Iterator(_)
                | Self::Iterable(_)
                | Self::Callable { .. }
                | Self::AnyModule
                | Self::Traceback
                | Self::TextFile
                | Self::BinaryFile(_)
                | Self::DescriptorFile(_)
                | Self::Range
                | Self::Slice
                | Self::Module(_)
                | Self::TypeAlias(_)
                | Self::VersionInfo
                | Self::NotImplemented
                | Self::CData(_)
                | Self::TypeObject(_)
                | Self::Union(_)
                | Self::Object
        )
    }

    /// Returns the nearest class every member of a union of instances derives from, when they
    /// share one; such a union is held as an instance of that class (ADR-0027).
    #[must_use]
    pub fn union_base(&self) -> Option<InstanceClass> {
        let Self::Union(members) = self else {
            return None;
        };
        let mut common: Option<InstanceClass> = None;
        for member in members {
            let Self::Instance(class) = member else {
                return None;
            };
            common = Some(match common {
                None => class.as_ref().clone(),
                Some(common) => common.common_base(class)?,
            });
        }
        common
    }
}

#[cfg(test)]
mod tests;
