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
        TyirType::Tuple(self.intern_id(items))
    }

    /// Returns the number of the tuple type with these items, adding it when it is new.
    pub fn intern_id(&mut self, items: Vec<TyirType>) -> TupleTypeId {
        self.interned(items, false)
    }

    /// Returns the union whose members are held as `members`: a tuple of an `int` tag and a
    /// slot for each member, adding it when it is new (TYP-14, ADR-0027).
    pub fn intern_union(&mut self, members: &[TyirType]) -> TyirType {
        let items = std::iter::once(TyirType::Int)
            .chain(members.iter().copied())
            .collect();
        TyirType::Tuple(self.interned(items, true))
    }

    fn interned(&mut self, items: Vec<TyirType>, union: bool) -> TupleTypeId {
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
        TupleTypeId(u32::try_from(index).unwrap_or(u32::MAX))
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
mod tests;
