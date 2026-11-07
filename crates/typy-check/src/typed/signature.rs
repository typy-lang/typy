//! The parameters and return type of a function.

use super::constant::TypedConstant;
use crate::types::Type;

/// One parameter of a function's signature.
#[derive(Clone, Debug, PartialEq)]
pub struct SignatureParameter {
    /// The parameter's name.
    pub name: String,
    /// Its declared type.
    pub ty: Type,
    /// Its default value; a constant in 0.1, so it is evaluated once (EXP-16).
    pub default: Option<TypedConstant>,
}

/// The parameters and return type of a function.
#[derive(Clone, Debug, PartialEq)]
pub struct FunctionSignature {
    /// The parameters, in order.
    pub parameters: Vec<SignatureParameter>,
    /// The return type; `NoneType` for `-> None`.
    pub returns: Type,
}
