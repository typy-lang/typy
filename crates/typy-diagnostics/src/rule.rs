//! IDs of the specification's requirements that a diagnostic can name (DIA-03).

use std::fmt;

/// The ID of a requirement in `docs/requirements/srs.md`, such as `TYP-03`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuleId(&'static str);

impl RuleId {
    /// Valid Python accepted only when CPython parses it from the Python floor up.
    pub const STD_01: Self = Self("STD-01");
    /// Valid Python that TyPy cannot compile with CPython's meaning.
    pub const STD_04: Self = Self("STD-04");
    /// Source files are UTF-8.
    pub const LEX_01: Self = Self("LEX-01");
    /// Imaginary literals are rejected.
    pub const LEX_08: Self = Self("LEX-08");
    /// Source files over 16 MiB are rejected.
    pub const LEX_09: Self = Self("LEX-09");
    /// Parameters and returns carry annotations.
    pub const TYP_01: Self = Self("TYP-01");
    /// A variable keeps one type.
    pub const TYP_03: Self = Self("TYP-03");
    /// `Any`, `cast` and untyped values are rejected.
    pub const TYP_10: Self = Self("TYP-10");
    /// A variable is assigned on every path before it is read.
    pub const TYP_12: Self = Self("TYP-12");
    /// Arithmetic on `bool` is rejected.
    pub const DAT_03: Self = Self("DAT-03");
    /// Arithmetic gives Python's results.
    pub const EXP_02: Self = Self("EXP-02");
    /// `int ** int` needs a non-negative literal exponent.
    pub const EXP_04: Self = Self("EXP-04");
    /// Comparisons between supported types only.
    pub const EXP_06: Self = Self("EXP-06");
    /// `is` only with `None` or class instances.
    pub const EXP_08: Self = Self("EXP-08");
    /// `and` and `or` need operands of one type.
    pub const EXP_09: Self = Self("EXP-09");
    /// Truth tests on supported types only.
    pub const EXP_10: Self = Self("EXP-10");
    /// Conditional expressions need branches of one type.
    pub const EXP_11: Self = Self("EXP-11");
    /// Importing a module TyPy does not provide is rejected.
    pub const LIB_08: Self = Self("LIB-08");
    /// The compiler reports internal errors instead of crashing.
    pub const NFR_09: Self = Self("NFR-09");
    /// The host has a C toolchain that can link for the target.
    pub const A_01: Self = Self("A-01");

    /// Every rule a diagnostic can name, in ID order.
    pub const ALL: &'static [Self] = &[
        Self::A_01,
        Self::DAT_03,
        Self::EXP_02,
        Self::EXP_04,
        Self::EXP_06,
        Self::EXP_08,
        Self::EXP_09,
        Self::EXP_10,
        Self::EXP_11,
        Self::LEX_01,
        Self::LEX_08,
        Self::LEX_09,
        Self::LIB_08,
        Self::NFR_09,
        Self::STD_01,
        Self::STD_04,
        Self::TYP_01,
        Self::TYP_03,
        Self::TYP_10,
        Self::TYP_12,
    ];

    /// Returns the ID as text, such as `TYP-03`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl fmt::Display for RuleId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::RuleId;

    fn specification() -> String {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/requirements/srs.md"
        );
        std::fs::read_to_string(path).expect("the specification is part of the repository")
    }

    #[test]
    fn every_rule_is_an_id_defined_in_the_specification() {
        let specification = specification();
        for rule in RuleId::ALL {
            let row = format!("\n| {rule} |");
            assert!(
                specification.contains(&row),
                "{rule} is not defined in srs.md"
            );
        }
    }

    #[test]
    fn the_list_of_rules_is_sorted_and_has_no_duplicates() {
        let ids: Vec<&str> = RuleId::ALL.iter().map(|rule| rule.as_str()).collect();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(ids, sorted);
    }
}
