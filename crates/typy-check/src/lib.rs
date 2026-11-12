//! Resolves names, checks types and the subset rules of a TyPy program (the Checking context).
#![forbid(unsafe_code)]

mod body;
mod builtin;
mod check;
mod codec;
mod exception;
mod math;
mod method;
mod open;
mod percent_format;
mod subset;
mod sys;
mod typed;
mod types;
mod unprovided;

pub use builtin::Builtin;
pub use check::TypeChecker;
pub use exception::{BuiltinException, ExceptionClass, ProgramException};
pub use math::{MathFunction, MathShape};
pub use method::Method;
pub use open::OpenAccess;
pub use sys::SysValue;
pub use typed::{
    Argument, AssignTarget, Attribute, ClassId, Conversion, FStringPart, FormattedValue,
    FunctionId, FunctionSignature, Generator, ImportBinding, InitCall, LoopSource,
    PercentConversion, PercentField, PercentNumber, PercentPart, PercentValue, PrintCall,
    RaiseCause, SignatureParameter, Symbol, SymbolId, SymbolKind, TypedClass, TypedConstant,
    TypedExpr, TypedExprKind, TypedFunction, TypedHandler, TypedModule, TypedProgram, TypedStmt,
    WithProtocol, dump,
};
pub use types::{Type, TypeObjectOf, View};
