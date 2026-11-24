//! Tests of `mode`.

use super::BuildMode;
use typy_codegen::Optimization;

#[test]
fn each_mode_runs_its_optimisation_and_links_its_runtime() {
    let cases = [
        (BuildMode::DEBUG, Optimization::None, "debug", "typy"),
        (BuildMode::RELEASE, Optimization::Speed, "release", "typy"),
        (
            BuildMode::RELEASE_SMALL,
            Optimization::Size,
            "release-small",
            "typy-small",
        ),
        (
            BuildMode::of(Optimization::Basic),
            Optimization::Basic,
            "O1",
            "typy",
        ),
        (
            BuildMode::of(Optimization::Aggressive),
            Optimization::Aggressive,
            "O3",
            "typy",
        ),
        (
            BuildMode::of(Optimization::Small),
            Optimization::Small,
            "Os",
            "typy-small",
        ),
    ];
    for (mode, optimization, name, runtime) in cases {
        assert_eq!(
            (mode.optimization(), mode.name(), mode.runtime_directory()),
            (optimization, name, runtime)
        );
        assert_eq!(mode.is_debug(), optimization == Optimization::None);
    }
    assert_eq!(BuildMode::default(), BuildMode::DEBUG);
}
