//! Tests of `command::generation`.

use typy_build::BuildMode;
use typy_codegen::{Optimization, Target, TargetTriple};

use super::{joined, resolve};
use crate::arguments::Choice;

#[test]
fn the_baseline_needs_no_processor_and_keeps_the_features_given() {
    let choice = Choice {
        mode: BuildMode::of(Optimization::Aggressive),
        processor: None,
        features: "+avx2".to_owned(),
    };
    let resolved = resolve(choice, Target::host().triple).unwrap();
    assert_eq!(resolved.mode, BuildMode::of(Optimization::Aggressive));
    assert_eq!(
        (resolved.processor, resolved.features),
        (None, "+avx2".to_owned())
    );
}

#[test]
fn cli_21_native_is_this_machine_and_only_for_its_target() {
    let host = Target::host().triple;
    let choice = |features: &str| Choice {
        processor: Some("native".to_owned()),
        features: features.to_owned(),
        ..Choice::default()
    };
    let resolved = resolve(choice("-avx"), host).unwrap();
    assert_eq!(resolved.processor, Some(typy_codegen::host_processor()));
    assert!(resolved.features.ends_with("-avx"), "{}", resolved.features);
    let other = TargetTriple::ALL
        .into_iter()
        .find(|triple| *triple != host)
        .unwrap();
    let error = resolve(choice(""), other).unwrap_err();
    assert!(
        error.starts_with("`native` is the processor of this machine"),
        "{error}"
    );
}

#[test]
fn the_baseline_named_needs_no_list() {
    let triple = Target::host().triple;
    let choice = Choice {
        processor: Some(triple.cpu().to_owned()),
        ..Choice::default()
    };
    assert_eq!(
        resolve(choice, triple).unwrap().processor.as_deref(),
        Some(triple.cpu())
    );
}

#[test]
fn feature_lists_join_with_the_later_last() {
    assert_eq!(joined("", ""), "");
    assert_eq!(joined("+a", ""), "+a");
    assert_eq!(joined("", "-b"), "-b");
    assert_eq!(joined("+a,+c", "-b"), "+a,+c,-b");
}
