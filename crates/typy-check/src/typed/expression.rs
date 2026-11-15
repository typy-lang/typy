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
    /// The type alias object at this index of `TypedProgram::type_aliases` (SMT-12).
    TypeAliasObject(u32),
    /// A read of `sys.version_info` held as a value: `value` is evaluated, then `result`,
    /// a constant: the tuple of its items, which a tuple's operations read, or a comparison
    /// of them already made (LIB-16).
    VersionRead {
        /// The `sys.version_info` value.
        value: Box<TypedExpr>,
        /// What the read gives.
        result: Box<TypedExpr>,
    },
    /// `alias.__name__`, or `alias.__module__` when `module` is set, of a type alias (SMT-12).
    TypeAliasPart {
        /// The type alias.
        alias: Box<TypedExpr>,
        /// Whether the part is `__module__`.
        module: bool,
    },
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
        /// True for a method called on an instance, which runs the method of the instance's
        /// class when that class defines it again; false for a function and for `super()`
        /// (ADR-0021).
        dispatch: bool,
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
    /// A value of `sys`: `argv`, `stdout` or `stderr` (LIB-04).
    SysValue(crate::sys::SysValue),
    /// `sys.exit(code)`, which raises `SystemExit` with `code` (LIB-04).
    SysExit(Option<Box<TypedExpr>>),
    /// `sys.getrecursionlimit()`, without a value, or `sys.setrecursionlimit(limit)`
    /// (LIB-13).
    RecursionLimit(Option<Box<TypedExpr>>),
    /// `format % values` with a format the runtime reads, which is known only when the
    /// program runs or takes the items of a `tuple[T, ...]` (LIB-11).
    PercentDynamic {
        format: Box<TypedExpr>,
        values: Box<TypedExpr>,
    },
    /// An operation on `args` of an exception or `code` of a `SystemExit`, read from the
    /// exception (EXC-11, EXC-13, ADR-0030).
    ExceptionPart {
        exception: Box<TypedExpr>,
        part: ExceptionPart,
        operation: PartOperation,
    },
    /// `errno`, `strerror`, `filename` or `filename2` of an `OSError`, `None` when it was not
    /// given (EXC-14).
    OsErrorField {
        exception: Box<TypedExpr>,
        field: OsErrorField,
    },
    /// `name`, `mode` or `closed` of a text file (LIB-14).
    FileAttribute {
        file: Box<TypedExpr>,
        attribute: FileAttribute,
    },
    /// `input(prompt)`: a line of standard input (LIB-04).
    Input(Option<Box<TypedExpr>>),
    /// `open(path, mode)` in a text mode: `mode` as written, which the file's `repr` shows,
    /// and how it opens the file (LIB-04, ADR-0019).
    Open {
        path: Box<TypedExpr>,
        mode: String,
        access: crate::open::OpenAccess,
        options: Box<OpenOptions>,
    },
    /// A call of a function of `math`, with its arguments in the order the function takes
    /// them; a keyword not given is `None` (LIB-05).
    MathCall {
        function: crate::math::MathFunction,
        arguments: Vec<TypedExpr>,
    },
    /// `format % values` with a literal format: the right operand, evaluated once, and the
    /// pieces that make the text (LIB-10).
    PercentFormat {
        values: Box<TypedExpr>,
        parts: Vec<super::percent::PercentPart>,
    },
    /// A call of `print` (LIB-01, LIB-04).
    Print(Box<PrintCall>),
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
    /// `target := value`: the value, stored in the variable and given (EXP-13).
    Named {
        target: SymbolId,
        value: Box<TypedExpr>,
    },
    /// An instance, or an optional one, held as an instance of a class it derives from: the
    /// same object, whose type is the expression's (CLS-04).
    Upcast(Box<TypedExpr>),
    /// The value of a `T | None` that a test showed is not `None` (TYP-13).
    Unwrap(Box<TypedExpr>),
    /// An instance, or an exception, held as an instance of a class derived from its static
    /// class, which an `isinstance` test showed it is: the same object (TYP-07).
    Downcast(Box<TypedExpr>),
    /// `isinstance(value, classes)` (CLS-09).
    IsInstance(Box<IsInstanceTest>),
    /// An operation on a variable that holds a union, checked for each member (TYP-14).
    EachMember(Box<EachMember>),
    /// The `int` a class's `__len__` returned, checked to be a length, which raises
    /// `ValueError` when it is negative (CLS-07).
    CheckedLength(Box<TypedExpr>),
    /// A comparison of instances that calls a comparison method (CLS-07, ADR-0026).
    OperatorCall(Box<OperatorCall>),
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

/// `print(values, sep=sep, end=end, file=file, flush=flush)`; each keyword is present only
/// when written, and `file` only when it is not `None` (LIB-04).
#[derive(Clone, Debug, PartialEq)]
pub struct PrintCall {
    /// The values printed, in order.
    pub values: Vec<TypedExpr>,
    /// What is written between two values.
    pub sep: Option<TypedExpr>,
    /// What is written after the last value.
    pub end: Option<TypedExpr>,
    /// The text file written to.
    pub file: Option<TypedExpr>,
    /// Whether standard output is written out after.
    pub flush: Option<TypedExpr>,
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

/// `left op right` that calls a comparison method: `forward` of the left operand's class, or
/// `reflected` of the right operand's. With both, the reflected method runs first when the
/// right operand's class derives from the left's and defines it otherwise, as in CPython;
/// with one, that one runs (CLS-07, ADR-0026).
#[derive(Clone, Debug, PartialEq)]
pub struct OperatorCall {
    /// The left operand, evaluated first.
    pub left: TypedExpr,
    /// The right operand.
    pub right: TypedExpr,
    /// The method of the left operand's class, which takes the right operand.
    pub forward: Option<FunctionId>,
    /// The reflected method of the right operand's class, which takes the left operand.
    pub reflected: Option<FunctionId>,
    /// The reflected method of the left operand's class, whose entry the runtime compares
    /// with the right operand's to decide which runs.
    pub left_reflected: Option<FunctionId>,
    /// Whether the result is negated: `!=` without `__ne__` is `not` of `__eq__`.
    pub negated: bool,
}

/// `isinstance(value, classes)`, with what the checker could decide when the program is
/// compiled (CLS-09).
#[derive(Clone, Debug, PartialEq)]
pub struct IsInstanceTest {
    /// The value, evaluated once.
    pub value: TypedExpr,
    /// Whether `None`, for an optional value, is an instance of one of the classes: `NoneType`
    /// or `object` is among them.
    pub none_matches: bool,
    /// Whether every value of the static type that is not `None` is an instance of one of the
    /// classes.
    pub always: bool,
    /// The classes the value's class decides, tested when the program runs, when `always` is
    /// false.
    pub checks: Vec<crate::types::TypeObjectOf>,
    /// The position, in the function's table of narrowed types, of the type a true test gives
    /// the variable the value reads, when it narrows one (TYP-07).
    pub narrowed: Option<u32>,
    /// The position of the type a false test gives that variable: the members of a union no
    /// tested class takes in every case (TYP-07, TYP-14).
    pub otherwise: Option<u32>,
    /// For a union, each member in order: whether every value of it is an instance of one of
    /// the classes, and the classes its value's class decides (TYP-14).
    pub members: Vec<(bool, Vec<crate::types::TypeObjectOf>)>,
}

/// An operation on a variable that holds a union of objects, checked once for each member
/// with the variable narrowed to it (TYP-14, ADR-0027).
#[derive(Clone, Debug, PartialEq)]
pub struct EachMember {
    /// The read of the variable, whose object decides which member it is.
    pub value: TypedExpr,
    /// Each member's class with the operation checked for it; the last runs when no class
    /// before it is the object's.
    pub arms: Vec<(crate::types::TypeObjectOf, TypedExpr)>,
}

impl TypedExpr {
    /// Returns the expression's operands in source order, the expressions it evaluates directly.
    #[must_use]
    #[allow(clippy::too_many_lines, reason = "one arm per expression kind")]
    pub fn operands(&self) -> Vec<&TypedExpr> {
        match &self.kind {
            TypedExprKind::Constant(_)
            | TypedExprKind::Local(_)
            | TypedExprKind::ModuleVariable(_)
            | TypedExprKind::ModuleAttribute { .. }
            | TypedExprKind::Submodule(_)
            | TypedExprKind::ModuleName
            | TypedExprKind::TypeObject(_)
            | TypedExprKind::TypeAliasObject(_)
            | TypedExprKind::SysValue(_) => Vec::new(),
            TypedExprKind::Binary { left, right, .. }
            | TypedExprKind::Index {
                value: left,
                index: right,
            } => {
                vec![left, right]
            }
            TypedExprKind::Unary { operand, .. }
            | TypedExprKind::TupleOf(operand)
            | TypedExprKind::TupleItem { tuple: operand, .. }
            | TypedExprKind::TupleSlice { tuple: operand, .. } => {
                vec![operand]
            }
            TypedExprKind::Tuple(items)
            | TypedExprKind::List(items)
            | TypedExprKind::Set(items) => items.iter().collect(),
            TypedExprKind::ListItem { list, index } => vec![list, index],
            TypedExprKind::ListOf(source)
            | TypedExprKind::SetOf(source)
            | TypedExprKind::DictOf(source) => source.expressions(),
            TypedExprKind::DictWith { source, entries } => source
                .iter()
                .flat_map(|source| source.expressions())
                .chain(entries.iter().map(|(_, value)| value))
                .collect(),
            TypedExprKind::DictFromKeys { source, value } => source
                .expressions()
                .into_iter()
                .chain(value.as_deref())
                .collect(),
            TypedExprKind::Dict { keys, values } => keys
                .iter()
                .zip(values)
                .flat_map(|(key, value)| [key, value])
                .collect(),
            TypedExprKind::DictItem { dict, key } => vec![dict, key],
            TypedExprKind::IsInstance(test) => vec![&test.value],
            TypedExprKind::EachMember(each) => std::iter::once(&each.value)
                .chain(each.arms.iter().map(|(_, body)| body))
                .collect(),
            TypedExprKind::OperatorCall(call) => vec![&call.left, &call.right],
            TypedExprKind::Wrap(inner)
            | TypedExprKind::CheckedLength(inner)
            | TypedExprKind::Upcast(inner)
            | TypedExprKind::Downcast(inner)
            | TypedExprKind::Unwrap(inner)
            | TypedExprKind::AsIterable(inner)
            | TypedExprKind::Lazy { value: inner, .. }
            | TypedExprKind::Named { value: inner, .. } => vec![inner],
            TypedExprKind::PercentFormat { values, .. } => vec![values],
            TypedExprKind::MathCall { arguments, .. } => arguments.iter().collect(),
            TypedExprKind::Open { path, options, .. } => [
                Some(path),
                options.mode.as_ref(),
                options.buffering.as_ref(),
                options.newline.as_ref(),
                options.closefd.as_ref(),
            ]
            .into_iter()
            .flatten()
            .map(Box::as_ref)
            .collect(),
            TypedExprKind::SysExit(value)
            | TypedExprKind::Input(value)
            | TypedExprKind::RecursionLimit(value) => value.iter().map(Box::as_ref).collect(),
            TypedExprKind::OsErrorField { exception, .. } => vec![exception],
            TypedExprKind::FileAttribute { file, .. } => vec![file],
            TypedExprKind::PercentDynamic { format, values } => vec![format, values],
            TypedExprKind::ExceptionPart {
                exception,
                operation,
                ..
            } => match operation {
                PartOperation::Equal(other) => vec![exception, other],
                _ => vec![exception],
            },
            TypedExprKind::Comprehension {
                element,
                value,
                generators,
            } => std::iter::once(element.as_ref())
                .chain(value.as_deref())
                .chain(generators.iter().flat_map(|generator| {
                    generator
                        .source
                        .expressions()
                        .into_iter()
                        .chain(&generator.conditions)
                }))
                .collect(),
            TypedExprKind::IterableBuiltin {
                iterable, option, ..
            } => std::iter::once(iterable.as_ref())
                .chain(option.as_deref())
                .collect(),
            TypedExprKind::Slice {
                value,
                lower,
                upper,
                step,
            } => std::iter::once(value.as_ref())
                .chain(lower.as_deref())
                .chain(upper.as_deref())
                .chain(step.as_deref())
                .collect(),
            TypedExprKind::Method {
                receiver,
                arguments,
                ..
            } => std::iter::once(receiver.as_ref())
                .chain(arguments.iter().map(|argument| &argument.value))
                .collect(),
            TypedExprKind::BoolOp { values, .. }
            | TypedExprKind::Builtin {
                arguments: values, ..
            } => values.iter().collect(),
            TypedExprKind::Compare {
                left, comparators, ..
            } => std::iter::once(left.as_ref()).chain(comparators).collect(),
            TypedExprKind::IfExp { test, body, orelse } => vec![test, body, orelse],
            TypedExprKind::Call { arguments, .. } | TypedExprKind::New { arguments, .. } => {
                arguments.iter().map(|argument| &argument.value).collect()
            }
            TypedExprKind::Attribute { object, .. }
            | TypedExprKind::TypeOf(object)
            | TypedExprKind::TypeName(object)
            | TypedExprKind::TypeAliasPart { alias: object, .. } => vec![object],
            TypedExprKind::VersionRead { value, result } => vec![value, result],
            TypedExprKind::NewException {
                arguments, init, ..
            } => arguments
                .iter()
                .chain(
                    init.iter()
                        .flat_map(|init| init.arguments.iter().map(|argument| &argument.value)),
                )
                .collect(),
            TypedExprKind::Iter(source) => vec![source],
            TypedExprKind::Next {
                iterator, default, ..
            } => std::iter::once(iterator.as_ref())
                .chain(default.as_deref())
                .collect(),
            TypedExprKind::SetArguments {
                exception,
                arguments,
            } => std::iter::once(exception.as_ref())
                .chain(arguments)
                .collect(),
            TypedExprKind::FString(parts) => crate::typed::dump::fstring_values(parts),
            TypedExprKind::Print(call) => call
                .values
                .iter()
                .chain(&call.sep)
                .chain(&call.end)
                .chain(&call.file)
                .chain(&call.flush)
                .collect(),
        }
    }
}

/// A part of an exception that may hold any value (EXC-11, EXC-13, ADR-0030).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExceptionPart {
    /// `exception.args`, the tuple of its arguments.
    Arguments,
    /// `system_exit.code`: `None`, its one argument, or the tuple of its arguments.
    Code,
}

/// What a program does with a part of an exception.
#[derive(Clone, Debug, PartialEq)]
pub enum PartOperation {
    /// `str` of it, or `repr` when `repr`.
    Text {
        /// Whether it is `repr`.
        repr: bool,
    },
    /// `len` of `args`.
    Length,
    /// Its truth.
    Truth,
    /// Whether it equals the value.
    Equal(Box<TypedExpr>),
}

/// An attribute of an `OSError` that its arguments give (EXC-14).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OsErrorField {
    /// `errno`, the first argument.
    Errno,
    /// `strerror`, the second.
    Strerror,
    /// `filename`, the third.
    Filename,
    /// `filename2`, the fifth.
    Filename2,
}

/// An attribute of a text file (LIB-14).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileAttribute {
    /// `name`: the path it was opened with, or `<stdin>`, `<stdout>` or `<stderr>`.
    Name,
    /// `mode`: the mode as written to `open`, or `r` or `w` for a standard file.
    Mode,
    /// `closed`: whether the file is closed.
    Closed,
    /// `name` of a file opened for a file descriptor: that descriptor, an `int`.
    Descriptor,
}

impl FileAttribute {
    /// Returns the attribute `name` names, with its type.
    #[must_use]
    pub fn named(name: &str) -> Option<(Self, crate::types::Type)> {
        use crate::types::Type;
        match name {
            "name" => Some((Self::Name, Type::Str)),
            "mode" => Some((Self::Mode, Type::Str)),
            "closed" => Some((Self::Closed, Type::Bool)),
            _ => None,
        }
    }
}

/// The arguments of `open` after its mode, as the runtime takes them (LIB-14).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct OpenOptions {
    /// `buffering`, when given and not -1.
    pub buffering: Option<Box<TypedExpr>>,
    /// `encoding` as written, which the file's `repr` shows; `None` for `None` or not given.
    pub encoding: Option<String>,
    /// The codec `encoding` names, numbered as the runtime takes it.
    pub codec: i64,
    /// The error handler `errors` names, numbered as the runtime takes it.
    pub handler: i64,
    /// `newline`, when given and not `None`.
    pub newline: Option<Box<TypedExpr>>,
    /// The mode, when it is not a literal: a text mode the runtime reads (ADR-0031).
    pub mode: Option<Box<TypedExpr>>,
    /// `closefd`, when given and not the literal `True`.
    pub closefd: Option<Box<TypedExpr>>,
}
