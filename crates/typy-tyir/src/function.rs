//! A function in TyIR.

use crate::block::{BasicBlock, BlockId};
use crate::slot::{Slot, SlotId};
use crate::types::TyirType;
use crate::value::{ValueId, ValueInfo};

/// Index of a function in its module.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FunctionId(pub u32);

/// A function: its blocks, values and slots.
#[derive(Clone, Debug, PartialEq)]
pub struct TyirFunction {
    /// The function's ID.
    pub id: FunctionId,
    /// The symbol it is emitted under (detailed design section 10.3).
    pub symbol: String,
    /// The name tracebacks show, such as `fib` or `<module>`.
    pub name: String,
    /// The name that tells it from the other functions of its module, Python's
    /// `__qualname__`, which a function object's `repr` writes (FUN-05).
    pub qualified_name: String,
    /// The parameters: the entry block's parameters.
    pub parameters: Vec<ValueId>,
    /// The return type.
    pub return_type: TyirType,
    /// True when the function may raise, so its callers use `invoke`.
    pub may_raise: bool,
    /// The blocks; the first is the entry.
    pub blocks: Vec<BasicBlock>,
    /// Every value, indexed by `ValueId`.
    pub values: Vec<ValueInfo>,
    /// Every slot, indexed by `SlotId`.
    pub slots: Vec<Slot>,
    /// The frame a generator's resume function keeps in its object, or `None` for any other
    /// function (ADR-0063).
    pub generator: Option<GeneratorFrame>,
    /// What the function stands for as a value: itself, or a built-in function or a method of
    /// a built-in type, whose object CPython writes otherwise (FUN-08, ADR-0077).
    pub stands_for: StandsFor,
}

/// What a function's object stands for, which its type and its text follow (FUN-08).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StandsFor {
    /// The function itself, a `function` object.
    #[default]
    Itself,
    /// A built-in function, such as `len`, a `builtin_function_or_method` object.
    BuiltinFunction,
    /// A method of a built-in type read through the type, such as `str.upper`, a
    /// `method_descriptor` object.
    BuiltinMethod,
    /// A built-in type called as a function, such as `int` in `key=int`, written as the type
    /// is (FUN-14).
    BuiltinType,
    /// An adapter that fits a function of the program to a `Callable`, which stands for that
    /// function: its value is that function's to `is`, `==` and `hash` (FUN-13, ADR-0080).
    Adapter(FunctionId),
}

/// What a generator's object holds between steps: the arguments its maker stores, then each
/// slot of its resume function, laid out as one tuple (ADR-0063).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratorFrame {
    /// The types of the arguments, the closure of a nested generator function first.
    pub arguments: Vec<TyirType>,
    /// Whether the first argument is the closure, which every step reads and the object
    /// keeps.
    pub closure: bool,
    /// The type of what a `yield` gives.
    pub yields: TyirType,
    /// The layout of the frame: the arguments, then the slots.
    pub frame: crate::types::TupleTypeId,
    /// The one-item tuple of what the body returns, or `None` when it returns `None`.
    pub result: Option<crate::types::TupleTypeId>,
}

impl TyirFunction {
    /// Returns a block by its ID.
    #[must_use]
    pub fn block(&self, id: BlockId) -> &BasicBlock {
        &self.blocks[id.0 as usize]
    }

    /// Returns what is known about a value.
    #[must_use]
    pub fn value(&self, id: ValueId) -> ValueInfo {
        self.values[id.0 as usize]
    }

    /// Returns a slot by its ID.
    #[must_use]
    pub fn slot(&self, id: SlotId) -> &Slot {
        &self.slots[id.0 as usize]
    }
}
