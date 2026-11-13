//! The tuple types of a module: each distinct list of item types, numbered once (DAT-08).

use crate::types::{OptionalBase, TupleTypeId, TyirType};

/// The tuple types a module uses. A tuple is a value of fixed layout, its items held in place,
/// so counting it counts each counted item. A union with a member held by value is a tuple
/// too, of an `int` tag and a slot for each member, marked as a union (ADR-0027).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TupleTypes {
    items: Vec<Vec<TyirType>>,
    unions: Vec<bool>,
}

impl TupleTypes {
    /// Returns the tuple type with these items, adding it when it is new.
    pub fn intern(&mut self, items: Vec<TyirType>) -> TyirType {
        self.interned(items, false)
    }

    /// Returns the union whose members are held as `members`: a tuple of an `int` tag and a
    /// slot for each member, adding it when it is new (TYP-14, ADR-0027).
    pub fn intern_union(&mut self, members: &[TyirType]) -> TyirType {
        let items = std::iter::once(TyirType::Int)
            .chain(members.iter().copied())
            .collect();
        self.interned(items, true)
    }

    fn interned(&mut self, items: Vec<TyirType>, union: bool) -> TyirType {
        let index = self
            .items
            .iter()
            .zip(&self.unions)
            .position(|(known, known_union)| *known == items && *known_union == union)
            .unwrap_or_else(|| {
                self.items.push(items);
                self.unions.push(union);
                self.items.len() - 1
            });
        TyirType::Tuple(TupleTypeId(u32::try_from(index).unwrap_or(u32::MAX)))
    }

    /// Returns true when a tuple type is a union of its slots after the tag (ADR-0027).
    #[must_use]
    pub fn is_union(&self, id: TupleTypeId) -> bool {
        self.unions.get(id.0 as usize).copied().unwrap_or(false)
    }

    /// Returns the item types of a tuple type, or `None` for a number the module does not use.
    #[must_use]
    pub fn items(&self, id: TupleTypeId) -> Option<&[TyirType]> {
        self.items.get(id.0 as usize).map(Vec::as_slice)
    }

    /// Returns every tuple type, in the order of their numbers.
    pub fn iter(&self) -> impl Iterator<Item = (TupleTypeId, &[TyirType])> {
        self.items.iter().enumerate().map(|(index, items)| {
            (
                TupleTypeId(u32::try_from(index).unwrap_or(u32::MAX)),
                items.as_slice(),
            )
        })
    }

    /// Returns true when a value of this type holds a reference: an `int`, an object, or a
    /// tuple with such an item (ADR-0001).
    #[must_use]
    pub fn is_counted(&self, ty: TyirType) -> bool {
        match ty {
            TyirType::Int | TyirType::Object(_) => true,
            TyirType::Tuple(id) => self
                .items(id)
                .is_some_and(|items| items.iter().any(|item| self.is_counted(*item))),
            TyirType::Float | TyirType::Bool | TyirType::Unit | TyirType::Bits => false,
            TyirType::Optional(base) => match base {
                OptionalBase::Int | OptionalBase::Object(_) => true,
                OptionalBase::Float | OptionalBase::Bool => false,
                OptionalBase::Tuple(pair) => self.is_counted(TyirType::Tuple(pair)),
            },
        }
    }

    /// Returns the type an optional value of `base` holds when it is not `None`.
    #[must_use]
    pub fn optional_value(&self, base: OptionalBase) -> TyirType {
        match base {
            OptionalBase::Int => TyirType::Int,
            OptionalBase::Float => TyirType::Float,
            OptionalBase::Bool => TyirType::Bool,
            OptionalBase::Object(id) => TyirType::Object(id),
            OptionalBase::Tuple(pair) => self
                .items(pair)
                .and_then(|items| items.get(1).copied())
                .unwrap_or(TyirType::Unit),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TupleTypes;
    use crate::types::{TupleTypeId, TyirType};

    #[test]
    fn a_union_is_a_tuple_of_its_own() {
        let mut types = TupleTypes::default();
        let tuple = types.intern(vec![TyirType::Int, TyirType::Int, TyirType::STR]);
        let union = types.intern_union(&[TyirType::Int, TyirType::STR]);
        assert_ne!(tuple, union);
        assert_eq!(types.intern_union(&[TyirType::Int, TyirType::STR]), union);
        let (TyirType::Tuple(tuple), TyirType::Tuple(union)) = (tuple, union) else {
            unreachable!("interning gives tuple types");
        };
        assert!(types.is_union(union));
        assert!(!types.is_union(tuple));
        assert!(!types.is_union(TupleTypeId(9)));
        assert_eq!(types.items(union), types.items(tuple));
    }

    #[test]
    fn equal_item_lists_share_one_number() {
        let mut types = TupleTypes::default();
        let first = types.intern(vec![TyirType::Int, TyirType::STR]);
        let second = types.intern(vec![TyirType::Float]);
        assert_eq!(types.intern(vec![TyirType::Int, TyirType::STR]), first);
        assert_ne!(first, second);
        assert_eq!(types.items(TupleTypeId(1)), Some(&[TyirType::Float][..]));
        assert_eq!(types.items(TupleTypeId(9)), None);
        assert_eq!(types.iter().count(), 2);
    }

    #[test]
    fn a_tuple_is_counted_when_an_item_is() {
        let mut types = TupleTypes::default();
        let floats = types.intern(vec![TyirType::Float, TyirType::Bool]);
        let strings = types.intern(vec![TyirType::STR]);
        let nested = types.intern(vec![TyirType::Float, strings]);
        assert!(!types.is_counted(floats));
        assert!(types.is_counted(nested));
        assert!(types.is_counted(TyirType::Int));
        assert!(types.is_counted(TyirType::STR));
        assert!(!types.is_counted(TyirType::Unit));
        assert!(!types.is_counted(TyirType::Tuple(TupleTypeId(9))));
    }
}
