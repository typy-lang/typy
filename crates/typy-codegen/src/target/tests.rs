//! Tests of `target`.

use super::{Target, TargetTriple};

#[test]
fn every_triple_parses_from_its_text() {
    for triple in TargetTriple::ALL {
        assert_eq!(TargetTriple::parse(triple.as_str()), Some(triple));
        assert_eq!(triple.to_string(), triple.as_str());
    }
}

#[test]
fn llvm_is_given_linux_triples_as_written_and_macos_with_its_oldest_release() {
    for triple in TargetTriple::ALL {
        let expected = if triple.is_macos() {
            "arm64-apple-macosx11.0.0"
        } else {
            triple.as_str()
        };
        assert_eq!(triple.llvm_triple(), expected);
    }
}

#[test]
fn unsupported_triples_are_refused() {
    for text in [
        "",
        "aarch64-unknown-linux-musl",
        "x86_64-apple-darwin",
        "arm64-apple-macosx11.0.0",
        "x86_64-pc-windows-msvc",
        "x86_64",
    ] {
        assert_eq!(TargetTriple::parse(text), None, "{text:?}");
    }
}

#[test]
fn narrow_integers_are_extended_on_x86_64_and_not_under_aapcs64() {
    assert!(TargetTriple::X86_64LinuxGnu.extends_narrow_integers());
    assert!(TargetTriple::X86_64LinuxMusl.extends_narrow_integers());
    assert!(!TargetTriple::Aarch64LinuxGnu.extends_narrow_integers());
    assert!(TargetTriple::Aarch64AppleDarwin.extends_narrow_integers());
}

#[test]
fn a_linux_host_builds_for_linux_and_a_macos_host_for_macos() {
    let linux = [
        TargetTriple::X86_64LinuxGnu,
        TargetTriple::X86_64LinuxMusl,
        TargetTriple::Aarch64LinuxGnu,
    ];
    for host in linux {
        for target in linux {
            assert!(target.builds_on(host), "{target} on {host}");
        }
        assert!(!TargetTriple::Aarch64AppleDarwin.builds_on(host));
        assert!(!host.builds_on(TargetTriple::Aarch64AppleDarwin));
    }
    let macos = TargetTriple::Aarch64AppleDarwin;
    assert!(macos.builds_on(macos));
}

#[test]
fn the_host_is_a_supported_target() {
    assert!(TargetTriple::ALL.contains(&Target::host().triple));
}
