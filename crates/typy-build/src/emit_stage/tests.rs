//! Tests of `emit_stage`.

use camino::Utf8Path;

use super::EmitStage;

#[test]
fn every_stage_parses_from_its_name() {
    for stage in EmitStage::ALL {
        assert_eq!(EmitStage::parse(stage.name()), Some(stage));
        assert_eq!(
            stage.is_binary(),
            matches!(stage, EmitStage::LlvmBc | EmitStage::Obj)
        );
        // Code generation writes the stages from unoptimised LLVM IR on.
        assert_eq!(
            stage.codegen_output().is_some(),
            stage >= EmitStage::LlvmIrUnoptimised
        );
    }
}

#[test]
fn other_names_are_refused() {
    for text in ["", "AST", "llvm", "typed_ast", "exe", "llvm-ir-unoptimized"] {
        assert_eq!(EmitStage::parse(text), None, "{text:?}");
    }
}

#[test]
fn the_stages_sort_in_pipeline_order() {
    let mut sorted = EmitStage::ALL;
    sorted.sort_unstable();
    assert_eq!(sorted, EmitStage::ALL);
}

#[test]
fn each_stage_is_written_beside_the_output_with_its_extension() {
    let cases = [
        (EmitStage::Ast, "app", "app.ast"),
        (EmitStage::TypedAst, "app", "app.typed-ast"),
        (EmitStage::Tyir, "out/prog", "out/prog.tyir"),
        (EmitStage::TyirMem, "app", "app.tyir-mem"),
        (EmitStage::LlvmIrUnoptimised, "app", "app.unoptimised.ll"),
        (EmitStage::LlvmIr, "libtools.so.1", "libtools.ll"),
        (EmitStage::LlvmBc, "app", "app.bc"),
        (EmitStage::Asm, "app.exe", "app.s"),
        (EmitStage::Obj, ".hidden", ".hidden.o"),
    ];
    for (stage, output, expected) in cases {
        assert_eq!(
            stage.path_beside(Utf8Path::new(output)),
            expected,
            "{stage:?}"
        );
    }
}
