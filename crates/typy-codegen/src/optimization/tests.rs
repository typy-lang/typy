//! Tests of `optimization`.

use super::Optimization;

#[test]
fn each_level_has_its_pipeline() {
    assert_eq!(Optimization::None.pipeline(), "default<O0>");
    assert_eq!(Optimization::Speed.pipeline(), "default<O2>");
    assert_eq!(Optimization::Size.pipeline(), "default<O2>");
    assert!(Optimization::Size.minimise_size());
    assert!(!Optimization::Speed.minimise_size());
    assert_eq!(Optimization::None.level(), inkwell::OptimizationLevel::None);
    assert_eq!(
        Optimization::Size.level(),
        inkwell::OptimizationLevel::Default
    );
}
