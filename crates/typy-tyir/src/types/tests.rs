//! Tests of `types`.

use super::{TupleTypeId, TyirType, TypeId};

#[test]
fn types_are_written_by_name() {
    let names: Vec<String> = [
        TyirType::Int,
        TyirType::Float,
        TyirType::Bool,
        TyirType::Unit,
        TyirType::STR,
        TyirType::EXCEPTION,
        TyirType::SET,
        TyirType::DICT,
        TyirType::VIEW,
        TyirType::Object(TypeId(300)),
        TyirType::Tuple(TupleTypeId(4)),
        TyirType::Optional(super::OptionalBase::Int),
        TyirType::Optional(super::OptionalBase::Tuple(TupleTypeId(2))),
    ]
    .iter()
    .map(ToString::to_string)
    .collect();
    assert_eq!(
        names,
        [
            "int",
            "float",
            "bool",
            "unit",
            "str",
            "exception",
            "set",
            "dict",
            "view",
            "object#300",
            "tuple#4",
            "optional int",
            "optional tuple#2"
        ]
    );
}
