//! Checks `from __future__ import` statements as CPython's compiler checks them, and finds the
//! ones at the start of a module, which the tree leaves out (LIB-29).

use ruff_python_ast::statement_visitor::{StatementVisitor, walk_stmt};
use ruff_python_ast::{self as ast};
use ruff_text_size::{Ranged, TextRange};
use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId};

use super::Converter;

/// The features that CPython 3.12 to 3.15 define and whose effect TyPy has: each is part of
/// Python 3 already, and `annotations` defers the module's annotations, as TyPy can.
const ACCEPTED: [&str; 9] = [
    "nested_scopes",
    "generators",
    "division",
    "absolute_import",
    "with_statement",
    "print_function",
    "unicode_literals",
    "generator_stop",
    "annotations",
];

/// What the `__future__` imports of a module ask for.
pub(super) struct FutureImports {
    /// The statements that hold them: after the docstring, if any, up to this index.
    pub(super) start: usize,
    /// The index of the first statement after them.
    pub(super) end: usize,
    /// Whether `annotations` is among the features.
    pub(super) defers_annotations: bool,
}

impl Converter<'_> {
    /// Returns the `__future__` imports at the start of `body`, or the errors CPython's
    /// compiler gives for them and for one anywhere else.
    pub(super) fn future_imports(
        &self,
        body: &[ast::Stmt],
    ) -> Result<FutureImports, Vec<Diagnostic>> {
        // A docstring may come first: a statement of one string literal.
        let start = usize::from(body.first().is_some_and(|statement| {
            matches!(statement, ast::Stmt::Expr(expression) if expression.value.is_string_literal_expr())
        }));
        let end = start
            + body[start..]
                .iter()
                .take_while(|statement| future(statement).is_some())
                .count();
        let mut errors = Vec::new();
        let mut defers_annotations = false;
        let imports = body[start..end].iter().filter_map(future);
        for alias in imports.flat_map(|import| &import.names) {
            match alias.name.as_str() {
                "annotations" => defers_annotations = true,
                name if ACCEPTED.contains(&name) => {}
                "braces" => errors.push(self.syntax_error(alias.range(), "not a chance")),
                "barry_as_FLUFL" => errors.push(
                    Diagnostic::new(
                        DiagnosticCode::error(202),
                        self.span(alias.range()),
                        "the future feature `barry_as_FLUFL` makes `!=` a syntax error and \
                         `<>` an operator, which TyPy does not parse",
                    )
                    .with_rule(RuleId::LIB_29),
                ),
                name => errors.push(self.syntax_error(
                    alias.range(),
                    &format!("future feature {name} is not defined"),
                )),
            }
        }
        let mut late = LateImports {
            leading: body[start..end].iter().map(Ranged::range).collect(),
            found: Vec::new(),
        };
        late.visit_body(body);
        for (range, lazy) in late.found {
            let message = if lazy {
                "lazy from __future__ import is not allowed"
            } else {
                "from __future__ imports must occur at the beginning of the file"
            };
            errors.push(self.syntax_error(range, message));
        }
        if errors.is_empty() {
            Ok(FutureImports {
                start,
                end,
                defers_annotations,
            })
        } else {
            Err(errors)
        }
    }
}

/// Returns the statement when it is a `from __future__ import` that is not lazy.
fn future(statement: &ast::Stmt) -> Option<&ast::StmtImportFrom> {
    match statement {
        ast::Stmt::ImportFrom(import) if is_future(import) && !import.is_lazy => Some(import),
        _ => None,
    }
}

fn is_future(import: &ast::StmtImportFrom) -> bool {
    import.level == 0
        && import
            .module
            .as_ref()
            .is_some_and(|module| module.as_str() == "__future__")
}

/// Finds every `from __future__ import` that is not at the start of the module, at any depth,
/// and every lazy one, which CPython refuses wherever it is.
struct LateImports {
    leading: Vec<TextRange>,
    found: Vec<(TextRange, bool)>,
}

impl<'a> StatementVisitor<'a> for LateImports {
    fn visit_stmt(&mut self, statement: &'a ast::Stmt) {
        if let ast::Stmt::ImportFrom(import) = statement
            && is_future(import)
            && (import.is_lazy || !self.leading.contains(&import.range()))
        {
            self.found.push((import.range(), import.is_lazy));
        }
        walk_stmt(self, statement);
    }
}
