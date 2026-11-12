//! Source locations and diagnostics, shared by every TyPy compiler stage.
#![forbid(unsafe_code)]

mod code;
mod diagnostic;
mod diagnostics;
mod registry;
mod render;
mod render_json;
mod rule;
mod span;

pub use code::{CodeError, DiagnosticCode, Severity};
pub use diagnostic::{Diagnostic, Fix, Label};
pub use diagnostics::Diagnostics;
pub use registry::{CODES, CodeInfo, CodeStatus, lookup};
pub use render::{SourceLookup, render_text};
pub use render_json::render_json;
pub use rule::RuleId;
pub use span::{FileId, Position, Span, TextOffset};
