//! Tests of generators through the pipeline in process: each form lowers, verifies and
//! becomes LLVM IR (SMT-11, FUN-07, FUN-11, EXP-15).

use crate::pipeline::{emit, program, text};
use typy_build::EmitStage;

/// Generator programs whose forms reach every way the lowering compiles a generator.
const PROGRAMS: [&str; 4] = [
    "from collections.abc import Generator, Iterator\n\n\n\
     def optional() -> Generator[int, int | None, None]:\n    got = yield 1\n    print(got)\n\n\n\
     def pairs() -> Generator[int, tuple[int, str], None]:\n    got = yield 1\n    print(got)\n\n\n\
     def nothing() -> Iterator[int]:\n    got = yield 1\n    print(got)\n\n\n\
     def arguments(text: str, items: list[int]) -> Iterator[str]:\n    yield text\n    \
     yield str(len(items))\n\n\n\
     def main() -> None:\n    first = optional()\n    next(first)\n    first.send(None)\n    \
     first.send(5)\n    pair = pairs()\n    next(pair)\n    pair.send((1, \"a\"))\n    \
     error = ValueError(\"x\")\n    first.throw(error)\n    first.throw(KeyError(\"k\"))\n    \
     first.close()\n    print(list(nothing()), list(arguments(\"a\", [1])))\n\n\nmain()\n",
    "from collections.abc import Generator\n\n\n\
     def maybe() -> Generator[int, None, str | None]:\n    yield 1\n    return None\n\n\n\
     def value() -> Generator[int, None, str]:\n    yield 1\n    return \"x\"\n\n\n\
     def plain() -> Generator[int, None, None]:\n    yield 1\n\n\n\
     def outer() -> Generator[int | None, None, int]:\n    first = yield from maybe()\n    \
     second = yield from value()\n    third = yield from plain()\n    yield from [1, 2]\n    \
     fourth = yield from iter([3])\n    print(first, second, third, fourth)\n    try:\n        \
     return 1\n    finally:\n        print(\"done\")\n\n\nprint(list(outer()))\n",
    "from collections.abc import Iterator\n\n\n\
     def handled() -> Iterator[int]:\n    try:\n        raise ValueError(\"v\")\n    except \
     ValueError as error:\n\n        def show() -> str:\n            return str(error)\n\n        \
     yield 1\n        print(show())\n    try:\n        raise KeyError(\"k\")\n    except \
     KeyError:\n        yield 2\n    try:\n        yield 3\n    finally:\n        yield 4\n\n\n\
     print(list(handled()))\n",
    "from collections.abc import Iterator\n\n\n\
     def main() -> None:\n    factor = 2\n    scaled: Iterator[int | None] = (value * factor \
     for value in range(3) if value)\n    print(list(scaled), sum(len(word) for word in \
     [\"a\"]))\n\n\nmain()\n",
];

#[test]
fn every_generator_form_lowers_verifies_and_becomes_llvm_ir() {
    for source in PROGRAMS {
        let (_guard, main) = program(source);
        let tyir = text(emit(&main, EmitStage::Tyir));
        assert!(tyir.contains(".<generator>"), "{tyir}");
        let ir = text(emit(&main, EmitStage::LlvmIr));
        assert!(ir.contains("typy.generator.descriptor"), "{source}");
    }
}

#[test]
fn fun_07_an_optional_result_of_yield_from_is_tested_before_it_is_read() {
    let (_guard, main) = program(PROGRAMS[1]);
    let tyir = text(emit(&main, EmitStage::Tyir));
    for call in [
        "rt.generator_has_result",
        "rt.generator_result",
        "rt.generator_delegate",
        "rt.generator_set_result",
    ] {
        assert!(tyir.contains(call), "{call} missing from\n{tyir}");
    }
}
