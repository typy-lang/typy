//! Tests of checking generics, unions and type aliases inside the crate: whole programs,
//! each checked as the compiler checks it (GEN-01 to GEN-03, GEN-08, TYP-14, SMT-10).

use camino::Utf8PathBuf;
use typy_source::{Project, ProjectSettings};

use super::TypeChecker;

/// Returns the code of each diagnostic of a one-module program, or nothing when it is
/// accepted.
fn codes(source: &str) -> Vec<String> {
    use std::sync::atomic::{AtomicU32, Ordering};
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let base = Utf8PathBuf::from_path_buf(std::env::temp_dir()).unwrap();
    let root = base.join(format!(
        "typy-generic-unit-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("app.py"), source).unwrap();
    let project = Project::load(&root.join("app.py"), ProjectSettings::new(root.clone())).unwrap();
    let found = match TypeChecker::check(&project) {
        Ok(_) => Vec::new(),
        Err(diagnostics) => diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code().to_string())
            .collect(),
    };
    let _ = std::fs::remove_dir_all(&root);
    found
}

#[test]
fn generic_classes_and_functions_check_and_copy() {
    let source = concat!(
        "class Shape:\n    def area(self) -> float:\n        return 1.0\n",
        "class Box[T]:\n    def __init__(self, value: T) -> None:\n        self.value = value\n",
        "    def get(self) -> T:\n        return self.value\n",
        "class Either[N: (int, str)]:\n    def __init__(self, value: N) -> None:\n        self.value = value\n",
        "class Held[S: Shape]:\n    def __init__(self, shape: S) -> None:\n        self.shape = shape\n",
        "def unwrap[T](box: Box[T]) -> T:\n    return box.get()\n",
        "def measure[S: Shape](held: Held[S]) -> float:\n    return held.shape.area()\n",
        "def pick[N: (int, str)](value: N) -> Either[N]:\n    return Either(value)\n",
        "def first[T](items: list[T], other: T | None) -> T:\n    return items[0]\n",
        "print(unwrap(Box(1)), measure(Held(Shape())), pick(1).value, first([1], None))\n",
        "print(Box[str]('a').get(), isinstance(Box(1), Box), type(Box(2)) is Box)\n",
    );
    assert_eq!(codes(source), Vec::<String>::new());
}

#[test]
fn what_generics_do_not_take_is_refused() {
    let classes =
        "class Box[T]:\n    def __init__(self, value: T) -> None:\n        self.value = value\n";
    let cases: &[(&str, &str)] = &[
        ("x: Box = Box(1)\n", "E0408"),
        ("x: Box[int, int] = Box(1)\n", "E0408"),
        ("class Counter[T]:\n    made = 0\n", "E0201"),
        ("def f[T](x: T) -> T:\n    return x + x\n", "E0401"),
        (
            "def f[T](x: T, n: int) -> int:\n    return f([x], n)\nprint(f(1, 1))\n",
            "E0428",
        ),
        (
            "def f[N: (int, str)](x: N) -> N:\n    return x\nprint(f(1.5))\n",
            "E0404",
        ),
        (
            "def f[T](x: T) -> T:\n    return x\nprint(f(1, 2))\n",
            "E0404",
        ),
        ("def f[T](x: T) -> T:\n    return x\nprint(f())\n", "E0404"),
        ("class Sized[T]:\n    size: int = 1\n", "E0201"),
        ("print(Box[missing](1))\n", "E0205"),
    ];
    for (body, code) in cases {
        let found = codes(&format!("{classes}{body}"));
        assert_eq!(
            found.first().map(String::as_str),
            Some(*code),
            "{body}\n{found:?}"
        );
    }
}

#[test]
fn every_one_file_case_of_unions_aliases_and_generics_gets_its_outcome() {
    let root = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance");
    let mut checked = 0;
    for (area, prefix) in [("typ", "TYP-14_"), ("smt", "SMT-10_"), ("gen", "GEN-0")] {
        for entry in std::fs::read_dir(root.join(area)).unwrap() {
            let path = Utf8PathBuf::from_path_buf(entry.unwrap().path()).unwrap();
            let named = path
                .file_name()
                .is_some_and(|name| name.starts_with(prefix));
            if !named || !path.is_file() {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap();
            let found = codes(&text);
            let wanted = text
                .lines()
                .find_map(|line| line.strip_prefix("# typy-expect: rejected "))
                .and_then(|rest| rest.split_whitespace().next());
            match wanted {
                Some(code) => assert!(found.iter().any(|each| each == code), "{path}: {found:?}"),
                None => assert_eq!(found, Vec::<String>::new(), "{path}"),
            }
            checked += 1;
        }
    }
    assert!(checked > 20, "{checked}");
}
