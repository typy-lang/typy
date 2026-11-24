//! The level of every warning code, set by `-W`, `-A` and `-D` in the order given (CLI-27).

use std::fmt;

use crate::code::{DiagnosticCode, Severity};
use crate::diagnostics::Diagnostics;
use crate::warning_level::WarningLevel;
use crate::warning_selector::WarningSelector;

/// The level of every warning code: one level for every warning, and the codes set apart
/// from it, sorted by code. Two sets of options that give each code the same level give
/// equal values, so the value can key a cache.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct WarningLevels {
    every: WarningLevel,
    codes: Vec<(DiagnosticCode, WarningLevel)>,
}

impl WarningLevels {
    /// Gives `level` to the warnings `selector` names. A later call wins over an earlier one
    /// for each code it names, so `-D warnings -A W0101` allows W0101 and denies the rest,
    /// while `-A W0101 -D warnings` denies every warning.
    pub fn set(&mut self, selector: WarningSelector, level: WarningLevel) {
        match selector {
            WarningSelector::All => {
                self.every = level;
                self.codes.clear();
            }
            WarningSelector::Code(code) => {
                self.codes.retain(|(named, _)| *named != code);
                if level != self.every {
                    let index = self.codes.partition_point(|(named, _)| *named < code);
                    self.codes.insert(index, (code, level));
                }
            }
        }
    }

    /// Returns the level of the warning `code`.
    #[must_use]
    pub fn level(&self, code: DiagnosticCode) -> WarningLevel {
        self.codes
            .iter()
            .find(|(named, _)| *named == code)
            .map_or(self.every, |(_, level)| *level)
    }

    /// Returns the diagnostics with each warning at its level: an allowed warning removed, a
    /// denied one reported as an error. Errors are kept as they are.
    #[must_use]
    pub fn apply(&self, diagnostics: &Diagnostics) -> Diagnostics {
        let mut kept = Diagnostics::new();
        for diagnostic in diagnostics.iter() {
            if diagnostic.code().severity() == Severity::Error {
                kept.push(diagnostic.clone());
                continue;
            }
            match self.level(diagnostic.code()) {
                WarningLevel::Warn => kept.push(diagnostic.clone()),
                WarningLevel::Allow => {}
                WarningLevel::Deny => kept.push(diagnostic.clone().denied()),
            }
        }
        kept
    }
}

/// Writes the level of every warning, then each code set apart from it, such as
/// `deny W0101=allow`.
impl fmt::Display for WarningLevels {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.every)?;
        for (code, level) in &self.codes {
            write!(formatter, " {code}={level}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
