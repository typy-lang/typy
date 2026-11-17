//! The `ctypes` module as TyPy compiles it: the C types of the declarations, which values
//! each takes as an argument and gives as a result, the classes of its objects, and the
//! libraries and functions a program declares (FFI-01 to FFI-03, ADR-0047, ADR-0048).

use crate::types::Type;

/// A C type a `ctypes` declaration names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CType {
    /// `c_int8` and `c_byte`.
    Int8,
    /// `c_uint8` and `c_ubyte`.
    UInt8,
    /// `c_int16` and `c_short`.
    Int16,
    /// `c_uint16` and `c_ushort`.
    UInt16,
    /// `c_int32` and `c_int`.
    Int32,
    /// `c_uint32` and `c_uint`.
    UInt32,
    /// `c_int64`, `c_long`, `c_longlong` and `c_ssize_t`.
    Int64,
    /// `c_uint64`, `c_ulong`, `c_ulonglong` and `c_size_t`.
    UInt64,
    /// `c_float`.
    Float,
    /// `c_double`.
    Double,
    /// `c_bool`.
    Bool,
    /// `c_char_p`: a string of bytes ended by a zero, or null.
    CharPointer,
    /// `c_void_p`: an address, or null.
    VoidPointer,
}

impl CType {
    /// Returns the C type a `ctypes` name stands for on 64-bit Linux, or `None` for a name
    /// that is not one TyPy compiles.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "c_int8" | "c_byte" => Self::Int8,
            "c_uint8" | "c_ubyte" => Self::UInt8,
            "c_int16" | "c_short" => Self::Int16,
            "c_uint16" | "c_ushort" => Self::UInt16,
            "c_int32" | "c_int" => Self::Int32,
            "c_uint32" | "c_uint" => Self::UInt32,
            "c_int64" | "c_long" | "c_longlong" | "c_ssize_t" => Self::Int64,
            "c_uint64" | "c_ulong" | "c_ulonglong" | "c_size_t" => Self::UInt64,
            "c_float" => Self::Float,
            "c_double" => Self::Double,
            "c_bool" => Self::Bool,
            "c_char_p" => Self::CharPointer,
            "c_void_p" => Self::VoidPointer,
            _ => return None,
        })
    }

    /// Returns CPython's name for the class on 64-bit Linux, where `c_int64` is `c_long`.
    #[must_use]
    pub const fn class_name(self) -> &'static str {
        match self {
            Self::Int8 => "c_byte",
            Self::UInt8 => "c_ubyte",
            Self::Int16 => "c_short",
            Self::UInt16 => "c_ushort",
            Self::Int32 => "c_int",
            Self::UInt32 => "c_uint",
            Self::Int64 => "c_long",
            Self::UInt64 => "c_ulong",
            Self::Float => "c_float",
            Self::Double => "c_double",
            Self::Bool => "c_bool",
            Self::CharPointer => "c_char_p",
            Self::VoidPointer => "c_void_p",
        }
    }

    /// Returns the bytes a value of this C type takes on 64-bit Linux.
    #[must_use]
    pub const fn size(self) -> u64 {
        match self {
            Self::Int8 | Self::UInt8 | Self::Bool => 1,
            Self::Int16 | Self::UInt16 => 2,
            Self::Int32 | Self::UInt32 | Self::Float => 4,
            _ => 8,
        }
    }

    /// Returns whether an object's `value`, or an item, takes a value of type `ty` as
    /// `ctypes` converts it: as an argument does, but for `c_void_p`, which takes an `int`
    /// or `None` and refuses `bytes`.
    #[must_use]
    pub fn stores(self, ty: &Type) -> bool {
        match self {
            Self::VoidPointer => {
                matches!(ty, Type::Int | Type::Bool | Type::NoneType)
                    || *ty == Type::Optional(Box::new(Type::Int))
            }
            _ => self.accepts(ty),
        }
    }

    /// Returns the type a message names for a value this C type stores.
    #[must_use]
    pub const fn stored_types(self) -> &'static str {
        match self {
            Self::VoidPointer => "an `int` or `None`",
            _ => self.argument_types(),
        }
    }

    /// Returns the type of a call's result of this C type: `ctypes` gives an `int` of the
    /// integer types, a `float`, a `bool`, `bytes` or `None` of `c_char_p`, and an `int` or
    /// `None` of `c_void_p`.
    #[must_use]
    pub fn result_type(self) -> Type {
        match self {
            Self::Float | Self::Double => Type::Float,
            Self::Bool => Type::Bool,
            Self::CharPointer => Type::Optional(Box::new(Type::Bytes)),
            Self::VoidPointer => Type::Optional(Box::new(Type::Int)),
            _ => Type::Int,
        }
    }

    /// Returns whether an argument of type `ty` converts to this C type as `ctypes` converts
    /// it. A `c_bool` argument takes any value with a truth, which the caller checks.
    #[must_use]
    pub fn accepts(self, ty: &Type) -> bool {
        let integer = matches!(ty, Type::Int | Type::Bool);
        match self {
            Self::Float | Self::Double => integer || *ty == Type::Float,
            Self::Bool => true,
            Self::CharPointer => {
                matches!(ty, Type::Bytes | Type::NoneType)
                    || *ty == Type::Optional(Box::new(Type::Bytes))
            }
            Self::VoidPointer => {
                matches!(ty, Type::Int | Type::Bool | Type::Bytes | Type::NoneType)
                    || *ty == Type::Optional(Box::new(Type::Int))
                    || *ty == Type::Optional(Box::new(Type::Bytes))
            }
            _ => integer,
        }
    }

    /// Returns the type a message names for an argument of this C type.
    #[must_use]
    pub const fn argument_types(self) -> &'static str {
        match self {
            Self::Float | Self::Double => "an `int` or a `float`",
            Self::Bool => "a value with a truth",
            Self::CharPointer => "`bytes` or `None`",
            Self::VoidPointer => "an `int`, `bytes` or `None`",
            _ => "an `int`",
        }
    }
}

/// A `ctypes` type, whose objects are counted objects over a C buffer: a scalar of a C
/// type, a pointer to an object of a type, or an array of a number of items of a type
/// (FFI-03, ADR-0048). Two of one shape are one class, as `c_int32` and `c_int` are in
/// CPython.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum CData {
    /// A scalar: `c_int`, `c_double`, `c_char_p` and the others of `CType`.
    Scalar(CType),
    /// `POINTER(T)`.
    Pointer(Box<CData>),
    /// `T * length`.
    Array(Box<CData>, u32),
}

impl CData {
    /// Returns CPython's name for the class: `c_long`, `LP_c_int` or `c_int_Array_3`.
    #[must_use]
    pub fn class_name(&self) -> String {
        match self {
            Self::Scalar(ty) => ty.class_name().to_owned(),
            Self::Pointer(target) => format!("LP_{}", target.class_name()),
            Self::Array(item, length) => format!("{}_Array_{length}", item.class_name()),
        }
    }

    /// Returns the bytes of an object's buffer, as `ctypes.sizeof` gives them.
    #[must_use]
    pub fn size(&self) -> u64 {
        match self {
            Self::Scalar(ty) => ty.size(),
            Self::Pointer(_) => 8,
            Self::Array(item, length) => item.size() * u64::from(*length),
        }
    }

    /// Returns the type of a pointer's target or an array's item, or `None` for a scalar.
    #[must_use]
    pub fn item(&self) -> Option<&Self> {
        match self {
            Self::Scalar(_) => None,
            Self::Pointer(item) | Self::Array(item, _) => Some(item),
        }
    }

    /// Returns the C type of a scalar, or `None` for a pointer or an array.
    #[must_use]
    pub const fn scalar(&self) -> Option<CType> {
        match self {
            Self::Scalar(ty) => Some(*ty),
            _ => None,
        }
    }

    /// Returns the type a C function's argument of this type takes, as `ctypes` checks it:
    /// a scalar's own values; for `POINTER(T)`, a pointer to `T`, an object of `T` passed by
    /// reference, an array of `T`, `byref` of an object of `T`, or `None`. `by_reference` is
    /// true for an argument `ctypes.byref(object)`, whose type is the object's.
    #[must_use]
    pub fn accepts(&self, ty: &Type, by_reference: bool) -> bool {
        match self {
            Self::Scalar(CType::VoidPointer) => {
                by_reference
                    || matches!(ty, Type::CData(Self::Pointer(_) | Self::Array(..)))
                    || CType::VoidPointer.accepts(ty)
            }
            Self::Scalar(scalar) => !by_reference && scalar.accepts(ty),
            Self::Pointer(target) => match ty {
                Type::CData(Self::Pointer(other)) if !by_reference => other == target,
                Type::CData(Self::Array(item, _)) => item == target,
                Type::CData(other) => other == target.as_ref(),
                Type::NoneType => !by_reference,
                _ => false,
            },
            Self::Array(..) => false,
        }
    }

    /// Returns the type a message names for an argument of this type.
    #[must_use]
    pub fn argument_types(&self) -> String {
        match self {
            Self::Scalar(CType::VoidPointer) => {
                "an `int`, `bytes`, `None`, a pointer, an array or `byref` of an object".to_owned()
            }
            Self::Scalar(scalar) => scalar.argument_types().to_owned(),
            Self::Pointer(target) => {
                let name = target.class_name();
                format!(
                    "a pointer to `{name}`, a `{name}` or an array of them, `byref` of a \
                     `{name}`, or `None`"
                )
            }
            Self::Array(..) => "an array".to_owned(),
        }
    }

    /// Returns the type of a call's result of this type: a scalar's value, or a new pointer
    /// object.
    #[must_use]
    pub fn result_type(&self) -> Type {
        match self {
            Self::Scalar(scalar) => scalar.result_type(),
            other => Type::CData(other.clone()),
        }
    }
}

/// A function of `ctypes` that makes or measures its classes and objects (FFI-03).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CtypesFunction {
    /// `POINTER(T)`: the class of pointers to `T`.
    PointerType,
    /// `pointer(object)`: a new pointer to the object.
    Pointer,
    /// `byref(object)`: the object passed by reference to a C function.
    Reference,
    /// `sizeof(class_or_object)`.
    Size,
}

impl CtypesFunction {
    /// Returns the function of `ctypes` named `name`.
    #[must_use]
    pub(crate) fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "POINTER" => Self::PointerType,
            "pointer" => Self::Pointer,
            "byref" => Self::Reference,
            "sizeof" => Self::Size,
            _ => return None,
        })
    }

    /// Returns the function's name in `ctypes`.
    #[must_use]
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::PointerType => "POINTER",
            Self::Pointer => "pointer",
            Self::Reference => "byref",
            Self::Size => "sizeof",
        }
    }
}

/// A library a module declares with `ctypes.CDLL` or `ctypes.cdll.LoadLibrary`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForeignLibrary {
    /// The name the declaration gives, which the build links.
    pub name: String,
}

/// A C function a module declares through its `argtypes` and `restype`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForeignFunction {
    /// The library it is declared on, by index.
    pub library: u32,
    /// Its C symbol.
    pub name: String,
    /// The `ctypes` type of each parameter.
    pub parameters: Vec<CData>,
    /// The `ctypes` type of the result, or `None` for `void`.
    pub result: Option<CData>,
}

#[cfg(test)]
mod tests {
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
}
