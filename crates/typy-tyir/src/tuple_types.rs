//! The tuple types of a module: each distinct list of item types, numbered once (DAT-08).

use crate::types::{TupleTypeId, TyirType};

/// The tuple types a module uses. A tuple is a value of fixed layout, its items held in place,
/// so counting it counts each counted item.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TupleTypes {
    items: Vec<Vec<TyirType>>,
}

impl TupleTypes {
    /// Returns the tuple type with these items, adding it when it is new.
    pub fn intern(&mut self, items: Vec<TyirType>) -> TyirType {
        let index = self
            .items
            .iter()
            .position(|known| *known == items)
            .unwrap_or_else(|| {
                self.items.push(items);
                self.items.len() - 1
            });
        TyirType::Tuple(TupleTypeId(u32::try_from(index).unwrap_or(u32::MAX)))
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
            TyirType::Float | TyirType::Bool | TyirType::Unit => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TupleTypes;
    use crate::types::{TupleTypeId, TyirType};

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
