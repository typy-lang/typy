//! Tests of `command::print`.

use typy_codegen::TargetTriple;

use super::{processor_names, targets, toml_key, toml_string};

#[test]
fn processor_names_are_read_from_llvms_list_and_its_features_are_left_out() {
    let listed = "Available CPUs for this target:\n\n  alderlake - Select the alderlake processor.\n  \
                  x86-64    - Select the x86-64 processor.\n\nAvailable features for this target:\n\n  \
                  avx - Enable AVX instructions.\n";
    assert_eq!(processor_names(listed), ["alderlake", "x86-64"]);
    assert_eq!(processor_names("no list here\n"), [] as [&str; 0]);
}

#[test]
fn strings_and_keys_are_written_as_toml_reads_them() {
    assert_eq!(toml_string("plain"), "\"plain\"");
    assert_eq!(
        toml_string("a \"b\" \\c\n\t\u{1}"),
        "\"a \\\"b\\\" \\\\c\\n\\t\\u0001\""
    );
    assert_eq!(toml_string("caf\u{e9}"), "\"caf\u{e9}\"");
    assert_eq!(
        toml_key("x86_64-unknown-linux-gnu"),
        "x86_64-unknown-linux-gnu"
    );
    assert_eq!(toml_key("lib.so"), "\"lib.so\"");
    assert_eq!(toml_key(""), "\"\"");
}

#[test]
fn every_target_is_listed_with_the_host_and_those_another_host_builds() {
    let text = targets(TargetTriple::X86_64LinuxGnu);
    assert_eq!(
        text,
        "x86_64-unknown-linux-gnu (host)\nx86_64-unknown-linux-musl\naarch64-unknown-linux-gnu\n\
         aarch64-apple-darwin (built on another host)\n"
    );
}
