//! Expressions after checking, each with its type.

use typy_diagnostics::Span;
use typy_source::syntax::{BinaryOperator, BoolOperator, CompareOperator, UnaryOperator};

use super::constant::TypedConstant;
use super::function::FunctionId;
use super::symbol::SymbolId;
use crate::builtin::Builtin;
use crate::types::Type;

/// An expression after checking, with its type.
#[derive(Clone, Debug, PartialEq)]
pub struct TypedExpr {
    /// What the expression does.
    pub kind: TypedExprKind,
    /// Its type.
    pub ty: Type,
    /// Where it is.
    pub span: Span,
}

/// One argument of a call, in source order, with the parameter it binds.
#[derive(Clone, Debug, PartialEq)]
pub struct Argument {
    /// The index of the parameter in the callee's signature.
    pub parameter: usize,
    /// The value.
    pub value: TypedExpr,
}

/// What an expression does.
#[derive(Clone, Debug, PartialEq)]
#[allow(
    missing_docs,
    reason = "each variant is the Python expression of the same name"
)]
pub enum TypedExprKind {
    Constant(TypedConstant),
    Local(SymbolId),
    /// A module variable, read as it is now; outside the module's own code it may be unbound
    /// and raise `NameError`, and a call may assign it (SMT-08, ADR-0015).
    ModuleVariable(SymbolId),
    /// `module.name`, a module variable of another module read through its module object;
    /// it raises `AttributeError` while unbound (MOD-10).
    ModuleAttribute {
        module: String,
        symbol: SymbolId,
    },
    /// `package.module`, the module object of a submodule, which raises `AttributeError`
    /// until the submodule is initialised.
    Submodule(String),
    /// `value`, a call or a class read through a lazy import, which first runs the import
    /// when its statement ran and its names were not used yet (MOD-06).
    Lazy {
        reify: FunctionId,
        value: Box<TypedExpr>,
    },
    /// A built-in iterable or an iterator held as an `Iterable[T]`: the same object (TYP-15).
    AsIterable(Box<TypedExpr>),
    /// `__name__`, which is `"__main__"` in the main module (STD-06).
    ModuleName,
    Binary {
        left: Box<TypedExpr>,
        operator: BinaryOperator,
        right: Box<TypedExpr>,
    },
    Unary {
        operator: UnaryOperator,
        operand: Box<TypedExpr>,
    },
    BoolOp {
        operator: BoolOperator,
        values: Vec<TypedExpr>,
    },
    Compare {
        left: Box<TypedExpr>,
        operators: Vec<CompareOperator>,
        comparators: Vec<TypedExpr>,
    },
    IfExp {
        test: Box<TypedExpr>,
        body: Box<TypedExpr>,
        orelse: Box<TypedExpr>,
    },
    /// A call of a program function. Arguments are in source order, which is evaluation order (EXP-01); missing ones take their defaults.
    Call {
        function: FunctionId,
        arguments: Vec<Argument>,
    },
    /// A new instance of a class, made by calling the class: the instance is made, then
    /// `__init__` runs on it with the arguments (CLS-01).
    New {
        class: super::class::ClassId,
        init: Option<FunctionId>,
        arguments: Vec<Argument>,
    },
    /// A new exception of a class, made by calling the class with positional arguments of
    /// any type, which the exception keeps (ADR-0013).
    NewException {
        class: crate::exception::ExceptionClass,
        arguments: Vec<TypedExpr>,
        /// The `__init__` of a class of the program, which takes the arguments in place of
        /// `arguments`.
        init: Option<Box<InitCall>>,
    },
    /// `super().__init__(arguments)` in the `__init__` of an exception class whose bases have
    /// no `__init__` of the program's: the exception's arguments become these (CLS-14).
    SetArguments {
        exception: Box<TypedExpr>,
        arguments: Vec<TypedExpr>,
    },
    /// `iter(source)` of a built-in iterable: a new iterator over it (CST-09).
    Iter(Box<TypedExpr>),
    /// `next(iterator)` or `next(iterator, default)`: the next item of an iterator, or of an
    /// instance whose class's `__next__` is `method`; at the end, the default, or
    /// `StopIteration` without one (CST-08, CST-09).
    Next {
        iterator: Box<TypedExpr>,
        method: Option<super::function::FunctionId>,
        default: Option<Box<TypedExpr>>,
    },
    /// An attribute of an instance, by its position in the class's attributes (CLS-01).
    Attribute {
        object: Box<TypedExpr>,
        index: usize,
    },
    /// `type(value)`: the value is evaluated, and its type object is the result (CLS-12).
    TypeOf(Box<TypedExpr>),
    /// The type object a class name or a built-in type name gives as a value.
    TypeObject(Box<crate::types::TypeObjectOf>),
    /// `__name__` of a type object.
    TypeName(Box<TypedExpr>),
    /// An f-string: its literal text and formatted fields, joined in order (LEX-07).
    FString(Vec<FStringPart>),
    /// `print(values, sep=sep, end=end)`; `sep` and `end` are present only when written.
    Print {
        values: Vec<TypedExpr>,
        sep: Option<Box<TypedExpr>>,
        end: Option<Box<TypedExpr>>,
    },
    /// `sorted`, `sum`, `any`, `all`, or `min` or `max` of one iterable, with the one optional
    /// value the builtin takes: `reverse`, `start` or `default` (LIB-02).
    IterableBuiltin {
        builtin: Builtin,
        iterable: Box<TypedExpr>,
        option: Option<Box<TypedExpr>>,
    },
    /// A `tuple[T, ...]` made from a fixed tuple whose items are all of type `T`, or from a new
    /// list that `tuple(source)` built (DAT-14).
    TupleOf(Box<TypedExpr>),
    /// `list(source)`: a new list of every value the source gives, in order (LIB-02).
    ListOf(Box<super::statement::LoopSource>),
    /// `set(source)`: a new set of every value the source gives, added in order, or a copy of
    /// the source when it is a set (DAT-10).
    SetOf(Box<super::statement::LoopSource>),
    /// A set display, one value per item, in order (DAT-10).
    Set(Vec<TypedExpr>),
    /// A list, set or dict comprehension: `element`, or `element: value` for a dict, made for
    /// each value its clauses give, in its own scope (EXP-12).
    Comprehension {
        element: Box<TypedExpr>,
        value: Option<Box<TypedExpr>>,
        generators: Vec<super::statement::Generator>,
    },
    /// A value of `T`, or `None`, held as a `T | None` (TYP-06, ADR-0011).
    Wrap(Box<TypedExpr>),
    /// The value of a `T | None` that a test showed is not `None` (TYP-13).
    Unwrap(Box<TypedExpr>),
    /// A dict display: each key and its value, in order (DAT-09).
    Dict {
        keys: Vec<TypedExpr>,
        values: Vec<TypedExpr>,
    },
    /// `dict(source)`: a copy of a dict, or a dict of the `(key, value)` pairs the source gives,
    /// set in order (DAT-09).
    DictOf(Box<super::statement::LoopSource>),
    /// `dict(name=value, ...)` or `dict(source, name=value, ...)`: the source's pairs, then
    /// each name as a `str` key with its value, in order (DAT-16).
    DictWith {
        source: Option<Box<super::statement::LoopSource>>,
        entries: Vec<(String, TypedExpr)>,
    },
    /// `dict.fromkeys(source, value)`: each item of the source as a key, mapped to the one
    /// value, or to `None` when none is given (DAT-16).
    DictFromKeys {
        source: Box<super::statement::LoopSource>,
        value: Option<Box<TypedExpr>>,
    },
    /// `dict[key]`, which raises `KeyError` for a missing key (DAT-09).
    DictItem {
        dict: Box<TypedExpr>,
        key: Box<TypedExpr>,
    },
    /// A call of any other builtin of 0.1, with positional arguments only.
    Builtin {
        builtin: Builtin,
        arguments: Vec<TypedExpr>,
    },
    /// `value[index]` on a `str`.
    Index {
        value: Box<TypedExpr>,
        index: Box<TypedExpr>,
    },
    /// A tuple display, one value per item (DAT-08).
    Tuple(Vec<TypedExpr>),
    /// A list display, one value per item, in order (DAT-07).
    List(Vec<TypedExpr>),
    /// `list[index]`, counted from the end when negative (DAT-11).
    ListItem {
        list: Box<TypedExpr>,
        index: Box<TypedExpr>,
    },
    /// `value[lower:upper:step]` on a `str` or a list, each bound optional (DAT-12).
    Slice {
        value: Box<TypedExpr>,
        lower: Option<Box<TypedExpr>>,
        upper: Option<Box<TypedExpr>>,
        step: Option<Box<TypedExpr>>,
    },
    /// A method of a built-in type called on `receiver` (LIB-03).
    Method {
        receiver: Box<TypedExpr>,
        method: crate::method::Method,
        arguments: Vec<Argument>,
    },
    /// A new tuple of the items of `tuple` at positions known when compiling (DAT-12).
    TupleSlice {
        tuple: Box<TypedExpr>,
        positions: Vec<usize>,
    },
    /// The item of a tuple at a position known when compiling, counted from the start.
    TupleItem {
        tuple: Box<TypedExpr>,
        index: usize,
    },
}

/// One piece of an f-string or of a format specification after checking.
#[derive(Clone, Debug, PartialEq)]
pub enum FStringPart {
    /// Literal text.
    Literal(String),
    /// A formatted value.
    Field(Box<FormattedValue>),
}

/// A conversion a replacement field applies before it formats.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Conversion {
    /// `!s`: `str(value)`.
    Str,
    /// `!r`: `repr(value)`.
    Repr,
    /// `!a`: `ascii(value)`.
    Ascii,
}

/// A replacement field after checking: `format(conversion(value), spec)`.
#[derive(Clone, Debug, PartialEq)]
pub struct FormattedValue {
    /// The value.
    pub value: TypedExpr,
    /// The conversion, if any.
    pub conversion: Option<Conversion>,
    /// The format specification, if the field has one, itself text and fields.
    pub spec: Option<Vec<FStringPart>>,
}

/// A call of the `__init__` of an exception class on the new exception (CLS-03). The arguments
/// written by position, which come first, are also the exception's arguments, as CPython's
/// `BaseException.__new__` keeps them.
#[derive(Clone, Debug, PartialEq)]
pub struct InitCall {
    /// The `__init__`, the class's own or the nearest base's.
    pub function: super::function::FunctionId,
    /// Its arguments after the instance, in source order.
    pub arguments: Vec<Argument>,
    /// How many of them come first, written by position.
    pub positional: usize,
}
