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
        self.isinstance_test(value, &classes, span)
    }

    /// Returns the test `isinstance(value, classes)` with the classes known, and what it
    /// narrows the value to; a `match` pattern tests its subject this way too (CST-06).
    pub(super) fn isinstance_test(
        &mut self,
        value: TypedExpr,
        classes: &[TypeObjectOf],
        span: Span,
    ) -> TypedExpr {
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
        let narrowed = self.narrowing(&value, &inner, classes, always, none_matches);
        let otherwise = self.union_otherwise(&value, &inner, classes);
        let members = match &inner {
            Type::Union(members) => members
                .iter()
                .map(|member| member_outcome(member, classes))
                .collect(),
            _ => Vec::new(),
        };
        Self::typed(
            TypedExprKind::IsInstance(Box::new(IsInstanceTest {
                value,
                none_matches,
                always,
                checks,
                narrowed,
                otherwise,
                members,
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
                // CPython raises `TypeError` for a protocol not marked `@runtime_checkable`,
                // and TyPy keeps no members to test at run time.
                if let TypedExprKind::TypeObject(of) = &typed.kind
                    && let TypeObjectOf::Instance(class, name) = of.as_ref()
                    && self.program.is_protocol(*class)
                {
                    return Err(Some(Box::new(
                        Diagnostic::new(
                            DiagnosticCode::error(401),
                            each.span(),
                            format!(
                                "`isinstance` with the protocol `{name}` raises `TypeError` in \
                                 CPython"
                            ),
                        )
                        .with_rule(RuleId::GEN_05)
                        .with_help("test for a class that satisfies the protocol"),
                    )));
                }
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
        } else if let Type::Union(members) = inner {
            let kept: Vec<Type> = members
                .iter()
                .flat_map(|member| self.member_when_true(member, classes))
                .collect();
            if kept.is_empty() {
                return None;
            }
            Type::union(kept)
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
        self.narrowing_position(value, narrowed)
    }

    /// Returns the position of `narrowed` in the function's table of narrowed types, unless
    /// it is the value's own type.
    fn narrowing_position(&mut self, value: &TypedExpr, narrowed: Type) -> Option<u32> {
        if narrowed == value.ty {
            return None;
        }
        Some(self.narrowing_index(narrowed))
    }

    /// Returns the position of `narrowed` in the function's table of narrowed types, adding
    /// it when it is not there yet.
    fn narrowing_index(&mut self, narrowed: Type) -> u32 {
        let position = self
            .narrowings
            .iter()
            .position(|each| *each == narrowed)
            .unwrap_or_else(|| {
                self.narrowings.push(narrowed);
                self.narrowings.len() - 1
            });
        // A function has far fewer narrowed types than `u32::MAX`.
        u32::try_from(position).unwrap_or(u32::MAX)
    }

    /// Returns what a member of a union may be when the test holds: the member, when a tested
    /// class takes all of it or may take part, or the tested classes derived from it, when
    /// only they can.
    fn member_when_true(&self, member: &Type, classes: &[TypeObjectOf]) -> Vec<Type> {
        let mut found = Vec::new();
        for class in classes {
            match outcome(member, class) {
                Outcome::Always => return vec![member.clone()],
                Outcome::Never => {}
                Outcome::Checked => match (member, class) {
                    (Type::Instance(own), TypeObjectOf::Instance(id, _))
                        if matches!(self.program.instance_type(*id),
                            Type::Instance(tested) if tested.derives_from(own)) =>
                    {
                        found.push(self.program.instance_type(*id));
                    }
                    // With single inheritance, an unrelated class takes no instance.
                    (Type::Instance(_), TypeObjectOf::Instance(..)) => {}
                    _ => return vec![member.clone()],
                },
            }
        }
        found
    }

    /// Returns the position of the type a false test gives a variable that holds a union: the
    /// members no tested class takes in every case, or `None` when that is every member or no
    /// member.
    fn union_otherwise(
        &mut self,
        value: &TypedExpr,
        inner: &Type,
        classes: &[TypeObjectOf],
    ) -> Option<u32> {
        let Type::Union(members) = inner else {
            return None;
        };
        super::flow::narrowable_variable(value)?;
        let left: Vec<Type> = members
            .iter()
            .filter(|member| {
                !classes
                    .iter()
                    .any(|class| matches!(outcome(member, class), Outcome::Always))
            })
            .cloned()
            .collect();
        if left.is_empty() {
            return None;
        }
        // `None` is among the classes only with `object`, which takes every member, so a
        // value that may be `None` still may here.
        let rest = Type::union(left);
        let rest = if matches!(value.ty, Type::Optional(_)) {
            Type::optional(rest)
        } else {
            rest
        };
        self.narrowing_position(value, rest)
    }
}

/// Returns, for a member of a union, whether every value of it is an instance of one of the
/// classes, and the classes its value's class decides.
fn member_outcome(member: &Type, classes: &[TypeObjectOf]) -> (bool, Vec<TypeObjectOf>) {
    let outcomes: Vec<Outcome> = classes.iter().map(|class| outcome(member, class)).collect();
    let always = outcomes.iter().any(|each| matches!(each, Outcome::Always));
    let checks = classes
        .iter()
        .zip(&outcomes)
        .filter(|(_, each)| matches!(each, Outcome::Checked))
        .map(|(class, _)| class.clone())
        .collect();
    (always, checks)
}

/// Returns what testing a value of type `ty`, which is not optional, against `class` gives.
fn outcome(ty: &Type, class: &TypeObjectOf) -> Outcome {
    match (ty, class) {
        // A union is an instance when each member is, and is not when no member is.
        (Type::Union(members), _) => {
            let each: Vec<Outcome> = members
                .iter()
                .map(|member| outcome(member, class))
                .collect();
            if each.iter().all(|one| matches!(one, Outcome::Always)) {
                Outcome::Always
            } else if each.iter().all(|one| matches!(one, Outcome::Never)) {
                Outcome::Never
            } else {
                Outcome::Checked
            }
        }
        (_, TypeObjectOf::NoneType) => Outcome::Never,
        (Type::Instance(own), TypeObjectOf::Instance(id, _)) => {
            let copy_of = own.generic.as_ref().is_some_and(|(root, _)| root == id);
            if own.id == *id || copy_of || own.ancestors.iter().any(|(each, _)| each == id) {
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
