//! Tests of `optimization`.

use super::Optimization;
use inkwell::OptimizationLevel;

#[test]
fn each_level_has_its_pipeline_and_its_attributes() {
    let expected = [
        (
            Optimization::None,
            "default<O0>",
            OptimizationLevel::None,
            false,
            false,
        ),
        (
            Optimization::Basic,
            "default<O1>",
            OptimizationLevel::Less,
            false,
            false,
        ),
        (
            Optimization::Speed,
            "default<O2>",
            OptimizationLevel::Default,
            false,
            false,
        ),
        (
            Optimization::Aggressive,
            "default<O3>",
            OptimizationLevel::Aggressive,
            false,
            false,
        ),
        (
            Optimization::Small,
            "default<O2>",
            OptimizationLevel::Default,
            true,
            false,
        ),
        (
            Optimization::Size,
            "default<O2>",
            OptimizationLevel::Default,
            true,
            true,
        ),
    ];
    for (level, pipeline, llvm, optsize, minsize) in expected {
        assert_eq!(level.pipeline(), pipeline, "{level:?}");
        assert_eq!(level.level(), llvm, "{level:?}");
        assert_eq!(level.optimise_size(), optsize, "{level:?}");
        assert_eq!(level.minimise_size(), minsize, "{level:?}");
    }
}

#[test]
fn cli_20_each_level_is_named_by_what_follows_minus_o() {
    for level in Optimization::ALL {
        assert_eq!(Optimization::parse(level.letter()), Some(level));
    }
    for text in ["", "4", "fast", "S", "-2"] {
        assert_eq!(Optimization::parse(text), None, "{text:?}");
    }
    assert_eq!(Optimization::default(), Optimization::None);
}
