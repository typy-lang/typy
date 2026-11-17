//! The whole program after checking.

use super::class::{ClassId, TypedClass};
use super::function::{FunctionId, TypedFunction};
use super::symbol::{Symbol, SymbolId};

/// A type alias held as a value: its name and the name of its module (SMT-12).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedTypeAlias {
    /// The alias's name.
    pub name: String,
    /// The qualified name of its module.
    pub module: String,
}

/// The whole program after checking.
#[derive(Clone, Debug, PartialEq)]
pub struct TypedProgram {
    /// Every symbol, indexed by `SymbolId`.
    pub symbols: Vec<Symbol>,
    /// Every function, indexed by `FunctionId`; the module body is one of them.
    pub functions: Vec<TypedFunction>,
    /// Every class, indexed by `ClassId`.
    pub classes: Vec<TypedClass>,
    /// Every module, each after the modules its module-level imports need (ADR-0015).
    pub modules: Vec<TypedModule>,
    /// Each function that runs a lazy import for one name, with the variable it binds; one
    /// for a function or a class binds none (MOD-06).
    pub lazy_imports: std::collections::BTreeMap<FunctionId, Option<SymbolId>>,
    /// The built-in modules the program imports, such as `math`, whose module objects follow
    /// those of `modules` (ADR-0018).
    pub builtin_modules: Vec<String>,
    /// The type aliases the program reads as values, indexed as
    /// `TypedExprKind::TypeAliasObject` names them (SMT-12).
    pub type_aliases: Vec<TypedTypeAlias>,
    /// Whether the program closes a text file, by `close()` or a `with` block, which may be
    /// `sys.stdout`; `print` then checks that standard output is open (LIB-14).
    pub closes_text_files: bool,
    /// Whether the program can make a `str` with a surrogate that standard output's
    /// `surrogateescape` cannot write: it calls `chr`, or a literal holds one outside U+DC80
    /// to U+DCFF; `print` then checks what it writes and may raise `UnicodeEncodeError`
    /// (LEX-10).
    pub makes_surrogates: bool,
    /// The module body of the main module.
    pub entry: FunctionId,
    /// The C libraries the program's `ctypes` declarations load, which the build links,
    /// indexed by `TypedExprKind::ForeignLibrary` (FFI-01).
    pub foreign_libraries: Vec<crate::ctypes::ForeignLibrary>,
    /// The C functions the program calls, indexed by `TypedExprKind::ForeignCall` (FFI-02).
    pub foreign_functions: Vec<crate::ctypes::ForeignFunction>,
    /// The functions a library exports, or `None` when the program is an executable
    /// (FFI-05).
    pub exports: Option<Vec<crate::exports::Export>>,
    /// The release a library's file names carry, such as `5.0`, or `None` when it has no
    /// version (CLI-13).
    pub library_version: Option<String>,
    /// The warnings of checking the program, such as W0301 (DIA-08).
    pub warnings: typy_diagnostics::Diagnostics,
}

/// A module of the program after checking.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedModule {
    /// The module's qualified name, `__main__` for the main file.
    pub name: String,
    /// The path of its file from the project root, which its text shows (OI-20).
    pub path: String,
    /// The function of its module-level code.
    pub body: FunctionId,
}

impl TypedProgram {
    /// Returns a symbol by its ID.
    #[must_use]
    pub fn symbol(&self, id: SymbolId) -> &Symbol {
        &self.symbols[id.0 as usize]
    }

    /// Returns a class by its ID.
    #[must_use]
    pub fn class(&self, id: ClassId) -> &TypedClass {
        &self.classes[id.0 as usize]
    }

    /// Returns the method `name` of a class, its own or the nearest base's.
    #[must_use]
    pub fn method(&self, id: ClassId, name: &str) -> Option<FunctionId> {
        let mut current = Some(id);
        while let Some(class) = current.map(|id| self.class(id)) {
            if let Some((_, method)) = class.methods.iter().find(|(each, _)| each == name) {
                return Some(*method);
            }
            current = class.base;
        }
        None
    }

    /// Returns true when a class of the program derives from `id`, so a value whose static
    /// class is `id` may hold an instance of another class (CLS-04).
    #[must_use]
    pub fn has_subclass(&self, id: ClassId) -> bool {
        self.classes.iter().any(|class| class.base == Some(id))
    }

    /// Returns a function by its ID.
    #[must_use]
    pub fn function(&self, id: FunctionId) -> &TypedFunction {
        &self.functions[id.0 as usize]
    }
}
