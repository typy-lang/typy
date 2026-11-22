//! Tests of `ctypes`.

use super::{CData, CType, CtypesFunction};
use crate::types::Type;

fn pointer(target: CData) -> CData {
    CData::Pointer(Box::new(target))
}

fn array(item: CData, length: u32) -> CData {
    CData::Array(Box::new(item), length)
}

#[test]
fn each_class_has_cpythons_name_and_size_on_64_bit_linux() {
    let int = CData::Scalar(CType::Int32);
    for (class, name, size) in [
        (CData::Scalar(CType::Int8), "c_byte", 1),
        (CData::Scalar(CType::UInt8), "c_ubyte", 1),
        (CData::Scalar(CType::Int16), "c_short", 2),
        (CData::Scalar(CType::UInt16), "c_ushort", 2),
        (int.clone(), "c_int", 4),
        (CData::Scalar(CType::UInt32), "c_uint", 4),
        (CData::Scalar(CType::Int64), "c_long", 8),
        (CData::Scalar(CType::UInt64), "c_ulong", 8),
        (CData::Scalar(CType::Float), "c_float", 4),
        (CData::Scalar(CType::Double), "c_double", 8),
        (CData::Scalar(CType::Bool), "c_bool", 1),
        (CData::Scalar(CType::CharPointer), "c_char_p", 8),
        (CData::Scalar(CType::VoidPointer), "c_void_p", 8),
        (pointer(int.clone()), "LP_c_int", 8),
        (array(int.clone(), 3), "c_int_Array_3", 12),
        (array(CData::Scalar(CType::Int64), 0), "c_long_Array_0", 0),
    ] {
        assert_eq!(class.class_name(), name);
        assert_eq!(class.size(), size, "{name}");
    }
    assert_eq!(pointer(int.clone()).item(), Some(&int));
    assert_eq!(array(int.clone(), 2).item(), Some(&int));
    assert_eq!(int.item(), None);
    assert_eq!(int.scalar(), Some(CType::Int32));
    assert_eq!(pointer(int).scalar(), None);
}

#[test]
fn a_pointer_argument_takes_what_ctypes_passes_by_reference() {
    let int = CData::Scalar(CType::Int32);
    let long = CData::Scalar(CType::Int64);
    let to_int = pointer(int.clone());
    for (ty, by_reference, accepted) in [
        (Type::CData(to_int.clone()), false, true),
        (Type::CData(int.clone()), false, true),
        (Type::CData(int.clone()), true, true),
        (Type::CData(array(int.clone(), 2)), false, true),
        (Type::CData(array(int.clone(), 2)), true, true),
        (Type::NoneType, false, true),
        (Type::CData(to_int.clone()), true, false),
        (Type::CData(long.clone()), false, false),
        (Type::CData(pointer(long.clone())), false, false),
        (Type::CData(array(long, 2)), false, false),
        (Type::Int, false, false),
    ] {
        assert_eq!(
            to_int.accepts(&ty, by_reference),
            accepted,
            "{ty} {by_reference}"
        );
    }
    let address = CData::Scalar(CType::VoidPointer);
    assert!(address.accepts(&Type::CData(to_int.clone()), false));
    assert!(address.accepts(&Type::CData(array(int.clone(), 1)), false));
    assert!(address.accepts(&Type::CData(int.clone()), true));
    assert!(address.accepts(&Type::Bytes, false));
    assert!(!address.accepts(&Type::CData(int.clone()), false));
    assert!(!int.accepts(&Type::CData(int.clone()), true));
    assert!(int.accepts(&Type::Int, false));
    assert!(!array(int.clone(), 1).accepts(&Type::CData(array(int, 1)), false));
}

#[test]
fn each_class_names_the_arguments_it_takes_and_gives_its_result() {
    let int = CData::Scalar(CType::Int32);
    assert_eq!(int.argument_types(), "an `int`");
    assert!(
        pointer(int.clone())
            .argument_types()
            .starts_with("a pointer to `c_int`, a `c_int` or an array of them")
    );
    assert!(
        CData::Scalar(CType::VoidPointer)
            .argument_types()
            .contains("a pointer, an array or `byref` of an object")
    );
    assert_eq!(array(int.clone(), 2).argument_types(), "an array");
    assert_eq!(int.result_type(), Type::Int);
    assert_eq!(
        pointer(int.clone()).result_type(),
        Type::CData(pointer(int))
    );
}

#[test]
fn a_c_void_p_stores_an_int_or_none_and_refuses_bytes() {
    assert!(CType::VoidPointer.stores(&Type::Int));
    assert!(CType::VoidPointer.stores(&Type::Bool));
    assert!(CType::VoidPointer.stores(&Type::NoneType));
    assert!(CType::VoidPointer.stores(&Type::Optional(Box::new(Type::Int))));
    assert!(!CType::VoidPointer.stores(&Type::Bytes));
    assert_eq!(CType::VoidPointer.stored_types(), "an `int` or `None`");
    assert!(CType::Double.stores(&Type::Int));
    assert!(!CType::Int8.stores(&Type::Float));
    assert_eq!(CType::Int8.stored_types(), "an `int`");
}

#[test]
fn each_function_of_ctypes_on_objects_is_named_as_ctypes_names_it() {
    for name in ["POINTER", "pointer", "byref", "sizeof"] {
        let function = CtypesFunction::from_name(name).unwrap();
        assert_eq!(function.name(), name);
    }
    assert_eq!(CtypesFunction::from_name("cast"), None);
}

#[test]
fn each_ctypes_name_stands_for_its_c_type_on_64_bit_linux() {
    for (names, ty) in [
        (&["c_int8", "c_byte"][..], CType::Int8),
        (&["c_int", "c_int32"], CType::Int32),
        (
            &["c_long", "c_int64", "c_longlong", "c_ssize_t"],
            CType::Int64,
        ),
        (
            &["c_ulong", "c_uint64", "c_ulonglong", "c_size_t"],
            CType::UInt64,
        ),
        (&["c_char_p"], CType::CharPointer),
    ] {
        for name in names {
            assert_eq!(CType::from_name(name), Some(ty), "{name}");
        }
    }
    for name in ["c_char", "c_wchar_p", "c_longdouble", "POINTER", "int"] {
        assert_eq!(CType::from_name(name), None, "{name}");
    }
    for (name, ty) in [
        ("c_uint8", CType::UInt8),
        ("c_ubyte", CType::UInt8),
        ("c_short", CType::Int16),
        ("c_uint16", CType::UInt16),
        ("c_ushort", CType::UInt16),
        ("c_uint", CType::UInt32),
        ("c_uint32", CType::UInt32),
        ("c_float", CType::Float),
        ("c_double", CType::Double),
        ("c_bool", CType::Bool),
        ("c_void_p", CType::VoidPointer),
    ] {
        assert_eq!(CType::from_name(name), Some(ty), "{name}");
    }
}

#[test]
fn each_c_type_names_the_arguments_it_takes() {
    for (ty, text) in [
        (CType::Int16, "an `int`"),
        (CType::Float, "an `int` or a `float`"),
        (CType::Bool, "a value with a truth"),
        (CType::CharPointer, "`bytes` or `None`"),
        (CType::VoidPointer, "an `int`, `bytes` or `None`"),
    ] {
        assert_eq!(ty.argument_types(), text);
    }
}

#[test]
fn each_c_type_takes_the_values_ctypes_converts() {
    let optional_bytes = Type::Optional(Box::new(Type::Bytes));
    assert!(CType::UInt8.accepts(&Type::Bool));
    assert!(!CType::Int32.accepts(&Type::Float));
    assert!(CType::Double.accepts(&Type::Int));
    assert!(CType::Bool.accepts(&Type::Str));
    assert!(CType::CharPointer.accepts(&optional_bytes));
    assert!(!CType::CharPointer.accepts(&Type::Str));
    assert!(CType::VoidPointer.accepts(&Type::NoneType));
    assert_eq!(CType::CharPointer.result_type(), optional_bytes);
    assert_eq!(CType::UInt64.result_type(), Type::Int);
}
