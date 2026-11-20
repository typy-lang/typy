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
