//! Node kinds that a release does not convert in detail (detailed design section 5.2).

use std::fmt;

/// A node kind that the current release does not convert in detail (detailed design section 5.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[allow(
    missing_docs,
    reason = "each variant is the CPython ast node of the same name"
)]
pub enum NodeKind {
    AsyncFor,
    AsyncFunctionDef,
    AsyncWith,
    Await,
    GeneratorExp,
    Global,
    IpyEscapeCommand,
    Lambda,
    Match,
    NamedExpr,
    Nonlocal,
    Starred,
    TemplateStr,
    TryStar,
    TypeAlias,
    Yield,
    YieldFrom,
}

impl fmt::Display for NodeKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, formatter)
    }
}
