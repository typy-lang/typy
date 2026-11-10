//! The whole program in TyIR.

use crate::constant::StringId;
use crate::function::{FunctionId, TyirFunction};
use crate::tuple_types::TupleTypes;
use crate::types::TupleTypeId;

/// Every function of the program, the string pool, and the entry function.
#[derive(Clone, Debug, PartialEq)]
pub struct TyirModule {
    /// The functions, indexed by `FunctionId`.
    pub functions: Vec<TyirFunction>,
    /// The string constants, indexed by `StringId`.
    pub strings: Vec<String>,
    /// The tuple types its values use.
    pub tuple_types: TupleTypes,
    /// The classes, indexed by the class index of their `TypeId`.
    pub classes: Vec<TyirClass>,
    /// The module body of `__main__`.
    pub entry: FunctionId,
}

/// A class of the program: its names, the layout of its attributes, and the methods that
/// write its instances (ADR-0012, CLS-01, CLS-08).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TyirClass {
    /// The class's name.
    pub name: String,
    /// The name of the module that defines it.
    pub module: String,
    /// The attributes, laid out as this tuple type is.
    pub fields: TupleTypeId,
    /// `__repr__`, which writes an instance inside a container, when the class has one.
    pub repr: Option<FunctionId>,
    /// `__str__`, which `str` of an exception's argument calls, when the class has one.
    pub str: Option<FunctionId>,
}

impl TyirModule {
    /// Returns the class whose instances have type `id`, if `id` is a class of the module.
    #[must_use]
    pub fn class(&self, id: crate::types::TypeId) -> Option<&TyirClass> {
        self.classes.get(id.class_index()? as usize)
    }

    /// Returns a function by its ID.
    #[must_use]
    pub fn function(&self, id: FunctionId) -> &TyirFunction {
        &self.functions[id.0 as usize]
    }

    /// Returns a string constant by its ID.
    #[must_use]
    pub fn string(&self, id: StringId) -> &str {
        &self.strings[id.0 as usize]
    }
}
