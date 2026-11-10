//! Tests of each verdict the suite gives, on a corpus written to a temporary directory: a test
//! TyPy fails, a test that cannot judge TyPy, and the files the suite skips.

use camino::{Utf8Path, Utf8PathBuf};
use typy_conformance::{Suite, Verdict};

fn target() -> Utf8PathBuf {
    option_env!("CARGO_TARGET_DIR").map_or_else(
        || Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target"),
        Utf8PathBuf::from,
    )
}

/// Writes `files` under a new directory and returns each file's verdict, by file name.
fn verdicts(suite: &Suite, files: &[(&str, &str)]) -> Vec<(String, Verdict)> {
    let directory = tempfile::tempdir().unwrap();
    let root = Utf8PathBuf::from_path_buf(directory.path().to_owned()).unwrap();
    for (name, text) in files {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    let report = suite.run(&root).unwrap();
    report
        .results
        .into_iter()
        .map(|(path, verdict)| (path.file_name().unwrap().to_owned(), verdict))
        .collect()
}

fn starts_with(verdict: &Verdict, kind: &str, text: &str) -> bool {
    match verdict {
        Verdict::Fail(reason) => kind == "fail" && reason.starts_with(text),
        Verdict::Invalid(reason) => kind == "invalid" && reason.starts_with(text),
        Verdict::Pass => kind == "pass",
    }
}

#[test]
fn each_way_a_test_fails_or_cannot_judge_typy_gets_its_verdict() {
    let suite = Suite::new(target().join("debug/typy"));
    let deep = "def depth(n: int) -> int:\n    if n == 0:\n        return 0\n    return 1 + depth(n - 1)\n\n\nprint(depth(5000))\n";
    let files = [
        ("EXP-02_rejected_by_typy.py", "# typy-test: EXP-02\n# typy-expect: accepted\nprint(eval(\"1\"))\n".to_owned()),
        ("FUN-02_deeper_than_cpython.py", format!("# typy-test: FUN-02\n# typy-expect: accepted\n{deep}")),
        ("TYP-03_accepted_by_typy.py", "# typy-test: TYP-03\n# typy-expect: rejected E0402 at 1:1\nprint(1)\n".to_owned()),
        ("TYP-03_another_code.py", "# typy-test: TYP-03\n# typy-expect: rejected E0401 at 3:10\nx: int = \"a\"\n".to_owned()),
        ("EXP-02_without_header.py", "print(1)\n".to_owned()),
        ("EXP-02_later_floor.py", "# typy-test: EXP-02\n# typy-expect: accepted\n# typy-python: 3.13\nprint(1)\n".to_owned()),
        ("STD-03_versions_disagree.py", "# typy-test: STD-03\n# typy-expect: accepted\nimport sys\nprint(sys.version_info[1])\n".to_owned()),
        ("nested/EXP-02_in_a_directory.py", "# typy-test: EXP-02\n# typy-expect: accepted\nprint(2)\n".to_owned()),
        ("README.txt", "not a test\n".to_owned()),
    ];
    let files: Vec<(&str, &str)> = files
        .iter()
        .map(|(name, text)| (*name, text.as_str()))
        .collect();
    let mut results = verdicts(&suite, &files);
    results.sort_by(|left, right| left.0.cmp(&right.0));
    let expected = [
        ("EXP-02_in_a_directory.py", "pass", ""),
        (
            "EXP-02_later_floor.py",
            "invalid",
            "a Python floor other than 3.12",
        ),
        (
            "EXP-02_rejected_by_typy.py",
            "fail",
            "TyPy rejected the program in debug mode",
        ),
        (
            "EXP-02_without_header.py",
            "invalid",
            "no `# typy-test:` line",
        ),
        (
            "FUN-02_deeper_than_cpython.py",
            "fail",
            "the debug build shows",
        ),
        (
            "STD-03_versions_disagree.py",
            "invalid",
            "the oracles disagree",
        ),
        (
            "TYP-03_accepted_by_typy.py",
            "fail",
            "TyPy accepted the program",
        ),
        (
            "TYP-03_another_code.py",
            "fail",
            "expected only E0401 at 3:10, got E0402",
        ),
    ];
    assert_eq!(results.len(), expected.len(), "{results:?}");
    for ((name, verdict), (wanted, kind, text)) in results.iter().zip(expected) {
        assert_eq!(name, wanted);
        assert!(starts_with(verdict, kind, text), "{name}: {verdict}");
    }
}

#[test]
fn the_sanitizer_run_without_its_instrumented_runtime_is_invalid() {
    let prefix = tempfile::tempdir().unwrap();
    let prefix = Utf8PathBuf::from_path_buf(prefix.path().to_owned()).unwrap();
    let triple = "x86_64-unknown-linux-gnu";
    let plain = Utf8Path::new("lib/typy")
        .join(triple)
        .join("libtypy_runtime.a");
    std::fs::create_dir_all(prefix.join(plain.parent().unwrap())).unwrap();
    std::fs::copy(target().join(&plain), prefix.join(&plain)).unwrap();
    let suite = Suite::new(prefix.join("bin/typy")).with_sanitizers();
    let results = verdicts(
        &suite,
        &[(
            "EXP-02_one.py",
            "# typy-test: EXP-02\n# typy-expect: accepted\nprint(1)\n",
        )],
    );
    let [(_, verdict)] = results.as_slice() else {
        panic!("{results:?}");
    };
    assert!(
        starts_with(
            verdict,
            "invalid",
            "the instrumented runtime archive is missing"
        ),
        "{verdict}"
    );
}

#[test]
fn verdicts_read_as_their_kind_and_reason() {
    assert_eq!(Verdict::Pass.to_string(), "pass");
    assert_eq!(Verdict::Fail("why".to_owned()).to_string(), "FAIL: why");
    assert_eq!(
        Verdict::Invalid("why".to_owned()).to_string(),
        "INVALID: why"
    );
}
