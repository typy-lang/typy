//! What the command line makes of a warning: a warning, nothing, or an error (CLI-27).

use std::fmt;

/// What the command line makes of a warning.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum WarningLevel {
    /// `-W`: reported as a warning, which never fails a build (DIA-07).
    #[default]
    Warn,
    /// `-A`: not reported.
    Allow,
    /// `-D`: reported as an error, which fails the build before any code is generated.
    Deny,
}

impl fmt::Display for WarningLevel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Warn => "warn",
            Self::Allow => "allow",
            Self::Deny => "deny",
        })
    }
}

#[cfg(test)]
mod tests;
