//! Tests of the user documentation in `docs/user`: every Python sample is judged as a
//! conformance test, with the outcome the page states (DOC-04), and the troubleshooting page
//! has a section for every active diagnostic code (DOC-03).
//!
//! A sample is a `python` code block directly after a comment that states its outcome:
//! `<!-- typy-expect: accepted -->`, or `<!-- typy-expect: rejected E0401 at 2:7 -->` with
//! the line and column inside the sample.

use camino::{Utf8Path, Utf8PathBuf};
use typy_conformance::Suite;
use typy_diagnostics::{CODES, CodeStatus};

fn workspace() -> Utf8PathBuf {
    Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Returns every Markdown page under `directory`, sorted.
fn pages(directory: &Utf8Path, found: &mut Vec<Utf8PathBuf>) {
    for entry in directory.read_dir_utf8().unwrap() {
        let path = entry.unwrap().into_path();
        if path.is_dir() {
            pages(&path, found);
        } else if path.extension() == Some("md") {
            found.push(path);
        }
    }
    found.sort();
}

/// A sample of a page: where it is, the outcome its comment states, and its code.
struct Sample {
    place: String,
    expectation: String,
    code: String,
}

/// Returns the samples of a page, or the places of `python` blocks without a stated outcome.
fn samples(path: &Utf8Path, text: &str) -> Result<Vec<Sample>, Vec<String>> {
    let lines: Vec<&str> = text.lines().collect();
    let mut found = Vec::new();
    let mut unstated = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        if lines[index].trim_end() != "```python" {
            index += 1;
            continue;
        }
        let place = format!("{path}:{}", index + 1);
        let expectation = index
            .checked_sub(1)
            .and_then(|before| lines[before].strip_prefix("<!-- typy-expect: "))
            .and_then(|rest| rest.strip_suffix(" -->"));
        let end = lines[index + 1..]
            .iter()
            .position(|line| line.trim_end() == "```")
            .map_or(lines.len(), |offset| index + 1 + offset);
        let code = lines[index + 1..end].join("\n") + "\n";
        match expectation {
            Some(expectation) => found.push(Sample {
                place,
                expectation: expectation.to_owned(),
                code,
            }),
            None => unstated.push(place),
        }
        index = end + 1;
    }
    if unstated.is_empty() {
        Ok(found)
    } else {
        Err(unstated)
    }
}

/// Returns a conformance test of a sample: the header lines, then the code, with a rejected
/// sample's line moved past the header.
fn conformance_test(sample: &Sample) -> String {
    let expectation = match sample.expectation.split_once(" at ") {
        Some((code, place)) => {
            let (line, column) = place.split_once(':').unwrap_or((place, "1"));
            let line: usize = line.parse().unwrap_or(0);
            format!("{code} at {}:{column}", line + 2)
        }
        None => sample.expectation.clone(),
    };
    format!(
        "# typy-test: DOC-04\n# typy-expect: {expectation}\n{}",
        sample.code
    )
}

#[test]
fn every_sample_in_the_user_documentation_has_the_outcome_its_page_states() {
    let mut found = Vec::new();
    pages(&workspace().join("docs/user"), &mut found);
    assert_ne!(found.len(), 0, "docs/user has pages");
    let corpus = tempfile::tempdir().unwrap();
    let root = Utf8PathBuf::from_path_buf(corpus.path().to_path_buf()).unwrap();
    let mut places = Vec::new();
    let mut unstated = Vec::new();
    for page in &found {
        let text = std::fs::read_to_string(page).unwrap();
        match samples(page, &text) {
            Ok(page_samples) => {
                for sample in page_samples {
                    let file = root.join(format!("DOC-04_sample_{:04}.py", places.len()));
                    std::fs::write(&file, conformance_test(&sample)).unwrap();
                    places.push((file, sample.place));
                }
            }
            Err(places) => unstated.extend(places),
        }
    }
    assert!(
        unstated.is_empty(),
        "python blocks without a typy-expect comment:\n{}",
        unstated.join("\n")
    );
    let target = option_env!("CARGO_TARGET_DIR")
        .map_or_else(|| workspace().join("target"), Utf8PathBuf::from);
    let report = Suite::new(target.join("debug/typy")).run(&root).unwrap();
    let problems: Vec<String> = report
        .problems()
        .iter()
        .map(|(path, verdict)| {
            let place = places
                .iter()
                .find(|(file, _)| file == path)
                .map_or(path.as_str(), |(_, place)| place.as_str());
            format!("{place}: {verdict}")
        })
        .collect();
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn the_troubleshooting_page_has_a_section_for_every_active_code() {
    let page = std::fs::read_to_string(workspace().join("docs/user/troubleshooting.md")).unwrap();
    let missing: Vec<String> = CODES
        .iter()
        .filter(|info| info.status == CodeStatus::Active)
        .map(|info| info.code.to_string())
        .filter(|code| {
            !page
                .lines()
                .any(|line| line.starts_with(&format!("## {code}")))
        })
        .collect();
    assert!(
        missing.is_empty(),
        "codes without a section: {}",
        missing.join(", ")
    );
}
