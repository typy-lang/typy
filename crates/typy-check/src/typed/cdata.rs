//! What a program does with a `ctypes` object after checking: makes one, reads and writes
//! its value and items, and passes it to a C function (FFI-03, ADR-0048).

use super::expression::TypedExpr;

/// An expression that makes or reads a `ctypes` object. One that makes an object has the
/// object's class as its type.
#[derive(Clone, Debug, PartialEq)]
pub enum CDataExpr {
    /// A class as a value, such as `ctypes.c_int`, which holds nothing.
    Class,
    /// `T(arguments)`: a new object of class `T`, whose bytes start at zero; a scalar is then
    /// given its one argument, and an array its items in order, each of a type the value or
    /// the item takes.
    New(Vec<TypedExpr>),
    /// `ctypes.pointer(target)`, or `POINTER(T)(target)`: a new pointer to the object, which
    /// it keeps alive.
    Pointer(Box<TypedExpr>),
    /// `object.value` of a scalar.
    Value(Box<TypedExpr>),
    /// `pointer.contents`: a new object over the memory the pointer holds.
    Contents(Box<TypedExpr>),
    /// `object[index]` of a pointer or an array: the value of a scalar item.
    Item {
        /// The pointer or the array.
        object: Box<TypedExpr>,
        /// The position, counted from the end when negative for an array.
        index: Box<TypedExpr>,
    },
    /// `len(array)`.
    Length(Box<TypedExpr>),
    /// `ctypes.sizeof(object)`; the size of a class is a constant.
    Size(Box<TypedExpr>),
    /// `ctypes.byref(object)` as a C function's argument: the object's address.
    Reference(Box<TypedExpr>),
}

impl CDataExpr {
    /// Returns every expression it holds, in the order they are evaluated.
    #[must_use]
    pub fn expressions(&self) -> Vec<&TypedExpr> {
        match self {
            Self::Class => Vec::new(),
            Self::New(arguments) => arguments.iter().collect(),
            Self::Pointer(object)
            | Self::Value(object)
            | Self::Contents(object)
            | Self::Length(object)
            | Self::Size(object)
            | Self::Reference(object) => vec![object],
            Self::Item { object, index } => vec![object, index],
        }
    }
}

/// The part of a `ctypes` object an assignment writes.
#[derive(Clone, Debug, PartialEq)]
pub enum CDataPlace {
    /// `object.value` of a scalar.
    Value,
    /// `object[index]` of a pointer or an array.
    Item(Box<TypedExpr>),
}
