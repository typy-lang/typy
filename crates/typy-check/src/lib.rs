//! Resolves names, checks types and the subset rules of a TyPy program (the Checking context).
#![forbid(unsafe_code)]

mod abstract_type;
mod body;
mod builtin;
mod callable_type;
mod check;
mod class_bases;
mod class_type;
mod codec;
mod ctypes;
mod exception;
mod exception_arguments;
mod exports;
mod generic;
mod iterator_fits;
mod library_version;
mod literal_type;
mod math;
mod method;
mod open;
mod percent_format;
mod protocol_fits;
pub use protocol_fits::ProgramScope;
mod subset;
mod sys;
mod typed;
mod types;
mod union_type;
mod unprovided;

pub use body::{COMPARISON_BRIDGES, EQUAL_BRIDGE, TRUTH_BRIDGE, member_class, member_test};
pub use builtin::Builtin;
pub use check::TypeChecker;
pub use check::builtin_iterators::built_in_class;
pub use check::decorators::python_name;
pub use class_type::{InstanceClass, Operators};
pub use ctypes::{CData, CType, ForeignFunction, ForeignLibrary, StructureClass, StructureField};
pub use exception::{BuiltinException, ExceptionClass, ProgramException};
pub use exports::{Export, RUNTIME_C_FUNCTIONS};
pub use generic::TypeParameter;
pub use literal_type::LiteralValue;
pub use math::{MathFunction, MathShape};
pub use method::Method;
pub use open::OpenAccess;
pub use sys::SysValue;
pub use typed::{
    Argument, AssignTarget, Attribute, BuiltinValue, CDataExpr, CDataPlace, CapturedVariable,
    CastCheck, ClassId, Conversion, DictPart, EachMember, ExceptionPart, ExceptionValues,
    FStringPart, FileAttribute, FormattedValue, FunctionId, FunctionSignature, Generator,
    GeneratorTypes, ImportBinding, InitCall, IsInstanceTest, KeyedOperation, LiteralOutcome,
    LiteralTest, LoopSource, MemberTest, OpenOptions, OperatorCall, OperatorKind, PartOperation,
    Passed, PercentConversion, PercentField, PercentNumber, PercentPart, PercentValue, PrintCall,
    RaiseCause, RuntimeKind, SignatureParameter, SpreadKind, SpreadPart, Symbol, SymbolId,
    SymbolKind, TypedCase, TypedClass, TypedConstant, TypedExpr, TypedExprKind, TypedFunction,
    TypedHandler, TypedModule, TypedPattern, TypedProgram, TypedStmt, TypedTypeAlias,
    TypedTypingValue, UnpackedException, WithProtocol, dump,
};
pub use types::{BinaryKind, Type, TypeObjectOf, View};
