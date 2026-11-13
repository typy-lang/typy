//! A class of the program after checking: its attributes and methods (CLS-01).

use typy_diagnostics::Span;

use super::function::FunctionId;
use crate::types::Type;

/// Index of a class in the `TypedProgram`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClassId(pub u32);

/// An attribute every instance of a class holds, with the one type it keeps.
#[derive(Clone, Debug, PartialEq)]
pub struct Attribute {
    /// The attribute's name.
    pub name: String,
    /// Its type.
    pub ty: Type,
}

/// A class of the program after checking. Its base is `object`, or an exception class
/// (ADR-0012, CLS-03).
#[derive(Clone, Debug, PartialEq)]
pub struct TypedClass {
    /// The class's ID.
    pub id: ClassId,
    /// The class's name.
    pub name: String,
    /// The module the class is defined in.
    pub module: String,
    /// The base when it is a class of the program.
    pub base: Option<ClassId>,
    /// The class as an exception class, when it derives from one.
    pub exception: Option<crate::exception::ExceptionClass>,
    /// The attributes: the base's first, then declared ones, then in the order `__init__`
    /// assigns them; an instance holds them in this order.
    pub attributes: Vec<Attribute>,
    /// The methods, by name, in source order; each is a function of the program whose first
    /// parameter is the instance.
    pub methods: Vec<(String, FunctionId)>,
    /// The `__init__` method, the class's own or the nearest base's, when there is one.
    pub init: Option<FunctionId>,
    /// The whole definition.
    pub span: Span,
}
