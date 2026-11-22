//! Lowers a call that binds its arguments when it runs (EXP-17, ADR-0073). The positional
//! arguments, a list of the positional tail, and the keyword arguments are evaluated in
//! source order, each keyword name recorded and each `**dict` merged as CPython merges them;
//! `call_check` then raises CPython's `TypeError` for a call that does not fit, and each
//! parameter takes its value: an argument the checker bound, an item of the tail, an entry
//! of a `**dict`, or its default.

use typy_check::{Argument, FunctionId, Passed, SignatureParameter, Type, TypedExpr};
use typy_diagnostics::Span;
use typy_source::syntax::ParameterKind;
use typy_tyir::{
    Comparison, Constant, IntOperator, Operation, RuntimeFunction, ScalarKind, TyirType, ValueId,
};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

/// The arguments of a call that binds when it runs, evaluated.
pub(crate) struct Unbound<'call> {
    /// The values the checker bound, by the index of their parameters.
    bound: Vec<(usize, Lowered)>,
    /// The positional values `*args` takes before the tail.
    extras: Vec<Lowered>,
    /// How many positional values come before the tail, the instance of a new object among
    /// them.
    positional: usize,
    /// The index of the first parameter the tail may fill, and the tail.
    tail: Option<(usize, Lowered)>,
    /// The keyword names, in order, for a call with keywords.
    names: Option<Lowered>,
    /// The entries of the `**` dicts.
    merged: Option<Lowered>,
    /// The callee's `**kwargs`, filled as the keywords are evaluated.
    kwargs: Option<Lowered>,
    /// The defaults a call makes, each lowered only where its parameter needs it.
    defaults: Vec<(usize, &'call TypedExpr)>,
    /// The description of the callee the runtime reads.
    description: ValueId,
}

impl FunctionLowerer<'_> {
    /// Returns true when a call's arguments bind when it runs.
    pub(crate) fn binds_when_run(arguments: &[Argument]) -> bool {
        arguments
            .iter()
            .any(|argument| matches!(argument.passed, Passed::Tail | Passed::Entries))
    }

    /// Evaluates the arguments of a call of `function` in source order, after `receivers`
    /// positional values the caller passes itself; `called` is the name with its module of the
    /// class a call makes an instance of.
    pub(crate) fn unbound_arguments<'call>(
        &mut self,
        function: FunctionId,
        arguments: &'call [Argument],
        (receivers, called): (usize, Option<&str>),
        span: Span,
    ) -> Unbound<'call> {
        let parameters = self.program.function(function).signature.parameters.clone();
        let text = self.call_description(function, called);
        let description = self.string(&text, span);
        let star = parameters
            .iter()
            .position(|parameter| parameter.kind == ParameterKind::VarPositional);
        let mut unbound = Unbound {
            bound: Vec::new(),
            extras: Vec::new(),
            positional: receivers,
            tail: None,
            names: None,
            merged: None,
            kwargs: None,
            defaults: Vec::new(),
            description,
        };
        for argument in arguments {
            match &argument.passed {
                Passed::Position => {
                    let value = self.expression(&argument.value);
                    let value = self.kept_for_call(&argument.value, value, span);
                    unbound.positional += 1;
                    if Some(argument.parameter) == star {
                        unbound.extras.push(value);
                    } else {
                        unbound.bound.push((argument.parameter, value));
                    }
                }
                Passed::Items(targets) => {
                    for (target, item) in self.tuple_items(function, argument, targets, span) {
                        unbound.positional += 1;
                        if Some(target) == star {
                            unbound.extras.push(item);
                        } else {
                            unbound.bound.push((target, item));
                        }
                    }
                }
                Passed::Tail => {
                    let list = self.expression(&argument.value);
                    unbound.tail = Some((argument.parameter, list));
                }
                Passed::Name | Passed::Keyword(_) | Passed::Entries => {
                    self.start_keywords(&mut unbound, &parameters, span);
                    self.keyword_argument(&mut unbound, argument, &parameters, span);
                }
                Passed::Default => unbound.defaults.push((argument.parameter, &argument.value)),
            }
        }
        unbound
    }

    /// Makes the dict of keyword names, and the callee's `**kwargs`, before the first keyword.
    fn start_keywords(
        &mut self,
        unbound: &mut Unbound<'_>,
        parameters: &[SignatureParameter],
        span: Span,
    ) {
        if unbound.names.is_some() {
            return;
        }
        unbound.names = Some(self.new_dict(TyirType::STR, TyirType::Int, 0, span));
        if let Some(kwargs) = parameters
            .iter()
            .find(|parameter| parameter.kind == ParameterKind::VarKeyword)
        {
            let (_, value) = self.dict_types(&kwargs.ty);
            unbound.kwargs = Some(self.new_dict(TyirType::STR, value, 0, span));
        }
    }

    /// Evaluates one keyword argument or `**dict`, recording its names.
    fn keyword_argument(
        &mut self,
        unbound: &mut Unbound<'_>,
        argument: &Argument,
        parameters: &[SignatureParameter],
        span: Span,
    ) {
        let names = unbound.names.map(|names| names.value);
        let Some(names) = names else {
            unreachable!("the keywords start before the first one")
        };
        match &argument.passed {
            Passed::Name => {
                let value = self.expression(&argument.value);
                let value = self.kept_for_call(&argument.value, value, span);
                let name = self.string(&parameters[argument.parameter].name, span);
                self.call_runtime(
                    RuntimeFunction::CallName,
                    vec![unbound.description, names, name],
                    span,
                );
                unbound.bound.push((argument.parameter, value));
            }
            Passed::Keyword(keyword) => {
                let value = self.expression(&argument.value);
                let name = self.string(keyword, span);
                self.call_runtime(
                    RuntimeFunction::CallName,
                    vec![unbound.description, names, name],
                    span,
                );
                if let Some(kwargs) = unbound.kwargs {
                    let key = Self::borrowed(name, TyirType::STR);
                    self.set_entry(kwargs.value, key, value, span);
                }
                self.release(value, span);
            }
            Passed::Entries => {
                let source = self.expression(&argument.value);
                let merged = if let Some(merged) = unbound.merged {
                    merged
                } else {
                    let (_, value) = self.dict_types(&argument.value.ty);
                    let merged = self.new_dict(TyirType::STR, value, 0, span);
                    unbound.merged = Some(merged);
                    merged
                };
                let kwargs = match unbound.kwargs {
                    Some(kwargs) => self.bits(kwargs, span),
                    None => self.null_bits(span),
                };
                self.call_runtime(
                    RuntimeFunction::CallMerge,
                    vec![
                        unbound.description,
                        names,
                        merged.value,
                        source.value,
                        kwargs,
                    ],
                    span,
                );
                self.release(source, span);
            }
            _ => unreachable!("a keyword argument"),
        }
    }

    /// Returns each item of the tuple `*` unpacks with the index of the parameter it fills,
    /// held as that parameter holds it, each with a reference of its own.
    pub(crate) fn tuple_items(
        &mut self,
        function: FunctionId,
        argument: &Argument,
        targets: &[usize],
        span: Span,
    ) -> Vec<(usize, Lowered)> {
        let parameters = self.program.function(function).signature.parameters.clone();
        let tuple = self.expression(&argument.value);
        let (_, types) = self.item_types(tuple.ty);
        let mut items = Vec::new();
        for (offset, target) in targets.iter().enumerate() {
            let item = self.borrowed_item(tuple, offset, types[offset], span);
            let parameter = &parameters[*target];
            let item = if parameter.kind == ParameterKind::VarPositional {
                item
            } else {
                let wanted = self.lowered_type(&parameter.ty);
                self.held_as(item, wanted, span)
            };
            items.push((*target, self.owned_copy(item.value, item.ty, span)));
        }
        self.release(tuple, span);
        items
    }

    /// Checks the call of `function` and returns each parameter's value, `receiver` first
    /// when the caller passes one; `dispatch` takes a missing argument's default from the
    /// class of the instance, as a call through the method table does.
    pub(crate) fn bind_when_run(
        &mut self,
        function: FunctionId,
        unbound: Unbound<'_>,
        (receiver, dispatch): (Option<Lowered>, bool),
        span: Span,
    ) -> Vec<(usize, Lowered)> {
        let parameters = self.program.function(function).signature.parameters.clone();
        let count = self.positional_count(&unbound, span);
        let names = match unbound.names {
            Some(names) => self.bits(names, span),
            None => self.null_bits(span),
        };
        self.call_runtime(
            RuntimeFunction::CallCheck,
            vec![unbound.description, count, names],
            span,
        );
        let mut bound = unbound.bound;
        let mut values = Vec::new();
        if let Some(receiver) = receiver {
            values.push((0, receiver));
        }
        let instance = values.first().map(|(_, value)| *value).or_else(|| {
            bound
                .iter()
                .find(|(index, _)| *index == 0)
                .map(|(_, value)| *value)
        });
        let positional = parameters
            .iter()
            .filter(|parameter| {
                matches!(
                    parameter.kind,
                    ParameterKind::PositionalOnly | ParameterKind::Normal
                )
            })
            .count();
        let skip = usize::from(receiver.is_some());
        for (index, parameter) in parameters.iter().enumerate().skip(skip) {
            let value = match parameter.kind {
                ParameterKind::VarPositional => self.star_args(
                    &parameter.ty,
                    &unbound.extras,
                    unbound.tail,
                    positional,
                    span,
                ),
                ParameterKind::VarKeyword => {
                    if let Some(kwargs) = unbound.kwargs {
                        kwargs
                    } else {
                        let (_, value) = self.dict_types(&parameter.ty);
                        self.new_dict(TyirType::STR, value, 0, span)
                    }
                }
                _ => {
                    if let Some(position) = bound.iter().position(|(at, _)| *at == index) {
                        bound.remove(position).1
                    } else {
                        let sources = Sources {
                            tail: unbound.tail,
                            merged: unbound.merged,
                            defaults: &unbound.defaults,
                            default_from: dispatch.then_some(instance).flatten(),
                        };
                        self.parameter_value(function, (index, parameter), &sources, span)
                    }
                }
            };
            values.push((index, value));
        }
        for extra in unbound.extras {
            self.release(extra, span);
        }
        for value in [
            unbound.tail.map(|(_, list)| list),
            unbound.names,
            unbound.merged,
        ]
        .into_iter()
        .flatten()
        {
            self.release(value, span);
        }
        values
    }

    /// Returns the number of positional values: those before the tail and the tail's.
    fn positional_count(&mut self, unbound: &Unbound<'_>, span: Span) -> ValueId {
        let before = self.constant(Constant::Int(unbound.positional.to_string()), span);
        let Some((_, list)) = unbound.tail else {
            return before;
        };
        let length = self.call_runtime(RuntimeFunction::ListLength, vec![list.value], span);
        let count = self.builder.push_value(
            Operation::IntBinary {
                operator: IntOperator::Add,
                left: before,
                right: length.value,
            },
            TyirType::Int,
            span,
        );
        self.release(length, span);
        count
    }

    /// Returns a new `tuple[T, ...]` for `*args`: the positional values before the tail that
    /// the parameters leave, then the tail's after the `positional` parameters.
    fn star_args(
        &mut self,
        ty: &Type,
        extras: &[Lowered],
        tail: Option<(usize, Lowered)>,
        positional: usize,
        span: Span,
    ) -> Lowered {
        let Type::TupleOf(item) = ty else {
            unreachable!("a `*args` parameter is a `tuple[T, ...]`")
        };
        let item = self.lowered_type(item);
        let tuple = self.list_display(&[], item, span);
        for extra in extras {
            let bits = self.bits(*extra, span);
            self.call_runtime(RuntimeFunction::ListAppend, vec![tuple.value, bits], span);
        }
        if let Some((start, list)) = tail {
            let rest = self.list_between(list.value, positional.saturating_sub(start), 0, span);
            self.call_runtime(
                RuntimeFunction::ListExtend,
                vec![tuple.value, rest.value],
                span,
            );
            self.release(rest, span);
        }
        self.call_runtime(RuntimeFunction::TupleSeal, vec![tuple.value], span);
        tuple
    }

    /// Returns the value of the parameter `index` that no static argument fills: the tail's
    /// item when the tail reaches it, else its entry in the `**` dicts, else its default.
    fn parameter_value(
        &mut self,
        function: FunctionId,
        (index, parameter): (usize, &SignatureParameter),
        sources: &Sources<'_, '_>,
        span: Span,
    ) -> Lowered {
        let wanted = self.lowered_type(&parameter.ty);
        let by_position = matches!(
            parameter.kind,
            ParameterKind::PositionalOnly | ParameterKind::Normal
        );
        let Some((start, list)) = sources
            .tail
            .filter(|(start, _)| by_position && index >= *start)
        else {
            return self.keyword_or_default(function, (index, parameter), sources, span);
        };
        let offset = self.constant(Constant::Int((index - start).to_string()), span);
        let length = self.call_runtime(RuntimeFunction::ListLength, vec![list.value], span);
        let inside = self.builder.push_value(
            Operation::Compare {
                kind: ScalarKind::Int,
                comparison: Comparison::Less,
                left: offset,
                right: length.value,
            },
            TyirType::Bool,
            span,
        );
        // Each count is released where it was made, which may be a branch of its own, so no
        // later error path releases it.
        self.release(length, span);
        self.either(
            inside,
            wanted,
            span,
            |this| {
                let bits =
                    this.call_runtime(RuntimeFunction::ListGet, vec![list.value, offset], span);
                let value = this.value_of_bits(bits.value, wanted, span);
                Self::borrowed(value, wanted)
            },
            |this| this.keyword_or_default(function, (index, parameter), sources, span),
        )
    }

    /// Returns the parameter's entry in the `**` dicts, when a keyword reaches it, or its
    /// default.
    fn keyword_or_default(
        &mut self,
        function: FunctionId,
        (index, parameter): (usize, &SignatureParameter),
        sources: &Sources<'_, '_>,
        span: Span,
    ) -> Lowered {
        let wanted = self.lowered_type(&parameter.ty);
        let by_name = matches!(
            parameter.kind,
            ParameterKind::Normal | ParameterKind::KeywordOnly
        );
        let Some(merged) = sources.merged.filter(|_| by_name) else {
            return self.default_of(function, (index, parameter), sources, span);
        };
        let name = self.string(&parameter.name, span);
        let key = self.bits(Self::borrowed(name, TyirType::STR), span);
        let position = self.call_runtime(RuntimeFunction::DictFind, vec![merged.value, key], span);
        let zero = self.constant(Constant::Int("0".to_owned()), span);
        let found = self.builder.push_value(
            Operation::Compare {
                kind: ScalarKind::Int,
                comparison: Comparison::GreaterEqual,
                left: position.value,
                right: zero,
            },
            TyirType::Bool,
            span,
        );
        let value = self.either(
            found,
            wanted,
            span,
            |this| {
                let bits = this.call_runtime(
                    RuntimeFunction::DictValue,
                    vec![merged.value, position.value],
                    span,
                );
                let value = this.value_of_bits(bits.value, wanted, span);
                Self::borrowed(value, wanted)
            },
            |this| this.default_of(function, (index, parameter), sources, span),
        );
        self.release(position, span);
        value
    }

    /// Returns the parameter's default: the one a call makes, the one of the instance's class
    /// for a call through the method table, or its own. A parameter without one never gets
    /// here, since the check raised first.
    fn default_of(
        &mut self,
        function: FunctionId,
        (index, parameter): (usize, &SignatureParameter),
        sources: &Sources<'_, '_>,
        span: Span,
    ) -> Lowered {
        let wanted = self.lowered_type(&parameter.ty);
        if let Some((_, made)) = sources.defaults.iter().find(|(at, _)| *at == index) {
            return self.expression(made);
        }
        let value = match (parameter.default.as_ref(), sources.default_from) {
            (Some(_), Some(instance)) => self.dispatched_default(function, index, instance, span),
            (Some(default), None) => self.default_value(parameter, Some(default), span),
            (None, _) => self.zero_value(wanted, span),
        };
        // A default the instance's class chooses is a value of its own, which the branch's
        // join takes over.
        let owned = self.temporaries.contains(&value);
        Lowered {
            value,
            ty: wanted,
            owned,
        }
    }

    /// Returns 0 as raw bits, for no dict.
    pub(crate) fn null_bits(&mut self, span: Span) -> ValueId {
        let none = self.constant(Constant::Unit, span);
        self.builder
            .push_value(Operation::ToBits(none), TyirType::Bits, span)
    }

    /// Returns the description the runtime reads: the callee's qualified name, its name with
    /// its module, or that of the class `called`, then a line for each parameter.
    fn call_description(&self, function: FunctionId, called: Option<&str>) -> String {
        let typed = self.program.function(function);
        let qualified = &typed.qualified_name;
        let full = called.map_or_else(|| format!("{}.{qualified}", typed.module), str::to_owned);
        let mut text = format!("{qualified}\n{full}");
        for parameter in &typed.signature.parameters {
            text.push('\n');
            text.push(match parameter.kind {
                ParameterKind::PositionalOnly => 'P',
                ParameterKind::Normal => 'N',
                ParameterKind::VarPositional => 'A',
                ParameterKind::KeywordOnly => 'K',
                ParameterKind::VarKeyword => 'W',
            });
            let defaulted = parameter.default.is_some() || parameter.default_call.is_some();
            text.push(if defaulted { '=' } else { '-' });
            text.push_str(&parameter.name);
        }
        text
    }
}

/// Where a parameter that no static argument fills may take its value from.
struct Sources<'unbound, 'call> {
    tail: Option<(usize, Lowered)>,
    merged: Option<Lowered>,
    defaults: &'unbound [(usize, &'call TypedExpr)],
    /// The instance whose class gives a default, for a call through the method table.
    default_from: Option<Lowered>,
}
