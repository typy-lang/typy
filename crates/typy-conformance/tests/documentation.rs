//! Tests of the user documentation, every page under `docs/` outside the engineering
//! documents: every Python sample is judged as a conformance test, with the outcome the page
//! states (DOC-04), and the troubleshooting page has a section for every active diagnostic
//! code (DOC-03). The site's navigation in `mkdocs.yml` names every page under `docs/`, and
//! only those, so no page is left where no reader finds it (DOC-05).
//!
//! A sample is a `python` code block directly after a comment that states its outcome:
//! `<!-- typy-expect: accepted -->`, or `<!-- typy-expect: rejected E0401 at 2:7 -->` with
//! the line and column inside the sample.

use camino::{Utf8Path, Utf8PathBuf};
use typy_conformance::Suite;
use typy_diagnostics::{CODES, CodeStatus};

/// The directories of `docs/` that hold the engineering documents. Their Python blocks are
/// design sketches, not samples of what TyPy accepts, so every other page is user
/// documentation and a new section of it is checked without being listed here.
const ENGINEERING: [&str; 9] = [
    "adr",
    "architecture",
    "design",
    "development",
    "diagrams",
    "modeling",
    "requirements",
    "research",
    "testing",
];

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

/// Returns every page of the user documentation, sorted.
fn user_pages() -> Vec<Utf8PathBuf> {
    let docs = workspace().join("docs");
    let mut found = Vec::new();
    pages(&docs, &mut found);
    found.retain(|page| {
        let section = page.strip_prefix(&docs).unwrap().components().next();
        !section.is_some_and(|section| ENGINEERING.contains(&section.as_str()))
    });
    found
}

/// Returns every page under `docs/`, as a path relative to it, sorted.
fn site_pages() -> Vec<String> {
    let docs = workspace().join("docs");
    let mut found = Vec::new();
    pages(&docs, &mut found);
    found
        .iter()
        .map(|page| page.strip_prefix(&docs).unwrap().to_string())
        .collect()
}

/// Returns the pages the `nav` of `mkdocs.yml` names, in its order: the last word of each
/// item, `- Title: page.md` or `- page.md`, that ends in `.md`.
fn navigation() -> Vec<String> {
    let config = std::fs::read_to_string(workspace().join("mkdocs.yml")).unwrap();
    let mut in_nav = false;
    let mut found = Vec::new();
    for line in config.lines() {
        if !line.is_empty() && !line.starts_with([' ', '-']) {
            in_nav = line == "nav:";
            continue;
        }
        let page = line
            .trim()
            .strip_prefix("- ")
            .and_then(|item| item.rsplit(' ').next())
            .filter(|word| Utf8Path::new(word).extension() == Some("md"));
        if let (true, Some(page)) = (in_nav, page) {
            found.push(page.to_owned());
        }
    }
    found
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
    let found = user_pages();
    assert_ne!(found.len(), 0, "docs/ has user pages");
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
    let page =
        std::fs::read_to_string(workspace().join("docs/diagnostics/troubleshooting.md")).unwrap();
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

#[test]
fn every_page_of_the_documentation_is_in_the_site_navigation() {
    let navigation = navigation();
    let missing: Vec<String> = site_pages()
        .into_iter()
        .filter(|page| !navigation.contains(page))
        .collect();
    assert!(
        missing.is_empty(),
        "pages that the nav of mkdocs.yml does not name: {}",
        missing.join(", ")
    );
}

#[test]
fn every_page_the_site_navigation_names_exists() {
    let pages = site_pages();
    let navigation = navigation();
    assert_ne!(navigation.len(), 0, "mkdocs.yml has a nav");
    let missing: Vec<String> = navigation
        .into_iter()
        .filter(|page| !pages.contains(page))
        .collect();
    assert!(
        missing.is_empty(),
        "pages the nav of mkdocs.yml names that docs/ does not have: {}",
        missing.join(", ")
    );
}
