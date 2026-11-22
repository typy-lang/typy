//! Tests of `library`.

use super::CType;
use crate::types::{OptionalBase, TyirType, TypeId};

const ALL: [CType; 13] = [
    CType::Int8,
    CType::UInt8,
    CType::Int16,
    CType::UInt16,
    CType::Int32,
    CType::UInt32,
    CType::Int64,
    CType::UInt64,
    CType::Float,
    CType::Double,
    CType::Bool,
    CType::CharPointer,
    CType::VoidPointer,
];

#[test]
fn each_integer_type_has_its_width_and_sign() {
    let integers: Vec<(u32, bool)> = ALL.iter().filter_map(|ty| ty.integer()).collect();
    assert_eq!(
        integers,
        [
            (8, true),
            (8, false),
            (16, true),
            (16, false),
            (32, true),
            (32, false),
            (64, true),
            (64, false)
        ]
    );
    let names: Vec<String> = ALL.iter().map(ToString::to_string).collect();
    assert_eq!(names[8..], ["float", "double", "bool", "char *", "void *"]);
}

#[test]
fn each_c_type_takes_and_gives_its_typy_types() {
    let bytes = TyirType::Object(TypeId::BYTES);
    let optional_bytes = TyirType::Optional(OptionalBase::Object(TypeId::BYTES));
    let optional_int = TyirType::Optional(OptionalBase::Int);
    for ty in ALL {
        let result = ty.result_type();
        match ty {
            CType::Float | CType::Double => assert_eq!(result, TyirType::Float),
            CType::Bool => assert_eq!(result, TyirType::Bool),
            CType::CharPointer => assert_eq!(result, optional_bytes),
            CType::VoidPointer => assert_eq!(result, optional_int),
            _ => assert_eq!(result, TyirType::Int),
        }
    }
    assert!(CType::Int8.takes(TyirType::Int));
    assert!(!CType::Int8.takes(TyirType::Bool));
    assert!(CType::Float.takes(TyirType::Float));
    assert!(!CType::Float.takes(TyirType::Int));
    assert!(CType::Bool.takes(TyirType::Bool));
    assert!(CType::CharPointer.takes(bytes));
    assert!(CType::CharPointer.takes(optional_bytes));
    assert!(!CType::CharPointer.takes(TyirType::Int));
    for taken in [TyirType::Int, optional_int, bytes, optional_bytes] {
        assert!(CType::VoidPointer.takes(taken), "{taken}");
    }
    assert!(!CType::VoidPointer.takes(TyirType::Float));
}

#[test]
fn a_pointer_to_a_class_passes_a_ctypes_object_and_gives_a_new_one() {
    let optional_cdata = TyirType::Optional(OptionalBase::Object(TypeId::CDATA));
    let pointer = CType::Pointer(0);
    assert_eq!(pointer.result_type(), TyirType::CDATA);
    assert_eq!(pointer.c_name(), "void *");
    assert_eq!(pointer.integer(), None);
    assert!(pointer.takes(TyirType::CDATA));
    assert!(pointer.takes(optional_cdata));
    assert!(!pointer.takes(TyirType::Int));
    assert!(CType::VoidPointer.takes(TyirType::CDATA));
    assert!(CType::VoidPointer.takes(optional_cdata));
    assert_eq!(TyirType::CDATA.to_string(), "cdata");
    assert_eq!(
        crate::constant::Constant::CDataClass(2).to_string(),
        "cdata_class#2"
    );
    assert_eq!(
        crate::constant::Constant::CDataClass(2).ty(),
        TyirType::Bits
    );
}
