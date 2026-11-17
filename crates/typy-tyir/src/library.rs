//! What a module built as a C library exports: each function under its own name, with the C
//! types of its parameters and result (CLI-09, FFI-05, ADR-0032, ADR-0044).

use std::fmt;

use crate::function::FunctionId;

/// The C type of an export's parameter or result.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CType {
    /// `int64_t`, which an `int` is.
    Int64,
    /// `double`, which a `float` is.
    Double,
    /// `bool`, which a `bool` is.
    Bool,
}

impl CType {
    /// Returns the type as C writes it.
    #[must_use]
    pub const fn c_name(self) -> &'static str {
        match self {
            Self::Int64 => "int64_t",
            Self::Double => "double",
            Self::Bool => "bool",
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

/// A module built as a C library: its exports. Its object also runs the module bodies when
/// it is loaded.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TyirLibrary {
    /// The library's name, which its module and its C header have.
    pub name: String,
    /// The functions it exports, in the order `__all__` lists them.
    pub exports: Vec<TyirExport>,
}
