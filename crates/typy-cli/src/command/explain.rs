//! `typy explain CODE` (CLI-10, UC-04): prints the troubleshooting section of a code, with a
//! program that causes it and the corrected program.

use std::fmt::Write as _;
use std::io::Write as _;

use typy_diagnostics::{CodeStatus, DiagnosticCode, lookup};

use crate::exit_status::ExitStatus;

/// The troubleshooting page, the one text both the documentation and this command show, so
/// they never disagree. The documentation test runs every sample in it.
const PAGE: &str = include_str!("../../../../docs/diagnostics/troubleshooting.md");

/// Prints the explanation of `text`, a code such as `E0402`. A text that is no code, or a
/// code no diagnostic has, is wrong usage.
pub(super) fn explain(text: &str) -> ExitStatus {
    let Ok(code) = DiagnosticCode::parse(text) else {
        eprintln!("error: `{text}` is not a diagnostic code, such as E0402");
        return ExitStatus::Usage;
    };
    let Some(info) = lookup(code) else {
        eprintln!("error: no diagnostic has the code {code}");
        return ExitStatus::Usage;
    };
    let mut text = match info.status {
        CodeStatus::Retired { replaced_by } => {
            let replacement = replaced_by.map_or_else(String::new, |by| format!("; see {by}"));
            format!("{code} is retired and no longer reported{replacement}\n")
        }
        CodeStatus::Active => explanation(code, info.summary),
    };
    if let Some(rule) = info.rule {
        let _ = writeln!(text, "\nRule: {rule}");
    }
    let mut output = std::io::stdout().lock();
    let _ = output.write_all(text.as_bytes());
    ExitStatus::Success
}

/// Returns the section of the page for `code` as plain text: its heading, its prose, and its
/// samples indented. A code the page has no section for gets its summary.
pub(crate) fn explanation(code: DiagnosticCode, summary: &str) -> String {
    let heading = format!("## {code}:");
    let Some(start) = PAGE.find(&heading) else {
        return format!("{code}: {summary}\n");
    };
    let section = &PAGE[start + 3..];
    let section = section.find("\n## ").map_or(section, |end| &section[..end]);
    let mut text = String::new();
    let mut in_code = false;
    for line in section.lines() {
        if line.starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if line.starts_with("<!-- typy-expect") {
            continue;
        }
        if in_code {
            text.push_str("    ");
            text.push_str(line);
        } else {
            text.push_str(&without_links(line));
        }
        text.push('\n');
    }
    while text.ends_with("\n\n") {
        text.pop();
    }
    text
}

/// Returns a line of Markdown with each link `[text](target)` as its text.
fn without_links(line: &str) -> String {
    let mut text = String::new();
    let mut rest = line;
    while let Some(open) = rest.find('[') {
        let Some(close) = rest[open..].find("](").map(|at| open + at) else {
            break;
        };
        let Some(end) = rest[close..].find(')').map(|at| close + at) else {
            break;
        };
        text.push_str(&rest[..open]);
        text.push_str(&rest[open + 1..close]);
        rest = &rest[end + 1..];
    }
    text.push_str(rest);
    text
}

#[cfg(test)]
mod tests;
