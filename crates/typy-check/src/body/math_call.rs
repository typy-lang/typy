//! Checks calls of the functions of `math` (LIB-05, ADR-0018): each argument must be of a
//! type CPython accepts for every value, a real number or an `int`, and the call gives the
//! function's result type.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Expr, Keyword};

use super::BodyChecker;
use crate::math::{MathFunction, MathShape};
use crate::typed::{TypedConstant, TypedExpr, TypedExprKind};
use crate::types::Type;

/// What an argument must be.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Wanted {
    /// An `int`, a `bool` or a `float`.
    Real,
    /// An `int` or a `bool`.
    Integer,
    /// An `int`, a `bool` or `None`.
    IntegerOrNone,
}

impl BodyChecker<'_> {
    /// Checks `math.name(arguments)` for the function `function`.
    pub(super) fn math_call(
        &mut self,
        function: MathFunction,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        let name = format!("math.{}", function.name());
        let shape = function.shape();
        let (positional, keyword) = match math_arguments(&name, shape, arguments, keywords, span) {
            Ok(arguments) => arguments,
            Err(diagnostic) => return self.error(*diagnostic, span),
        };
        let mut checked = Vec::with_capacity(positional.len() + keyword.len());
        let mut failed = false;
        for (expression, wanted) in positional.iter().chain(&keyword) {
            // A keyword not given takes its default, which lowering supplies.
            let Some(expression) = expression else {
                checked.push(Self::typed(
                    TypedExprKind::Constant(TypedConstant::None),
                    Type::NoneType,
                    span,
                ));
                continue;
            };
            let value = self.expression(expression);
            if let Err(message) = accepts(*wanted, &value.ty, function) {
                if !value.ty.has_error() {
                    self.diagnostics.push(
                        Diagnostic::new(DiagnosticCode::error(404), value.span, message)
                            .with_rule(RuleId::LIB_05),
                    );
                }
                failed = true;
            }
            checked.push(value);
        }
        if failed {
            return Self::typed(
                TypedExprKind::Constant(TypedConstant::None),
                Type::Error,
                span,
            );
        }
        Self::typed(
            TypedExprKind::MathCall {
                function,
                arguments: checked,
            },
            result_type(shape),
            span,
        )
    }
}

/// The positional and the keyword arguments of a call, each with what it must be.
type Arguments<'call> = (
    Vec<(Option<&'call Expr>, Wanted)>,
    Vec<(Option<&'call Expr>, Wanted)>,
);

/// Returns the positional arguments and the keyword arguments in their fixed order,
/// each with what it must be; a keyword not given is `None`, its default.
fn math_arguments<'call>(
    name: &str,
    shape: MathShape,
    arguments: &'call [Expr],
    keywords: &'call [Keyword],
    span: Span,
) -> Result<Arguments<'call>, Box<Diagnostic>> {
    let given = arguments.len();
    let count = |low: usize, high: Option<usize>| -> Result<(), Box<Diagnostic>> {
        let fits = given >= low && high.is_none_or(|high| given <= high);
        if fits {
            return Ok(());
        }
        let wanted = match high {
            Some(high) if high == low => format!("{low} argument{}", plural(low)),
            Some(high) => format!("from {low} to {high} arguments"),
            None => format!("at least {low} argument{}", plural(low)),
        };
        Err(mismatch(
            span,
            format!(
                "`{name}` takes {wanted}, and {given} {} given",
                if given == 1 { "is" } else { "are" }
            ),
        ))
    };
    let (low, high, wanted, names): (usize, Option<usize>, Wanted, &[(&str, Wanted)]) = match shape
    {
        MathShape::Real(count) => (count, Some(count), Wanted::Real, &[]),
        MathShape::Rounding
        | MathShape::Test
        | MathShape::Logarithm
        | MathShape::Frexp
        | MathShape::Modf => (1, Some(1), Wanted::Real, &[]),
        MathShape::Log => (1, Some(2), Wanted::Real, &[]),
        MathShape::Ldexp => (2, Some(2), Wanted::Real, &[]),
        MathShape::Hypot => (0, None, Wanted::Real, &[]),
        MathShape::Isclose => (
            2,
            Some(2),
            Wanted::Real,
            &[("rel_tol", Wanted::Real), ("abs_tol", Wanted::Real)],
        ),
        MathShape::Nextafter => (
            2,
            Some(2),
            Wanted::Real,
            &[("steps", Wanted::IntegerOrNone)],
        ),
        MathShape::Integer => (1, Some(1), Wanted::Integer, &[]),
        MathShape::Comb => (2, Some(2), Wanted::Integer, &[]),
        MathShape::Perm => (1, Some(2), Wanted::Integer, &[]),
        MathShape::Fold => (0, None, Wanted::Integer, &[]),
    };
    let mut keyword: Vec<(Option<&Expr>, Wanted)> =
        names.iter().map(|(_, wanted)| (None, *wanted)).collect();
    for given in keywords {
        let found = given.name.as_ref().and_then(|identifier| {
            names
                .iter()
                .position(|(known, _)| *known == identifier.name)
        });
        let Some(index) = found else {
            let message = match &given.name {
                Some(identifier) if !names.is_empty() => {
                    format!("`{name}` has no keyword argument `{}`", identifier.name)
                }
                _ => format!("`{name}` takes no keyword arguments"),
            };
            return Err(mismatch(given.span, message));
        };
        keyword[index].0 = Some(&given.value);
    }
    count(low, high)?;
    let mut positional: Vec<(Option<&Expr>, Wanted)> = arguments
        .iter()
        .map(|argument| (Some(argument), wanted))
        .collect();
    if shape == MathShape::Ldexp {
        positional[1].1 = Wanted::Integer;
    }
    if shape == MathShape::Perm && positional.len() == 2 {
        positional[1].1 = Wanted::IntegerOrNone;
    }
    Ok((positional, keyword))
}

fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}

/// Returns E0404 at `span`.
fn mismatch(span: Span, message: String) -> Box<Diagnostic> {
    Box::new(Diagnostic::new(DiagnosticCode::error(404), span, message).with_rule(RuleId::LIB_05))
}

/// Returns whether a value of type `ty` is what an argument must be, or CPython's message.
fn accepts(wanted: Wanted, ty: &Type, function: MathFunction) -> Result<(), String> {
    let integer = matches!(ty, Type::Int | Type::Bool | Type::Error);
    match wanted {
        Wanted::Real if integer || *ty == Type::Float => Ok(()),
        Wanted::Real if function == MathFunction::Trunc => Err(format!(
            "type {} doesn't define __trunc__ method",
            ty.class_name()
        )),
        Wanted::Real => Err(format!("must be real number, not {}", ty.class_name())),
        Wanted::Integer | Wanted::IntegerOrNone if integer => Ok(()),
        Wanted::IntegerOrNone if matches!(ty, Type::NoneType) => Ok(()),
        Wanted::IntegerOrNone if matches!(ty, Type::Optional(inner) if matches!(**inner, Type::Int | Type::Bool)) => {
            Ok(())
        }
        Wanted::Integer if function == MathFunction::Ldexp => {
            Err("Expected an int as second argument to ldexp".to_owned())
        }
        Wanted::Integer | Wanted::IntegerOrNone => Err(format!(
            "'{}' object cannot be interpreted as an integer",
            ty.class_name()
        )),
    }
}

/// Returns the type a function of `shape` gives.
fn result_type(shape: MathShape) -> Type {
    match shape {
        MathShape::Rounding
        | MathShape::Integer
        | MathShape::Comb
        | MathShape::Perm
        | MathShape::Fold => Type::Int,
        MathShape::Test | MathShape::Isclose => Type::Bool,
        MathShape::Frexp => Type::Tuple(vec![Type::Float, Type::Int]),
        MathShape::Modf => Type::Tuple(vec![Type::Float, Type::Float]),
        _ => Type::Float,
    }
}
