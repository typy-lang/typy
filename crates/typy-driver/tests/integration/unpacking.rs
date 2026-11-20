//! Tests of unpacking through the pipeline in process: displays and targets that unpack
//! lower, verify and become LLVM IR (SMT-02, EXP-17).

use crate::pipeline::{emit, program, text};
use typy_build::EmitStage;

#[test]
fn exp_17_every_unpacking_form_lowers_verifies_and_becomes_llvm_ir() {
    let source = "from collections.abc import Iterator\n\n\n\
                  def numbers() -> Iterator[int]:\n    yield 1\n\n\n\
                  a = [1, 2]\nb = (3, 4)\nprint([*a, *b], (*a, 5), {*numbers()}, {**{\"x\": 1}, \
                  \"y\": 2})\nfirst, *rest = a\n*init, last = (1, 2, 3)\nleft, right = \
                  numbers()\nhead, *middle, tail = \"abcd\"\nprint(first, rest, init, last, \
                  left, right, head, middle, tail)\n";
    let (_guard, main) = program(source);
    let tyir = text(emit(&main, EmitStage::Tyir));
    for call in ["rt.unpack", "rt.tuple_seal", "rt.list_slice"] {
        assert!(tyir.contains(call), "{call} missing from\n{tyir}");
    }
    let ir = text(emit(&main, EmitStage::LlvmIr));
    assert!(ir.contains("typy_rt_unpack"), "{ir}");
}

#[test]
fn exp_17_star_args_and_star_star_kwargs_pack_after_every_argument_is_evaluated() {
    let source = "def tagged(name: str, *tags: str, **sizes: int) -> str:\n    \
                  return name + str(len(tags)) + str(len(sizes))\n\n\n\
                  class Shape:\n    def __init__(self, *sides: float) -> None:\n        \
                  self.sides = sides\n\n\n\
                  print(tagged(\"a\", \"b\", \"c\", width=2), tagged(\"x\"), Shape(1.0).sides)\n";
    let (_guard, main) = program(source);
    let tyir = text(emit(&main, EmitStage::Tyir));
    for call in ["rt.tuple_seal", "rt.dict_set"] {
        assert!(tyir.contains(call), "{call} missing from\n{tyir}");
    }
    let ir = text(emit(&main, EmitStage::LlvmIr));
    assert!(ir.contains("typy_rt_tuple_seal"), "{ir}");
}

#[test]
fn exp_17_a_call_that_unpacks_checks_and_binds_its_arguments_when_it_runs() {
    let source = "def pair(a: int, b: int = 0) -> int:\n    return a + b\n\n\n\
                  def sink(*args: int, **kwargs: int) -> int:\n    return len(args)\n\n\n\
                  values = [1, 2]\nprint(pair(*values), pair(**{\"a\": 1}, b=2), \
                  sink(*values, **{\"x\": 1}), pair(*(1, 2)))\nprint(*values, sep=\"-\")\n";
    let (_guard, main) = program(source);
    let tyir = text(emit(&main, EmitStage::Tyir));
    for call in [
        "rt.call_check",
        "rt.call_merge",
        "rt.call_name",
        "rt.list_get",
    ] {
        assert!(tyir.contains(call), "{call} missing from\n{tyir}");
    }
    let ir = text(emit(&main, EmitStage::LlvmIr));
    assert!(ir.contains("typy_rt_call_check"), "{ir}");
}

#[test]
fn typ_21_a_generator_in_a_union_and_an_unpacked_call_lower_and_verify() {
    let source = "from collections.abc import Generator, Iterable, Iterator\n\n\n\
                  def sink(*args: int, **kwargs: int) -> str:\n    return f\"{args} {kwargs}\"\n\n\n\
                  def numbers() -> Generator[int, None, None]:\n    yield 1\n\n\n\
                  def take(value: Iterable[int] | int) -> str:\n    \
                  if isinstance(value, Generator):\n        return \"generator\"\n    \
                  return \"other\"\n\n\n\
                  def step(value: Iterator[int] | None) -> int:\n    \
                  return -1 if value is None else next(value)\n\n\n\
                  values = [1, 2]\n\
                  print(sink(*values), take(numbers()), take(4), step(numbers()))\n";
    let (_guard, main) = program(source);
    let tyir = text(emit(&main, EmitStage::Tyir));
    for call in [
        "rt.call_check",
        "rt.object_has_type_id",
        "rt.iterator_stepper",
    ] {
        assert!(tyir.contains(call), "{call} missing from\n{tyir}");
    }
    let ir = text(emit(&main, EmitStage::LlvmIr));
    assert!(ir.contains("typy_rt_iterator_stepper"), "{ir}");
}
