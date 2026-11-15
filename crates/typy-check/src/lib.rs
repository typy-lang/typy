//! Resolves names, checks types and the subset rules of a TyPy program (the Checking context).
#![forbid(unsafe_code)]

mod body;
mod builtin;
mod check;
mod class_type;
mod codec;
mod exception;
mod generic;
mod math;
mod method;
mod open;
mod percent_format;
mod subset;
mod sys;
mod typed;
mod types;
mod union_type;
mod unprovided;

pub use body::member_class;
pub use builtin::Builtin;
pub use check::TypeChecker;
pub use class_type::{InstanceClass, Operators};
pub use exception::{BuiltinException, ExceptionClass, ProgramException};
pub use generic::TypeParameter;
pub use math::{MathFunction, MathShape};
pub use method::Method;
pub use open::OpenAccess;
pub use sys::SysValue;
pub use typed::{
    Argument, AssignTarget, Attribute, ClassId, Conversion, EachMember, ExceptionPart, FStringPart,
    FileAttribute, FormattedValue, FunctionId, FunctionSignature, Generator, ImportBinding,
    InitCall, IsInstanceTest, LoopSource, OpenOptions, OperatorCall, OsErrorField, PartOperation,
    PercentConversion, PercentField, PercentNumber, PercentPart, PercentValue, PrintCall,
    RaiseCause, SignatureParameter, Symbol, SymbolId, SymbolKind, TypedCase, TypedClass,
    TypedConstant, TypedExpr, TypedExprKind, TypedFunction, TypedHandler, TypedModule,
    TypedPattern, TypedProgram, TypedStmt, WithProtocol, dump,
};
pub use types::{BinaryKind, Type, TypeObjectOf, View};
