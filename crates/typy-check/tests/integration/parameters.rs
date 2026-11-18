//! The binding of a call's arguments to positional-only, keyword-only and ordinary parameters,
//! as CPython binds them (FUN-03).

use camino::Utf8PathBuf;
use typy_check::TypeChecker;
use typy_source::{Project, ProjectSettings};

/// Returns the message and the rule of each diagnostic of the one-module program `source`,
/// or nothing when it is accepted.
fn diagnostics(source: &str) -> Vec<(String, String)> {
    use std::sync::atomic::{AtomicU32, Ordering};
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let base = Utf8PathBuf::from_path_buf(std::env::temp_dir()).unwrap();
    let root = base.join(format!(
        "typy-parameters-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("app.py"), source).unwrap();
    let project = Project::load(&root.join("app.py"), ProjectSettings::new(root.clone())).unwrap();
    let found = match TypeChecker::check(&project) {
        Ok(_) => Vec::new(),
        Err(found) => found
            .into_sorted(&project)
            .iter()
            .map(|diagnostic| {
                let rule = diagnostic
                    .rule()
                    .map(|rule| rule.to_string())
                    .unwrap_or_default();
                (diagnostic.message().to_owned(), rule)
            })
            .collect(),
    };
    let _ = std::fs::remove_dir_all(&root);
    found
}

/// A function and a method with every kind of parameter, before the call each case adds.
const DEFINITIONS: &str = "\
def f(a: int, /, b: int, *, c: int = 3, d: int) -> int:
    return a + b + c + d


class Box:
    def put(self, item: int, /, *, twice: bool = False) -> int:
        return item * 2 if twice else item


";

#[test]
fn fun_03_each_parameter_takes_the_arguments_its_kind_allows() {
    for call in [
        "print(f(1, 2, d=4))",
        "print(f(1, b=2, c=5, d=1))",
        "print(f(1, d=1, b=2))",
        "print(Box().put(5, twice=True), Box().put(6))",
    ] {
        assert_eq!(diagnostics(&format!("{DEFINITIONS}{call}\n")), [], "{call}");
    }
}

#[test]
fn fun_03_an_argument_a_parameter_kind_refuses_is_e0404() {
    for (call, message, rule) in [
        (
            "f(1, 2, 3, 4)",
            "`f` takes 2 positional arguments, and more are given; the rest are keyword-only",
            "FUN-03",
        ),
        (
            "f(a=1, b=2, d=3)",
            "`f` takes `a` by position only, not by keyword",
            "FUN-03",
        ),
        ("f(1, 2)", "`f` is missing the argument `d`", "TYP-01"),
        (
            "Box().put(item=1)",
            "`Box.put` takes `item` by position only, not by keyword",
            "FUN-03",
        ),
        (
            "Box().put(1, True)",
            "`Box.put` takes 1 positional argument, and more are given; the rest are \
             keyword-only",
            "FUN-03",
        ),
    ] {
        let found = diagnostics(&format!("{DEFINITIONS}print({call})\n"));
        assert_eq!(found, [(message.to_owned(), rule.to_owned())], "{call}");
    }
}

#[test]
fn fun_03_a_function_of_ordinary_parameters_counts_every_one() {
    let found = diagnostics("def g(a: int) -> int:\n    return a\n\n\nprint(g(1, 2))\n");
    assert_eq!(
        found,
        [(
            "`g` takes 1 argument, and more are given".to_owned(),
            "TYP-01".to_owned()
        )]
    );
}
