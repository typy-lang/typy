//! Empty collections that local variables hold, which take their item types from the first
//! statement of the function that adds to them (TYP-27, ADR-0092). Looks at the body find
//! those statements, and the body is then checked with the types they give.

use std::collections::BTreeMap;

use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId, Span};
use typy_source::syntax::{Expr, Keyword, Stmt};

use super::list::Empty;
use super::nested_function::Closure;
use super::{BodyChecker, Program};
use crate::builtin::Builtin;
use crate::check::Definition;
use crate::typed::{FunctionId, SymbolId};
use crate::types::Type;

/// What a check of a body knows and learns of the empty collections its variables hold.
#[derive(Default)]
pub(super) struct FirstAdds {
    /// Whether this check is a look that finds the statements that add to them.
    looking: bool,
    /// The types the looks found, by where the empty collection is written.
    known: BTreeMap<Span, Type>,
    /// The types this look found, by where the empty collection is written.
    found: BTreeMap<Span, Type>,
    /// The variables that hold an empty collection no statement has added to yet.
    waiting: BTreeMap<SymbolId, Waiting>,
    /// The empty collections whose types this look could not find, with their errors.
    unknown: BTreeMap<Span, (Type, Diagnostic)>,
    /// The errors the looks found, which the check after them reports.
    errors: Vec<Diagnostic>,
}

/// A variable that holds an empty collection whose item types are not known yet.
struct Waiting {
    name: String,
    kind: Empty,
    /// Where the empty collection is written, where E0410 points.
    at: Span,
}

/// Why the types of an empty collection are not known.
#[derive(Clone, Copy)]
enum Unknown {
    /// It is read here before a statement adds to it.
    Read(Span),
    /// What the statement adds here has no type of its own.
    Added(Span),
    /// No statement of the function adds to it.
    NeverAdded,
}

/// An empty collection an assignment gives a variable of a function without an annotation.
pub(super) struct EmptyLocal {
    name: String,
    kind: Empty,
    at: Span,
    known: Option<Type>,
}

impl EmptyLocal {
    /// Returns the type the collection takes: the one the first statement that adds to it
    /// gives, or a collection of the error type while a look finds that statement.
    pub(super) fn ty(&self) -> Type {
        self.known.clone().unwrap_or_else(|| unknown(self.kind))
    }
}

impl BodyChecker<'_> {
    /// Returns what looks at the body of `function` found of the empty collections it assigns
    /// to variables, for the check after them. A look that finds a type is followed by
    /// another, since one collection's type may give another's; each look is undone.
    pub(super) fn look_for_adds(
        program: &mut Program,
        function: FunctionId,
        definition: &Definition<'_>,
        closure: Option<&Closure>,
    ) -> FirstAdds {
        let mut known = BTreeMap::new();
        if !definition.body.iter().any(assigns_empty) {
            return FirstAdds::default();
        }
        loop {
            let snapshot = program.snapshot(program.module_first_class());
            // A result being inferred takes the types the check after the looks returns
            // (TYP-26).
            let returned = program
                .inferring
                .as_mut()
                .map(|(_, types)| std::mem::take(types));
            let looking = FirstAdds {
                looking: true,
                known: known.clone(),
                ..FirstAdds::default()
            };
            let mut ignored = Diagnostics::new();
            let (_, look) = Self::check_body_with(
                program,
                function,
                definition,
                closure.cloned(),
                looking,
                &mut ignored,
            );
            if let (Some((_, types)), Some(returned)) = (program.inferring.as_mut(), returned) {
                *types = returned;
            }
            program.undo(snapshot);
            program.forget_function(function);
            program.cells.remove(&function);
            if look.found.is_empty() {
                let mut errors = Vec::new();
                for (at, (ty, error)) in look.unknown {
                    known.insert(at, ty);
                    errors.push(error);
                }
                return FirstAdds {
                    known,
                    errors,
                    ..FirstAdds::default()
                };
            }
            known.extend(look.found);
        }
    }

    /// Returns the empty collection an assignment of `value` to `target` gives a variable of
    /// a function when no annotation types the variable, or `None` for any other assignment.
    pub(super) fn empty_local(&self, target: &Expr, value: &Expr) -> Option<EmptyLocal> {
        let Expr::Name { id, .. } = target else {
            return None;
        };
        if !self.in_function()
            || self.global_names.contains(id)
            || self.nonlocal_names.contains_key(id)
        {
            return None;
        }
        let kind = match self.called_builtin(value) {
            Some((Builtin::List, [], [])) => Empty::List,
            _ => self.empty_collection(value)?,
        };
        let at = value.span();
        let known = self.first_adds.known.get(&at).cloned();
        // The check after the looks gives a collection its type only when a look reached it.
        if known.is_none() && !self.first_adds.looking {
            return None;
        }
        Some(EmptyLocal {
            name: id.clone(),
            kind,
            at,
            known,
        })
    }

    /// Notes that the variable an assignment gave an empty collection waits for a statement
    /// that adds to it, while a look finds its type.
    pub(super) fn wait_for_add(&mut self, empty: EmptyLocal) {
        if empty.known.is_some() {
            return;
        }
        if let Some(symbol) = self.locals.get(&empty.name).copied() {
            let waiting = Waiting {
                name: empty.name,
                kind: empty.kind,
                at: empty.at,
            };
            self.first_adds.waiting.insert(symbol, waiting);
        }
    }

    /// Notes a read of `symbol` at `span`; a read of an empty collection before a statement
    /// adds to it leaves its type unknown, E0410.
    pub(super) fn read_before_add(&mut self, symbol: SymbolId, span: Span) {
        if let Some(waiting) = self.first_adds.waiting.remove(&symbol) {
            self.not_known(&waiting, Unknown::Read(span));
        }
    }

    /// Notes the type a call such as `items.append(value)` gives the empty collection it adds
    /// to: `append` and `insert` of a list, `add` of a set, `extend` of a list and `update` of
    /// a set with an iterable, and `setdefault` and `update` of a dict.
    pub(super) fn add_to_empty(
        &mut self,
        function: &Expr,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) {
        let Expr::Attribute {
            value, attribute, ..
        } = function
        else {
            return;
        };
        let Some((symbol, kind)) = self.waiting_variable(value) else {
            return;
        };
        if !keywords.is_empty()
            || arguments
                .iter()
                .any(|argument| matches!(argument, Expr::Starred { .. }))
        {
            return;
        }
        let ty = match (kind, attribute.name.as_str(), arguments) {
            (Empty::List, "append", [item])
            | (Empty::List, "insert", [_, item])
            | (Empty::Set, "add", [item]) => {
                let item = self.expression(item).ty;
                holding(kind, item)
            }
            (Empty::List, "extend", [items]) | (Empty::Set, "update", [items]) => {
                let items = self.expression(items).ty;
                let item = super::unpacked_arguments::iterated_type(&items).unwrap_or(Type::Error);
                holding(kind, item)
            }
            (Empty::Dict, "setdefault", [key, value]) => {
                let key = self.expression(key).ty;
                let value = self.expression(value).ty;
                Type::Dict(Box::new(key), Box::new(value))
            }
            (Empty::Dict, "update", [other]) => match self.expression(other).ty {
                dict @ Type::Dict(..) => dict,
                _ => Type::Error,
            },
            _ => return,
        };
        self.added(symbol, ty, span);
    }

    /// Notes the type `items[key] = value` gives the empty dict it adds to. The value is
    /// evaluated first, as CPython does, so a value that reads the dict leaves its type
    /// unknown.
    pub(super) fn set_in_empty(&mut self, object: &Expr, key: &Expr, value: &Expr) {
        let Some((symbol, Empty::Dict)) = self.waiting_variable(object) else {
            return;
        };
        let span = value.span();
        let value = self.expression(value).ty;
        let key = self.expression(key).ty;
        self.added(symbol, Type::Dict(Box::new(key), Box::new(value)), span);
    }

    /// Ends the body's check: a look notes each empty collection no statement added to,
    /// E0410, and the check after the looks reports the errors they found.
    pub(super) fn finish_first_adds(&mut self) {
        if !self.first_adds.looking {
            for error in std::mem::take(&mut self.first_adds.errors) {
                self.diagnostics.push(error);
            }
            return;
        }
        for waiting in std::mem::take(&mut self.first_adds.waiting).into_values() {
            self.not_known(&waiting, Unknown::NeverAdded);
        }
    }

    /// Returns the variable named by `expression` that waits for a statement to add to it,
    /// with the kind of collection it holds.
    fn waiting_variable(&self, expression: &Expr) -> Option<(SymbolId, Empty)> {
        let Expr::Name { id, .. } = expression else {
            return None;
        };
        let symbol = *self.locals.get(id)?;
        Some((symbol, self.first_adds.waiting.get(&symbol)?.kind))
    }

    /// Gives a waiting variable the type the statement at `span` that adds to it gives. A
    /// type with an error leaves the collection's type unknown.
    fn added(&mut self, symbol: SymbolId, ty: Type, span: Span) {
        let Some(waiting) = self.first_adds.waiting.remove(&symbol) else {
            return;
        };
        if ty.has_error() {
            self.not_known(&waiting, Unknown::Added(span));
            return;
        }
        self.types.insert(symbol, ty.clone());
        self.first_adds.found.insert(waiting.at, ty);
    }

    /// Notes that the types of a waiting variable's collection are not known, E0410, which
    /// the check after the looks reports; that check gives the collection the error type.
    fn not_known(&mut self, waiting: &Waiting, cause: Unknown) {
        let error = unknown_items(waiting, cause);
        self.first_adds
            .unknown
            .insert(waiting.at, (unknown(waiting.kind), error));
    }
}

/// Returns whether a statement, or a block it holds outside the functions and classes it
/// defines, assigns an empty collection to a name, as `items = []` does.
fn assigns_empty(statement: &Stmt) -> bool {
    if let Stmt::Assign { targets, value, .. } = statement
        && matches!(targets.as_slice(), [Expr::Name { .. }])
        && is_empty(value)
    {
        return true;
    }
    crate::check::nested_blocks(statement)
        .into_iter()
        .any(|block| block.iter().any(assigns_empty))
}

/// Returns whether an expression is written as an empty collection: `[]`, `{}`, `list()`,
/// `set()` or `dict()`.
fn is_empty(expression: &Expr) -> bool {
    match expression {
        Expr::List { elements, .. } => elements.is_empty(),
        Expr::Dict { keys, .. } => keys.is_empty(),
        Expr::Call {
            function,
            arguments,
            keywords,
            ..
        } => {
            arguments.is_empty()
                && keywords.is_empty()
                && matches!(function.as_ref(), Expr::Name { id, .. }
                    if matches!(id.as_str(), "list" | "set" | "dict"))
        }
        _ => false,
    }
}

/// Returns a collection of `kind` whose items, and for a dict its keys and values, are
/// `item`.
fn holding(kind: Empty, item: Type) -> Type {
    match kind {
        Empty::List => Type::List(Box::new(item)),
        Empty::Set => Type::Set(Box::new(item)),
        Empty::Dict => Type::Dict(Box::new(item.clone()), Box::new(item)),
    }
}

/// Returns a collection of `kind` whose items have the error type, which an empty collection
/// holds while its type is not known.
fn unknown(kind: Empty) -> Type {
    holding(kind, Type::Error)
}

/// Returns E0410 for an empty collection whose type is not known, for `cause`.
fn unknown_items(waiting: &Waiting, cause: Unknown) -> Diagnostic {
    let name = &waiting.name;
    let (message, annotated, added) = match waiting.kind {
        Empty::List => (
            "an empty list needs an annotation that names its item type",
            format!("{name}: list[int] = []"),
            format!("{name}.append(1)"),
        ),
        Empty::Set => (
            "an empty set needs an annotation that names its item type",
            format!("{name}: set[int] = set()"),
            format!("{name}.add(1)"),
        ),
        Empty::Dict => (
            "an empty dict needs an annotation that names its key and value types",
            format!("{name}: dict[str, int] = {{}}"),
            format!("{name}[\"key\"] = 1"),
        ),
    };
    let error =
        Diagnostic::new(DiagnosticCode::error(410), waiting.at, message).with_rule(RuleId::TYP_05);
    let annotate = format!("annotate it, such as `{annotated}`");
    match cause {
        Unknown::Read(read) => error
            .with_label(
                read,
                format!("`{name}` is read here before a statement adds to it"),
            )
            .with_help(annotate),
        Unknown::Added(added) => error
            .with_label(added, "the type of what this adds is not known")
            .with_help(annotate),
        Unknown::NeverAdded => error.with_help(format!(
            "add to `{name}` in this function, such as `{added}`, or {annotate}"
        )),
    }
}

#[cfg(test)]
mod tests;
