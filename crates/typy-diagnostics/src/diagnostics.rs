//! The diagnostics of one command: kept once each, and sorted for output.

use crate::code::Severity;
use crate::diagnostic::Diagnostic;
use crate::render::SourceLookup;

/// The diagnostics of one command, without duplicates.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Diagnostics {
    items: Vec<Diagnostic>,
}

impl Diagnostics {
    /// Returns an empty collection.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a diagnostic, unless one with the same code and span is already present.
    pub fn push(&mut self, diagnostic: Diagnostic) {
        let duplicate = self
            .items
            .iter()
            .any(|item| item.code() == diagnostic.code() && item.span() == diagnostic.span());
        if !duplicate {
            self.items.push(diagnostic);
        }
    }

    /// Adds every diagnostic of another collection.
    pub fn extend(&mut self, other: Self) {
        for diagnostic in other.items {
            self.push(diagnostic);
        }
    }

    /// Returns true when any diagnostic is an error.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.items
            .iter()
            .any(|item| item.severity() == Severity::Error)
    }

    /// Returns the diagnostics in the order they were added.
    pub fn iter(&self) -> impl Iterator<Item = &Diagnostic> {
        self.items.iter()
    }

    /// Returns the number of diagnostics.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Drops every diagnostic added after the first `length`, as a check that is repeated
    /// with more knowledge drops what its first try found.
    pub fn truncate(&mut self, length: usize) {
        self.items.truncate(length);
    }

    /// Returns true when there are none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Returns the diagnostics sorted by file path, line, column and code; those without a
    /// location come first.
    #[must_use]
    pub fn into_sorted(mut self, lookup: &dyn SourceLookup) -> Vec<Diagnostic> {
        self.items.sort_by_cached_key(|item| {
            let place = item.span().map(|span| {
                let position = lookup.position(span.file(), span.start());
                (lookup.path(span.file()).to_owned(), position)
            });
            (place, item.code())
        });
        self.items
    }
}

impl From<Diagnostic> for Diagnostics {
    fn from(diagnostic: Diagnostic) -> Self {
        Self {
            items: vec![diagnostic],
        }
    }
}

#[cfg(test)]
mod tests;
