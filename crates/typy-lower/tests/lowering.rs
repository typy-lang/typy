//! Tests of lowering through the public API: programs are parsed, checked, lowered and verified.

use camino::Utf8PathBuf;
use typy_check::TypeChecker;
use typy_source::{Project, ProjectSettings};
use typy_tyir::{Operation, Terminator, TyirModule, print};

fn lower(source: &str) -> TyirModule {
    let directory =
        std::env::temp_dir().join(format!("typy-lower-{}-{}", std::process::id(), next_id()));
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

fn function<'module>(module: &'module TyirModule, name: &str) -> &'module typy_tyir::TyirFunction {
    module
        .functions
        .iter()
        .find(|function| function.name == name)
        .unwrap_or_else(|| panic!("no function {name}"))
}

#[test]
fn print_of_an_addition_lowers_to_an_add_a_conversion_and_writes() {
    let text = text("print(1 + 2)\n");
    for line in [
        "fn __typy_main() -> unit raises",
        "int.add",
        "invoke rt.int_to_str",
        "call rt.write_stdout",
        " = \"\\n\"",
    ] {
        assert!(text.contains(line), "{line:?} missing from\n{text}");
    }
}

#[test]
fn every_program_of_the_checker_tests_lowers_and_verifies() {
    for source in [
        "def fib(n: int) -> int:\n    if n < 2:\n        return n\n    return fib(n - 1) + fib(n - 2)\nprint(fib(10))\n",
        "def greet(name: str, punctuation: str = \"!\") -> str:\n    return \"hello \" + name + punctuation\nprint(greet(\"world\"), greet(\"you\", \"?\"))\n",
        "total = 0\nfor i in range(10):\n    total += i\nwhile total > 100:\n    total = total // 2\nelse:\n    pass\nprint(total, sep=\", \", end=\"\\n\")\n",
        "print(len(\"abc\"), abs(-2.5), abs(-3), min(3, 1, 2), max(\"a\", \"b\"), int(\"12\"), float(3), str(1.5), bool(0))\n",
        "print(repr(\"x\"), ord(\"a\"), chr(98), \"abc\"[-1], 7 / 2, 2 ** 10, 1 < 2 <= 3, not True)\n",
        "print(int(), float(), str(), bool(), int(\"ff\", 16), int(2.5), int(True), float(\"1.5\"), float(True))\n",
        "x = 1\nprint(x is None, x is not None, None is None, None == None)\n",
        "print(1 < 2.5, 2.5 >= 1, \"a\" < \"b\" < \"c\", True == False, 1 != 2)\n",
        "print(not 1, not \"\", not None, 1 and 2, \"\" or \"x\", 1.5 or 2.5)\n",
        "x = 5\nx -= 1\nx *= 2\nx //= 3\nx %= 2\nx **= 2\nx <<= 1\nx >>= 1\nx |= 1\nx ^= 1\nx &= 1\nprint(x)\n",
        "y = 1.5\ny /= 2.0\ny += 1.0\ns = \"a\"\ns += \"b\"\ns *= 2\nprint(y, s, 3 * \"z\")\n",
        "print(~5, +5, -5.5, 7 % 3, 7.5 % 2, 7 // 2.0, 1 + 2.0, 2.0 ** -1)\n",
        "def f(flag: bool = True, name: str = \"n\", ratio: float = 0.5, none: None = None) -> None:\n    return None\nf()\n",
        "for i in range(1, 5):\n    if i == 2:\n        continue\n    if i == 4:\n        break\nelse:\n    print(\"done\")\n",
        "n = 3\nfor i in range(10, 0, -n):\n    print(i)\n",
        "a = b = \"shared\"\nprint(a, b, a if a else b)\nprint(True & False, True | False, True ^ True)\n",
        "while True:\n    break\nprint(__name__)\n",
        "def g() -> str:\n    return \"g\"\nprint(g())\nreturned = g()\n",
    ] {
        let module = lower(source);
        assert!(typy_tyir::verify(&module).is_ok(), "{source}");
    }
}

#[test]
fn functions_that_cannot_raise_are_called_without_invoke() {
    let module = lower("def double(n: int) -> int:\n    return n + n\nx = double(2)\n");
    assert!(!function(&module, "double").may_raise);
    let text = print(&module);
    assert!(text.contains("call _TY8__main__6double("), "{text}");
}

#[test]
fn raising_propagates_to_callers_through_invoke() {
    let module = lower(
        "def inner(n: int) -> int:\n    return 10 // n\ndef outer(n: int) -> int:\n    return inner(n) + 1\nx = outer(2)\n",
    );
    assert!(function(&module, "inner").may_raise);
    assert!(function(&module, "outer").may_raise);
    assert!(function(&module, "<module>").may_raise);
    let text = print(&module);
    assert!(text.contains("invoke _TY8__main__5inner("), "{text}");
    assert!(text.contains("invoke _TY8__main__5outer("), "{text}");
}

#[test]
fn recursive_functions_check_the_stack_on_entry() {
    let module = lower(
        "def down(n: int) -> int:\n    if n == 0:\n        return 0\n    return down(n - 1)\nx = down(3)\n",
    );
    let down = function(&module, "down");
    assert!(down.may_raise);
    let entry_block = &down.blocks[0];
    assert!(matches!(
        entry_block.terminator,
        Terminator::Invoke {
            callee: typy_tyir::Callee::Runtime(typy_tyir::RuntimeFunction::CheckStack),
            ..
        }
    ));
}

#[test]
fn error_paths_release_the_slots_and_return_the_exception() {
    let module = lower("def f(s: str, n: int) -> str:\n    t = s + \"!\"\n    return t[n]\n");
    let f = function(&module, "f");
    let error_block = f
        .blocks
        .iter()
        .find(|block| matches!(block.terminator, Terminator::ReturnError(_)))
        .expect("an error path");
    let decrements = error_block
        .instructions
        .iter()
        .filter(|instruction| matches!(instruction.operation, Operation::Decrement(_)))
        .count();
    // `t` is released; `s` borrows its caller's reference and is not (MEM-04).
    assert!(
        decrements >= 1,
        "the slot of `t` is released:\n{}",
        print(&module)
    );
}

#[test]
fn stores_count_the_new_value_and_release_the_old_one() {
    let text = text("s = \"a\"\ns = \"b\"\n");
    assert!(text.contains("inc v"), "{text}");
    assert!(text.contains("dec v"), "{text}");
}

#[test]
fn the_text_without_counting_leaves_out_only_inc_and_dec() {
    let module = lower("s = \"a\"\ns = \"b\"\nprint(s)\n");
    let full = print(&module);
    let plain = typy_tyir::print_without_counting(&module);
    assert!(
        !plain.contains("inc v") && !plain.contains("dec v"),
        "{plain}"
    );
    let kept: Vec<&str> = full
        .lines()
        .filter(|line| {
            !line.trim_start().starts_with("inc ") && !line.trim_start().starts_with("dec ")
        })
        .collect();
    assert_eq!(plain.lines().collect::<Vec<_>>(), kept);
}

#[test]
fn a_zero_step_is_checked_only_when_the_step_is_not_a_nonzero_constant() {
    assert!(!text("for i in range(0, 3, 1):\n    pass\n").contains("range_check_step"));
    assert!(
        text("n = 1\nfor i in range(0, 3, n):\n    pass\n").contains("invoke rt.range_check_step")
    );
}

#[test]
fn comparing_an_int_with_a_float_is_exact_in_the_runtime() {
    let text = text("print(1 < 2.5, 2.5 < 1)\n");
    assert_eq!(
        text.matches("call rt.compare_int_float").count(),
        2,
        "{text}"
    );
}

#[test]
fn the_module_body_is_the_entry_symbol_and_strings_are_pooled_once() {
    let module = lower("print(\"x\", \"x\")\n");
    assert_eq!(module.function(module.entry).symbol, "__typy_main");
    assert_eq!(
        module
            .strings
            .iter()
            .filter(|string| string.as_str() == "x")
            .count(),
        1
    );
}

#[test]
fn only_displays_of_three_to_thirty_constants_are_copied_as_cpython_merges_them() {
    let copies = |display: &str| {
        text(&format!("x = 4\nprint({display})\n"))
            .matches("rt.set_copy")
            .count()
    };
    let thirty: Vec<String> = (0..30).map(|value| value.to_string()).collect();
    let thirty_one: Vec<String> = (0..31).map(|value| value.to_string()).collect();
    for (display, copied) in [
        ("{1, 2}".to_owned(), 0),
        ("{1, -2, +3}".to_owned(), 1),
        ("{-(-1.0), 2.5, -0.5}".to_owned(), 1),
        ("{(1, -2), (3, 4), (5, +6)}".to_owned(), 1),
        ("{'a', 'b', 'c'}".to_owned(), 1),
        ("{True, False, True}".to_owned(), 1),
        ("{x, 2, 3}".to_owned(), 0),
        ("{-x, 2, 3}".to_owned(), 0),
        ("{(x, 1), (2, 1), (3, 1)}".to_owned(), 0),
        ("{1 + 2, 4, 5}".to_owned(), 0),
        (format!("{{{}}}", thirty.join(", ")), 1),
        (format!("{{{}}}", thirty_one.join(", ")), 0),
    ] {
        assert_eq!(copies(&display), copied, "{display}");
    }
}

/// Returns the text of one function of a program, with its counting.
fn function_text(source: &str, name: &str) -> String {
    let text = text(source);
    let start = text
        .find(&format!("# {name}\n"))
        .unwrap_or_else(|| panic!("no function {name} in\n{text}"));
    let rest = &text[start..];
    rest[..rest.find("\n}\n").unwrap_or(rest.len())].to_owned()
}

#[test]
fn a_parameter_the_body_never_assigns_is_not_counted() {
    let source = "def size(items: list[int]) -> int:\n    return len(items)\nprint(size([1]))\n";
    let size = function_text(source, "size");
    assert!(!size.contains("inc ") && !size.contains("dec "), "{size}");
    let source = "def size(items: list[int]) -> int:\n    items = [2]\n    return len(items)\nprint(size([1]))\n";
    let size = function_text(source, "size");
    assert!(size.contains("inc v0"), "{size}");
}

#[test]
fn a_returned_variable_moves_to_the_caller() {
    let source = "def make() -> str:\n    text = 'a' + str(1)\n    return text\nprint(make())\n";
    let make = function_text(source, "make");
    let after_load = make.split("return").next().unwrap_or_default();
    assert_eq!(make.matches("inc ").count(), 0, "{make}");
    assert!(after_load.contains("load s0"), "{make}");
}

#[test]
fn an_argument_read_from_a_container_is_kept_for_the_call() {
    let source = "def first(word: str) -> str:\n    return word\nwords = ['a']\nprint(first(words[0]), first('b'))\n";
    let main = function_text(source, "<module>");
    let call = main.find("first").unwrap_or(0);
    assert!(main[..call].matches("inc ").count() >= 1, "{main}");
}

#[test]
fn dict_calls_evaluate_every_argument_before_calling_iter() {
    let source = "\
class Pairs:
    def __iter__(self) -> 'Pairs':
        return self

    def __next__(self) -> tuple[str, int]:
        raise StopIteration


def note() -> int:
    return 1


made = dict(Pairs(), k=note())
keys = dict.fromkeys(Pairs(), note())
";
    let text = text(source);
    let calls: Vec<&str> = text
        .lines()
        .filter_map(|line| {
            let callee = line.split("call _TY8__main__").nth(1)?;
            callee.split('(').next()
        })
        .collect();
    // CPython's `dict` and `dict.fromkeys` call `__iter__` once their arguments are evaluated.
    assert_eq!(
        calls,
        ["4note", "14Pairs.__iter__", "4note", "14Pairs.__iter__"],
        "{text}"
    );
}
