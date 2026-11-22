//! Calls of an exception class with `*` and `**` (EXP-22, ADR-0083). The values `*` gives go
//! with the others into one list of one type, which becomes the exception's arguments when the
//! call runs, so `args` has the member `tuple[T, ...]`; the dicts `**` gives must be empty when
//! the call runs, since an exception class takes no keywords. The `OSError` family reads its
//! `errno`, `strerror` and file names from arguments whose number the program shows, so it
//! takes no `*` of a value only the run knows (ADR-0035). A class with an `__init__` of the
//! program's binds the values to it when the call runs, as any class does (ADR-0073).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Expr, Keyword, ParameterKind};

use super::BodyChecker;
use super::spread::held_alike;
use crate::exception::ExceptionClass;
use crate::typed::{
    Argument, ExceptionValues, FunctionId, InitCall, Passed, SignatureParameter, SpreadKind,
    SpreadPart, TypedExpr, TypedExprKind, UnpackedException,
};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Checks a call of the exception class `class`, without an `__init__` of the program's,
    /// whose arguments unpack with `*` or `**`.
    pub(super) fn exception_unpacked(
        &mut self,
        class: ExceptionClass,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        // The call takes what it unpacks, so its own errors stand (ADR-0073).
        self.unpacking_calls.push(span);
        let listed = arguments
            .iter()
            .any(|argument| matches!(argument, Expr::Starred { .. }));
        let values = if listed {
            let (list, item) = self.one_type_values(
                arguments,
                "the arguments of an exception class called with `*`",
                span,
            );
            if class.builtin().is_os_error() {
                return self.os_error_unpacked(&class, span);
            }
            self.program.record_variadic_creation(&class, &item);
            ExceptionValues::Listed(list)
        } else {
            let values: Vec<TypedExpr> = arguments
                .iter()
                .map(|argument| self.expression(argument))
                .collect();
            if let Some(error) = self.os_error_arguments(&class, &values, span) {
                return error;
            }
            let types: Vec<Type> = values.iter().map(|value| value.ty.clone()).collect();
            self.program.record_creation(&class, &types);
            ExceptionValues::Each(values)
        };
        let mut options = Vec::new();
        let mut entries = None;
        for keyword in keywords {
            if keyword.name.is_some() {
                return self.error(
                    Diagnostic::new(
                        DiagnosticCode::error(404),
                        keyword.span,
                        format!("`{}` takes positional arguments only", class.name()),
                    )
                    .with_rule(RuleId::EXC_01),
                    span,
                );
            }
            if let Some(argument) = self.bind_entries(&keyword.value, &mut entries) {
                options.push(argument.value);
            }
        }
        let ty = Type::Exception(class.clone());
        Self::typed(
            TypedExprKind::NewExceptionUnpacked(Box::new(UnpackedException {
                class,
                values,
                options,
            })),
            ty,
            span,
        )
    }

    /// Checks a call of the exception class `class` whose `__init__` of the program's, `init`,
    /// binds what the call unpacks when it runs (ADR-0073). The exception keeps the values
    /// given by position as `args`, as `BaseException.__new__` does: of one type when a `*`
    /// gives any whose number only the run knows (E0409), and none for the `OSError` family,
    /// whose `__init__` gives them.
    pub(super) fn exception_init_unpacked(
        &mut self,
        class: ExceptionClass,
        init: FunctionId,
        name: &str,
        (arguments, keywords): (&[Expr], &[Keyword]),
        span: Span,
    ) -> TypedExpr {
        let bound = match self.bind_arguments(init, 1, name, arguments, keywords, span) {
            Ok(bound) => bound,
            Err(error) => return error,
        };
        if class.builtin().is_os_error() {
            self.program.record_creation(&class, &[]);
        } else {
            let parameters = self.program.signatures[init.0 as usize].parameters.clone();
            let (fixed, tail) = kept_types(&bound, &parameters);
            match tail {
                None => self.program.record_creation(&class, &fixed),
                Some(item) => match fixed.iter().find(|ty| **ty != item) {
                    Some(other) if !other.has_error() && !item.has_error() => {
                        let message = format!(
                            "the arguments of an exception class called with `*` have one \
                             type, as `args` keeps them; these are {other} and {item}"
                        );
                        self.diagnostics.push(
                            Diagnostic::new(DiagnosticCode::error(409), span, message)
                                .with_rule(RuleId::EXP_22)
                                .with_help(
                                    "pass values of one type, or give the exception a tuple",
                                ),
                        );
                    }
                    _ => self.program.record_variadic_creation(&class, &item),
                },
            }
        }
        let ty = Type::Exception(class.clone());
        Self::typed(
            TypedExprKind::NewException {
                class,
                arguments: Vec::new(),
                init: Some(Box::new(InitCall {
                    function: init,
                    arguments: bound,
                    positional: 0,
                })),
            },
            ty,
            span,
        )
    }

    /// Returns the list of the values `arguments` give, those of each `*iterable` among them,
    /// of the first value's type, which each other must have (E0409, the values named
    /// `what`), with that type.
    pub(super) fn one_type_values(
        &mut self,
        arguments: &[Expr],
        what: &str,
        span: Span,
    ) -> (TypedExpr, Type) {
        let mut item: Option<Type> = None;
        let mut parts = Vec::new();
        for argument in arguments {
            let (part, ty, at) = if let Expr::Starred { value, .. } = argument {
                let value = self.tuple_expression(value);
                let at = value.span;
                let (source, given) = self.sequence_source(Self::sequence_of(value), "`*`");
                (SpreadPart::Each(Box::new(source)), given, at)
            } else {
                let value = self.expression_expecting(argument, item.as_ref());
                let ty = value.ty.clone();
                (SpreadPart::Item(value), ty, argument.span())
            };
            let first = item.get_or_insert_with(|| ty.clone()).clone();
            let (part, fits) = match part {
                SpreadPart::Item(value) => {
                    let fits = value.ty.is_assignable_to(&first);
                    (SpreadPart::Item(Self::converted(value, Some(&first))), fits)
                }
                SpreadPart::Each(source) => (SpreadPart::Each(source), held_alike(&ty, &first)),
            };
            if !fits && !ty.has_error() && !first.has_error() {
                let unpacked = matches!(part, SpreadPart::Each(_));
                self.unfit_value((&ty, &first), what, at, unpacked);
            }
            parts.push(part);
        }
        let item = item.unwrap_or(Type::Error);
        let list = Self::typed(
            TypedExprKind::Spread {
                kind: SpreadKind::List,
                parts,
            },
            Type::List(Box::new(item.clone())),
            span,
        );
        (list, item)
    }

    /// Reports a value of type `ty` among those named `what` that is not of type `first`, the
    /// first value's.
    fn unfit_value(&mut self, (ty, first): (&Type, &Type), what: &str, at: Span, unpacked: bool) {
        let value = if unpacked {
            format!("this unpacks items of type {ty}")
        } else {
            format!("this is {}", ty.with_article())
        };
        let message = format!(
            "{what} have one type; {value}, and the first is {}",
            first.with_article()
        );
        self.diagnostics.push(
            Diagnostic::new(DiagnosticCode::error(409), at, message)
                .with_rule(RuleId::EXP_22)
                .with_help("pass values of one type, each one by one where they differ"),
        );
    }

    /// Returns E0202 for `*` of a value only the run knows in a call of `class`, of the
    /// `OSError` family.
    fn os_error_unpacked(&mut self, class: &ExceptionClass, span: Span) -> TypedExpr {
        let message = format!(
            "`{}` reads `errno`, `strerror` and the file names from its arguments by their \
             position, so `*` of a value only the run knows cannot give them",
            class.name()
        );
        self.error(
            Diagnostic::new(DiagnosticCode::error(202), span, message)
                .with_rule(RuleId::EXP_22)
                .with_help("pass the arguments one by one"),
            span,
        )
    }
}

/// Returns the types of the values a call binds by position before its tail, as the callee
/// holds them, and the item type of the tail, the list of the values from the first
/// `*iterable` of unknown length on.
fn kept_types(bound: &[Argument], parameters: &[SignatureParameter]) -> (Vec<Type>, Option<Type>) {
    let mut fixed = Vec::new();
    let mut tail = None;
    for argument in bound {
        match (&argument.passed, &argument.value.ty) {
            (Passed::Position, ty) => fixed.push(ty.clone()),
            (Passed::Items(targets), Type::Tuple(items)) => {
                for (item, target) in items.iter().zip(targets) {
                    let parameter = &parameters[*target];
                    fixed.push(if parameter.kind == ParameterKind::VarPositional {
                        item.clone()
                    } else {
                        parameter.ty.clone()
                    });
                }
            }
            (Passed::Tail, Type::List(item)) => tail = Some(item.as_ref().clone()),
            _ => {}
        }
    }
    (fixed, tail)
}
