//! The `ctypes` module as TyPy compiles it: the C types of the declarations, which values
//! each takes as an argument and gives as a result, and the libraries and functions a
//! program declares (FFI-01 to FFI-03, ADR-0047).

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

    /// Returns whether the type is one of the integer types, whose arguments wrap to its
    /// width.
    #[must_use]
    pub const fn is_integer(self) -> bool {
        matches!(
            self,
            Self::Int8
                | Self::UInt8
                | Self::Int16
                | Self::UInt16
                | Self::Int32
                | Self::UInt32
                | Self::Int64
                | Self::UInt64
        )
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
    /// The C type of each parameter.
    pub parameters: Vec<CType>,
    /// The C type of the result, or `None` for `void`.
    pub result: Option<CType>,
}

#[cfg(test)]
mod tests {
    use super::CType;
    use crate::types::Type;

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
