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
    "",
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
