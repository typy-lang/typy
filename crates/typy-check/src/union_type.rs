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
                | Self::AnyModule
                | Self::Traceback
                | Self::TextFile
                | Self::Range
                | Self::Module(_)
                | Self::TypeObject(_)
                | Self::Union(_)
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
mod tests {
    use crate::class_type::InstanceClass;
    use crate::typed::ClassId;
    use crate::types::Type;

    fn class(id: u32, name: &str, ancestors: &[(u32, &str)]) -> Type {
        let mut class = InstanceClass::root(ClassId(id), name);
        class.ancestors = ancestors
            .iter()
            .map(|(id, name)| (ClassId(*id), (*name).to_owned()))
            .collect();
        Type::instance(class)
    }

    #[test]
    fn a_union_has_one_normal_form() {
        let list = Type::List(Box::new(Type::Int));
        let forward = Type::union(vec![Type::Str, list.clone()]);
        let backward = Type::union(vec![list.clone(), Type::Str, Type::Str]);
        assert_eq!(forward, backward);
        assert_eq!(forward.members().len(), 2);
        assert_eq!(Type::union(vec![Type::Str]), Type::Str);
        assert_eq!(Type::union(vec![Type::Str, Type::Error]), Type::Error);
        assert_eq!(Type::union(vec![Type::NoneType]), Type::NoneType);
        let nested = Type::union(vec![forward.clone(), Type::NoneType]);
        assert_eq!(nested, Type::optional(forward.clone()));
        let again = Type::union(vec![nested, Type::Str]);
        assert_eq!(again, Type::optional(forward));
    }

    #[test]
    fn a_member_another_member_accepts_is_dropped() {
        let shape = class(0, "Shape", &[]);
        let circle = class(1, "Circle", &[(0, "Shape")]);
        assert_eq!(Type::union(vec![circle, shape.clone()]), shape);
    }

    #[test]
    fn an_optional_value_is_held_only_where_none_is() {
        let either = Type::union(vec![Type::Int, Type::Str]);
        let maybe = Type::optional(Type::Int);
        assert!(!maybe.is_assignable_to(&either));
        assert!(!maybe.is_upcast_to(&either));
        assert!(maybe.is_assignable_to(&Type::optional(either.clone())));
        assert!(Type::Int.is_assignable_to(&either));
    }

    #[test]
    fn instances_of_one_base_share_it() {
        let circle = class(1, "Circle", &[(0, "Shape")]);
        let square = class(2, "Square", &[(0, "Shape")]);
        let both = Type::union(vec![circle, square.clone()]);
        assert_eq!(both.union_base().map(|base| base.id), Some(ClassId(0)));
        let unrelated = Type::union(vec![class(3, "Other", &[]), square]);
        assert_eq!(unrelated.union_base(), None);
        assert_eq!(Type::union(vec![Type::Str, both]).union_base(), None);
        assert_eq!(Type::Str.union_base(), None);
        assert!(Type::union(vec![Type::Str, Type::Bytes]).is_object());
        assert!(!Type::Int.is_object());
    }
}
