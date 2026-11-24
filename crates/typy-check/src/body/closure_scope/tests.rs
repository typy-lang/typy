//! Tests of `body::closure_scope`.

use std::collections::BTreeSet;

use typy_diagnostics::FileId;
use typy_source::syntax::Stmt;
use typy_source::{PythonFloor, SourceFile, parse};

use super::ClosureScope;

/// Returns the names of the parameters and the body of the function `source` defines
/// first.
fn first_function(source: &str) -> (Vec<String>, Vec<Stmt>) {
    let file = SourceFile::from_text(FileId(0), "app.py".into(), source);
    let tree = parse(&file, PythonFloor::V3_12).expect("the source parses");
    match tree.body.into_iter().next() {
        Some(Stmt::FunctionDef {
            parameters, body, ..
        }) => (
            parameters
                .into_iter()
                .map(|parameter| parameter.name.name)
                .collect(),
            body,
        ),
        _ => panic!("the source defines a function first"),
    }
}

/// Returns, for the one function `source` defines, the captures of each of its nested
/// functions in source order, and its cell variables.
fn scope(source: &str) -> (Vec<Vec<String>>, Vec<String>) {
    let (parameters, body) = first_function(source);
    let names: Vec<&str> = parameters.iter().map(String::as_str).collect();
    let found = ClosureScope::of(&names, &body, &BTreeSet::new());
    let captures = found
        .captures
        .values()
        .map(|names| names.iter().cloned().collect())
        .collect();
    (captures, found.cells.into_iter().collect())
}

/// Returns, for the one function `source` defines, the group of each of its nested
/// functions in source order, empty for one in none.
fn groups(source: &str) -> Vec<Vec<String>> {
    let (_, body) = first_function(source);
    let found = ClosureScope::of(&[], &body, &BTreeSet::new());
    found
        .captures
        .keys()
        .map(|span| {
            found
                .groups
                .get(span)
                .map(|names| names.iter().cloned().collect())
                .unwrap_or_default()
        })
        .collect()
}

const EVEN_ODD: &str = "def f(n: int) -> bool:\n    limit = n\n    \
                            def even(k: int) -> bool:\n        return k == 0 or odd(k - 1)\n    \
                            def odd(k: int) -> bool:\n        return k < limit and even(k - 1)\n    \
                            return even(n)\n";

#[test]
fn fun_04_functions_that_call_one_another_by_name_share_their_captures() {
    let both = vec!["even".to_owned(), "odd".to_owned()];
    assert_eq!(groups(EVEN_ODD), [both.clone(), both]);
    // Each holds what either captures, and neither holds the other.
    let (captures, cells) = scope(EVEN_ODD);
    assert_eq!(
        captures,
        [vec!["limit".to_owned()], vec!["limit".to_owned()]]
    );
    assert_eq!(cells, Vec::<String>::new());
}

#[test]
fn fun_04_a_cycle_of_three_with_another_def_between_is_one_group() {
    let found = groups(
        "def f() -> None:\n    def a() -> int:\n        return b()\n    \
             def other() -> int:\n        return 1\n    def b() -> int:\n        return c()\n    \
             def c() -> int:\n        return a()\n",
    );
    let all = vec!["a".to_owned(), "b".to_owned(), "c".to_owned()];
    assert_eq!(found, [all.clone(), Vec::new(), all.clone(), all]);
}

#[test]
fn fun_04_functions_that_may_run_before_the_group_is_made_form_no_group() {
    for source in [
        // A function of the group read as a value.
        "def f() -> None:\n    def a() -> int:\n        return b()\n    \
             def b() -> int:\n        print(a)\n        return a()\n",
        // A function of the group read by a function nested in another.
        "def f() -> None:\n    def a() -> int:\n        return b()\n    \
             def b() -> int:\n        def inner() -> int:\n            return a()\n        \
             return inner()\n",
        // A statement between the `def`s, which may leave the block.
        "def f() -> None:\n    def a() -> int:\n        return b()\n    print(1)\n    \
             def b() -> int:\n        return a()\n",
        // Another function, defined before the group is complete, that reads one of it.
        "def f() -> None:\n    def early() -> int:\n        return a()\n    \
             def a() -> int:\n        return b()\n    def b() -> int:\n        return a()\n",
        // A function bound again.
        "def f() -> None:\n    def a() -> int:\n        return b()\n    \
             def b() -> int:\n        return a()\n    b = a\n",
        // A function declared global in another.
        "def f() -> None:\n    def a() -> int:\n        global b\n        return b()\n    \
             def b() -> int:\n        return a()\n",
    ] {
        assert!(groups(source).iter().all(Vec::is_empty), "{source}");
    }
}

#[test]
fn fun_04_a_variable_assigned_after_the_group_lives_in_a_cell_both_hold() {
    let (captures, cells) = scope(
        "def f() -> None:\n    def a() -> int:\n        return b()\n    \
             def b() -> int:\n        return a() + late\n    late = 1\n",
    );
    assert_eq!(captures, [vec!["late".to_owned()], vec!["late".to_owned()]]);
    assert_eq!(cells, ["late"]);
}

#[test]
fn fun_04_a_variable_bound_once_before_the_def_is_copied() {
    let (captures, cells) = scope(
        "def outer(n: int) -> None:\n    total = n + 1\n    def inner() -> int:\n        \
             return total + n + len([n])\n",
    );
    assert_eq!(captures, [vec!["n".to_owned(), "total".to_owned()]]);
    assert_eq!(cells, Vec::<String>::new());
}

#[test]
fn fun_04_a_variable_a_loop_binds_or_bound_again_or_after_the_def_lives_in_a_cell() {
    for (source, cell) in [
        (
            "def f() -> None:\n    for i in range(3):\n        def g() -> int:\n            \
                 return i\n",
            "i",
        ),
        (
            "def f() -> None:\n    x = 1\n    def g() -> int:\n        return x\n    x = 2\n",
            "x",
        ),
        (
            "def f() -> None:\n    def g() -> int:\n        return x\n    x = 1\n",
            "x",
        ),
        (
            "def f(x: int) -> None:\n    x += 1\n    def g() -> int:\n        return x\n",
            "x",
        ),
        (
            "def f() -> None:\n    while True:\n        x = 1\n        break\n    \
                 def g() -> int:\n        return x\n",
            "x",
        ),
        (
            "def f() -> None:\n    try:\n        pass\n    except ValueError as e:\n        \
                 def g() -> str:\n            return str(e)\n",
            "e",
        ),
        (
            "def f() -> None:\n    y = [z := 1 for _ in range(2)]\n    \
                 def g() -> int:\n        return z\n",
            "z",
        ),
    ] {
        let (_, cells) = scope(source);
        assert_eq!(cells, [cell.to_owned()], "{source}");
    }
}

#[test]
fn fun_04_only_the_functions_own_variables_are_captured() {
    // A global, a name the inner function binds, a comprehension's own target and a
    // module name are not captures; a name a deeper function reads passes through.
    let (captures, cells) = scope(
        "def f() -> None:\n    global g\n    g = 1\n    a = 1\n    b = 2\n    \
             def inner() -> None:\n        b = 3\n        print(g, b, [a for a in range(2)])\n        \
             def deeper() -> int:\n            return a\n",
    );
    assert_eq!(captures, [vec!["a".to_owned()]]);
    assert_eq!(cells, Vec::<String>::new());
}

/// Returns, for the one function `source` defines, its cell variables and the variables
/// its nested functions assign through `nonlocal`.
fn rebound(source: &str) -> (Vec<String>, Vec<String>) {
    let (_, body) = first_function(source);
    let found = ClosureScope::of(&[], &body, &BTreeSet::new());
    (
        found.cells.into_iter().collect(),
        found.rebound.into_iter().collect(),
    )
}

#[test]
fn smt_09_a_variable_a_nested_function_assigns_through_nonlocal_lives_in_a_cell() {
    for source in [
        // Bound once before the `def`, which would copy it, and assigned in the function.
        "def f() -> None:\n    count = 0\n    def bump() -> None:\n        nonlocal count\n        \
             count += 1\n",
        // Assigned by a function nested two deep, through one that never names it.
        "def f() -> None:\n    count = 0\n    def middle() -> None:\n        \
             def inner() -> None:\n            nonlocal count\n            count = 2\n        \
             inner()\n",
        // Declared by an annotation alone, which binds it as CPython's symbol table does.
        "def f() -> None:\n    count: int\n    def bump() -> None:\n        nonlocal count\n        \
             count = 1\n",
    ] {
        let (cells, rebound) = rebound(source);
        assert_eq!(cells, ["count"], "{source}");
        assert_eq!(rebound, ["count"], "{source}");
    }
}

#[test]
fn smt_09_a_variable_named_nonlocal_passes_through_the_function_in_between() {
    let (_, body) = first_function(
        "def middle() -> None:\n    def inner() -> None:\n        nonlocal far\n        far = 1\n",
    );
    let enclosing: BTreeSet<String> = ["far".to_owned()].into();
    let found = ClosureScope::of(&[], &body, &enclosing);
    let captures: Vec<Vec<String>> = found
        .captures
        .values()
        .map(|names| names.iter().cloned().collect())
        .collect();
    assert_eq!(captures, [vec!["far".to_owned()]]);
    // `far` is the enclosing function's, so this function neither keeps nor rebinds it.
    assert!(found.cells.is_empty());
    assert!(found.rebound.is_empty());
}

#[test]
fn smt_09_an_annotation_alone_gives_no_value_so_one_assignment_is_still_copied() {
    let (captures, cells) = scope(
        "def f() -> None:\n    total: int\n    total = 1\n    def g() -> int:\n        \
             return total\n",
    );
    assert_eq!(captures, [vec!["total".to_owned()]]);
    assert_eq!(cells, Vec::<String>::new());
}

#[test]
fn smt_09_a_function_another_assigns_through_nonlocal_is_in_no_group() {
    let found = groups(
        "def f() -> None:\n    def a() -> int:\n        return b()\n    \
             def b() -> int:\n        nonlocal a\n        return a()\n",
    );
    assert!(found.iter().all(Vec::is_empty), "{found:?}");
}

#[test]
fn exp_14_a_lambda_captures_as_a_nested_function_does() {
    // A variable bound once before the `lambda` is copied; a loop variable is shared.
    let (captures, cells) = scope(
        "def f(step: int) -> None:\n    add = lambda x: x + step\n    for i in range(3):\n        \
             show = lambda: i\n",
    );
    assert_eq!(captures, [vec!["step".to_owned()], vec!["i".to_owned()]]);
    assert_eq!(cells, ["i"]);
}

#[test]
fn exp_14_a_lambda_in_the_value_of_its_own_assignment_shares_the_variable() {
    let (captures, cells) =
        scope("def f() -> None:\n    fact = lambda n: 1 if n == 0 else n * fact(n - 1)\n");
    assert_eq!(captures, [vec!["fact".to_owned()]]);
    assert_eq!(cells, ["fact"]);
}

#[test]
fn exp_14_a_comprehension_variable_a_lambda_reads_lives_in_a_cell() {
    let (captures, cells) =
        scope("def f() -> None:\n    made = [lambda: i * k for i in range(3)]\n    k = 2\n");
    assert_eq!(captures, [vec!["i".to_owned(), "k".to_owned()]]);
    assert_eq!(cells, ["i", "k"]);
}

#[test]
fn exp_14_names_a_lambda_binds_are_its_own() {
    let (captures, cells) = scope("def f(x: int) -> None:\n    g = lambda y: (z := y + x) + z\n");
    assert_eq!(captures, [vec!["x".to_owned()]]);
    assert_eq!(cells, Vec::<String>::new());
}

#[test]
fn smt_09_nonlocal_under_a_function_that_declares_the_name_global_rebinds_nothing_here() {
    let (cells, rebound) = rebound(
        "def f() -> None:\n    x = 1\n    def g() -> None:\n        global x\n        \
             def h() -> None:\n            nonlocal x\n            x = 2\n",
    );
    assert_eq!(
        (cells, rebound),
        (Vec::<String>::new(), Vec::<String>::new())
    );
}

#[test]
fn fun_04_a_function_declared_global_or_read_by_a_later_def_keeps_its_group_rules() {
    // A name declared `global` is the module's, so its `def`s form no group.
    let global = groups(
        "def f() -> None:\n    global a\n    def a() -> int:\n        return b()\n    \
             def b() -> int:\n        return a()\n",
    );
    assert!(global.iter().all(Vec::is_empty), "{global:?}");
    // A `def` after the group that reads one of it runs once the group is complete.
    let later = groups(
        "def f() -> None:\n    def a() -> int:\n        return b()\n    \
             def b() -> int:\n        return a()\n    def last() -> int:\n        return a()\n",
    );
    let both = vec!["a".to_owned(), "b".to_owned()];
    assert_eq!(later, [both.clone(), both, Vec::new()]);
}

#[test]
fn fun_04_an_annotated_attribute_binds_no_variable() {
    let (captures, cells) = scope(
        "def f(box: Box) -> None:\n    box.size: int\n    def g() -> Box:\n        return box\n",
    );
    assert_eq!(captures, [vec!["box".to_owned()]]);
    assert_eq!(cells, Vec::<String>::new());
}

#[test]
fn fun_04_an_assignment_expression_in_a_comprehension_in_a_loop_binds_in_the_loop() {
    let (_, cells) = scope(
        "def f() -> None:\n    for _ in range(2):\n        xs = [y := 1 for _ in range(2)]\n    \
             def g() -> int:\n        return y\n",
    );
    assert_eq!(cells, ["y"]);
}

#[test]
fn fun_04_a_name_a_nested_function_declares_global_is_no_capture() {
    let (captures, cells) = scope(
        "def f() -> None:\n    g = 1\n    def inner() -> int:\n        global g\n        \
             return g\n    g = 2\n",
    );
    assert_eq!(captures, [Vec::<String>::new()]);
    assert_eq!(cells, Vec::<String>::new());
}

#[test]
fn fun_04_the_enclosing_functions_variables_pass_through_to_a_nested_function() {
    let (_, body) = first_function(
        "def middle() -> None:\n    def inner() -> int:\n        return far + near\n    near = 1\n",
    );
    let enclosing: BTreeSet<String> = ["far".to_owned()].into();
    let found = ClosureScope::of(&[], &body, &enclosing);
    let captures: Vec<Vec<String>> = found
        .captures
        .values()
        .map(|names| names.iter().cloned().collect())
        .collect();
    assert_eq!(captures, [vec!["far".to_owned(), "near".to_owned()]]);
    // `far` is the enclosing function's, so only `near` is this function's cell.
    assert_eq!(found.cells.into_iter().collect::<Vec<_>>(), ["near"]);
}
