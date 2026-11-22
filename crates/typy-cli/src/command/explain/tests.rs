//! Tests of `command::explain`.

use super::{PAGE, explanation, without_links};
use typy_diagnostics::{CODES, CodeStatus, DiagnosticCode};

/// The codes whose cause is not one program file: the files, the settings, a library
/// build, the C toolchain, and imports between modules, whose sections show their files
/// as text or as programs that are correct as programs.
const WITHOUT_PROGRAM: [&str; 13] = [
    "E0101", "E0102", "E0105", "E0106", "E0107", "E0422", "E0425", "E0426", "E0429", "E0701",
    "E0702", "E0703", "E9001",
];

#[test]
fn every_program_code_shows_a_program_that_causes_it_and_a_corrected_one() {
    let mut missing = Vec::new();
    for info in CODES
        .iter()
        .filter(|info| info.status == CodeStatus::Active)
    {
        let code = info.code.to_string();
        if WITHOUT_PROGRAM.contains(&code.as_str()) {
            continue;
        }
        let start = PAGE.find(&format!("## {code}:")).unwrap();
        let section = &PAGE[start + 3..];
        let section = section.find("\n## ").map_or(section, |end| &section[..end]);
        let causes = if code.starts_with('W') {
            format!("This program gives {code} and still runs:")
        } else {
            format!("<!-- typy-expect: rejected {code} at ")
        };
        let complete = [causes.as_str(), "This one is correct:"]
            .iter()
            .all(|needle| section.contains(needle));
        missing.extend((!complete).then_some(code));
    }
    assert!(
        missing.is_empty(),
        "sections without both samples: {missing:?}"
    );
}

#[test]
fn an_explanation_is_the_section_as_plain_text() {
    let text = explanation(DiagnosticCode::error(402), "unused");
    assert!(text.starts_with("E0402: assignment changes the type of a variable\n"));
    assert!(text.contains("\n    count = \"none\"\n"), "{text}");
    assert!(!text.contains("typy-expect"), "{text}");
    assert!(!text.contains("```"), "{text}");
    assert!(text.ends_with("print(count, label)\n"), "{text}");
}

#[test]
fn a_code_without_a_section_gives_its_summary() {
    assert_eq!(
        explanation(DiagnosticCode::parse("E9999").unwrap(), "a summary"),
        "E9999: a summary\n"
    );
}

#[test]
fn links_become_their_text() {
    assert_eq!(
        without_links("see the [guide](guide/index.md) and [this](a.md)."),
        "see the guide and this."
    );
    assert_eq!(without_links("a [broken link"), "a [broken link");
    assert_eq!(without_links("a [text] (b)"), "a [text] (b)");
    assert_eq!(without_links("a [text](no end"), "a [text](no end");
}
