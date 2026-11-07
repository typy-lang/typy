//! Maps checked types to TyIR types.

use typy_check::Type;
use typy_tyir::TyirType;

/// Returns the TyIR type of a checked type; `None` maps to `unit`. A checked program has no error types.
pub(crate) fn tyir_type(ty: Type) -> TyirType {
    match ty {
        Type::Int => TyirType::Int,
        Type::Float => TyirType::Float,
        Type::Bool => TyirType::Bool,
        Type::Str => TyirType::STR,
        Type::NoneType | Type::Error => TyirType::Unit,
    }
}

#[cfg(test)]
mod tests {
    use super::tyir_type;
    use typy_check::Type;
    use typy_tyir::TyirType;

    #[test]
    fn each_checked_type_has_its_tyir_type() {
        assert_eq!(tyir_type(Type::Int), TyirType::Int);
        assert_eq!(tyir_type(Type::Float), TyirType::Float);
        assert_eq!(tyir_type(Type::Bool), TyirType::Bool);
        assert_eq!(tyir_type(Type::Str), TyirType::STR);
        assert_eq!(tyir_type(Type::NoneType), TyirType::Unit);
    }
}
