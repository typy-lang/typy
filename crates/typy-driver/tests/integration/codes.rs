//! Every diagnostic code the compiler reports is in the code registry (DIA-06).

use camino::Utf8PathBuf;
use typy_diagnostics::{DiagnosticCode, lookup};

/// Returns every `DiagnosticCode::error(N)` and `DiagnosticCode::warning(N)` in the source of
/// `directory`.
fn reported(directory: &Utf8PathBuf, found: &mut Vec<(DiagnosticCode, Utf8PathBuf)>) {
    for entry in directory.read_dir_utf8().unwrap() {
        let path = entry.unwrap().into_path();
        if path.is_dir() {
            reported(&path, found);
            continue;
        }
        if path.extension() != Some("rs") {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap();
        for (call, make) in [
            (
                "DiagnosticCode::error(",
                DiagnosticCode::error as fn(u16) -> DiagnosticCode,
            ),
            ("DiagnosticCode::warning(", DiagnosticCode::warning),
        ] {
            for piece in text.split(call).skip(1) {
                let digits: String = piece.chars().take_while(char::is_ascii_digit).collect();
                if let Ok(number) = digits.parse() {
                    found.push((make(number), path.clone()));
                }
            }
        }
    }
}

#[test]
fn every_reported_code_is_registered() {
    let crates = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut found = Vec::new();
    for name in [
        "typy-source",
        "typy-check",
        "typy-lower",
        "typy-codegen",
        "typy-build",
        "typy-driver",
        "typy-cli",
    ] {
        let source = crates.join(name).join("src");
        if source.is_dir() {
            reported(&source, &mut found);
        }
    }
    assert_ne!(found.len(), 0, "the sources report codes");
    let missing: Vec<String> = found
        .iter()
        .filter(|(code, _)| lookup(*code).is_none())
        .map(|(code, path)| format!("{code} in {path}"))
        .collect();
    assert!(
        missing.is_empty(),
        "codes without a registry entry:\n{}",
        missing.join("\n")
    );
}
