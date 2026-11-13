//! End-to-end tests of deeply nested programs (LEX-11, NFR-09): `typy` compiles what CPython
//! compiles, however deep, and refuses what nests past the limits with an error, never a crash.

mod common;

use std::process::Command;

use common::Program;

#[test]
fn a_program_nested_as_deep_as_cpython_compiles_builds_and_runs() {
    let source = format!(
        "x = 1{}\ny = {}1\nprint(x, y)\n",
        " + 1".repeat(22_685),
        "-".repeat(5_964)
    );
    let output = Program::build(&source).run(&[], b"");
    assert_eq!(String::from_utf8_lossy(&output.stdout), "22686 1\n");
}

#[test]
fn nesting_past_the_limits_is_an_error_and_not_a_crash() {
    let cases = [
        (
            format!("x = {}1{}\n", "(".repeat(201), ")".repeat(201)),
            "too many nested parentheses",
        ),
        (
            format!("x = {}1\n", "-".repeat(200_000)),
            "nests more than 25000 levels deep",
        ),
        (
            format!("x = 1{}\n", " + 1".repeat(30_000)),
            "nests more than 25000 levels deep",
        ),
    ];
    for (source, message) in cases {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("app.py"), &source).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_typy"))
            .args(["check", "app.py"])
            .current_dir(directory.path())
            .output()
            .unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(1), "{message}: {stderr}");
        assert!(stderr.contains("error[E0103]"), "{stderr}");
        assert!(stderr.contains(message), "{message}: {stderr}");
    }
}
