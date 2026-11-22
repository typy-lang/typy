//! Calls of a built-in function with `*` of a value only the run knows (EXP-22, ADR-0083).
//! The positional arguments are evaluated first, in order, into hidden variables, as CPython
//! evaluates them; the call is then checked once for each number of arguments the built-in
//! takes, and the number of values `*` gave picks that arm when the call runs. Any other
//! number raises CPython 3.15's `TypeError` for the built-in. The keyword arguments are
//! evaluated in the arm, where the built-in's parameter gives them their type, so a number
//! the built-in does not take raises before they are. The arms give one type; a built-in
//! whose result type depends on the number is refused.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{BinaryOperator, CompareOperator, Expr, Keyword};

use super::BodyChecker;
use super::flow::Fact;
use crate::builtin::Builtin;
use crate::exception::{BuiltinException, ExceptionClass};
use crate::typed::{SymbolId, TypedConstant, TypedExpr, TypedExprKind};
use crate::types::Type;

/// How many positional arguments a built-in takes, and CPython 3.15's `TypeError` for any
/// other number; `{}` in a message stands for the number given.
struct Arity {
    minimum: usize,
    maximum: usize,
    /// A number between the two that the built-in does not take.
    gap: Option<usize>,
    /// The message for too few arguments, which counts those given by position.
    few: &'static str,
    /// The message for too many.
    many: &'static str,
}

impl Arity {
    const fn new(minimum: usize, maximum: usize, few: &'static str, many: &'static str) -> Self {
        Self {
            minimum,
            maximum,
            gap: None,
            few,
            many,
        }
    }
}

/// Returns the arity of `builtin` as a call with `*` binds it, or `None` for one that takes
/// any number of arguments.
fn arity(builtin: Builtin) -> Option<Arity> {
    use Builtin as B;
    Some(match builtin {
        B::Abs => one("abs() takes exactly one argument ({} given)"),
        B::All => one("all() takes exactly one argument ({} given)"),
        B::Any => one("any() takes exactly one argument ({} given)"),
        B::Callable => one("callable() takes exactly one argument ({} given)"),
        B::Chr => one("chr() takes exactly one argument ({} given)"),
        B::Len => one("len() takes exactly one argument ({} given)"),
        B::Ord => one("ord() takes exactly one argument ({} given)"),
        B::Repr => one("repr() takes exactly one argument ({} given)"),
        B::Bool => Arity::new(0, 1, "", "bool expected at most 1 argument, got {}"),
        B::Dict => Arity::new(0, 1, "", "dict expected at most 1 argument, got {}"),
        B::Float => Arity::new(0, 1, "", "float expected at most 1 argument, got {}"),
        B::List => Arity::new(0, 1, "", "list expected at most 1 argument, got {}"),
        B::Set => Arity::new(0, 1, "", "set expected at most 1 argument, got {}"),
        B::Tuple => Arity::new(0, 1, "", "tuple expected at most 1 argument, got {}"),
        B::Input => Arity::new(0, 1, "", "input expected at most 1 argument, got {}"),
        B::Int => Arity::new(0, 2, "", "int expected at most 2 arguments, got {}"),
        B::Str => Arity::new(0, 3, "", "str expected at most 3 arguments, got {}"),
        B::Bytes => Arity::new(0, 3, "", "bytes() takes at most 3 arguments ({} given)"),
        B::Enumerate => Arity::new(
            1,
            2,
            "enumerate() missing required argument 'iterable'",
            "enumerate() takes at most 2 arguments ({} given)",
        ),
        B::IsInstance => two("isinstance expected 2 arguments, got {}"),
        B::IsSubclass => two("issubclass expected 2 arguments, got {}"),
        B::Iter => Arity::new(
            1,
            2,
            "iter expected at least 1 argument, got {}",
            "iter expected at most 2 arguments, got {}",
        ),
        B::Next => Arity::new(
            1,
            2,
            "next expected at least 1 argument, got {}",
            "next expected at most 2 arguments, got {}",
        ),
        B::Range => Arity::new(
            1,
            3,
            "range expected at least 1 argument, got {}",
            "range expected at most 3 arguments, got {}",
        ),
        B::Slice => Arity::new(
            1,
            3,
            "slice expected at least 1 argument, got {}",
            "slice expected at most 3 arguments, got {}",
        ),
        B::Reversed => Arity::new(
            1,
            1,
            "reversed expected 1 argument, got {}",
            "reversed expected 1 argument, got {}",
        ),
        B::Sorted => Arity::new(
            1,
            1,
            "sorted expected 1 argument, got {}",
            "sorted expected 1 argument, got {}",
        ),
        B::Round => Arity::new(
            1,
            2,
            "round() missing required argument 'number' (pos 1)",
            "round() takes at most 2 arguments ({} given)",
        ),
        B::Sum => Arity::new(
            1,
            2,
            "sum() takes at least 1 positional argument ({} given)",
            "sum() takes at most 2 arguments ({} given)",
        ),
        B::Open => Arity::new(
            1,
            8,
            "open() missing required argument 'file' (pos 1)",
            "open() takes at most 8 arguments ({} given)",
        ),
        B::Type => Arity {
            gap: Some(2),
            ..Arity::new(
                1,
                3,
                "type() takes 1 or 3 arguments",
                "type() takes 1 or 3 arguments",
            )
        },
        B::Max | B::Min | B::Zip | B::Print => return None,
    })
}

/// Returns the arity of a built-in that takes exactly one argument.
const fn one(message: &'static str) -> Arity {
    Arity::new(1, 1, message, message)
}

/// Returns the arity of a built-in that takes exactly two arguments.
const fn two(message: &'static str) -> Arity {
    Arity::new(2, 2, message, message)
}

impl BodyChecker<'_> {
    /// Checks `builtin(arguments, keywords)`, a call of a built-in function with `*` of a
    /// value only the run knows; `None` for a built-in that takes any number of arguments.
    pub(super) fn builtin_unpacked(
        &mut self,
        (function, builtin): (&Expr, Builtin),
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> Option<TypedExpr> {
        if keywords.iter().any(|keyword| keyword.name.is_none()) {
            return None;
        }
        if matches!(builtin, Builtin::Max | Builtin::Min) {
            return Some(self.extreme_unpacked((function, builtin), arguments, keywords, span));
        }
        let arity = arity(builtin)?;
        // The call takes what it unpacks, so its own errors stand (ADR-0073).
        self.unpacking_calls.push(span);
        let first = arguments
            .iter()
            .position(|argument| matches!(argument, Expr::Starred { .. }))
            .unwrap_or(arguments.len());
        let mut prelude = Vec::new();
        let mut written = Vec::new();
        for argument in &arguments[..first] {
            let (store, name) = self.held_argument("argument", argument);
            prelude.push(store);
            written.push(name);
        }
        let (list, item) = self.one_type_values(
            &arguments[first..],
            "the positional arguments from the first `*` on",
            span,
        );
        let tail = self.hidden_variable("unpacked", list.ty.clone(), span);
        prelude.push(Self::stored(tail, list));
        let counts: Vec<usize> = (arity.minimum.max(first)..=arity.maximum)
            .filter(|count| Some(*count) != arity.gap)
            .collect();
        if counts.is_empty() {
            return Some(self.too_many_before_star(builtin, &arity, span));
        }
        let items: Vec<SymbolId> = (0..arity.maximum - first)
            .map(|_| self.call_variable("item", item.clone(), span))
            .collect();
        let mut arms = Vec::new();
        for count in counts {
            let taken = count - first;
            let mut call_arguments = written.clone();
            call_arguments.extend(
                items[..taken]
                    .iter()
                    .map(|symbol| self.hidden_name(*symbol, span)),
            );
            let reported = self.diagnostics.len();
            let call = self.call_expression(function, &call_arguments, keywords, span);
            // An arm's error says how many values `*` gave it.
            let shown = format!(
                "when `*` gives {taken} value{}",
                if taken == 1 { "" } else { "s" }
            );
            let refused: Vec<Diagnostic> =
                self.diagnostics.iter().skip(reported).cloned().collect();
            self.diagnostics.truncate(reported);
            for diagnostic in refused {
                self.diagnostics
                    .push(diagnostic.with_label(span, shown.clone()));
            }
            let stores = (0..taken)
                .map(|index| Self::stored(items[index], Self::tail_item(tail, &item, index, span)))
                .collect();
            arms.push((taken, Self::in_order(stores, call)));
        }
        let ty = self.arms_type(builtin, &arms, span);
        let refusal = self.wrong_count(&arity, first, (tail, &ty), span);
        let switch = arms
            .into_iter()
            .rev()
            .fold(refusal, |orelse, (taken, call)| {
                let test = self.length_is(tail, CompareOperator::Eq, taken, span);
                Self::typed(
                    TypedExprKind::IfExp {
                        test: Box::new(test),
                        body: Box::new(Self::converted(call, Some(&ty))),
                        orelse: Box::new(orelse),
                    },
                    ty.clone(),
                    span,
                )
            });
        Some(Self::in_order(prelude, switch))
    }
}

impl BodyChecker<'_> {
    /// Returns the store of `argument`, evaluated, in a new hidden variable named for `kind`,
    /// and the name that reads it.
    fn held_argument(&mut self, kind: &str, argument: &Expr) -> (TypedExpr, Expr) {
        let value = self.expression(argument);
        let span = argument.span();
        let symbol = self.call_variable(kind, value.ty.clone(), span);
        (Self::stored(symbol, value), self.hidden_name(symbol, span))
    }

    /// Returns a new hidden variable of type `ty`, which the call stores before an arm reads
    /// it by its name.
    fn call_variable(&mut self, kind: &str, ty: Type, span: Span) -> SymbolId {
        let symbol = self.hidden_variable(kind, ty, span);
        self.assigned.iter_mut().for_each(|assigned| {
            assigned.insert(Fact::Assigned(symbol));
        });
        symbol
    }

    /// Returns a name that reads the hidden variable `symbol`.
    fn hidden_name(&self, symbol: SymbolId, span: Span) -> Expr {
        Expr::Name {
            id: self.program.symbols[symbol.0 as usize].name.clone(),
            span,
        }
    }

    /// Returns item `index` of the list the hidden variable `tail` holds, of type `item`.
    fn tail_item(tail: SymbolId, item: &Type, index: usize, span: Span) -> TypedExpr {
        let list = Self::typed(
            TypedExprKind::Local(tail),
            Type::List(Box::new(item.clone())),
            span,
        );
        Self::typed(
            TypedExprKind::ListItem {
                list: Box::new(list),
                index: Box::new(Self::number(index, span)),
            },
            item.clone(),
            span,
        )
    }

    /// Returns the `int` constant `value`.
    fn number(value: usize, span: Span) -> TypedExpr {
        Self::typed(
            TypedExprKind::Constant(TypedConstant::Int(value.to_string())),
            Type::Int,
            span,
        )
    }

    /// Returns `len(tail) operator count` of the list the hidden variable `tail` holds.
    fn length_is(
        &self,
        tail: SymbolId,
        operator: CompareOperator,
        count: usize,
        span: Span,
    ) -> TypedExpr {
        Self::typed(
            TypedExprKind::Compare {
                left: Box::new(self.tail_length(tail, span)),
                operators: vec![operator],
                comparators: vec![Self::number(count, span)],
            },
            Type::Bool,
            span,
        )
    }

    /// Returns `len(tail)` of the list the hidden variable `tail` holds.
    fn tail_length(&self, tail: SymbolId, span: Span) -> TypedExpr {
        let ty = self.program.symbols[tail.0 as usize].ty.clone();
        Self::typed(
            TypedExprKind::Builtin {
                builtin: Builtin::Len,
                arguments: vec![Self::typed(TypedExprKind::Local(tail), ty, span)],
            },
            Type::Int,
            span,
        )
    }

    /// Returns the type the arms of a call of `builtin` give, which must be one: E0202 when
    /// it depends on the number of values `*` gives.
    fn arms_type(&mut self, builtin: Builtin, arms: &[(usize, TypedExpr)], span: Span) -> Type {
        let mut types: Vec<&Type> = Vec::new();
        for (_, arm) in arms {
            if arm.ty.has_error() {
                return Type::Error;
            }
            if !types.contains(&&arm.ty) {
                types.push(&arm.ty);
            }
        }
        if let [ty] = types.as_slice() {
            return (*ty).clone();
        }
        let shown: Vec<String> = types.iter().map(ToString::to_string).collect();
        let message = format!(
            "`{}` gives {} as the number of values `*` gives decides, so its result has no one \
             type",
            builtin.name(),
            shown.join(" or ")
        );
        self.diagnostics.push(
            Diagnostic::new(DiagnosticCode::error(202), span, message)
                .with_rule(RuleId::EXP_22)
                .with_help("call it with as many values as it needs, one by one"),
        );
        Type::Error
    }

    /// Returns the `TypeError` CPython 3.15 raises for a call of a built-in of `arity` given
    /// a number of values it does not take, `first` before the list the hidden variable
    /// `tail` holds, in place of a value of type `ty`.
    fn wrong_count(
        &mut self,
        arity: &Arity,
        first: usize,
        (tail, ty): (SymbolId, &Type),
        span: Span,
    ) -> TypedExpr {
        let given = |this: &Self| {
            Self::typed(
                TypedExprKind::Binary {
                    left: Box::new(this.tail_length(tail, span)),
                    operator: BinaryOperator::Add,
                    right: Box::new(Self::number(first, span)),
                },
                Type::Int,
                span,
            )
        };
        let many = self.raised_type_error(arity.many, given(self), ty, span);
        if arity.minimum <= first {
            return many;
        }
        let few = self.raised_type_error(arity.few, given(self), ty, span);
        let test = self.length_is(tail, CompareOperator::Lt, arity.minimum - first, span);
        Self::typed(
            TypedExprKind::IfExp {
                test: Box::new(test),
                body: Box::new(few),
                orelse: Box::new(many),
            },
            ty.clone(),
            span,
        )
    }

    /// Checks `max(arguments)` or `min(arguments)` with `*` of a value only the run knows
    /// (ADR-0083). Every positional value goes into one list of one type: none raises
    /// CPython's `TypeError`, one is iterated, which only a `str` gives the list's type, and
    /// more are compared, as the list itself is.
    fn extreme_unpacked(
        &mut self,
        (function, builtin): (&Expr, Builtin),
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        // The call takes what it unpacks, so its own errors stand (ADR-0073).
        self.unpacking_calls.push(span);
        let name = builtin.name();
        let what = format!("the positional arguments of `{name}` called with `*`");
        let (list, item) = self.one_type_values(arguments, &what, span);
        let tail = self.call_variable("unpacked", list.ty.clone(), span);
        let prelude = vec![Self::stored(tail, list)];
        let defaulted = keywords.iter().any(|keyword| {
            keyword
                .name
                .as_ref()
                .is_some_and(|name| name.name == "default")
        });
        let many = if defaulted {
            let message =
                format!("Cannot specify a default for {name}() with multiple positional arguments");
            self.raised_type_error(&message, Self::number(0, span), &Type::Error, span)
        } else {
            let listed = self.hidden_name(tail, span);
            self.call_expression(function, &[listed], keywords, span)
        };
        let first = self.call_variable("item", item.clone(), span);
        let one = if item.iterable_item().is_some() {
            let read = self.hidden_name(first, span);
            self.call_expression(function, &[read], keywords, span)
        } else {
            let message = self.not_iterable(first, span);
            self.raised(message, &Type::Error, span)
        };
        let none = format!("{name} expected at least 1 argument, got 0");
        let none = self.raised_type_error(&none, Self::number(0, span), &Type::Error, span);
        let ty = self.extreme_type(builtin, [&many, &one], &item, span);
        let one = Self::in_order(
            vec![Self::stored(first, Self::tail_item(tail, &item, 0, span))],
            retyped(one, &ty),
        );
        let switch = [(0, none), (1, one)].into_iter().rev().fold(
            Self::converted(retyped(many, &ty), Some(&ty)),
            |orelse, (count, arm)| {
                let test = self.length_is(tail, CompareOperator::Eq, count, span);
                Self::typed(
                    TypedExprKind::IfExp {
                        test: Box::new(test),
                        body: Box::new(Self::converted(retyped(arm, &ty), Some(&ty))),
                        orelse: Box::new(orelse),
                    },
                    ty.clone(),
                    span,
                )
            },
        );
        Self::in_order(prelude, switch)
    }

    /// Returns the type `max` or `min` with `*` gives: that of each arm that gives a value,
    /// which must be one, E0202 otherwise; `item`, the type of the values, when none does.
    fn extreme_type(
        &mut self,
        builtin: Builtin,
        arms: [&TypedExpr; 2],
        item: &Type,
        span: Span,
    ) -> Type {
        let given: Vec<(usize, TypedExpr)> = arms
            .into_iter()
            .filter(|arm| !matches!(arm.kind, TypedExprKind::Raise(_)))
            .map(|arm| (0, arm.clone()))
            .collect();
        if given.is_empty() {
            return item.clone();
        }
        self.arms_type(builtin, &given, span)
    }

    /// Returns `"'" + type(item).__name__ + "' object is not iterable"` of the hidden variable
    /// `item`, as CPython's message for iterating a value that cannot be iterated.
    fn not_iterable(&mut self, item: SymbolId, span: Span) -> TypedExpr {
        let text = |value: &str| Expr::Constant {
            value: typy_source::syntax::Constant::Str(value.to_owned()),
            span,
        };
        let class = Expr::Attribute {
            value: Box::new(Expr::Call {
                function: Box::new(Expr::Name {
                    id: "type".to_owned(),
                    span,
                }),
                arguments: vec![self.hidden_name(item, span)],
                keywords: Vec::new(),
                span,
            }),
            attribute: typy_source::syntax::Identifier {
                name: "__name__".to_owned(),
                span,
            },
            span,
        };
        let joined = |left: Expr, right: Expr| Expr::BinOp {
            left: Box::new(left),
            operator: BinaryOperator::Add,
            right: Box::new(right),
            span,
        };
        let message = joined(joined(text("'"), class), text("' object is not iterable"));
        self.expression(&message)
    }

    /// Returns `raise TypeError(message)` in place of a value of type `ty`, `{}` in `template`
    /// standing for the number `given`.
    fn raised_type_error(
        &mut self,
        template: &str,
        given: TypedExpr,
        ty: &Type,
        span: Span,
    ) -> TypedExpr {
        let text = |value: &str| {
            Self::typed(
                TypedExprKind::Constant(TypedConstant::Str(value.to_owned())),
                Type::Str,
                span,
            )
        };
        let message = match template.split_once("{}") {
            Some((before, after)) => {
                let count = Self::typed(
                    TypedExprKind::Builtin {
                        builtin: Builtin::Str,
                        arguments: vec![given],
                    },
                    Type::Str,
                    span,
                );
                let joined = |left: TypedExpr, right: TypedExpr| {
                    Self::typed(
                        TypedExprKind::Binary {
                            left: Box::new(left),
                            operator: BinaryOperator::Add,
                            right: Box::new(right),
                        },
                        Type::Str,
                        span,
                    )
                };
                joined(joined(text(before), count), text(after))
            }
            None => text(template),
        };
        self.raised(message, ty, span)
    }

    /// Returns `raise TypeError(message)` in place of a value of type `ty`.
    fn raised(&mut self, message: TypedExpr, ty: &Type, span: Span) -> TypedExpr {
        let class = ExceptionClass::Builtin(BuiltinException::TypeError);
        self.program.record_creation(&class, &[Type::Str]);
        let exception = Self::typed(
            TypedExprKind::NewException {
                class: class.clone(),
                arguments: vec![message],
                init: None,
            },
            Type::Exception(class),
            span,
        );
        Self::typed(TypedExprKind::Raise(Box::new(exception)), ty.clone(), span)
    }

    /// Returns E0404 for a call of `builtin` with more values before its first `*` than it
    /// takes.
    fn too_many_before_star(&mut self, builtin: Builtin, arity: &Arity, span: Span) -> TypedExpr {
        let message = format!(
            "`{}` takes at most {} argument{}, and more come before `*`",
            builtin.name(),
            arity.maximum,
            if arity.maximum == 1 { "" } else { "s" }
        );
        self.error(
            Diagnostic::new(DiagnosticCode::error(404), span, message).with_rule(RuleId::EXP_22),
            span,
        )
    }

    /// Returns `then` after each of `first`, which are evaluated in order for their effects.
    fn in_order(first: Vec<TypedExpr>, then: TypedExpr) -> TypedExpr {
        first.into_iter().rev().fold(then, |then, first| {
            let (ty, span) = (then.ty.clone(), then.span);
            Self::typed(
                TypedExprKind::Sequence {
                    first: Box::new(first),
                    then: Box::new(then),
                },
                ty,
                span,
            )
        })
    }

    /// Returns `symbol := value`, which stores `value` in the hidden variable `symbol`.
    fn stored(symbol: SymbolId, value: TypedExpr) -> TypedExpr {
        let (ty, span) = (value.ty.clone(), value.span);
        Self::typed(
            TypedExprKind::Named {
                target: symbol,
                value: Box::new(value),
            },
            ty,
            span,
        )
    }
}

/// Returns `arm` given the type `ty`, which a raise takes in place of a value.
fn retyped(arm: TypedExpr, ty: &Type) -> TypedExpr {
    if matches!(arm.kind, TypedExprKind::Raise(_)) {
        return TypedExpr {
            ty: ty.clone(),
            ..arm
        };
    }
    arm
}
