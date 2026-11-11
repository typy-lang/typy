//! Checks what a `for` loop goes over: a sequence, or `enumerate`, `zip` or `reversed` of
//! sequences (CST-03, LIB-02).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Expr, Keyword};

use super::BodyChecker;
use crate::builtin::Builtin;
use crate::typed::{LoopSource, TypedConstant, TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Returns the builtin a call names, with its arguments, when no local or program function
    /// of that name hides it, as the call would resolve it.
    pub(super) fn called_builtin<'call>(
        &self,
        expression: &'call Expr,
    ) -> Option<(Builtin, &'call [Expr], &'call [Keyword])> {
        let Expr::Call {
            function,
            arguments,
            keywords,
            ..
        } = expression
        else {
            return None;
        };
        let Expr::Name { id, .. } = function.as_ref() else {
            return None;
        };
        if self.locals.contains_key(id) || self.program.function_ids.contains_key(id) {
            return None;
        }
        Some((Builtin::named(id)?, arguments, keywords))
    }

    /// Checks the source of a `for` loop, or an argument of `enumerate` or `zip`, and returns
    /// it with the type of what each step gives.
    pub(super) fn loop_source(&mut self, iter: &Expr, user: &str) -> (LoopSource, Type) {
        let call = self.called_builtin(iter).filter(|(builtin, ..)| {
            matches!(
                builtin,
                Builtin::Enumerate | Builtin::Zip | Builtin::Reversed | Builtin::Range
            )
        });
        let Some((builtin, arguments, keywords)) = call else {
            let sequence = self.sequence(iter);
            if sequence.ty.class_id().is_some() {
                return self.protocol_source(sequence, user);
            }
            let item = self.iterated_item(&sequence, user);
            return (LoopSource::Items(sequence), item);
        };
        let span = iter.span();
        let source = match builtin {
            Builtin::Enumerate => self.enumerate(arguments, keywords, span),
            Builtin::Zip => self.zip(arguments, keywords),
            Builtin::Range => self.range(arguments, keywords, false, span),
            _ => self.reversed(arguments, keywords, span),
        };
        let ty = source.item_type();
        (source, ty)
    }

    /// Checks iterating over an instance: its class's `__iter__` gives an instance of a class
    /// with `__next__`, whose result is each value (CST-08).
    fn protocol_source(&mut self, iterable: TypedExpr, user: &str) -> (LoopSource, Type) {
        let class = iterable.ty.class_id().unwrap_or(crate::typed::ClassId(0));
        let not_iterable = |reason: String| {
            Diagnostic::new(
                DiagnosticCode::error(411),
                iterable.span,
                format!("{user} over {} {reason}", iterable.ty.with_article()),
            )
            .with_rule(RuleId::CST_08)
        };
        let Some(iter) = self.program.method(class, "__iter__") else {
            self.diagnostics
                .push(not_iterable("needs `__iter__` in its class".to_owned()));
            return (LoopSource::Items(iterable), Type::Error);
        };
        let iterator = self.program.signatures[iter.0 as usize].returns.clone();
        let next = iterator
            .class_id()
            .and_then(|class| self.program.method(class, "__next__"));
        let Some(next) = next else {
            if !iterator.has_error() {
                self.diagnostics.push(not_iterable(format!(
                    "needs `__iter__` to return an instance of a class with `__next__`, and it \
                     returns {}",
                    iterator.with_article()
                )));
            }
            return (LoopSource::Items(iterable), Type::Error);
        };
        let item = self.program.signatures[next.0 as usize].returns.clone();
        let source = LoopSource::Protocol {
            iterable: Box::new(iterable),
            iter,
            next,
            iterator,
            item: item.clone(),
        };
        (source, item)
    }

    fn enumerate(&mut self, arguments: &[Expr], keywords: &[Keyword], span: Span) -> LoopSource {
        let keyword = self.only_keyword(Builtin::Enumerate, "start", keywords);
        let given = arguments.len() + usize::from(keyword.is_some());
        if arguments.is_empty() || given > 2 {
            return self.source_arity(Builtin::Enumerate, "1 or 2", given, arguments, span);
        }
        let (source, _) = self.loop_source(&arguments[0], "`enumerate`");
        let start = match arguments.get(1).or(keyword) {
            Some(start) => {
                let start = self.expression(start);
                self.expect(&start, Type::Int, "the start of `enumerate`");
                start
            }
            None => TypedExpr {
                kind: TypedExprKind::Constant(TypedConstant::Int("0".to_owned())),
                ty: Type::Int,
                span,
            },
        };
        LoopSource::Enumerate {
            source: Box::new(source),
            start,
        }
    }

    fn zip(&mut self, arguments: &[Expr], keywords: &[Keyword]) -> LoopSource {
        let strict = self.only_keyword(Builtin::Zip, "strict", keywords);
        let sources = arguments
            .iter()
            .map(|argument| self.loop_source(argument, "`zip`").0)
            .collect();
        let strict = strict.map(|strict| {
            let strict = self.expression(strict);
            self.expect(&strict, Type::Bool, "`strict`");
            strict
        });
        LoopSource::Zip { sources, strict }
    }

    fn reversed(&mut self, arguments: &[Expr], keywords: &[Keyword], span: Span) -> LoopSource {
        self.only_keyword(Builtin::Reversed, "", keywords);
        if arguments.len() != 1 {
            return self.source_arity(Builtin::Reversed, "1", arguments.len(), arguments, span);
        }
        if let Some((Builtin::Range, range_arguments, range_keywords)) =
            self.called_builtin(&arguments[0])
        {
            return self.range(range_arguments, range_keywords, true, span);
        }
        let sequence = self.sequence(&arguments[0]);
        if let Type::Dict(..) | Type::DictView(..) = sequence.ty {
            return self.reversed_dict(&sequence, span);
        }
        if let Type::Set(_) = sequence.ty {
            self.diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(411),
                    sequence.span,
                    format!(
                        "`reversed` needs a sequence, and {} has no order to reverse",
                        sequence.ty.with_article()
                    ),
                )
                .with_rule(RuleId::LIB_02),
            );
            return LoopSource::Reversed(Self::typed(
                TypedExprKind::Constant(TypedConstant::None),
                Type::Error,
                span,
            ));
        }
        self.iterated_item(&sequence, "`reversed`");
        LoopSource::Reversed(sequence)
    }

    /// Reports `reversed` of a dict or a view, which CPython reads from the last entry; it is
    /// planned with the other forms of DAT-16.
    fn reversed_dict(&mut self, sequence: &TypedExpr, span: Span) -> LoopSource {
        self.diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(201),
                sequence.span,
                "`reversed` of a dict or a dict view is planned for TyPy 0.3 (DAT-16)",
            )
            .with_rule(RuleId::DAT_16),
        );
        LoopSource::Reversed(Self::typed(
            TypedExprKind::Constant(TypedConstant::None),
            Type::Error,
            span,
        ))
    }

    /// Checks `range(stop)`, `range(start, stop)` or `range(start, stop, step)`, each an `int`,
    /// or `reversed` of one.
    fn range(
        &mut self,
        arguments: &[Expr],
        keywords: &[Keyword],
        reversed: bool,
        span: Span,
    ) -> LoopSource {
        if !keywords.is_empty() {
            // `range` takes no keyword argument; each is reported, and the call has no value.
            self.only_keyword(Builtin::Range, "", keywords);
            for argument in arguments {
                self.expression(argument);
            }
            return LoopSource::Items(Self::typed(
                TypedExprKind::Constant(TypedConstant::None),
                Type::Error,
                span,
            ));
        }
        if !(1..=3).contains(&arguments.len()) {
            return self.source_arity(Builtin::Range, "1 to 3", arguments.len(), arguments, span);
        }
        let values: Vec<TypedExpr> = arguments
            .iter()
            .map(|argument| {
                let value = self.expression(argument);
                self.expect(&value, Type::Int, "an argument of `range`");
                value
            })
            .collect();
        let constant = |digits: &str| TypedExpr {
            kind: TypedExprKind::Constant(TypedConstant::Int(digits.to_owned())),
            ty: Type::Int,
            span,
        };
        let mut given = values.into_iter();
        let (first, end, increment) = match (given.next(), given.next(), given.next()) {
            (Some(end), None, _) => (constant("0"), end, constant("1")),
            (Some(first), Some(end), None) => (first, end, constant("1")),
            (Some(first), Some(end), Some(increment)) => (first, end, increment),
            (None, ..) => unreachable!("`range` has one to three arguments here"),
        };
        if [&first, &end, &increment]
            .iter()
            .any(|value| value.ty.has_error())
        {
            return LoopSource::Items(Self::typed(
                TypedExprKind::Constant(TypedConstant::None),
                Type::Error,
                span,
            ));
        }
        LoopSource::Range {
            start: first,
            stop: end,
            step: increment,
            reversed,
        }
    }

    /// Checks a sequence a loop reads. A fixed tuple whose items have one type is read as a
    /// `tuple[T, ...]` (DAT-14).
    pub(super) fn sequence(&mut self, expression: &Expr) -> TypedExpr {
        let value = self.expression(expression);
        let Type::Tuple(items) = &value.ty else {
            return value;
        };
        match items.split_first() {
            Some((first, rest)) if rest.iter().all(|item| item == first) => {
                let wanted = Type::TupleOf(Box::new(first.clone()));
                Self::converted(value, Some(&wanted))
            }
            _ => value,
        }
    }

    /// Returns the value of the one keyword argument `accepted`, after reporting any other.
    fn only_keyword<'call>(
        &mut self,
        builtin: Builtin,
        accepted: &str,
        keywords: &'call [Keyword],
    ) -> Option<&'call Expr> {
        let mut value = None;
        for keyword in keywords {
            let name = keyword.name.as_ref().map(|name| name.name.as_str());
            if name == Some(accepted) && value.is_none() {
                value = Some(&keyword.value);
                continue;
            }
            let message = match name {
                Some(name) if name == accepted => format!("`{name}` is given twice"),
                Some(name) => format!("`{}` has no parameter named `{name}`", builtin.name()),
                None => format!("`{}` takes no `**` arguments", builtin.name()),
            };
            self.diagnostics.push(
                Diagnostic::new(DiagnosticCode::error(404), keyword.span, message)
                    .with_rule(RuleId::LIB_02),
            );
        }
        value
    }

    /// Reports a call with the wrong number of arguments, and checks them for their own errors.
    fn source_arity(
        &mut self,
        builtin: Builtin,
        expected: &str,
        given: usize,
        arguments: &[Expr],
        span: Span,
    ) -> LoopSource {
        for argument in arguments {
            self.expression(argument);
        }
        self.diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(404),
                span,
                format!(
                    "`{}` takes {expected} arguments, and {given} {} given",
                    builtin.name(),
                    if given == 1 { "is" } else { "are" }
                ),
            )
            .with_rule(RuleId::LIB_02),
        );
        let error = TypedExpr {
            kind: TypedExprKind::Constant(TypedConstant::None),
            ty: Type::Error,
            span,
        };
        LoopSource::Items(error)
    }
}
