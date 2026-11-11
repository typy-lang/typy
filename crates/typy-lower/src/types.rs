//! Maps checked types to TyIR types.

use typy_check::Type;
use typy_tyir::{OptionalBase, TupleTypes, TyirType, TypeId};

/// Returns the TyIR type of a checked type; `None` maps to `unit`, and a tuple to the module's
/// tuple type with the same items. A checked program has no error types.
pub(crate) fn tyir_type(ty: &Type, tuples: &mut TupleTypes) -> TyirType {
    match ty {
        Type::Int => TyirType::Int,
        Type::Float => TyirType::Float,
        Type::Bool => TyirType::Bool,
        Type::Str => TyirType::STR,
        Type::Bytes => TyirType::BYTES,
        Type::NoneType | Type::Error => TyirType::Unit,
        // A `tuple[T, ...]` is a list object with the tuple type ID, so every list function
        // of the runtime reads it (DAT-14).
        Type::List(_) | Type::TupleOf(_) => TyirType::LIST,
        Type::Set(_) => TyirType::SET,
        Type::Dict(..) => TyirType::DICT,
        Type::DictView(..) => TyirType::VIEW,
        Type::Instance(class, _) => TyirType::Object(TypeId::class(class.0)),
        Type::TypeObject(_) => TyirType::TYPE,
        Type::Exception(_) => TyirType::EXCEPTION,
        Type::Iterator(_) => TyirType::ITERATOR,
        Type::Traceback => TyirType::TRACEBACK,
        Type::Tuple(items) => {
            let items = items.iter().map(|item| tyir_type(item, tuples)).collect();
            tuples.intern(items)
        }
        Type::Optional(inner) => TyirType::Optional(match tyir_type(inner, tuples) {
            TyirType::Int => OptionalBase::Int,
            TyirType::Float => OptionalBase::Float,
            TyirType::Bool => OptionalBase::Bool,
            TyirType::Object(id) => OptionalBase::Object(id),
            // A tuple has no value to spare, so it is held beside a presence flag (ADR-0011).
            tuple @ TyirType::Tuple(_) => match tuples.intern(vec![TyirType::Bool, tuple]) {
                TyirType::Tuple(pair) => OptionalBase::Tuple(pair),
                _ => unreachable!("interning gives a tuple type"),
            },
            other => unreachable!("`{other} | None` is not an optional type the checker makes"),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::tyir_type;
    use typy_check::Type;
    use typy_tyir::{TupleTypes, TyirType};

    #[test]
    fn each_checked_type_has_its_tyir_type() {
        let mut tuples = TupleTypes::default();
        assert_eq!(tyir_type(&Type::Int, &mut tuples), TyirType::Int);
        assert_eq!(tyir_type(&Type::Float, &mut tuples), TyirType::Float);
        assert_eq!(tyir_type(&Type::Bool, &mut tuples), TyirType::Bool);
        assert_eq!(tyir_type(&Type::Str, &mut tuples), TyirType::STR);
        assert_eq!(tyir_type(&Type::NoneType, &mut tuples), TyirType::Unit);
        let pair = tyir_type(&Type::Tuple(vec![Type::Int, Type::Str]), &mut tuples);
        assert_eq!(pair, tuples.intern(vec![TyirType::Int, TyirType::STR]));
    }
}
