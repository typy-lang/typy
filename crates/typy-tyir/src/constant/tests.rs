//! Tests of `constant`.

use super::{Constant, StringId};
use crate::types::{TupleTypeId, TyirType};

#[test]
fn constants_have_their_types_and_text() {
    let cases = [
        (Constant::Int("-12".to_owned()), TyirType::Int, "int -12"),
        (Constant::Float(1.5), TyirType::Float, "float 1.5"),
        (Constant::Bool(true), TyirType::Bool, "bool true"),
        (Constant::Unit, TyirType::Unit, "unit"),
        (Constant::Str(StringId(3)), TyirType::STR, "str s3"),
        (Constant::Bytes(StringId(4)), TyirType::BYTES, "bytes s4"),
        (Constant::StrPoints(2), TyirType::STR, "str p2"),
        (
            Constant::PlaceholderList,
            TyirType::LIST,
            "placeholder list",
        ),
        (Constant::PlaceholderSet, TyirType::SET, "placeholder set"),
        (
            Constant::PlaceholderDict,
            TyirType::DICT,
            "placeholder dict",
        ),
        (
            Constant::NoneOf(crate::types::OptionalBase::Int),
            TyirType::Optional(crate::types::OptionalBase::Int),
            "none optional int",
        ),
        (
            Constant::TupleDescriptor(TupleTypeId(2)),
            TyirType::Bits,
            "descriptor tuple#2",
        ),
    ];
    for (constant, ty, text) in cases {
        assert_eq!(constant.ty(), ty);
        assert_eq!(constant.to_string(), text);
    }
}
