//! Tests of `toolchain`.

use typy_codegen::TargetTriple;

use super::Toolchain;

#[test]
fn each_target_gets_the_tools_its_host_builds_it_with() {
    let gnu = TargetTriple::X86_64LinuxGnu;
    let musl = TargetTriple::X86_64LinuxMusl;
    let arm = TargetTriple::Aarch64LinuxGnu;
    let cases = [
        (gnu, gnu, ["cc", "ar", "objcopy"], false),
        (musl, gnu, ["musl-gcc", "ar", "objcopy"], true),
        (musl, musl, ["cc", "ar", "objcopy"], true),
        (
            arm,
            gnu,
            [
                "aarch64-linux-gnu-gcc",
                "aarch64-linux-gnu-ar",
                "aarch64-linux-gnu-objcopy",
            ],
            false,
        ),
        (arm, arm, ["cc", "ar", "objcopy"], false),
        (
            gnu,
            arm,
            [
                "x86_64-linux-gnu-gcc",
                "x86_64-linux-gnu-ar",
                "x86_64-linux-gnu-objcopy",
            ],
            false,
        ),
        (
            musl,
            arm,
            [
                "x86_64-linux-musl-gcc",
                "x86_64-linux-musl-ar",
                "x86_64-linux-musl-objcopy",
            ],
            true,
        ),
        (
            TargetTriple::Aarch64AppleDarwin,
            TargetTriple::Aarch64AppleDarwin,
            ["cc", "libtool", ""],
            false,
        ),
    ];
    for (target, host, [linker, archiver, object_copier], static_link) in cases {
        let tools = Toolchain::of(target, host);
        assert_eq!(
            (
                tools.linker.as_str(),
                tools.archiver.as_str(),
                tools.object_copier.as_str(),
                tools.static_link
            ),
            (linker, archiver, object_copier, static_link),
            "{target} on {host}"
        );
    }
}

#[test]
fn typy_toml_names_tools_in_place_of_the_defaults() {
    let tools = Toolchain::of(TargetTriple::Aarch64LinuxGnu, TargetTriple::X86_64LinuxGnu)
        .with_overrides(Some("/opt/cross/bin/gcc"), None, Some("llvm-objcopy"));
    assert_eq!(
        (
            tools.linker.as_str(),
            tools.archiver.as_str(),
            tools.object_copier.as_str()
        ),
        ("/opt/cross/bin/gcc", "aarch64-linux-gnu-ar", "llvm-objcopy")
    );
}

#[test]
fn typy_toml_knows_the_targets_the_code_generator_supports() {
    let triples: Vec<&str> = TargetTriple::ALL
        .iter()
        .map(|triple| triple.as_str())
        .collect();
    assert_eq!(triples, typy_source::TARGET_TRIPLES);
}
