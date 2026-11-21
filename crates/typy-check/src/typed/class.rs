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
#[expect(
    clippy::struct_excessive_bools,
    reason = "each flag is a fact about the class that lowering reads on its own"
)]
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
    /// For each attribute with a default, its position and the module variable that holds
    /// the default, which a new instance holds before `__init__` runs (ADR-0024).
    pub defaults: Vec<(usize, super::symbol::SymbolId)>,
    /// The adapter of each method that overrides one held otherwise, by its name, which the
    /// method table holds where the base's method is (CLS-04).
    pub adapters: Vec<(String, FunctionId)>,
    /// The bridge of each static method of the class's own, by its name, which the method
    /// table holds in the static method's place (CLS-06).
    pub static_bridges: Vec<(String, FunctionId)>,
    /// The whole definition.
    pub span: Span,
    /// For a copy of a generic class, the generic class, which `type()` and `isinstance`
    /// answer as (ADR-0028).
    pub generic: Option<ClassId>,
    /// Whether the class has no instances: a generic class, or a copy whose type arguments
    /// hold a type parameter, which is checked and never laid out (ADR-0028).
    pub opaque: bool,
    /// Whether its instances are iterators, whose `__next__` the runtime calls (TYP-20).
    pub steps: bool,
    /// Whether its instances are iterators or iterables, whose `__iter__` the runtime calls
    /// (TYP-20).
    pub iterates: bool,
    /// Whether the class is a named tuple or derives from one, whose type object names
    /// `tuple` as the base of the first (CLS-27, ADR-0088).
    pub named_tuple: bool,
}
