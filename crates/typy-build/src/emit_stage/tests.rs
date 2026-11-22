//! Tests of `emit_stage`.

use super::EmitStage;

#[test]
fn every_stage_parses_from_its_name() {
    for stage in EmitStage::ALL {
        assert_eq!(EmitStage::parse(stage.name()), Some(stage));
        assert_eq!(stage.is_binary(), stage == EmitStage::Obj);
    }
}

#[test]
fn other_names_are_refused() {
    for text in ["", "AST", "llvm", "typed_ast", "exe"] {
        assert_eq!(EmitStage::parse(text), None, "{text:?}");
    }
}
