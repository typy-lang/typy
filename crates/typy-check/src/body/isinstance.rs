//! Checks `isinstance(value, classes)` and the type it narrows the value to (CLS-09, TYP-07,
//! ADR-0021).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Expr, Keyword};

use super::BodyChecker;
use crate::class_type::InstanceClass;
use crate::typed::{IsInstanceTest, TypedExpr, TypedExprKind};
use crate::types::{Type, TypeObjectOf};

/// What a test of a value against one class gives, as far as the checker can tell.
enum Outcome {
    /// Every value of the static type is an instance of the class.
    Always,
    /// No value of the static type is.
    Never,
    /// The value's class decides, when the program runs.
    Checked,
}

impl BodyChecker<'_> {
    /// Checks `isinstance(value, class)` with a class, or a tuple of classes, named as values:
    /// a class of the program or a built-in type or exception class.
    pub(super) fn call_isinstance(
        &mut self,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        let ([value, classes], []) = (arguments, keywords) else {
            for argument in arguments {
                self.expression(argument);
            }
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    span,
                    "`isinstance` takes a value and a class or a tuple of classes",
                )
                .with_rule(RuleId::CLS_09),
                span,
            );
        };
        let value = self.expression(value);
        let classes = match self.isinstance_classes(classes) {
            Ok(classes) => classes,
            Err(Some(diagnostic)) => return self.error(*diagnostic, span),
            // The class is reported where it is named.
            Err(None) => {
                return Self::typed(
                    TypedExprKind::Constant(crate::typed::TypedConstant::None),
                    Type::Error,
                    span,
                );
            }
        };
        let inner = match &value.ty {
            Type::Optional(inner) => inner.as_ref().clone(),
            other => other.clone(),
        };
        let none_matches = matches!(value.ty, Type::Optional(_))
            && classes
                .iter()
                .any(|class| matches!(class, TypeObjectOf::NoneType | TypeObjectOf::Object));
        let outcomes: Vec<Outcome> = classes.iter().map(|class| outcome(&inner, class)).collect();
        let always = outcomes.iter().any(|each| matches!(each, Outcome::Always));
        let checks: Vec<TypeObjectOf> = classes
            .iter()
            .zip(&outcomes)
            .filter(|(_, outcome)| matches!(outcome, Outcome::Checked))
            .map(|(class, _)| class.clone())
            .collect();
        let narrowed = self.narrowing(&value, &inner, &classes, always, none_matches);
        Self::typed(
            TypedExprKind::IsInstance(Box::new(IsInstanceTest {
                value,
                none_matches,
                always,
                checks,
                narrowed,
            })),
            Type::Bool,
            span,
        )
    }

    /// Returns the classes the second argument names: a class or a tuple of them, each named
    /// as a value.
    fn isinstance_classes(
        &mut self,
        classes: &Expr,
    ) -> Result<Vec<TypeObjectOf>, Option<Box<Diagnostic>>> {
        let named = match classes {
            Expr::Tuple { elements, .. } => elements.iter().collect(),
            other => vec![other],
        };
        named
            .into_iter()
            .map(|each| {
                let typed = self.expression(each);
                match typed.kind {
                    TypedExprKind::TypeObject(of) => Ok(*of),
                    _ if typed.ty == Type::Error => Err(None),
                    _ => Err(Some(Box::new(
                        Diagnostic::new(
                            DiagnosticCode::error(201),
                            each.span(),
                            "`isinstance` with a class held in a variable or computed is \
                             planned for TyPy 0.5 (CLS-09)",
                        )
                        .with_rule(RuleId::CLS_09)
                        .with_help("name the class, or a tuple of classes, in the call"),
                    ))),
                }
            })
            .collect()
    }

    /// Returns the position, in the function's table of narrowed types, of the type a true
    /// test gives a variable: the nearest class every tested class of the program derives
    /// from, when each derives from the variable's static class.
    fn narrowing(
        &mut self,
        value: &TypedExpr,
        inner: &Type,
        classes: &[TypeObjectOf],
        always: bool,
        none_matches: bool,
    ) -> Option<u32> {
        if none_matches || super::flow::narrowable_variable(value).is_none() {
            return None;
        }
        let narrowed = if always {
            inner.clone()
        } else if let Type::Exception(own) = inner {
            let mut tested = Vec::new();
            for class in classes {
                match class {
                    TypeObjectOf::Exception(each) if each.derives_from(own) => {
                        tested.push(each.clone());
                    }
                    _ => return None,
                }
            }
            if tested.is_empty() {
                return None;
            }
            Type::Exception(crate::exception::common_base(&tested))
        } else {
            let Type::Instance(own) = inner else {
                return None;
            };
            let mut common: Option<InstanceClass> = None;
            for class in classes {
                let TypeObjectOf::Instance(id, _) = class else {
                    return None;
                };
                let Type::Instance(tested) = self.program.instance_type(*id) else {
                    return None;
                };
                if !tested.derives_from(own) {
                    return None;
                }
                common = Some(match common {
                    None => tested.as_ref().clone(),
                    Some(common) => common.common_base(&tested)?,
                });
            }
            Type::instance(common?)
        };
        if narrowed == value.ty {
            return None;
        }
        let position = self
            .narrowings
            .iter()
            .position(|each| *each == narrowed)
            .unwrap_or_else(|| {
                self.narrowings.push(narrowed);
                self.narrowings.len() - 1
            });
        u32::try_from(position).ok()
    }
}

/// Returns what testing a value of type `ty`, which is not optional, against `class` gives.
fn outcome(ty: &Type, class: &TypeObjectOf) -> Outcome {
    match (ty, class) {
        (_, TypeObjectOf::NoneType) => Outcome::Never,
        (Type::Instance(own), TypeObjectOf::Instance(id, _)) => {
            if own.id == *id || own.ancestors.iter().any(|(each, _)| each == id) {
                Outcome::Always
            } else {
                // The instance's class may derive from the tested class; when the two are not
                // related, the check gives false.
                Outcome::Checked
            }
        }
        (Type::Exception(own), TypeObjectOf::Exception(tested)) => {
            if own.derives_from(tested) {
                Outcome::Always
            } else if tested.derives_from(own) {
                Outcome::Checked
            } else {
                Outcome::Never
            }
        }
        // Every value is an `object`, and a `bool` is an `int` in CPython.
        (_, TypeObjectOf::Object) | (Type::Bool, TypeObjectOf::Int) => Outcome::Always,
        // These hold an object whose type only the object knows.
        (Type::Iterator(_) | Type::Iterable(_) | Type::AnyModule, _) => Outcome::Checked,
        _ => match TypeObjectOf::of(ty) {
            Some(own) if own == *class => Outcome::Always,
            _ => Outcome::Never,
        },
    }
}
