//! The import a module gets of a built-in TyPy writes in Python: `frozendict`, from the
//! Python floor 3.15, given by the `typy-stdlib` module `_frozendict` (DAT-18, ADR-0123).

use typy_diagnostics::{FileId, Span, TextOffset};

use crate::settings::PythonFloor;
use crate::syntax::{Alias, Expr, Identifier, Stmt};

/// The built-in `frozendict`, and the module that writes it.
const FROZENDICT: &str = "frozendict";
const FROZENDICT_MODULE: &str = "_frozendict";

/// Adds `from _frozendict import frozendict` to the start of a module of the Python floor
/// 3.15 or later whose text holds the word `frozendict` and whose module level binds no such
/// name, after its docstring. A word in a comment or a string adds an import that changes
/// nothing the module does.
pub(crate) fn add_built_in_imports(
    body: &mut Vec<Stmt>,
    text: &str,
    floor: PythonFloor,
    file: FileId,
) {
    if floor < PythonFloor::V3_15 || !holds_word(text, FROZENDICT) || binds(body, FROZENDICT) {
        return;
    }
    let span = Span::empty(file, TextOffset(0));
    let identifier = |name: &str| Identifier {
        name: name.to_owned(),
        span,
    };
    let import = Stmt::ImportFrom {
        module: Some(identifier(FROZENDICT_MODULE)),
        names: vec![Alias {
            name: identifier(FROZENDICT),
            as_name: None,
        }],
        level: 0,
        is_lazy: false,
        span,
    };
    let docstring = matches!(
        body.first(),
        Some(Stmt::Expr {
            value: Expr::Constant { .. },
            ..
        })
    );
    body.insert(usize::from(docstring), import);
}

/// Returns true when `text` holds `word` with no letter, digit or underscore on either side.
fn holds_word(text: &str, word: &str) -> bool {
    let is_name = |character: Option<char>| {
        character.is_some_and(|character| character.is_alphanumeric() || character == '_')
    };
    text.match_indices(word).any(|(start, _)| {
        !is_name(text[..start].chars().next_back())
            && !is_name(text[start + word.len()..].chars().next())
    })
}

/// Returns true when a statement of the module level binds `name`: a `def`, a `class`, an
/// import, an assignment or a `for` loop's target.
fn binds(body: &[Stmt], name: &str) -> bool {
    body.iter().any(|statement| match statement {
        Stmt::FunctionDef { name: defined, .. } | Stmt::ClassDef { name: defined, .. } => {
            defined.name == name
        }
        Stmt::Import { names, .. } | Stmt::ImportFrom { names, .. } => names.iter().any(|alias| {
            let bound = alias.as_name.as_ref().unwrap_or(&alias.name);
            bound.name.split('.').next() == Some(name)
        }),
        Stmt::Assign { targets, .. } => targets.iter().any(|target| target_binds(target, name)),
        Stmt::AnnAssign { target, .. }
        | Stmt::AugAssign { target, .. }
        | Stmt::For { target, .. } => target_binds(target, name),
        _ => false,
    })
}

/// Returns true when an assignment target binds `name`.
fn target_binds(target: &Expr, name: &str) -> bool {
    match target {
        Expr::Name { id, .. } => id == name,
        Expr::Tuple { elements, .. } | Expr::List { elements, .. } => {
            elements.iter().any(|element| target_binds(element, name))
        }
        Expr::Starred { value, .. } => target_binds(value, name),
        _ => false,
    }
}

#[cfg(test)]
mod tests;
