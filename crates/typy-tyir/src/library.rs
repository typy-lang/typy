//! The C side of a program: the C types its exports and its `ctypes` declarations name, what
//! a module built as a C library exports, and the C functions the program calls (CLI-09,
//! FFI-01 to FFI-05, ADR-0032, ADR-0044, ADR-0047).

use std::fmt;

use crate::function::FunctionId;

/// The C type of a parameter or a result of a C function.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[allow(
    missing_docs,
    reason = "each variant is the C type of the same name, as `c_name` writes it"
)]
pub enum CType {
    Int8,
    UInt8,
    Int16,
    UInt16,
    Int32,
    UInt32,
    Int64,
    UInt64,
    Float,
    Double,
    Bool,
    /// A string of bytes ended by a zero, or null: `char *`.
    CharPointer,
    /// An address, or null: `void *`.
    VoidPointer,
    /// A pointer to a `ctypes` type, of the pointer class at this index of
    /// `TyirModule::cdata_classes`: an argument passes the address a `ctypes` object gives,
    /// and a result is a new pointer object of the class (FFI-03, ADR-0048).
    Pointer(u32),
}

impl CType {
    /// Returns the type as C writes it.
    #[must_use]
    pub const fn c_name(self) -> &'static str {
        match self {
            Self::Int8 => "int8_t",
            Self::UInt8 => "uint8_t",
            Self::Int16 => "int16_t",
            Self::UInt16 => "uint16_t",
            Self::Int32 => "int32_t",
            Self::UInt32 => "uint32_t",
            Self::Int64 => "int64_t",
            Self::UInt64 => "uint64_t",
            Self::Float => "float",
            Self::Double => "double",
            Self::Bool => "bool",
            Self::CharPointer => "char *",
            Self::VoidPointer | Self::Pointer(_) => "void *",
        }
    }

    /// Returns the width in bits and the signedness of an integer type, or `None` for any
    /// other type.
    #[must_use]
    pub const fn integer(self) -> Option<(u32, bool)> {
        Some(match self {
            Self::Int8 => (8, true),
            Self::UInt8 => (8, false),
            Self::Int16 => (16, true),
            Self::UInt16 => (16, false),
            Self::Int32 => (32, true),
            Self::UInt32 => (32, false),
            Self::Int64 => (64, true),
            Self::UInt64 => (64, false),
            _ => return None,
        })
    }

    /// Returns the TyPy type a result of this C type is: `int` of an integer type, `float`,
    /// `bool`, `bytes | None` of `char *` and `int | None` of `void *`.
    #[must_use]
    pub const fn result_type(self) -> crate::types::TyirType {
        use crate::types::{OptionalBase, TyirType, TypeId};
        match self {
            Self::Float | Self::Double => TyirType::Float,
            Self::Bool => TyirType::Bool,
            Self::CharPointer => TyirType::Optional(OptionalBase::Object(TypeId::BYTES)),
            Self::VoidPointer => TyirType::Optional(OptionalBase::Int),
            Self::Pointer(_) => TyirType::CDATA,
            _ => TyirType::Int,
        }
    }

    /// Returns whether an argument of TyPy type `ty` passes as this C type: the conversion
    /// of an integer and a `float` takes the TyPy type of its result, a `char *` takes
    /// `bytes`, a `void *` an `int`, `bytes` or a `ctypes` object, and a pointer a `ctypes`
    /// object, each also optional.
    #[must_use]
    pub fn takes(self, ty: crate::types::TyirType) -> bool {
        use crate::types::{OptionalBase, TyirType, TypeId};
        let bytes = TyirType::Object(TypeId::BYTES);
        let optional_bytes = TyirType::Optional(OptionalBase::Object(TypeId::BYTES));
        let cdata =
            ty == TyirType::CDATA || ty == TyirType::Optional(OptionalBase::Object(TypeId::CDATA));
        match self {
            Self::Float | Self::Double => ty == TyirType::Float,
            Self::Bool => ty == TyirType::Bool,
            Self::CharPointer => ty == bytes || ty == optional_bytes,
            Self::VoidPointer => {
                ty == TyirType::Int
                    || ty == TyirType::Optional(OptionalBase::Int)
                    || ty == bytes
                    || ty == optional_bytes
                    || cdata
            }
            Self::Pointer(_) => cdata,
            _ => ty == TyirType::Int,
        }
    }
}

impl fmt::Display for CType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.c_name())
    }
}

/// A function a library exports.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TyirExport {
    /// The C symbol, the function's own name.
    pub name: String,
    /// The function it calls.
    pub function: FunctionId,
    /// The C type of each parameter, in order.
    pub parameters: Vec<CType>,
    /// The Python name of each parameter, in order, which the C header shows.
    pub parameter_names: Vec<String>,
    /// The C type of the result, or `None` for `void`.
    pub result: Option<CType>,
}

/// A `ctypes` class a program uses, which code generation emits once and each of its
/// objects points to (FFI-03, ADR-0048).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TyirCDataClass {
    /// CPython's name for it, such as `c_long`, `LP_c_int` or `c_int_Array_3`.
    pub name: String,
    /// What its objects hold.
    pub shape: CDataShape,
    /// The bytes of an object's buffer.
    pub size: u64,
}

/// What the objects of a `ctypes` class hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CDataShape {
    /// One value of a C type other than a pointer to a class.
    Scalar(CType),
    /// The address of an object of the class at this index.
    Pointer(u32),
    /// This many items of the class at this index.
    Array(u32, u64),
}

/// A C function a program calls through a `ctypes` declaration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TyirForeignFunction {
    /// Its C symbol.
    pub name: String,
    /// The library that defines it, by its index in `TyirModule::foreign_libraries`.
    pub library: u32,
    /// The C type of each parameter, in order.
    pub parameters: Vec<CType>,
    /// The C type of the result, or `None` for `void`.
    pub result: Option<CType>,
}

/// A module built as a C library: its exports. Its object also runs the module bodies when
/// it is loaded.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TyirLibrary {
    /// The library's name, which its module and its C header have.
    pub name: String,
    /// The release its file names carry, such as `5.0`, or `None` (CLI-13).
    pub version: Option<String>,
    /// The functions it exports, in the order `__all__` lists them.
    pub exports: Vec<TyirExport>,
}

#[cfg(test)]
mod tests {
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
        assert!(CType::Int8.takes(TyirType::Int) && !CType::Int8.takes(TyirType::Bool));
        assert!(CType::Float.takes(TyirType::Float) && !CType::Float.takes(TyirType::Int));
        assert!(CType::Bool.takes(TyirType::Bool));
        assert!(CType::CharPointer.takes(bytes) && CType::CharPointer.takes(optional_bytes));
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
        assert!(pointer.takes(TyirType::CDATA) && pointer.takes(optional_cdata));
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
}
