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
fn fun_02_a_function_that_calls_through_a_value_checks_the_stack_and_its_callee_does_not() {
    let module = lower(
        "from typing import Callable\n\n\ndef double(n: int) -> int:\n    return n * 2\n\n\n\
         def apply(f: Callable[[int], int], n: int) -> int:\n    return f(n)\n\n\n\
         x = apply(double, 3)\n",
    );
    // A cycle through a value runs a call through a value, so its caller checks the stack.
    let checks = |name: &str| {
        matches!(
            function(&module, name).blocks[0].terminator,
            Terminator::Invoke {
                callee: typy_tyir::Callee::Runtime(typy_tyir::RuntimeFunction::CheckStack),
                ..
            }
        )
    };
    assert!(checks("apply"));
    assert!(!checks("double"));
    let text = print(&module);
    assert!(text.contains("invoke value("), "{text}");
    assert!(text.contains("const function fn"), "{text}");
}

#[test]
fn fun_02_a_function_that_orders_by_a_key_checks_the_stack() {
    let module = lower(
        "def size(text: str) -> int:\n    return len(text)\n\n\n\
         def longest(words: list[str]) -> str:\n    return max(words, key=size)\n",
    );
    let longest = function(&module, "longest");
    assert!(matches!(
        longest.blocks[0].terminator,
        Terminator::Invoke {
            callee: typy_tyir::Callee::Runtime(typy_tyir::RuntimeFunction::CheckStack),
            ..
        }
    ));
    let text = print(&module);
    assert!(text.contains("call rt.list_keep_extreme("), "{text}");
}

#[test]
fn fun_04_a_def_makes_a_closure_of_its_captures_which_its_code_reads_from_its_object() {
    let module = lower(
        "def outer(step: int, label: str) -> int:\n    def add(value: int) -> int:\n        \
         return value + step + len(label)\n    return add(1)\n",
    );
    let text = print(&module);
    let outer = &text[text.find("# outer").unwrap()..];
    let outer = &outer[..outer.find("\n}").unwrap()];
    for line in [
        "new_closure fn",
        "set_capture v",
        "call _TY8__main__18outer.<locals>.add(",
    ] {
        assert!(outer.contains(line), "{line} missing from {outer}");
    }
    let add = function(&module, "add");
    // The function object comes first, then the parameter.
    assert_eq!(
        add.value(add.parameters[0]).ty,
        typy_tyir::TyirType::FUNCTION
    );
    assert_eq!(add.parameters.len(), 2);
    assert_eq!(add.qualified_name, "outer.<locals>.add");
    let reads = add
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .filter(|instruction| matches!(instruction.operation, Operation::Capture { .. }))
        .count();
    assert_eq!(reads, 2);
}

#[test]
fn fun_05_a_bound_method_tests_each_overriding_class_after_its_bases() {
    let module = lower(
        "class A:\n    def m(self) -> int:\n        return 1\n\n\nclass B(A):\n    \
         def m(self) -> int:\n        return 2\n\n\nclass C(B):\n    def m(self) -> int:\n        \
         return 3\n\n\ndef get(a: A) -> int:\n    f = a.m\n    return f()\n",
    );
    let text = print(&module);
    let get = &text[text.find("# get").unwrap()..];
    // The descriptor of `A.m`, then a test and a descriptor for `B`, then for `C`, so the
    // last test that holds is of the object's own class.
    let order: Vec<usize> = [
        "const method_descriptor fn1",
        "const type class#1",
        "const method_descriptor fn2",
        "const type class#2",
        "const method_descriptor fn3",
        "call rt.method_new(",
    ]
    .iter()
    .map(|line| {
        get.find(line)
            .unwrap_or_else(|| panic!("{line} missing from {get}"))
    })
    .collect();
    assert!(order.is_sorted(), "{get}");
    assert_eq!(get.matches("rt.is_instance").count(), 2, "{get}");
}

#[test]
fn nfr_07_an_operand_a_later_call_may_free_holds_a_reference_and_a_quiet_one_does_not() {
    let source = |right: &str| {
        format!(
            "class Box:\n    def __init__(self) -> None:\n        self.text = \"t\"\n\n\n\
             box = Box()\n\n\ndef other() -> str:\n    return \"o\"\n\n\n\
             def joined() -> str:\n    return box.text + {right}\n"
        )
    };
    let counted = |right: &str| {
        let module = lower(&source(right));
        let joined = function(&module, "joined");
        joined
            .blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .filter(|instruction| matches!(instruction.operation, Operation::Increment(_)))
            .count()
    };
    // `other()` may bind `box` again and free its text, which the addition still reads.
    assert_eq!(counted("other()"), 1);
    assert_eq!(counted("box.text"), 0);
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

#[test]
fn only_functions_test_that_a_module_variable_is_bound() {
    let source = "\
count = 0
names: list[str] = []


def bump() -> int:
    global count
    count += 1
    names.append('x')
    return count


bump()
print(count, names)
";
    let module = lower(source);
    assert_eq!(
        module
            .globals
            .iter()
            .map(|global| global.name.as_str())
            .collect::<Vec<_>>(),
        [
            "count",
            "count bound",
            "names",
            "names bound",
            "__main__ state"
        ]
    );
    let text = text(source);
    let bump = text
        .split("fn ")
        .find(|function| function.contains("# bump"))
        .unwrap();
    let main = text
        .split("fn ")
        .find(|function| function.contains("# <module>"))
        .unwrap();
    // The function tests the flag at each of its three reads; the module's code never does.
    assert_eq!(bump.matches("rt.name_error(").count(), 3, "{bump}");
    assert_eq!(main.matches("rt.name_error(").count(), 0, "{main}");
    assert!(main.contains("store_global g1"), "{main}");
}

#[test]
fn imports_initialise_each_module_once_and_test_what_they_read() {
    use typy_check::TypeChecker;
    let directory = std::env::temp_dir().join(format!("typy-lower-modules-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(directory.join("pkg")).unwrap();
    let files = [
        (
            "app.py",
            "import pkg.data\nfrom pkg.data import value\nprint(pkg.data.value, value, pkg.data)\n",
        ),
        ("pkg/__init__.py", ""),
        ("pkg/data.py", "value = 1\n"),
    ];
    for (path, text) in files {
        std::fs::write(directory.join(path), text).unwrap();
    }
    let root = Utf8PathBuf::from_path_buf(directory.clone()).unwrap();
    let project = Project::load(&root.join("app.py"), ProjectSettings::new(root)).unwrap();
    let program = TypeChecker::check(&project).unwrap();
    let module = typy_lower::lower(&program).unwrap();
    let _ = std::fs::remove_dir_all(directory);
    let names: Vec<&str> = module
        .modules
        .iter()
        .map(|module| module.name.as_str())
        .collect();
    assert_eq!(names, ["pkg", "pkg.data", "__main__"]);
    assert_eq!(module.modules[1].path, "pkg/data.py");
    let text = print(&module);
    let main = text
        .split("fn ")
        .find(|function| function.contains("__typy_main"))
        .unwrap();
    // Each import calls the code of `pkg` and `pkg.data` behind a test of their states, so
    // they run once; a read through the module object or `from` tests the variable.
    assert_eq!(
        main.matches("call _TY").count() + main.matches("invoke _TY").count(),
        4,
        "{main}"
    );
    assert!(main.contains("rt.module_import_error("), "{main}");
    assert!(main.contains("rt.module_attribute_error("), "{main}");
    assert!(main.contains("const module m1"), "{main}");
}

#[test]
fn a_try_whose_body_cannot_raise_lowers_and_verifies() {
    let module = lower(concat!(
        "def f() -> list[str]:\n",
        "    out: list[str] = []\n",
        "    try:\n",
        "        out.append(\"a\")\n",
        "    except ValueError:\n",
        "        print(\"never\")\n",
        "    return out\n",
        "\n",
        "\n",
        "print(f())\n",
    ));
    assert!(function(&module, "f").may_raise);
}

#[test]
fn print_to_a_file_inside_an_except_clause_lowers_and_verifies() {
    lower(concat!(
        "def f() -> None:\n",
        "    file = open(\"x.txt\", \"w\")\n",
        "    try:\n",
        "        raise ValueError(\"x\")\n",
        "    except ValueError as error:\n",
        "        print([error], 1, file=file, sep=\"-\")\n",
        "\n",
        "\n",
        "f()\n",
    ));
}

#[test]
fn classes_with_overrides_dispatch_and_tests_narrow() {
    let source = "\
class Shape:
    def area(self) -> float:
        return 0.0

    def __str__(self) -> str:
        return 'shape'


class Square(Shape):
    def __init__(self, side: float) -> None:
        self.side = side

    def area(self) -> float:
        if self.side < 0:
            raise ValueError('negative')
        return self.side * self.side + super().area()


class Count:
    def __init__(self) -> None:
        self.at = 0

    def __iter__(self) -> 'Count':
        return self

    def __next__(self) -> int:
        raise StopIteration

    def __enter__(self) -> 'Count':
        return self

    def __exit__(self, kind, error, trace) -> None:
        pass


class Twice(Count):
    def __next__(self) -> int:
        if self.at > 1:
            raise StopIteration
        self.at += 1
        return self.at

    def __enter__(self) -> 'Count':
        return self


def area(shape: Shape | None) -> float:
    if isinstance(shape, Square):
        return shape.side
    if isinstance(shape, (Shape, object)):
        return 0.5
    return 0.0


counts: list[Count] = [Count(), Twice()]
for count in counts:
    with count as entered:
        for value in entered:
            print(value, next(iter(Twice()), 0))
maybe: Shape | None = Square(2.0)
print(area(maybe), area(None), type(maybe).__name__, str(maybe), [maybe])
print(isinstance(3, (int, str)), isinstance(maybe, object), isinstance(None, int))


def kind(shape: Shape | None) -> str:
    return type(shape).__name__


def lookup(error: Exception) -> str:
    if isinstance(error, KeyError):
        return str(error)
    return ''


print(kind(None), kind(Square(1.0)), lookup(KeyError('k')), repr(Shape()))
";
    let printed = text(source);
    for expected in [
        "rt.is_instance",
        "downcast",
        "upcast",
        "call class#",
        "rt.object_type",
        "rt.object_str",
    ] {
        assert!(printed.contains(expected), "{expected} in\n{printed}");
    }
}

#[test]
fn class_variables_are_globals_and_defaults_fill_new_instances() {
    let printed = text(
        "class A:\n    count = 0\n    size: int = 1\n    def __init__(self) -> None:\n        A.count += 1\n\
         class B(A):\n    size: int = 2\n\
         print(A().size, B().size, A.count)\n",
    );
    for expected in ["\"A.count\"", "\"B.size\"", "set_field"] {
        assert!(printed.contains(expected), "{expected} in\n{printed}");
    }
}

#[test]
fn properties_call_their_accessors_and_static_methods_take_no_instance() {
    let printed = text(concat!(
        "class A:\n",
        "    def __init__(self) -> None:\n",
        "        self._x = 1\n",
        "    @staticmethod\n",
        "    def twice(n: int) -> int:\n",
        "        return n * 2\n",
        "    @property\n",
        "    def x(self) -> int:\n",
        "        return self._x\n",
        "    @x.setter\n",
        "    def x(self, value: int) -> None:\n",
        "        self._x = value\n",
        "class B(A):\n",
        "    @property\n",
        "    def x(self) -> int:\n",
        "        return 5\n",
        "items: list[A] = [A(), B()]\n",
        "for item in items:\n",
        "    item.x = A.twice(item.x)\n",
        "    print(item.x, item.twice(3))\n",
    ));
    assert!(printed.contains("call class#0.method#"), "{printed}");
    assert!(printed.contains("A.twice"), "{printed}");
}

#[test]
fn each_class_has_its_own_copy_of_a_class_method() {
    let printed = text(concat!(
        "class Shape:\n",
        "    count = 0\n",
        "    def __init__(self) -> None:\n",
        "        self.n = 1\n",
        "    @classmethod\n",
        "    def make(cls) -> 'Shape':\n",
        "        cls.count += 1\n",
        "        return cls()\n",
        "class Square(Shape):\n",
        "    count = 5\n",
        "print(Shape.make().n, Square.make().n, Square.count)\n",
    ));
    assert!(printed.contains("Shape.make"), "{printed}");
    assert!(printed.contains("Square.make"), "{printed}");
}

#[test]
fn comparisons_of_instances_call_their_methods_and_swap_for_a_subclass() {
    let printed = text(concat!(
        "class V:\n",
        "    def __init__(self, n: int) -> None:\n",
        "        self.n = n\n",
        "    def __eq__(self, other: 'V') -> bool:\n",
        "        return self.n == other.n\n",
        "class W(V):\n",
        "    def __eq__(self, other: 'V') -> bool:\n",
        "        return True\n",
        "a: V = V(1)\n",
        "b: V = W(2)\n",
        "print(a == b, a != b, b == a)\n",
    ));
    // A comparison swaps for any subclass operand, without comparing methods (CLS-07).
    assert!(!printed.contains("same_method"), "{printed}");
    assert!(printed.contains("rt.is_instance"), "{printed}");
}

#[test]
fn a_reflected_method_inherited_from_a_base_is_compared_through_the_base_s_table() {
    let printed = text(concat!(
        "class B:\n",
        "    def __add__(self, other: 'B') -> int:\n",
        "        return 0\n",
        "    def __radd__(self, other: 'B') -> int:\n",
        "        return 1\n",
        "class V(B):\n",
        "    pass\n",
        "class W(V):\n",
        "    def __radd__(self, other: 'B') -> int:\n",
        "        return 2\n",
        "a: V = V()\n",
        "b: V = W()\n",
        "print(a + b)\n",
    ));
    assert!(printed.contains("same_method class#0"), "{printed}");
}

#[test]
fn a_runtime_call_that_may_run_a_raising_slot_is_followed_by_its_test() {
    let printed = text(concat!(
        "class V:\n",
        "    def __init__(self, n: int) -> None:\n",
        "        self.n = n\n",
        "    def __lt__(self, other: 'V') -> bool:\n",
        "        return 1 // other.n > 0\n",
        "print(sorted([V(1), V(2)])[0].n, len('ab'))\n",
    ));
    let sort = printed.find("rt.list_sort").expect("the list is sorted");
    assert!(printed[sort..].contains("rt.slot_raised"), "{printed}");
}

#[test]
fn a_program_whose_slots_cannot_raise_has_no_slot_test() {
    let printed = text(concat!(
        "class V:\n",
        "    def __init__(self, n: int) -> None:\n",
        "        self.n = n\n",
        "    def __lt__(self, other: 'V') -> bool:\n",
        "        return self.n < other.n\n",
        "print(sorted([V(1), V(2)])[0].n)\n",
    ));
    assert!(!printed.contains("rt.slot_raised"), "{printed}");
}

#[test]
fn an_ordering_method_that_may_give_way_fills_the_slot_with_its_bridge() {
    let printed = text(concat!(
        "class V:\n",
        "    def __init__(self, n: int) -> None:\n",
        "        self.n = n\n",
        "    def __lt__(self, other: 'V') -> bool:\n",
        "        if other.n < 0:\n",
        "            return NotImplemented\n",
        "        return self.n < other.n\n",
        "print(V(1) < V(2), sorted([V(2), V(1)])[0].n)\n",
    ));
    for line in ["V.<less>", "rt.ordering_unsupported", "rt.slot_raised"] {
        assert!(printed.contains(line), "{line:?} missing from\n{printed}");
    }
}

#[test]
fn an_eq_that_may_give_way_fills_the_slot_with_its_bridge() {
    let printed = text(concat!(
        "class V:\n",
        "    def __init__(self, n: int) -> None:\n",
        "        self.n = n\n",
        "    def __eq__(self, other: 'V') -> bool:\n",
        "        if other.n < 0:\n",
        "            return NotImplemented\n",
        "        return self.n == other.n\n",
        "print(V(1) in [V(1)], V(1) == V(-1))\n",
    ));
    for line in ["V.<equal>", "rt.not_implemented"] {
        assert!(printed.contains(line), "{line:?} missing from\n{printed}");
    }
}

#[test]
fn an_operator_whose_reflected_method_is_overridden_decides_in_a_loop() {
    let printed = text(concat!(
        "class V:\n",
        "    def __init__(self, n: int) -> None:\n",
        "        self.n = n\n",
        "    def __add__(self, other: 'V') -> 'V':\n",
        "        return V(self.n + other.n)\n",
        "    def __radd__(self, other: 'V') -> 'V':\n",
        "        return V(other.n + self.n)\n",
        "    def __eq__(self, other: 'V') -> bool:\n",
        "        return self.n == other.n\n",
        "class W(V):\n",
        "    def __radd__(self, other: 'V') -> 'V':\n",
        "        return V(0)\n",
        "    def __eq__(self, other: 'V') -> bool:\n",
        "        return True\n",
        "total: V = V(0)\n",
        "for each in [V(1), W(2)]:\n",
        "    total = total + each\n",
        "    print(total == each)\n",
        "print(total.n)\n",
    ));
    assert!(printed.contains("same_method"), "{printed}");
}

#[test]
fn a_right_operand_typed_as_a_base_is_compared_through_the_left_class_s_table() {
    let printed = text(concat!(
        "class B:\n",
        "    def __add__(self, other: 'B') -> int:\n",
        "        return 0\n",
        "    def __radd__(self, other: 'B') -> int:\n",
        "        return 1\n",
        "class V(B):\n",
        "    def __radd__(self, other: 'B') -> int:\n",
        "        return 2\n",
        "class W(V):\n",
        "    def __radd__(self, other: 'B') -> int:\n",
        "        return 3\n",
        "left: V = V()\n",
        "right: B = W()\n",
        "print(left + right)\n",
    ));
    assert!(printed.contains("downcast"), "{printed}");
}

#[test]
fn container_methods_and_truth_call_the_class_s_methods() {
    let printed = text(concat!(
        "class G:\n",
        "    def __len__(self) -> int:\n",
        "        return 2\n",
        "    def __delitem__(self, key: int) -> None:\n",
        "        pass\n",
        "class F:\n",
        "    def __bool__(self) -> bool:\n",
        "        return False\n",
        "g = G()\n",
        "del g[0]\n",
        "print(len(g), bool(g), not F(), any([F()]))\n",
        "if g:\n",
        "    print('full')\n",
    ));
    assert!(printed.contains("rt.int_length"), "{printed}");
}

#[test]
fn rarely_written_programs_lower_and_verify() {
    for source in [
        "d = {1: 2}\nd.update()\nprint(d)\n",
        "def f(x: int | None = 3) -> int | None:\n    return x\nprint(f(), f(None))\n",
        "f = open('out.txt', 'w')\ntry:\n    print(f'{f:>3}')\nexcept TypeError as e:\n    print(e)\nf.close()\n",
        "e: list[int] = []\nprint(max([1, 2], default=0), max(e, default=0), min(e, default=5))\n",
        "try:\n    for i in range(0, 3, 0):\n        print(i)\nexcept ValueError as e:\n    print(e)\n",
        "import math\nprint(math.perm(5, None), math.perm(5))\n",
        "def f() -> None:\n    x = None\n    print(x)\nf()\n",
        "xs = [1, 2]\nxs[0] += 1\nprint(xs)\n",
    ] {
        lower(source);
    }
}

#[test]
fn a_bool_recursion_limit_is_set_as_an_int() {
    let module = lower("import sys\nflag = True\nsys.setrecursionlimit(flag)\n");
    assert!(typy_tyir::verify(&module).is_ok());
    let text = print(&module);
    // The `bool` picks the `int` 1 or 0, which the runtime takes.
    for line in [
        "const int 1",
        "const int 0",
        "invoke rt.sys_setrecursionlimit(",
    ] {
        assert!(text.contains(line), "{line:?} missing from\n{text}");
    }
}
