//! The feature table of release 0.1: what each construct the release does not compile is.

use typy_diagnostics::RuleId;
use typy_source::syntax::NodeKind;

/// Whether a construct comes in a later release or is not compiled at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Feature {
    /// Provided by a later release, under a requirement.
    Planned {
        what: &'static str,
        release: &'static str,
        rule: RuleId,
    },
    /// Listed in section 6.2 of the specification, planned for after 1.0.
    AfterOne { what: &'static str },
    /// Not compiled with CPython's meaning (section 6.1 of the specification).
    Excluded { what: &'static str },
}

pub(super) const fn planned(what: &'static str, release: &'static str, rule: RuleId) -> Feature {
    Feature::Planned {
        what,
        release,
        rule,
    }
}

/// Classifies every node kind the syntax tree does not convert in detail; the match has no wildcard, so a new kind does not compile until it is classified.
pub(super) fn feature(kind: NodeKind) -> Feature {
    match kind {
        NodeKind::JoinedStr => planned("f-strings", "0.2", RuleId::LEX_07),
        NodeKind::Delete => planned("`del` statements", "0.3", RuleId::SMT_05),
        NodeKind::Raise => planned("`raise` statements", "0.3", RuleId::SMT_06),
        NodeKind::Try => planned("`try` statements", "0.3", RuleId::CST_04),
        NodeKind::With => planned("`with` statements", "0.3", RuleId::CST_05),
        NodeKind::Assert => planned("`assert` statements", "0.3", RuleId::SMT_04),
        NodeKind::Global => planned("`global` statements", "0.3", RuleId::SMT_08),
        NodeKind::TypeAlias => planned("`type` statements", "0.4", RuleId::SMT_10),
        NodeKind::Match => planned("`match` statements", "0.5", RuleId::CST_06),
        NodeKind::NamedExpr => planned("assignment expressions", "0.5", RuleId::EXP_13),
        NodeKind::Nonlocal => planned("`nonlocal` statements", "1.0", RuleId::SMT_09),
        NodeKind::Lambda => planned("`lambda` expressions", "1.0", RuleId::EXP_14),
        NodeKind::GeneratorExp => planned("generator expressions", "1.0", RuleId::EXP_15),
        NodeKind::Yield | NodeKind::YieldFrom => planned("generators", "1.0", RuleId::SMT_11),
        NodeKind::Starred => planned("unpacking with `*`", "1.0", RuleId::EXP_17),
        NodeKind::AsyncFunctionDef | NodeKind::AsyncFor | NodeKind::AsyncWith | NodeKind::Await => {
            Feature::AfterOne {
                what: "`async` and `await`",
            }
        }
        NodeKind::TryStar => Feature::AfterOne {
            what: "`except*` clauses",
        },
        NodeKind::TemplateStr => Feature::Excluded {
            what: "template strings",
        },
        NodeKind::IpyEscapeCommand => Feature::Excluded {
            what: "IPython escape commands",
        },
    }
}
