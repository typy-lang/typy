//! Checks the arguments a call unpacks with `*` and `**` (EXP-17, ADR-0073): the items of a
//! tuple of a fixed number of items bind as positional arguments would; from the first
//! `*iterable` of unknown length on, every positional value goes into one list of one type
//! that binds when the call runs; and each `**dict` is a `dict[str, V]` of one `V` whose
//! entries bind by their keys when the call runs.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Expr, ParameterKind};

use super::BodyChecker;
use super::binding::{Parameters, Slot};
use super::spread::held_alike;
use crate::typed::{
    Argument, Passed, SignatureParameter, SpreadKind, SpreadPart, TypedExpr, TypedExprKind,
};
use crate::types::Type;

/// The positional values from the first `*iterable` of unknown length on.
pub(super) struct Tail {
    /// The index of the first parameter they may fill.
    start: usize,
    /// The type of every parameter they may reach, when those share one.
    item: Option<Type>,
    /// Each part, with the type of its values and where it is.
    parts: Vec<(SpreadPart, Type, Span)>,
    span: Span,
}

impl Tail {
    pub(super) const fn start(&self) -> usize {
        self.start
    }
}

/// Which parameters a call's unpacked arguments may fill when it runs.
#[derive(Clone, Copy, Default)]
pub(super) struct Reach {
    /// The index of the first parameter the positional tail may fill.
    pub(super) tail_from: Option<usize>,
    /// Whether a `**dict` may fill parameters by name.
    pub(super) entries: bool,
}

impl Reach {
    /// Returns true when an unpacked argument may fill parameter `index` of `kind`.
    pub(super) fn reaches(self, index: usize, kind: ParameterKind) -> bool {
        let by_position = matches!(kind, ParameterKind::PositionalOnly | ParameterKind::Normal)
            && self.tail_from.is_some_and(|start| index >= start);
        let by_name =
            self.entries && matches!(kind, ParameterKind::Normal | ParameterKind::KeywordOnly);
        by_position || by_name
    }
}

impl BodyChecker<'_> {
    /// Binds the items of `tuple`, a tuple of a fixed number of items `*` unpacks, to the
    /// parameters from `position` on, as positional arguments; marks the ones they fill.
    /// Returns the argument, or the error of an item past the parameters.
    pub(super) fn bind_items(
        &mut self,
        (shape, parameters): (&Parameters<'_>, &[SignatureParameter]),
        (tuple, position): (TypedExpr, usize),
        (filled, skip): (&mut [bool], usize),
        name: &str,
    ) -> Result<Argument, TypedExpr> {
        let Type::Tuple(items) = &tuple.ty else {
            unreachable!("a tuple of a fixed number of items")
        };
        let mut targets = Vec::new();
        for (offset, item) in items.iter().enumerate() {
            let Some(slot) = shape.of_position(position + offset) else {
                let span = tuple.span;
                let message = format!(
                    "`{name}` takes {} positional argument{}, and the tuple `*` unpacks gives \
                     more",
                    shape.positional(),
                    if shape.positional() == 1 { "" } else { "s" }
                );
                return Err(self.error(
                    Diagnostic::new(DiagnosticCode::error(404), span, message)
                        .with_rule(RuleId::TYP_01),
                    span,
                ));
            };
            let (index, expected, what) = match slot {
                Slot::Named(index, ty) => {
                    filled[index] = true;
                    (index, ty, format!("`{}`", parameters[index].name))
                }
                Slot::Packed(index, ty) => (index, ty, format!("`*{}`", parameters[index].name)),
                Slot::PositionalOnly(_) => unreachable!("a position reaches no keyword"),
            };
            if !item.has_error() && !expected.has_error() && !held_alike(item, expected) {
                let message = format!(
                    "item {offset} of the tuple `*` unpacks goes to {what}, which takes {}, and \
                     it is {}",
                    expected.with_article(),
                    item.with_article()
                );
                self.diagnostics.push(
                    Diagnostic::new(DiagnosticCode::error(404), tuple.span, message)
                        .with_rule(RuleId::EXP_17),
                );
            }
            targets.push(index + skip);
        }
        Ok(Argument {
            parameter: position + skip,
            passed: Passed::Items(targets),
            value: tuple,
        })
    }

    /// Starts the positional tail at the parameter `start` with the iterable `value` `*`
    /// unpacks.
    pub(super) fn start_tail(
        &mut self,
        parameters: &[SignatureParameter],
        start: usize,
        value: TypedExpr,
    ) -> Tail {
        let mut reached = parameters
            .iter()
            .enumerate()
            .filter_map(|(index, parameter)| tail_target(parameter, index, start));
        let first = reached.next().cloned();
        let item = first.filter(|first| reached.all(|other| other == first));
        let mut tail = Tail {
            start,
            item,
            parts: Vec::new(),
            span: value.span,
        };
        self.add_unpacked_to_tail(&mut tail, value);
        tail
    }

    /// Adds a positional argument after the first `*iterable` of unknown length to the tail.
    pub(super) fn add_to_tail(&mut self, tail: &mut Tail, argument: &Expr) {
        if let Expr::Starred { value, .. } = argument {
            let value = Self::version_tuple(self.expression(value));
            self.add_unpacked_to_tail(tail, value);
            return;
        }
        let value = self.expression_expecting(argument, tail.item.as_ref());
        let ty = value.ty.clone();
        tail.parts
            .push((SpreadPart::Item(value), ty, argument.span()));
    }

    fn add_unpacked_to_tail(&mut self, tail: &mut Tail, value: TypedExpr) {
        let span = value.span;
        let sequence = Self::sequence_of(value);
        let (source, item) = self.sequence_source(sequence, "`*`");
        tail.parts
            .push((SpreadPart::Each(Box::new(source)), item, span));
    }

    /// Returns a new list of the values of the iterable `value` `*` unpacks, read as a `for`
    /// loop reads them.
    pub(super) fn unpacked_list(&mut self, value: &Expr) -> TypedExpr {
        let value = Self::version_tuple(self.expression(value));
        let span = value.span;
        let sequence = Self::sequence_of(value);
        let (source, item) = self.sequence_source(sequence, "`*`");
        Self::typed(
            TypedExprKind::Spread {
                kind: SpreadKind::List,
                parts: vec![SpreadPart::Each(Box::new(source))],
            },
            Type::List(Box::new(item)),
            span,
        )
    }

    /// Returns the argument of the positional tail, a list of one type, after checking each
    /// part holds its values alike and that type fits each parameter it may reach.
    pub(super) fn finish_tail(
        &mut self,
        tail: Tail,
        parameters: &[SignatureParameter],
        skip: usize,
    ) -> Argument {
        // The parameters the tail may reach give its type when they share one; a value that
        // does not fit is theirs to refuse. Otherwise the first value gives it.
        let from_parameters = tail.item.is_some();
        let item = tail
            .item
            .clone()
            .or_else(|| tail.parts.first().map(|(_, ty, _)| ty.clone()))
            .unwrap_or(Type::Error);
        let mut parts = Vec::new();
        for (part, ty, at) in tail.parts {
            let (part, fits) = match part {
                SpreadPart::Item(value) => {
                    let value = Self::converted(value, Some(&item));
                    let fits = value.ty.is_assignable_to(&item);
                    (SpreadPart::Item(value), fits)
                }
                SpreadPart::Each(source) => {
                    let fits = ty.has_error() || item.has_error() || held_alike(&ty, &item);
                    (SpreadPart::Each(source), fits)
                }
            };
            if !fits {
                let unpacked = matches!(part, SpreadPart::Each(_));
                self.unfit_tail_value((&ty, &item), at, unpacked, from_parameters);
            }
            parts.push(part);
        }
        if from_parameters {
            return Self::tail_argument(tail.start, parts, item, (skip, tail.span));
        }
        for (index, parameter) in parameters.iter().enumerate() {
            let Some(target) = tail_target(parameter, index, tail.start) else {
                continue;
            };
            if !item.has_error() && !target.has_error() && !held_alike(&item, target) {
                let message = format!(
                    "`{}` takes {}, and `*` may give it values of type {item}",
                    written(parameter),
                    target.with_article()
                );
                self.diagnostics.push(
                    Diagnostic::new(DiagnosticCode::error(404), tail.span, message)
                        .with_rule(RuleId::EXP_17),
                );
            }
        }
        Self::tail_argument(tail.start, parts, item, (skip, tail.span))
    }

    /// Returns the argument of the tail, a list of `item` values made of `parts`.
    fn tail_argument(
        start: usize,
        parts: Vec<SpreadPart>,
        item: Type,
        (skip, span): (usize, Span),
    ) -> Argument {
        let list = Self::typed(
            TypedExprKind::Spread {
                kind: SpreadKind::List,
                parts,
            },
            Type::List(Box::new(item)),
            span,
        );
        Argument {
            parameter: start + skip,
            passed: Passed::Tail,
            value: list,
        }
    }

    /// Reports a positional value after the first `*iterable` that does not fit `item`: the
    /// type the parameters it may reach take, E0404, or that of the first value, E0409.
    fn unfit_tail_value(
        &mut self,
        (ty, item): (&Type, &Type),
        span: Span,
        unpacked: bool,
        from_parameters: bool,
    ) {
        let what = if unpacked {
            format!("this unpacks items of type {ty}")
        } else {
            format!("this is {}", ty.with_article())
        };
        let (code, message) = if from_parameters {
            (
                404,
                format!("the parameters `*` may fill take {item}, and {what}"),
            )
        } else {
            (
                409,
                format!(
                    "the positional arguments from the first `*` on have one type; {what}, and \
                     they are {item}"
                ),
            )
        };
        self.diagnostics.push(
            Diagnostic::new(DiagnosticCode::error(code), span, message).with_rule(RuleId::EXP_17),
        );
    }

    /// Checks the dict `**` unpacks, which must be a `dict[str, V]` of the `V` of the call's
    /// first one, and returns its argument; `None` after an error.
    pub(super) fn bind_entries(
        &mut self,
        value: &Expr,
        entries: &mut Option<Type>,
    ) -> Option<Argument> {
        let value = self.expression(value);
        let item = match &value.ty {
            Type::Dict(key, item) if **key == Type::Str => item.as_ref().clone(),
            Type::Error => return None,
            other => {
                let message = format!(
                    "`**` in a call takes a dict of `str` keys, and this is {}",
                    other.with_article()
                );
                self.diagnostics.push(
                    Diagnostic::new(DiagnosticCode::error(404), value.span, message)
                        .with_rule(RuleId::EXP_17),
                );
                return None;
            }
        };
        match entries {
            None => *entries = Some(item),
            Some(first) if *first == item => {}
            Some(first) => {
                let message = format!(
                    "the dicts `**` unpacks in a call have one value type; this one holds {item}, \
                     and the first {first}"
                );
                self.diagnostics.push(
                    Diagnostic::new(DiagnosticCode::error(409), value.span, message)
                        .with_rule(RuleId::EXP_17),
                );
                return None;
            }
        }
        Some(Argument {
            parameter: 0,
            passed: Passed::Entries,
            value,
        })
    }

    /// Checks that `item`, the value type of the dicts `**` unpacks, fits each parameter a
    /// keyword may still fill and the values of `**kwargs`, which take the entries as they are.
    pub(super) fn check_entries_reach(
        &mut self,
        item: &Type,
        parameters: &[SignatureParameter],
        filled: &[bool],
        span: Span,
    ) {
        for (index, parameter) in parameters.iter().enumerate() {
            let fits = match parameter.kind {
                ParameterKind::Normal | ParameterKind::KeywordOnly if !filled[index] => {
                    held_alike(item, &parameter.ty)
                }
                ParameterKind::VarKeyword => match &parameter.ty {
                    Type::Dict(_, value) => **value == *item,
                    _ => true,
                },
                _ => continue,
            };
            if fits || item.has_error() || parameter.ty.has_error() {
                continue;
            }
            let message = format!(
                "`{}` takes {}, and `**` may give it values of type {item}",
                written(parameter),
                held(parameter).with_article()
            );
            self.diagnostics.push(
                Diagnostic::new(DiagnosticCode::error(404), span, message)
                    .with_rule(RuleId::EXP_17),
            );
        }
    }
}

/// Returns the type of each value of an iterable of type `ty`, as a `for` loop reads it, or
/// `None` for a type that is no iterable a loop reads with one type.
pub(super) fn iterated_type(ty: &Type) -> Option<Type> {
    match ty {
        Type::List(item)
        | Type::TupleOf(item)
        | Type::Set(item)
        | Type::Iterator(item)
        | Type::Iterable(item)
        | Type::Dict(item, _) => Some(item.as_ref().clone()),
        Type::Generator { yields, .. } => Some(yields.as_ref().clone()),
        Type::Str => Some(Type::Str),
        Type::Bytes | Type::Range => Some(Type::Int),
        Type::Tuple(items) => {
            let (first, rest) = items.split_first()?;
            rest.iter().all(|item| item == first).then(|| first.clone())
        }
        _ => None,
    }
}

/// Returns the type a value of the positional tail must fit to fill `parameter`, at `index`,
/// when the tail starts at `start`: its own for a parameter a position fills, the item type of
/// `*args`, or `None` for one the tail does not reach.
fn tail_target(parameter: &SignatureParameter, index: usize, start: usize) -> Option<&Type> {
    match parameter.kind {
        ParameterKind::PositionalOnly | ParameterKind::Normal if index >= start => {
            Some(&parameter.ty)
        }
        ParameterKind::VarPositional => Some(held(parameter)),
        _ => None,
    }
}

/// Returns the type of each value a parameter holds: an item of `*args` or `**kwargs`, or the
/// parameter's own.
fn held(parameter: &SignatureParameter) -> &Type {
    match (&parameter.kind, &parameter.ty) {
        (ParameterKind::VarPositional, Type::TupleOf(item))
        | (ParameterKind::VarKeyword, Type::Dict(_, item)) => item,
        (_, ty) => ty,
    }
}

/// Returns a parameter as its definition writes it: `*args`, `**kwargs` or its name.
fn written(parameter: &SignatureParameter) -> String {
    let stars = match parameter.kind {
        ParameterKind::VarPositional => "*",
        ParameterKind::VarKeyword => "**",
        _ => "",
    };
    format!("{stars}{}", parameter.name)
}
