//! The standard library modules TyPy gives, each written in typed Python that CPython also
//! runs (ADR-0007, ADR-0119), embedded in the compiler. A program that imports one and has no
//! module of that name of its own gets the module's source, which the loader reads as one of
//! the program's modules (LIB-27).
#![forbid(unsafe_code)]

/// Each module TyPy gives, by its name, with its source.
const MODULES: [(&str, &str); 1] = [("bisect", include_str!("../lib/bisect.py"))];

/// Returns the source of the standard library module `name`, or `None` when TyPy gives no
/// module of that name in Python.
#[must_use]
pub fn source(name: &str) -> Option<&'static str> {
    MODULES
        .iter()
        .find(|(module, _)| *module == name)
        .map(|(_, source)| *source)
}

/// Returns the names of the modules TyPy gives in Python, in order.
pub fn names() -> impl Iterator<Item = &'static str> {
    MODULES.iter().map(|(module, _)| *module)
}

#[cfg(test)]
mod tests;
