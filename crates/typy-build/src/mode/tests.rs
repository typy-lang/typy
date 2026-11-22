//! Tests of `mode`.

use super::BuildMode;
use typy_codegen::Optimization;

#[test]
fn each_mode_runs_its_optimisation() {
    let cases = [
        (BuildMode::Debug, Optimization::None, "debug"),
        (BuildMode::Release, Optimization::Speed, "release"),
        (BuildMode::ReleaseSmall, Optimization::Size, "release-small"),
    ];
    for (mode, optimization, name) in cases {
        assert_eq!((mode.optimization(), mode.name()), (optimization, name));
    }
    assert_eq!(BuildMode::default(), BuildMode::Debug);
}
