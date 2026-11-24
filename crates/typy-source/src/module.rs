//! Module names and the modules of a project.

use std::fmt;

use typy_diagnostics::FileId;

use crate::syntax::SyntaxTree;

/// A dotted module name such as `__main__` or `app.models`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModuleName(String);

impl ModuleName {
    /// Returns the name of the module a program starts in (STD-06).
    #[must_use]
    pub fn main() -> Self {
        Self("__main__".to_owned())
    }

    /// Returns the name when every dotted part is an identifier.
    pub fn new(text: &str) -> Result<Self, ModuleNameError> {
        if text.split('.').all(is_identifier) {
            Ok(Self(text.to_owned()))
        } else {
            Err(ModuleNameError {
                text: text.to_owned(),
            })
        }
    }

    /// Returns the name as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the submodule `name` of this package, `a.b` for `b` of `a`; `name` is an
    /// identifier, as the parser gives it.
    #[must_use]
    pub fn child(&self, name: &str) -> Self {
        Self(format!("{}.{name}", self.0))
    }

    /// Returns the top package of the name, `a` for `a.b.c`.
    #[must_use]
    pub fn top(&self) -> Self {
        Self(self.0.split('.').next().unwrap_or_default().to_owned())
    }

    /// Returns the package that holds the module, `a` for `a.b`, or `None` at the top.
    #[must_use]
    pub fn parent(&self) -> Option<Self> {
        self.0
            .rsplit_once('.')
            .map(|(parent, _)| Self(parent.to_owned()))
    }
}

impl fmt::Display for ModuleName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Approximates Python's identifier rule with Unicode letters and digits; the parser checks the exact rule in source.
fn is_identifier(part: &str) -> bool {
    let mut chars = part.chars();
    chars
        .next()
        .is_some_and(|first| first == '_' || first.is_alphabetic())
        && chars.all(|rest| rest == '_' || rest.is_alphanumeric())
}

/// The reason a text is not a module name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleNameError {
    /// The text that was given.
    pub text: String,
}

impl fmt::Display for ModuleNameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "`{}` is not a module name; each dotted part must be an identifier",
            self.text
        )
    }
}

impl std::error::Error for ModuleNameError {}

/// One Python module of the program.
#[derive(Clone, Debug, PartialEq)]
pub struct Module {
    /// The module's qualified name.
    pub name: ModuleName,
    /// The file it was read from.
    pub file: FileId,
    /// Its parsed form.
    pub syntax: SyntaxTree,
    /// Whether it is a package, read from an `__init__.py` (MOD-02).
    pub is_package: bool,
    /// For the main module that `-m` runs, its own name, such as `tools.run`, which gives
    /// the package its relative imports start from, as CPython's `__spec__.name` does;
    /// `None` for any other module (CLI-28).
    pub spec_name: Option<ModuleName>,
}

impl Module {
    /// Returns the package a relative import starts from: the module itself when it is a
    /// package, the package that holds it otherwise, and `None` for a module outside any
    /// package, such as `__main__` (MOD-02).
    #[must_use]
    pub fn package(&self) -> Option<ModuleName> {
        if self.is_package {
            Some(self.name.clone())
        } else {
            self.spec_name.as_ref().unwrap_or(&self.name).parent()
        }
    }

    /// Returns the packages that hold the main module `-m` runs, outermost first, which run
    /// before it, as CPython imports them; empty for any other module (CLI-28).
    #[must_use]
    pub fn enclosing_packages(&self) -> Vec<ModuleName> {
        let mut packages = Vec::new();
        let mut package = self.spec_name.as_ref().and_then(ModuleName::parent);
        while let Some(name) = package {
            package = name.parent();
            packages.push(name);
        }
        packages.reverse();
        packages
    }
}

#[cfg(test)]
mod tests;
