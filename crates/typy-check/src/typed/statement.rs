//! Statements after checking.

use typy_diagnostics::Span;
use typy_source::syntax::BinaryOperator;

use super::FunctionId;
use super::expression::TypedExpr;
use super::symbol::SymbolId;
use crate::types::Type;

/// Where an assignment stores its value: a variable, or a tuple of targets that the value is
/// unpacked into, item by item (SMT-02).
#[derive(Clone, Debug, PartialEq)]
pub enum AssignTarget {
    /// A variable.
    Name(SymbolId),
    /// One target for each item of a tuple value.
    Tuple(Vec<AssignTarget>),
    /// An item of a list, `list[index] = value`; the list and the index are evaluated after the
    /// value, as CPython does (DAT-07).
    Item {
        /// The list.
        list: Box<TypedExpr>,
        /// The position, counted from the end when negative.
        index: Box<TypedExpr>,
    },
    /// A slice of a list, `list[lower:upper:step] = items`, which the items of a list replace;
    /// the list and the bounds are evaluated after the value, as CPython does (DAT-13).
    Slice {
        /// The list.
        list: Box<TypedExpr>,
        /// The start, if given.
        lower: Option<Box<TypedExpr>>,
        /// The stop, if given.
        upper: Option<Box<TypedExpr>>,
        /// The step, if given.
        step: Option<Box<TypedExpr>>,
    },
    /// An attribute of an instance, `object.name = value`, by its position in the class's
    /// attributes; the object is evaluated after the value, as CPython does (CLS-01).
    Attribute {
        /// The instance.
        object: Box<TypedExpr>,
        /// The attribute's position.
        index: usize,
    },
    /// An entry of a dict, `dict[key] = value`; the dict and the key are evaluated after the
    /// value, as CPython does (DAT-09).
    Entry {
        /// The dict.
        dict: Box<TypedExpr>,
        /// The key.
        key: Box<TypedExpr>,
    },
}

/// One `for target in source if condition ...` clause of a comprehension (EXP-12).
#[derive(Clone, Debug, PartialEq)]
pub struct Generator {
    /// What each value is bound to, in the comprehension's own scope.
    pub target: AssignTarget,
    /// What the clause goes over.
    pub source: LoopSource,
    /// The conditions a value must meet, tested in order, each a truth test.
    pub conditions: Vec<super::expression::TypedExpr>,
}

/// What a `for` loop goes over (CST-03, LIB-02). A sequence is read as CPython's list
/// iterator reads it: each step checks the length again, so a list that grows or shrinks in
/// the body changes how many steps there are.
#[derive(Clone, Debug, PartialEq)]
pub enum LoopSource {
    /// The items of a list, or the characters of a `str`.
    Items(TypedExpr),
    /// `reversed(sequence)`: the items from the last to the first.
    Reversed(TypedExpr),
    /// `range(start, stop, step)`, or `reversed` of one: the arguments are evaluated once, in
    /// order, and a zero step raises `ValueError`, as `range()` does.
    Range {
        /// The first value, 0 when `range` is given only a stop.
        start: TypedExpr,
        /// The value the range stops before.
        stop: TypedExpr,
        /// The step, 1 when it is not given.
        step: TypedExpr,
        /// True for `reversed(range(...))`.
        reversed: bool,
    },
    /// `enumerate(source, start)`: a count from `start`, with each value of `source`.
    Enumerate {
        /// What is counted.
        source: Box<LoopSource>,
        /// The first count.
        start: TypedExpr,
    },
    /// An instance of a class with `__iter__`: `__iter__` is called once, then the `__next__`
    /// of what it returns until that raises `StopIteration` (CST-08).
    Protocol {
        /// The instance.
        iterable: Box<TypedExpr>,
        /// Its class's `__iter__`.
        iter: super::function::FunctionId,
        /// The `__next__` of the class `__iter__` returns.
        next: super::function::FunctionId,
        /// What `__iter__` returns.
        iterator: Type,
        /// What `__next__` returns.
        item: Type,
    },
    /// `zip(first, second, ...)`: a tuple of one value from each source, until one ends. With
    /// `strict` true, sources of different lengths raise `ValueError`.
    Zip {
        /// The sources, in order.
        sources: Vec<LoopSource>,
        /// The `strict` argument, when given.
        strict: Option<TypedExpr>,
    },
}

impl LoopSource {
    /// Returns every expression of the source, in the order they are evaluated.
    #[must_use]
    pub fn expressions(&self) -> Vec<&TypedExpr> {
        match self {
            Self::Items(sequence) | Self::Reversed(sequence) => vec![sequence],
            Self::Protocol { iterable, .. } => vec![iterable],
            Self::Range {
                start, stop, step, ..
            } => vec![start, stop, step],
            Self::Enumerate { source, start } => {
                let mut expressions = source.expressions();
                expressions.push(start);
                expressions
            }
            Self::Zip { sources, strict } => sources
                .iter()
                .flat_map(Self::expressions)
                .chain(strict)
                .collect(),
        }
    }

    /// Returns the type of what one step gives.
    #[must_use]
    pub fn item_type(&self) -> Type {
        match self {
            Self::Range { .. } => Type::Int,
            Self::Protocol { item, .. } => item.clone(),
            Self::Items(sequence) | Self::Reversed(sequence) => match &sequence.ty {
                Type::Bytes | Type::Range => Type::Int,
                Type::Set(item)
                | Type::Dict(item, _)
                | Type::Iterator(item)
                | Type::Iterable(item) => item.as_ref().clone(),
                Type::DictView(view, key, value) => view.item(key, value),
                other => other
                    .sequence_item()
                    .cloned()
                    .unwrap_or_else(|| other.clone()),
            },
            Self::Enumerate { source, .. } => Type::Tuple(vec![Type::Int, source.item_type()]),
            Self::Zip { sources, .. } => Type::Tuple(sources.iter().map(Self::item_type).collect()),
        }
    }
}

/// A name an import binds to a value.
#[derive(Clone, Debug, PartialEq)]
pub enum ImportBinding {
    /// A variable bound to a module object.
    Module {
        /// The variable.
        target: SymbolId,
        /// The module.
        module: String,
    },
    /// A variable bound to the value a module variable of `module` has, which raises
    /// `ImportError` while unbound.
    Value {
        /// The variable.
        target: SymbolId,
        /// The module that holds the value.
        module: String,
        /// The module variable whose value it takes.
        source: SymbolId,
    },
}

/// A statement after checking.
#[derive(Clone, Debug, PartialEq)]
#[allow(
    missing_docs,
    reason = "each variant is the Python statement of the same name"
)]
pub enum TypedStmt {
    /// `a = b = value`: the value is evaluated once, then bound to each target in order.
    Assign {
        targets: Vec<AssignTarget>,
        value: TypedExpr,
        span: Span,
    },
    /// `target op= value`: an item target evaluates its list and index once, then reads the item
    /// before the value is evaluated, as CPython does. A list target with `+=` or `*=` changes
    /// the list in place.
    AugAssign {
        target: AssignTarget,
        operator: BinaryOperator,
        value: TypedExpr,
        span: Span,
    },
    Expr {
        value: TypedExpr,
        span: Span,
    },
    Return {
        value: Option<TypedExpr>,
        span: Span,
    },
    /// `with manager as target: body`: `__enter__` gives what the target binds, and `__exit__`
    /// runs on every way out of the body; when it may return a true value, it may suppress an
    /// exception (CST-05). Several items nest.
    With {
        manager: TypedExpr,
        enter: super::function::FunctionId,
        exit: super::function::FunctionId,
        suppresses: bool,
        target: Option<AssignTarget>,
        body: Vec<TypedStmt>,
        span: Span,
    },
    /// `del target, ...`: each target, a list item, a list slice or a dict key, is removed in
    /// order (SMT-05).
    Delete {
        targets: Vec<TypedExpr>,
        span: Span,
    },
    /// `assert test, message`: when the test is false, the exception, an `AssertionError` of
    /// the message, is raised (SMT-04).
    Assert {
        test: TypedExpr,
        exception: TypedExpr,
        span: Span,
    },
    /// `try` with its `except` clauses, its `else` block and its `finally` block (CST-04).
    Try {
        body: Vec<TypedStmt>,
        handlers: Vec<TypedHandler>,
        orelse: Vec<TypedStmt>,
        finalbody: Vec<TypedStmt>,
        span: Span,
    },
    /// `raise exception`, with its cause; a class is called with no arguments first. A bare
    /// `raise`, with no exception, raises the one being handled again (SMT-06, EXC-07).
    Raise {
        exception: Option<TypedExpr>,
        cause: RaiseCause,
        span: Span,
    },
    If {
        test: TypedExpr,
        body: Vec<TypedStmt>,
        orelse: Vec<TypedStmt>,
        span: Span,
    },
    While {
        test: TypedExpr,
        body: Vec<TypedStmt>,
        orelse: Vec<TypedStmt>,
        span: Span,
    },
    /// `for target in source`, over a sequence or a builtin that goes over sequences (CST-03).
    ForEach {
        target: AssignTarget,
        source: LoopSource,
        body: Vec<TypedStmt>,
        orelse: Vec<TypedStmt>,
        span: Span,
    },
    /// `for target in range(start, stop, step)`; the arguments are evaluated once, in order.
    ForRange {
        target: SymbolId,
        start: TypedExpr,
        stop: TypedExpr,
        step: TypedExpr,
        body: Vec<TypedStmt>,
        orelse: Vec<TypedStmt>,
        span: Span,
    },
    /// `import` or `from ... import`: each module, in order, is initialised unless it has
    /// begun, then each binding takes its value (SMT-07, MOD-03).
    Import {
        modules: Vec<String>,
        bindings: Vec<ImportBinding>,
        span: Span,
    },
    /// A lazy import: its names wait, unbound, until the first use of one runs its function
    /// in `reifies`, which runs the import for that name (MOD-06).
    LazyImport {
        reifies: Vec<FunctionId>,
        targets: Vec<SymbolId>,
        span: Span,
    },
    Pass {
        span: Span,
    },
    Break {
        span: Span,
    },
    Continue {
        span: Span,
    },
}

/// What `from` gives a raised exception (EXC-04).
#[derive(Clone, Debug, PartialEq)]
pub enum RaiseCause {
    /// No `from`: the exception keeps the context it is raised in.
    Absent,
    /// `from None`: no cause, and the context is left out of the report.
    None,
    /// `from cause`: an exception, or a class called with no arguments.
    Exception(TypedExpr),
}

/// One `except` clause: the classes it catches, in order, with `BaseException` for a bare
/// `except`; the name it binds, for its body only; and its body (EXC-01, EXC-09).
#[derive(Clone, Debug, PartialEq)]
pub struct TypedHandler {
    /// The classes the clause catches.
    pub classes: Vec<crate::exception::ExceptionClass>,
    /// The variable `as` binds, which holds the exception while the body runs.
    pub name: Option<SymbolId>,
    /// The clause's body.
    pub body: Vec<TypedStmt>,
    /// The whole clause.
    pub span: Span,
}
