//! Maps checked types to TyIR types.

use typy_check::Type;
use typy_tyir::{TupleTypes, TyirType};

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
        Type::Tuple(items) => {
            let items = items.iter().map(|item| tyir_type(item, tuples)).collect();
            tuples.intern(items)
        }
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
