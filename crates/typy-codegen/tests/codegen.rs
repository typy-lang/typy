//! Tests of code generation through the public API: programs are parsed, checked, lowered and turned into LLVM IR and object code.

use camino::Utf8PathBuf;
use typy_check::TypeChecker;
use typy_codegen::{Optimization, Target, assembly, llvm_ir, object};
use typy_source::{Project, ProjectSettings};
use typy_tyir::TyirModule;

fn compile(source: &str) -> (Project, TyirModule) {
    use std::sync::atomic::{AtomicU32, Ordering};
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let directory = std::env::temp_dir().join(format!(
        "typy-codegen-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let root = Utf8PathBuf::from_path_buf(directory.clone()).unwrap();
    std::fs::write(root.join("app.py"), source).unwrap();
    let project = Project::load(&root.join("app.py"), ProjectSettings::new(root))
        .expect("the program parses");
    let program = TypeChecker::check(&project)
        .unwrap_or_else(|diagnostics| panic!("rejected: {diagnostics:?}"));
    let module =
        typy_lower::lower(&program).unwrap_or_else(|diagnostics| panic!("{diagnostics:?}"));
    let _ = std::fs::remove_dir_all(directory);
    (project, module)
}

const PROGRAMS: &[&str] = &[
    "print(1 + 2)\n",
    "def fib(n: int) -> int:\n    if n < 2:\n        return n\n    return fib(n - 1) + fib(n - 2)\nprint(fib(10))\n",
    "def greet(name: str, punctuation: str = \"!\") -> str:\n    return \"hello \" + name + punctuation\nprint(greet(\"world\"), greet(\"you\", \"?\"))\n",
    "total = 0\nfor i in range(10):\n    total += i\nwhile total > 100:\n    total = total // 2\nelse:\n    pass\nprint(total, sep=\", \", end=\"\\n\")\n",
    "print(len(\"abc\"), abs(-2.5), abs(-3), min(3, 1, 2), max(\"a\", \"b\"), int(\"12\"), float(3), str(1.5), bool(0))\n",
    "print(repr(\"x\"), ord(\"a\"), chr(98), \"abc\"[-1], 7 / 2, 2 ** 10, 1 < 2 <= 3, not True, 1.5 != 2.5)\n",
    "print(1 < 2.5, 2.5 >= 1, \"a\" < \"b\" < \"c\", True == False, 1 != 2, not 1, not \"\", 1 and 2, \"\" or \"x\")\n",
    "y = 1.5\ny /= 2.0\ny += 1.0\ns = \"a\"\ns += \"\\u00e9\\u03b1\\U0001f600\"\ns *= 2\nprint(y, s, -y, 123456789012345678901234567890)\n",
    "def f(flag: bool = True, ratio: float = 0.5, none: None = None) -> None:\n    return None\nf()\nx = 1\nprint(x is None, None is None)\n",
    "n = 3\nfor i in range(10, 0, -n):\n    if i == 4:\n        break\n    print(i)\nelse:\n    print(\"done\")\n",
    "x = 0\n",
    "def less(a: int, b: int) -> bool:\n    return a < b\npair = (less(1, 2), (\"x\", 2.5), None)\na, (b, c), d = pair\nprint(pair, a, b, c, d, pair[1][0], len(pair), pair == pair, pair < pair)\n",
    "",
    "class E:\n    pass\ne = E()\nprint(e == e, e != E(), e is e, bool(e), str(e) == repr(e), [e] == [e])\n",
    "class P:\n    \"\"\"A point.\"\"\"\n    def __init__(self, x: int, s: str) -> None:\n        self.x = x\n        self.s = s\n        self.none: None = None\n        self.items: list[int] = []\n        self.next: P | None = None\n    def add(self, v: int) -> None:\n        self.items += [v]\n        self.x += v\n        self.s *= 2\n    def __repr__(self) -> str:\n        return 'P' + str(self.x)\np = P(1, 'a')\np.add(2)\nq = P(2, 'b')\nq.next = p\nn = q.next\nprint(P(3, 'c').x, P(4, 'd').s, p.none, p.items, p, repr(q), [p, q], n is p, n is not None, p is q)\n",
    "class R:\n    def __init__(self, n: int) -> None:\n        self.n = n\n    def __str__(self) -> str:\n        return str(10 // self.n)\n    def __repr__(self) -> str:\n        return 'R' + str(10 // self.n)\nr = R(2)\nprint(r, repr(r), [r], {r: r.n})\nr.n = 0\nprint(r)\n",
    "class T:\n    def __init__(self, pair: tuple[int, str], flag: bool, ratio: float) -> None:\n        self.pair = pair\n        self.flag = flag\n        self.ratio = ratio\n        self.maybe: bool | None = None\nt = T((1, 'x'), True, 0.5)\nt.maybe = False\nt.ratio /= 2.0\nprint(t.pair, t.flag, t.ratio, t.maybe, {t} == {t})\n",
    "class P:\n    pass\nx: int | None = None\nkinds = [int, P, type(P()), type(x)]\nprint(kinds, type(1) == int, int != str, P.__name__, type(x).__name__, bool(int), {str: 1}[type('a')], type({1: 1}.values()), object, type(int))\n",
    "class P:\n    pass\nx = 1\nf = 2.5\nb = True\ns = 'h\\u00e9'\nm: int | None = None\nn: int | None = 4\nitems = [1]\nprint(f'', f'{x}', f'{x:>5}', f'{f:.2f}', f'{b}', f'{b:d}', f'{s:^7}', f'{m}', f'{m:>4}', f'{n:03}', f'{items}', f'{items:}', f'{P():}', f'{x!s}', f'{s!r:>8}', f'{s!a}', f'{x = }', f'{f=:.1e}', f'{x:{x}}', f'{{}}')\n",
    "def check(n: int) -> int:\n    if n < 0:\n        raise ValueError('negative', n, [n])\n    return n\ne: LookupError = KeyError('k')\nprint(e, repr(e), f'{e}', type(e), [e], check(1))\nraise RuntimeError('r') from e\n",
    "def fail() -> str:\n    raise TypeError from None\nraise NotImplementedError\n",
    "def sign(b: bool) -> int:\n    if b:\n        return 1\n    else:\n        return -1\nprint(sign(True), sign(False))\n",
    "def first(items: list[int]) -> int:\n    try:\n        return items[0]\n    except KeyError:\n        return -1\n    except LookupError as e:\n        print(repr(e))\n    return 0\ntotal = 0\nfor n in [1, 0, 2]:\n    try:\n        total += 10 // n\n    except ZeroDivisionError as e:\n        print(e)\n        continue\n    except:\n        break\n    else:\n        print(total)\ntry:\n    print(first([]))\nexcept ValueError:\n    raise\n",
    "def label(n: int) -> str:\n    text = str(n)\n    try:\n        if n > 1:\n            return text * 2\n        print(10 // n)\n    except ZeroDivisionError as e:\n        return str(e)\n    finally:\n        print('done', n)\n    return text\nfor n in range(4):\n    try:\n        if n == 1:\n            continue\n        if n == 3:\n            break\n    finally:\n        print(label(n))\ntry:\n    raise KeyError('k')\nfinally:\n    raise RuntimeError('r')\n",
    "class AppError(Exception):\n    pass\n\n\nclass ConfigError(AppError):\n    def __init__(self, key: str, line: int) -> None:\n        super().__init__(f\"bad key {key}\")\n        self.key = key\n        self.line = line\n\n    def where(self) -> str:\n        return f\"{self.key} at line {self.line}\"\n\n\nclass MissingKey(ConfigError):\n    pass\n\n\nclass Quiet(ValueError):\n    def __str__(self) -> str:\n        return \"quiet!\"\n\n\ndef load(key: str) -> int:\n    if key == \"port\":\n        raise MissingKey(key, 7)\n    if key == \"host\":\n        raise Quiet(\"host\", 1)\n    if key == \"app\":\n        raise AppError\n    return 1\n\n\nfor key in [\"port\", \"host\", \"app\", \"user\"]:\n    try:\n        print(key, load(key))\n    except ConfigError as error:\n        print(\"config:\", error, repr(error), error.where(), error.key, type(error).__name__)\n    except ValueError as error:\n        print(\"value:\", error, repr(error), type(error).__name__)\n    except AppError as error:\n        print(\"app:\", repr(error), str(error) == \"\")\ne = MissingKey(\"k\", 2)\nprint(e.line)\nraise MissingKey(\"timeout\", 3)\n",
    "class HttpError(Exception):\n    def __init__(self, status: int, reason: str) -> None:\n        super().__init__(status, reason)\n        self.status = status\n        self.reason = reason\n\n\nclass RetryableError(HttpError):\n    def __init__(self, status: int, reason: str, delay: float) -> None:\n        super().__init__(status, reason)\n        self.delay = delay\n        self.attempts: list[int] = []\n\n\nerrors: list[HttpError] = [HttpError(404, \"missing\"), RetryableError(503, \"busy\", 1.5)]\nfor error in errors:\n    print(error.status, error.reason, repr(error), str(error))\nretry = RetryableError(429, \"slow down\", 0.25)\nretry.attempts.append(1)\nretry.status += 1\nprint(retry.status, retry.delay, retry.attempts, retry)\ntry:\n    raise retry\nexcept HttpError as caught:\n    print(\"caught\", caught.status, type(caught).__name__)\n",
    "class Coded(Exception):\n    def __init__(self, code: int = 1) -> None:\n        super().__init__()\n        self.code = code\ne = Coded()\nf = Coded(code=2)\nprint(e.code, f.code, repr(e))\ndef g() -> int:\n    try:\n        return 1\n    finally:\n        raise ValueError('in finally')\nprint(g())\n",
    "def first(items: list[int] | None) -> int:\n    assert items is not None, 'required'\n    assert len(items) > 0\n    return items[0]\nprint(first([3]), first([]) if False else 0)\nassert first([1]) == 2, ('mismatch', 2)\n",
];

#[test]
fn every_program_generates_valid_llvm_ir_at_every_level() {
    for source in PROGRAMS {
        let (project, module) = compile(source);
        for optimization in [Optimization::None, Optimization::Speed, Optimization::Size] {
            let ir = llvm_ir(&module, Target::host(), optimization, &project)
                .unwrap_or_else(|diagnostics| panic!("{diagnostics:?}\n{source}"));
            assert!(
                ir.contains("define ") && ir.contains("@__typy_main"),
                "{source}\n{ir}"
            );
        }
    }
}

#[test]
fn object_files_are_elf_and_deterministic() {
    let (project, module) = compile(PROGRAMS[1]);
    let first = object(&module, Target::host(), Optimization::Speed, &project).unwrap();
    let second = object(&module, Target::host(), Optimization::Speed, &project).unwrap();
    assert_eq!(&first.bytes[..4], b"\x7fELF");
    assert_eq!(first, second);
    assert_eq!(first.target, Target::host());
}

#[test]
fn the_entry_is_the_only_external_function() {
    let (project, module) = compile(PROGRAMS[1]);
    let ir = llvm_ir(&module, Target::host(), Optimization::None, &project).unwrap();
    assert!(ir.contains("define ptr @__typy_main()"), "{ir}");
    assert!(
        ir.contains("define internal { i64, ptr } @_TY8__main__3fib(i64"),
        "{ir}"
    );
}

#[test]
fn raising_calls_add_a_traceback_entry_with_the_line() {
    let (project, module) = compile("def f(n: int) -> int:\n    return 10 // n\nprint(f(0))\n");
    let ir = llvm_ir(&module, Target::host(), Optimization::None, &project).unwrap();
    assert!(ir.contains("call void @typy_rt_traceback_add("), "{ir}");
    assert!(ir.contains("i32 2)"), "the division is on line 2:\n{ir}");
    assert!(ir.contains("c\"app.py\\00\""), "{ir}");
}

#[test]
fn constants_are_immortal_static_objects() {
    let (project, module) = compile("print(\"hi\", 123456789012345678901234567890)\n");
    let ir = llvm_ir(&module, Target::host(), Optimization::None, &project).unwrap();
    assert!(ir.contains("private unnamed_addr constant { i32, i32, i64, i32, i32, [2 x i8] } { i32 -1, i32 1, i64 2, i32 1, i32 0, [2 x i8] c\"hi\" }"), "{ir}");
    assert!(
        ir.contains("{ i32 -1, i32 3, i32 2, i32 0, [2 x i64]"),
        "{ir}"
    );
}

#[test]
fn assembly_is_text_for_the_target() {
    let (project, module) = compile("print(1)\n");
    let text = assembly(&module, Target::host(), Optimization::Speed, &project).unwrap();
    assert!(text.contains("__typy_main"), "{text}");
}

#[test]
fn release_small_marks_every_function_minsize() {
    let (project, module) = compile(PROGRAMS[1]);
    let ir = llvm_ir(&module, Target::host(), Optimization::Size, &project).unwrap();
    assert!(ir.contains("minsize") && ir.contains("optsize"), "{ir}");
}

#[test]
fn reading_an_item_of_a_value_that_is_not_a_tuple_is_an_internal_error() {
    let (project, mut module) = compile("x = 0\n");
    let entry = module.entry.0 as usize;
    let function = &mut module.functions[entry];
    let item = typy_tyir::ValueId(u32::try_from(function.values.len()).unwrap());
    function.values.push(typy_tyir::ValueInfo {
        ty: typy_tyir::TyirType::Int,
        representation: typy_tyir::ValueRepresentation::Inline,
    });
    let span = function.blocks[0].instructions[0].span;
    function.blocks[0]
        .instructions
        .push(typy_tyir::Instruction {
            result: Some(item),
            operation: typy_tyir::Operation::TupleItem {
                tuple: typy_tyir::ValueId(0),
                index: 0,
            },
            span,
        });
    let error = llvm_ir(&module, Target::host(), Optimization::None, &project).unwrap_err();
    let text = format!("{error:?}");
    assert!(text.contains("is not a tuple"), "{text}");
}
