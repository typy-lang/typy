//! Checks `list(...)`, and the iterables the builtins of one iterable take (LIB-02).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Expr, Keyword};

use super::BodyChecker;
use crate::builtin::Builtin;
use crate::method::ParameterType;
use crate::typed::{Argument, LoopSource, TypedConstant, TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Checks `list()` or `list(source)`. With no argument the list has the expected type, as an
    /// empty display does (TYP-05); with one it holds every value the source gives: the items
    /// of a list, the characters of a `str`, or what `enumerate`, `zip` or `reversed` give.
    pub(super) fn call_list(
        &mut self,
        arguments: &[Expr],
        keywords: &[Keyword],
        expected: Option<&Type>,
        span: Span,
    ) -> TypedExpr {
        if let Some(keyword) = keywords.first() {
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    keyword.span,
                    "`list` takes no keyword arguments",
                )
                .with_rule(RuleId::LIB_02),
                span,
            );
        }
        match arguments {
            [] => self.list_display(&[], expected, span),
            [source] => {
                let (source, item) = self.loop_source(source, "`list`");
                if item.has_error() {
                    return Self::typed(
                        TypedExprKind::Constant(TypedConstant::None),
                        Type::Error,
                        span,
                    );
                }
                Self::typed(
                    TypedExprKind::ListOf(Box::new(source)),
                    Type::List(Box::new(item)),
                    span,
                )
            }
            _ => self.error(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    span,
                    format!(
                        "`list` takes at most 1 argument, and {} are given",
                        arguments.len()
                    ),
                )
                .with_rule(RuleId::LIB_02),
                span,
            ),
        }
    }

    /// Checks `tuple()` or `tuple(source)`. With no argument it is the empty tuple, of the
    /// expected type when that is a `tuple[T, ...]`; with one it is a `tuple[T, ...]` of every
    /// value the source gives (DAT-14).
    pub(super) fn call_tuple(
        &mut self,
        arguments: &[Expr],
        keywords: &[Keyword],
        expected: Option<&Type>,
        span: Span,
    ) -> TypedExpr {
        if let Some(keyword) = keywords.first() {
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    keyword.span,
                    "`tuple` takes no keyword arguments",
                )
                .with_rule(RuleId::LIB_02),
                span,
            );
        }
        let list = match (arguments, expected) {
            ([], Some(Type::TupleOf(item))) => {
                let ty = Type::List(item.clone());
                self.list_display(&[], Some(&ty), span)
            }
            ([], _) => {
                return Self::typed(
                    TypedExprKind::Tuple(Vec::new()),
                    Type::Tuple(Vec::new()),
                    span,
                );
            }
            ([_], _) => self.call_list(arguments, keywords, None, span),
            _ => {
                return self.error(
                    Diagnostic::new(
                        DiagnosticCode::error(404),
                        span,
                        format!(
                            "`tuple` takes at most 1 argument, and {} are given",
                            arguments.len()
                        ),
                    )
                    .with_rule(RuleId::LIB_02),
                    span,
                );
            }
        };
        let Some(item) = list.ty.sequence_item().cloned() else {
            return list;
        };
        Self::typed(
            TypedExprKind::TupleOf(Box::new(list)),
            Type::TupleOf(Box::new(item)),
            span,
        )
    }

    /// Checks `bytes()`, `bytes(count)`, `bytes(data)` or `bytes(values)` for an iterable of
    /// `int`. A `str` needs an encoding, which `str.encode` gives.
    pub(super) fn call_bytes(
        &mut self,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        if arguments.len() > 1 || !keywords.is_empty() {
            return self.bytes_of_text(arguments, keywords, span);
        }
        let argument = match arguments {
            [] => {
                return Self::typed(
                    TypedExprKind::Constant(TypedConstant::Bytes(Vec::new())),
                    Type::Bytes,
                    span,
                );
            }
            [argument, ..] => argument,
        };
        let Some(value) = self.bytes_argument(argument) else {
            return Self::typed(
                TypedExprKind::Constant(TypedConstant::None),
                Type::Error,
                span,
            );
        };
        Self::typed(
            TypedExprKind::Builtin {
                builtin: Builtin::Bytes,
                arguments: vec![value],
            },
            Type::Bytes,
            span,
        )
    }

    /// Checks `bytes(text, encoding, errors)`, which is `text.encode(encoding, errors)`; the
    /// encoding and the errors may be given by keyword, and the encoding must be given.
    fn bytes_of_text(&mut self, arguments: &[Expr], keywords: &[Keyword], span: Span) -> TypedExpr {
        let mut given: Vec<Option<&Expr>> = vec![arguments.get(1), arguments.get(2)];
        for keyword in keywords {
            let slot = match keyword.name.as_ref().map(|name| name.name.as_str()) {
                Some("encoding") => 0,
                Some("errors") => 1,
                _ => {
                    return self.error(
                        Diagnostic::new(
                            DiagnosticCode::error(404),
                            keyword.span,
                            "`bytes` takes only the keyword arguments `encoding` and `errors`",
                        )
                        .with_rule(RuleId::LIB_02),
                        span,
                    );
                }
            };
            given[slot] = Some(&keyword.value);
        }
        let (Some(text), Some(encoding), true) =
            (arguments.first(), given[0], arguments.len() <= 3)
        else {
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    span,
                    "`bytes` of a `str` takes the text, its encoding and the errors",
                )
                .with_rule(RuleId::LIB_02),
                span,
            );
        };
        let text = self.expression(text);
        self.expect(&text, Type::Str, "the text `bytes` encodes");
        let mut bound = vec![Argument {
            parameter: 0,
            value: self.codec_argument(&ParameterType::Codec, encoding, "the encoding of `bytes`"),
        }];
        if let Some(errors) = given[1] {
            let kind = ParameterType::Handler { decoding: false };
            bound.push(Argument {
                parameter: 1,
                value: self.codec_argument(&kind, errors, "the errors of `bytes`"),
            });
        }
        Self::typed(
            TypedExprKind::Method {
                receiver: Box::new(text),
                method: crate::method::Method::StrEncode,
                arguments: bound,
            },
            Type::Bytes,
            span,
        )
    }

    /// Checks the one argument of `bytes`: a count, a `bytes`, or `int` values from any
    /// iterable; returns `None` after reporting any other.
    fn bytes_argument(&mut self, argument: &Expr) -> Option<TypedExpr> {
        let source = self.called_builtin(argument).is_some_and(|(builtin, ..)| {
            matches!(
                builtin,
                Builtin::Enumerate | Builtin::Zip | Builtin::Reversed
            )
        });
        let value = if source {
            self.iterable_argument(argument, Builtin::Bytes)
        } else {
            let value = self.expression(argument);
            let ints = Type::TupleOf(Box::new(Type::Int));
            Self::converted(value, Some(&ints))
        };
        match &value.ty {
            Type::Int | Type::Bytes | Type::Error => Some(value),
            Type::Str => {
                self.diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::error(404),
                        value.span,
                        "`bytes` of a `str` needs an encoding, such as `text.encode(\"utf-8\")`",
                    )
                    .with_rule(RuleId::DAT_06),
                );
                None
            }
            Type::List(_) | Type::TupleOf(_) => {
                let values = value;
                if let Some(item) = values.ty.sequence_item()
                    && !matches!(item, Type::Int | Type::Error)
                {
                    let item = item.clone();
                    self.diagnostics.push(
                        Diagnostic::new(
                            DiagnosticCode::error(404),
                            values.span,
                            format!(
                                "`bytes` takes `int` values from 0 to 255, and these are {item}"
                            ),
                        )
                        .with_rule(RuleId::DAT_06),
                    );
                    return None;
                }
                Some(values)
            }
            other => {
                let other = other.clone();
                self.diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::error(411),
                        value.span,
                        format!(
                            "`bytes` takes a count or `int` values, and this is {}",
                            other.with_article()
                        ),
                    )
                    .with_rule(RuleId::DAT_06),
                );
                None
            }
        }
    }

    /// Checks the iterable argument of `builtin` and returns it as a list: a list as it is, and
    /// any other source, a `str` or `enumerate`, `zip` or `reversed`, as a new list of its
    /// values, which the builtin then reads.
    pub(super) fn iterable_argument(&mut self, argument: &Expr, builtin: Builtin) -> TypedExpr {
        let span = argument.span();
        let (source, item) = self.loop_source(argument, &format!("`{}`", builtin.name()));
        if item.has_error() {
            return Self::typed(
                TypedExprKind::Constant(TypedConstant::None),
                Type::Error,
                span,
            );
        }
        match source {
            LoopSource::Items(sequence) if sequence.ty.sequence_item().is_some() => sequence,
            source => Self::typed(
                TypedExprKind::ListOf(Box::new(source)),
                Type::List(Box::new(item)),
                span,
            ),
        }
    }
}
