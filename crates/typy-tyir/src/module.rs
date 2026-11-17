//! The whole program in TyIR.

use crate::constant::StringId;
use crate::function::{FunctionId, TyirFunction};
use crate::tuple_types::TupleTypes;
use crate::types::{TupleTypeId, TyirType};

/// Index of a global in its module.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GlobalId(pub u32);

/// A variable in static storage for the whole run: a module variable, or the flag that says
/// whether one is bound (ADR-0015). It holds the zero value of its type until the entry
/// function stores the first value, and a counted global owns the reference it holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TyirGlobal {
    /// The Python name, for the text form.
    pub name: String,
    /// The type of the values it holds.
    pub ty: TyirType,
}

/// A module object: the module's qualified name and the path of its file from the project
/// root, which its text shows (OI-20).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TyirModuleObject {
    /// The qualified name.
    pub name: String,
    /// The path from the project root.
    pub path: String,
}

/// A type alias object: the alias's name and the name of its module (SMT-12).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TyirTypeAlias {
    /// The alias's name.
    pub name: String,
    /// The qualified name of the module that defines it.
    pub module: String,
}

/// Every function of the program, the string pool, and the entry function.
#[derive(Clone, Debug, PartialEq)]
pub struct TyirModule {
    /// The functions, indexed by `FunctionId`.
    pub functions: Vec<TyirFunction>,
    /// The globals, indexed by `GlobalId`.
    pub globals: Vec<TyirGlobal>,
    /// The module objects, indexed as `Constant::Module` names them.
    pub modules: Vec<TyirModuleObject>,
    /// The type alias objects, indexed as `Constant::TypeAlias` names them (SMT-12).
    pub type_aliases: Vec<TyirTypeAlias>,
    /// The string constants, indexed by `StringId`.
    pub strings: Vec<String>,
    /// The code points of the string constants that hold a surrogate, indexed as
    /// `Constant::StrPoints` names them (LEX-10).
    pub str_points: Vec<Vec<u32>>,
    /// The tuple types its values use.
    pub tuple_types: TupleTypes,
    /// The classes, indexed by the class index of their `TypeId`.
    pub classes: Vec<TyirClass>,
    /// The module body of `__main__`.
    pub entry: FunctionId,
    /// The C functions the runtime allocates with (MEM-08).
    pub allocator: TyirAllocator,
    /// What the module exports when it is built as a C library, or `None` for an executable
    /// (CLI-09).
    pub library: Option<crate::library::TyirLibrary>,
    /// The C libraries the program's `ctypes` declarations load, by name, which the build
    /// links (FFI-01).
    pub foreign_libraries: Vec<String>,
    /// The C functions the program calls, indexed by `Operation::CallForeign` (FFI-02).
    pub foreign_functions: Vec<crate::library::TyirForeignFunction>,
}

/// The C functions through which the runtime allocates, frees and resizes memory: by
/// default the C library's, or those the project names (MEM-08, ADR-0043).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TyirAllocator {
    /// The function that allocates, as `malloc` does.
    pub allocate: String,
    /// The function that resizes an allocation, as `realloc` does.
    pub reallocate: String,
    /// The function that frees an allocation, as `free` does.
    pub free: String,
}

impl Default for TyirAllocator {
    /// Returns the C library's `malloc`, `realloc` and `free`.
    fn default() -> Self {
        Self {
            allocate: "malloc".to_owned(),
            reallocate: "realloc".to_owned(),
            free: "free".to_owned(),
        }
    }
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
    /// `__eq__`, which the type object's `equal` slot calls, with the class whose instances
    /// its parameter takes, by index (ADR-0026).
    pub equal: Option<(FunctionId, u32)>,
    /// `__lt__`, `__le__`, `__gt__` and `__ge__`, which the type object's `order` slot calls.
    pub order: [Option<FunctionId>; 4],
    /// `__hash__`, which the type object's `hash` slot calls.
    pub hash: Option<FunctionId>,
    /// `__bool__`, which the type object's `truth` slot calls.
    pub truth: Option<FunctionId>,
    /// The method table: the methods a subclass may override, its own or a base's, each at
    /// the position it has in the base's table, then those the class adds (ADR-0021).
    pub methods: Vec<FunctionId>,
    /// The class's base when it is a class of the module, by its index; its attributes come
    /// first (ADR-0021).
    pub base: Option<u32>,
    /// The base of an exception class, whose instances are exceptions; `None` for a class
    /// whose base is `object` (ADR-0013).
    pub exception_base: Option<crate::types::TypeObjectRef>,
    /// Whether the class derives from `OSError`, whose objects a program built with two to
    /// five arguments are written as `[Errno 2] text` (EXC-14, ADR-0035).
    pub os_error: bool,
    /// For a copy of a generic class, the generic class by its index, which its type object
    /// names as its base and as the class `type()` gives (ADR-0028).
    pub public: Option<u32>,
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
