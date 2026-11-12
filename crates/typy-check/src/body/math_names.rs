//! Looks up the names of `math` for imports and attributes: a name the module gained after
//! the program's Python floor is not there at that floor, as CPython of that version has no
//! such name (LIB-05, STD-01).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::Alias;

use super::Program;
use super::import::Imported;
use crate::math::{MathName, math_name};

impl Program {
    /// Returns what `name` is in `math`, or why the program cannot use it.
    pub(crate) fn math_lookup(&self, name: &str, span: Span) -> Result<MathName, Box<Diagnostic>> {
        match math_name(name) {
            None => Err(Box::new(
                Diagnostic::new(
                    DiagnosticCode::error(425),
                    span,
                    format!("module `math` has no name `{name}`"),
                )
                .with_rule(RuleId::LIB_05),
            )),
            Some(MathName::Planned) => Err(Box::new(
                Diagnostic::new(
                    DiagnosticCode::error(201),
                    span,
                    format!("`math.{name}` is planned for TyPy 0.4 (LIB-12)"),
                )
                .with_rule(RuleId::LIB_12),
            )),
            Some(MathName::Function(function)) if function.since() > self.floor => Err(Box::new(
                Diagnostic::new(
                    DiagnosticCode::error(425),
                    span,
                    format!(
                        "module `math` has no name `{name}` before Python 3.{}",
                        function.since()
                    ),
                )
                .with_rule(RuleId::LIB_05)
                .with_help(format!(
                    "the program's Python floor is 3.{}; set `python-floor = \"3.{}\"` in \
                         typy.toml to use it",
                    self.floor,
                    function.since()
                )),
            )),
            Some(found) => Ok(found),
        }
    }

    /// Returns what `from math import name` binds.
    pub(crate) fn math_import(&self, alias: &Alias) -> Result<Imported, Box<Diagnostic>> {
        Ok(match self.math_lookup(&alias.name.name, alias.name.span)? {
            MathName::Constant(value) => Imported::MathConstant(value),
            MathName::Function(function) => Imported::MathFunction(function),
            // The lookup refuses a planned name.
            MathName::Planned => unreachable!("`math_lookup` refuses a planned name"),
        })
    }
}
