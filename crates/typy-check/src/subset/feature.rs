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
        // The parser refuses `:=` to anything but a name, so only that error reaches here.
        NodeKind::NamedExpr => Feature::Excluded {
            what: "assignment expressions to anything but a name",
        },
        NodeKind::GeneratorExp => planned("generator expressions", "0.9", RuleId::EXP_15),
        NodeKind::Yield | NodeKind::YieldFrom => planned("generators", "0.9", RuleId::SMT_11),
        NodeKind::Starred => planned("iterable unpackings with `*`", "0.9", RuleId::EXP_17),
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
