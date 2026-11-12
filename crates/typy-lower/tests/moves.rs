//! Tests of moving a variable's value at its last use, and an owned value into a collection,
//! through the public API (MEM-09): which stores count and which hand the reference over.

use camino::Utf8PathBuf;
use typy_check::TypeChecker;
use typy_source::{Project, ProjectSettings};
use typy_tyir::{TyirModule, print};

fn lower(source: &str) -> TyirModule {
    let directory =
        std::env::temp_dir().join(format!("typy-moves-{}-{}", std::process::id(), next_id()));
    std::fs::create_dir_all(&directory).unwrap();
    let root = Utf8PathBuf::from_path_buf(directory.clone()).unwrap();
    std::fs::write(root.join("app.py"), source).unwrap();
    let project = Project::load(&root.join("app.py"), ProjectSettings::new(root))
        .expect("the program parses");
    let program = TypeChecker::check(&project)
        .unwrap_or_else(|diagnostics| panic!("rejected: {diagnostics:?}\n{source}"));
    let module = typy_lower::lower(&program)
        .unwrap_or_else(|diagnostics| panic!("{diagnostics:?}\n{source}"));
    let _ = std::fs::remove_dir_all(directory);
    module
}

fn next_id() -> u32 {
    use std::sync::atomic::{AtomicU32, Ordering};
    static NEXT: AtomicU32 = AtomicU32::new(0);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

fn text(source: &str) -> String {
    print(&lower(source))
}

/// Returns the TyIR text of function `name` of `source`, which defines it at module level.
fn function_text(source: &str, name: &str) -> String {
    let all = text(source);
    let start = all
        .lines()
        .position(|line| line.starts_with("fn ") && line.ends_with(&format!("# {name}")))
        .unwrap_or_else(|| panic!("no function {name} in\n{all}"));
    all.lines()
        .skip(start)
        .take_while(|line| *line != "}")
        .collect::<Vec<_>>()
        .join("\n")
}

fn counts(text: &str, what: &str) -> usize {
    text.lines().filter(|line| line.contains(what)).count()
}

#[test]
fn the_last_use_in_an_assignment_moves_without_counting() {
    let text = function_text(
        "def f() -> list[int]:\n    a = [1]\n    b = a\n    return b\n\n\nprint(f())\n",
        "f",
    );
    assert_eq!(counts(&text, "inc v"), 0, "{text}");
    assert!(text.contains("const placeholder list"), "{text}");
}

#[test]
fn a_store_of_a_last_use_or_of_an_owned_value_hands_the_reference_over() {
    let source = concat!(
        "def f() -> list[str]:\n",
        "    out: list[str] = []\n",
        "    word = \"a\" + \"b\"\n",
        "    out.append(word)\n",
        "    out.append(str(1))\n",
        "    out.insert(0, str(2))\n",
        "    pair = [str(3), \"c\" * 2]\n",
        "    table: dict[str, list[str]] = {}\n",
        "    table[\"k\"] = pair\n",
        "    found: set[str] = set()\n",
        "    found.add(str(4))\n",
        "    print(table, found)\n",
        "    return out\n",
        "\n\nprint(f())\n",
    );
    let text = function_text(source, "f");
    for moved in [
        "rt.list_append_moved",
        "rt.list_insert_moved",
        "rt.dict_set_moved",
        "rt.set_add_moved",
    ] {
        assert!(text.contains(moved), "{moved} missing from\n{text}");
    }
    assert_eq!(counts(&text, "rt.list_append("), 0, "{text}");
}

#[test]
fn a_variable_read_again_is_counted() {
    let cases = [
        // The next iteration reads the variable.
        "def f() -> list[str]:\n    out: list[str] = []\n    word = \"a\" + \"b\"\n    for _ in range(2):\n        out.append(word)\n    return out\n",
        // The statement reads it twice.
        "def f() -> list[str]:\n    word = \"a\" + \"b\"\n    out = [word, word]\n    return out\n",
        // A later statement reads it.
        "def f() -> list[str]:\n    word = \"a\" + \"b\"\n    out = [word]\n    print(word)\n    return out\n",
        // An `except` clause may read it.
        "def f() -> list[str]:\n    out: list[str] = []\n    word = \"a\" + \"b\"\n    try:\n        out.append(word)\n    except ValueError:\n        print(word)\n    return out\n",
        // A `finally` block reads it after the `return`.
        "def f() -> list[str]:\n    word = \"a\" + \"b\"\n    try:\n        out = [word]\n        return out\n    finally:\n        print(word)\n",
        // A later `if` test, a loop source or an assignment target reads it.
        "def f() -> list[str]:\n    word = \"a\" + \"b\"\n    out = [word]\n    if word:\n        print(1)\n    return out\n",
        "def f() -> list[str]:\n    word = \"a\" + \"b\"\n    out = [word]\n    for c in word:\n        print(c)\n    return out\n",
        "def f() -> list[str]:\n    word = \"a\" + \"b\"\n    out = [word]\n    table = {\"\": 1}\n    table[word] = 2\n    return out\n",
        // A clause's name, inside an `if` or a `with`, is released when the clause ends.
        "def f() -> list[str]:\n    out: list[str] = []\n    if len(out) == 0:\n        try:\n            raise ValueError(\"x\")\n        except ValueError as error:\n            print([error])\n    return out\n",
        "def f() -> list[str]:\n    out: list[str] = []\n    with open(\"x.txt\", \"w\") as file:\n        try:\n            raise ValueError(\"x\")\n        except ValueError as error:\n            print([error], file=file)\n    return out\n",
        // A comprehension's body runs once for each value.
        "def f() -> list[list[str]]:\n    word = \"a\" + \"b\"\n    return [[word] for _ in range(2)]\n",
    ];
    for body in cases {
        let source = format!("{body}\n\nprint(f())\n");
        let text = function_text(&source, "f");
        // The variable's store counts; a comprehension's own list takes each new list by move.
        assert!(text.contains("rt.list_append("), "{body}\n{text}");
        if !body.contains(" for _ in ") {
            assert!(!text.contains("rt.list_append_moved"), "{body}\n{text}");
        }
    }
}

#[test]
fn a_loop_moves_a_value_the_next_iteration_assigns_again() {
    let source = concat!(
        "def f() -> list[str]:\n",
        "    out: list[str] = []\n",
        "    for index in range(3):\n",
        "        word = str(index) + \"!\"\n",
        "        if index == 1:\n",
        "            continue\n",
        "        out.append(word)\n",
        "    return out\n",
        "\n\nprint(f())\n",
    );
    let text = function_text(source, "f");
    assert!(text.contains("rt.list_append_moved"), "{text}");
}

#[test]
fn a_borrowed_parameter_a_clause_name_and_a_module_variable_are_not_moved() {
    let cases = [
        (
            "def f(item: list[int]) -> list[list[int]]:\n    return [item]\n\n\nprint(f([1]))\n",
            "f",
        ),
        (
            "def f() -> list[ValueError]:\n    try:\n        raise ValueError(\"x\")\n    except ValueError as error:\n        return [error]\n\n\nprint(f())\n",
            "f",
        ),
    ];
    for (source, name) in cases {
        let text = function_text(source, name);
        assert!(!text.contains("_moved"), "{source}\n{text}");
    }
    let text = text("word = \"a\" + \"b\"\nout = [word]\nprint(out)\n");
    assert!(text.contains("rt.list_append("), "{text}");
}

#[test]
fn each_display_moves_a_local_item() {
    let source = concat!(
        "def f() -> None:\n",
        "    a = \"a\" + \"1\"\n",
        "    b = \"b\" + \"1\"\n",
        "    c = \"c\" + \"1\"\n",
        "    d = \"d\" + \"1\"\n",
        "    row = [a]\n",
        "    found = {b}\n",
        "    table = {\"k\": c}\n",
        "    pair = (d, 1)\n",
        "    print(row, found, table, pair)\n",
        "\n\nf()\n",
    );
    let text = function_text(source, "f");
    for moved in [
        "rt.list_append_moved",
        "rt.set_add_moved",
        "rt.dict_set_moved",
    ] {
        assert!(text.contains(moved), "{moved} missing from\n{text}");
    }
    // Each variable moves; the one count left is of the constant `1`, an `int` word.
    assert_eq!(counts(&text, "inc v"), 1, "{text}");
}

#[test]
fn a_tuple_assignment_ends_the_value_a_store_before_it_moves() {
    let source = concat!(
        "def f() -> list[str]:\n",
        "    out: list[str] = []\n",
        "    word = \"a\" + \"b\"\n",
        "    out.append(word)\n",
        "    word, other = \"x\", \"y\"\n",
        "    print(word, other)\n",
        "    return out\n",
        "\n\nprint(f())\n",
    );
    let text = function_text(source, "f");
    assert!(text.contains("rt.list_append_moved"), "{text}");
}

#[test]
fn a_view_of_a_dict_is_counted_since_its_zero_value_allocates() {
    for body in [
        "    v = d.keys()\n    w = v\n    print(w)\n",
        "    v = (d.keys(), 1)\n    w = v\n    print(w)\n",
    ] {
        let source = format!("def f(d: dict[str, int]) -> None:\n{body}\n\nf({{}})\n");
        let text = function_text(&source, "f");
        assert!(text.contains("inc v"), "{body}\n{text}");
    }
}
