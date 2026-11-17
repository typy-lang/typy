//! TyIR, TyPy's intermediate representation: its types, builder, verifier and text form (the Lowering context).
#![forbid(unsafe_code)]

mod block;
mod builder;
mod constant;
mod escape;
mod function;
mod instruction;
mod library;
mod module;
mod operation;
mod print;
mod runtime;
mod slot;
mod terminator;
mod tuple_types;
mod types;
mod value;
mod verify;

pub use block::{BasicBlock, BlockId};
pub use builder::FunctionBuilder;
pub use constant::{Constant, StringId};
pub use escape::place_instances;
pub use function::{FunctionId, TyirFunction};
pub use instruction::Instruction;
pub use library::{CType, TyirExport, TyirForeignFunction, TyirLibrary};
pub use module::{
    GlobalId, TyirAllocator, TyirClass, TyirGlobal, TyirModule, TyirModuleObject, TyirTypeAlias,
};
pub use operation::{
    BoolOperator, Comparison, FloatOperator, IntOperator, IntUnaryOperator, Operation, ScalarKind,
};
pub use print::{print, print_without_counting};
pub use runtime::{RuntimeFunction, RuntimeSignature};
pub use slot::{Slot, SlotId};
pub use terminator::{Callee, Terminator};
pub use tuple_types::TupleTypes;
pub use types::{OptionalBase, RuntimeType, TupleTypeId, TyirType, TypeId, TypeObjectRef};
pub use value::{ValueId, ValueInfo, ValueRepresentation};
pub use verify::{VerifyError, verify};
