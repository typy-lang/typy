//! Which imports of a module are lazy: `lazy import` and `lazy from ... import`, and the
//! imports of the modules a module-level `__lazy_modules__` lists, as PEP 810 defines them
//! for Python 3.15 (MOD-06, MOD-07).

use std::collections::BTreeSet;

use crate::syntax::{Constant, Expr, Stmt};

/// Returns the module names a module-level `__lazy_modules__ = [...]` of string literals
/// lists, in any of its assignments at module level.
#[must_use]
pub fn lazy_modules(statements: &[Stmt]) -> BTreeSet<String> {
    let mut listed = BTreeSet::new();
    for statement in statements {
        let (targets, value) = match statement {
            Stmt::Assign { targets, value, .. } => (targets.as_slice(), value),
            Stmt::AnnAssign {
                target,
                value: Some(value),
                ..
            } => (std::slice::from_ref(target), value),
            _ => continue,
        };
        let names_list = targets
            .iter()
            .any(|target| matches!(target, Expr::Name { id, .. } if id == "__lazy_modules__"));
        if let (true, Expr::List { elements, .. } | Expr::Tuple { elements, .. }) =
            (names_list, value)
        {
            listed.extend(elements.iter().filter_map(|element| match element {
                Expr::Constant {
                    value: Constant::Str(text),
                    ..
                } => Some(text.clone()),
                _ => None,
            }));
        }
    }
    listed
}

/// Returns whether an import statement is lazy: written `lazy`, or naming only modules that
/// `__lazy_modules__` lists. A `from` import counts by the module it imports from, as given.
#[must_use]
pub fn is_lazy(statement: &Stmt, listed: &BTreeSet<String>) -> bool {
    match statement {
        Stmt::Import { is_lazy, names, .. } => {
            *is_lazy
                || (!names.is_empty()
                    && names.iter().all(|alias| listed.contains(&alias.name.name)))
        }
        Stmt::ImportFrom {
            is_lazy,
            module,
            level,
            ..
        } => {
            *is_lazy
                || (*level == 0
                    && module
                        .as_ref()
                        .is_some_and(|module| listed.contains(&module.name)))
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{is_lazy, lazy_modules};
    use crate::parse::parse;
    use crate::settings::PythonFloor;
    use crate::source_file::SourceFile;
    use crate::syntax::Stmt;
    use std::collections::BTreeSet;
    use typy_diagnostics::FileId;

    fn body(source: &str) -> Vec<Stmt> {
        let file = SourceFile::from_text(FileId(0), "app.py".into(), source);
        parse(&file, PythonFloor::V3_15)
            .expect("the source is valid")
            .body
    }

    fn names(listed: &[&str]) -> BTreeSet<String> {
        listed.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn every_module_level_assignment_of_lazy_modules_lists_its_strings() {
        let cases: [(&str, &[&str]); 6] = [
            ("__lazy_modules__ = [\"a\", \"b\"]\n", &["a", "b"]),
            ("__lazy_modules__: list[str] = [\"a\"]\n", &["a"]),
            (
                "__lazy_modules__ = (\"t\",)\n__lazy_modules__ = [\"u\"]\n",
                &["t", "u"],
            ),
            ("x = __lazy_modules__ = [\"c\", 1]\n", &["c"]),
            ("other = [\"d\"]\n__lazy_modules__ = \"e\"\n", &[]),
            ("__lazy_modules__: list[str]\n", &[]),
        ];
        for (source, expected) in cases {
            assert_eq!(lazy_modules(&body(source)), names(expected), "{source}");
        }
    }

    #[test]
    fn an_import_is_lazy_when_written_lazy_or_when_it_names_only_listed_modules() {
        let listed = names(&["a", "b", "p.q"]);
        let cases = [
            ("import a\n", true),
            ("import a, b\n", true),
            ("import a, c\n", false),
            ("lazy import c\n", true),
            ("from a import x\n", true),
            ("from p.q import x\n", true),
            ("from c import x\n", false),
            ("lazy from c import x\n", true),
            ("from . import a\n", false),
            ("x = 1\n", false),
        ];
        for (source, expected) in cases {
            assert_eq!(is_lazy(&body(source)[0], &listed), expected, "{source}");
        }
    }
}
