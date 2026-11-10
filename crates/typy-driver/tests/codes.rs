//! Every diagnostic code the compiler reports is in the code registry (DIA-06).

use camino::Utf8PathBuf;
use typy_diagnostics::{DiagnosticCode, lookup};

/// Returns the numbers of every `DiagnosticCode::error(N)` in the source of `directory`.
fn reported(directory: &Utf8PathBuf, found: &mut Vec<(u16, Utf8PathBuf)>) {
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
        for piece in text.split("DiagnosticCode::error(").skip(1) {
            let digits: String = piece.chars().take_while(char::is_ascii_digit).collect();
            if let Ok(number) = digits.parse() {
                found.push((number, path.clone()));
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
        .filter(|(number, _)| lookup(DiagnosticCode::error(*number)).is_none())
        .map(|(number, path)| format!("E{number:04} in {path}"))
        .collect();
    assert!(
        missing.is_empty(),
        "codes without a registry entry:\n{}",
        missing.join("\n")
    );
}
