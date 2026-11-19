//! Finds constructs that the current release does not compile (detailed design section 7.2, step 1).

mod class;
mod expression;
mod feature;
mod global_scope;
mod matching;
mod pattern_key;
mod statement;

use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId, Span};
use typy_source::syntax::{Stmt, SyntaxTree};

use feature::Feature;

/// Returns a diagnostic for every construct of `tree` that the 0.1 feature table does not accept.
#[must_use]
pub(crate) fn check(tree: &SyntaxTree) -> Diagnostics {
    let mut walker = Walker {
        diagnostics: Diagnostics::new(),
        loop_depth: 0,
        try_depth: 0,
        scope: None,
        comprehension_names: Vec::new(),
        comprehension_sources: 0,
        comprehension_kinds: Vec::new(),
        lambda_depth: 0,
        generic_classes: tree
            .body
            .iter()
            .filter_map(|statement| match statement {
                typy_source::syntax::Stmt::ClassDef {
                    name,
                    type_parameters,
                    ..
                } if !type_parameters.is_empty() => Some(name.name.clone()),
                _ => None,
            })
            .collect(),
        generic_aliases: tree
            .body
            .iter()
            .filter_map(|statement| match statement {
                typy_source::syntax::Stmt::TypeAlias {
                    name,
                    type_parameters,
                    ..
                } if !type_parameters.is_empty() => Some(name.name.clone()),
                _ => None,
            })
            .collect(),
        imported: std::collections::BTreeSet::new(),
        imported_modules: std::collections::BTreeSet::new(),
    };
    for statement in &tree.body {
        match statement {
            typy_source::syntax::Stmt::ImportFrom { names, .. } => {
                walker.imported.extend(
                    names
                        .iter()
                        .map(|alias| alias.as_name.as_ref().unwrap_or(&alias.name).name.clone()),
                );
            }
            typy_source::syntax::Stmt::Import { names, .. } => {
                walker
                    .imported_modules
                    .extend(names.iter().map(|alias| match &alias.as_name {
                        Some(bound) => bound.name.clone(),
                        None => alias.name.name.split('.').next().unwrap_or("").to_owned(),
                    }));
            }
            _ => {}
        }
    }
    walker.statements(&tree.body, Place::ModuleTop);
    walker.diagnostics
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Place {
    ModuleTop,
    ModuleBlock,
    Function,
    /// The body of a class, which holds its methods.
    Class,
}

struct Walker {
    diagnostics: Diagnostics,
    loop_depth: usize,
    /// How many `try` statements enclose the statement being walked, where CPython refuses
    /// a lazy import.
    try_depth: usize,
    /// The names of the function being walked, for CPython's rules of `global`.
    scope: Option<global_scope::GlobalScope>,
    /// The module's generic classes, whose names an annotation may subscript (GEN-01).
    generic_classes: std::collections::BTreeSet<String>,
    /// The module's generic type aliases, which an annotation of an alias's value may
    /// subscript before the checker expands it (SMT-10).
    generic_aliases: std::collections::BTreeSet<String>,
    /// The names the module's top-level `from` imports bind, which may be generic aliases of
    /// another module that the checker reads (SMT-12).
    imported: std::collections::BTreeSet<String>,
    /// The names the module's top-level `import` statements bind to modules.
    imported_modules: std::collections::BTreeSet<String>,
    /// The names each enclosing comprehension binds, innermost last, which an assignment
    /// expression may not rebind (EXP-13).
    comprehension_names: Vec<Vec<String>>,
    /// How many comprehension sources enclose the expression being walked, where CPython
    /// refuses an assignment expression.
    comprehension_sources: usize,
    /// What each enclosing comprehension is, innermost last, which CPython's message of a
    /// `yield` inside one names.
    comprehension_kinds: Vec<&'static str>,
    /// How many `lambda` expressions enclose the expression being walked.
    lambda_depth: usize,
}

impl Walker {
    pub(super) fn report(&mut self, span: Span, feature: Feature) {
        let diagnostic = match feature {
            Feature::Planned {
                what,
                release,
                rule,
            } => Diagnostic::new(
                DiagnosticCode::error(201),
                span,
                format!("{what} are planned for TyPy {release} ({rule})"),
            )
            .with_rule(rule),
            Feature::AfterOne { what } => Diagnostic::new(
                DiagnosticCode::error(201),
                span,
                format!("{what} are planned for a release after TyPy 1.0"),
            )
            .with_rule(RuleId::STD_04),
            Feature::Excluded { what } => Diagnostic::new(
                DiagnosticCode::error(202),
                span,
                format!("{what} cannot be compiled by TyPy with CPython's meaning"),
            )
            .with_rule(RuleId::STD_04),
        };
        self.diagnostics.push(diagnostic);
    }

    /// Reports what CPython's compiler rejects as a `SyntaxError` but the parser accepts (STD-01).
    pub(super) fn syntax_error(&mut self, span: Span, message: &str) {
        self.diagnostics.push(
            Diagnostic::new(DiagnosticCode::error(103), span, message).with_rule(RuleId::STD_01),
        );
    }

    pub(super) fn statements(&mut self, statements: &[Stmt], place: Place) {
        for statement in statements {
            self.statement(statement, place);
        }
    }

    pub(super) fn block(&mut self, statements: &[Stmt], place: Place) {
        let inner = if place == Place::ModuleTop {
            Place::ModuleBlock
        } else {
            place
        };
        self.statements(statements, inner);
    }

    pub(super) fn loop_body(&mut self, body: &[Stmt], place: Place) {
        self.loop_depth += 1;
        self.block(body, place);
        self.loop_depth -= 1;
    }
}

#[cfg(test)]
mod tests;
