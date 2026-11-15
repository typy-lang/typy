//! Checks class patterns, `Point(x, y=0)` (CST-06). The subject is tested as
//! `isinstance(subject, Point)`, and each sub-pattern matches an attribute: a keyword names
//! it, and a positional one takes the name at its position in `__match_args__`. For `bool`,
//! `bytes`, `dict`, `float`, `int`, `list`, `set`, `str` and `tuple`, one positional
//! sub-pattern matches the subject itself, as in CPython.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Expr, Identifier, Pattern};

use super::BodyChecker;
use crate::typed::{ClassId, TypedConstant, TypedExprKind, TypedPattern};
use crate::types::{Type, TypeObjectOf};

/// What a class pattern's positional sub-patterns match.
enum Positional {
    /// The subject itself, for one sub-pattern at most.
    Subject,
    /// The attributes `__match_args__` names, in order.
    Attributes(Vec<String>),
}

impl BodyChecker<'_> {
    /// Returns the steps of a class pattern: the class test, then each sub-pattern against
    /// the attribute it names, positional ones first.
    pub(super) fn class_pattern(
        &mut self,
        whole: &Pattern,
        class: &Expr,
        (patterns, keywords): (&[Pattern], &[(Identifier, Pattern)]),
        subject: &Expr,
        span: Span,
    ) -> TypedPattern {
        let mut steps = Vec::new();
        let tested = match self.isinstance_classes(class) {
            Ok(mut classes) if classes.len() == 1 => classes.remove(0),
            other => {
                let error = match other {
                    Err(Some(diagnostic)) => self.error(*diagnostic, span),
                    // The class was reported where it is named.
                    _ => Self::typed(
                        TypedExprKind::Constant(TypedConstant::None),
                        Type::Error,
                        span,
                    ),
                };
                self.bind_after_error(whole, &error);
                return TypedPattern::All(steps);
            }
        };
        let name = class_name(class);
        let what = format!("the class pattern `{name}()`");
        if !self.subject_class_test(
            whole,
            subject,
            std::slice::from_ref(&tested),
            &what,
            span,
            &mut steps,
        ) {
            return TypedPattern::All(steps);
        }
        let positional = match self.positional(&tested, &name, patterns.len(), span) {
            Ok(positional) => positional,
            Err(diagnostic) => {
                let error = self.error(*diagnostic, span);
                self.bind_after_error(whole, &error);
                return TypedPattern::All(steps);
            }
        };
        let mut named: Vec<(&str, &Pattern)> = Vec::new();
        match &positional {
            Positional::Subject => {
                if let Some(pattern) = patterns.first() {
                    self.part_pattern(pattern, subject, &mut steps);
                }
            }
            Positional::Attributes(attributes) => {
                named.extend(attributes.iter().map(String::as_str).zip(patterns));
            }
        }
        named.extend(
            keywords
                .iter()
                .map(|(attribute, pattern)| (attribute.name.as_str(), pattern)),
        );
        if let Some(repeated) = repeated(&named) {
            let error = self.error(
                type_error(
                    span,
                    &format!("{name}() got multiple sub-patterns for attribute '{repeated}'"),
                ),
                span,
            );
            self.bind_after_error(whole, &error);
            return TypedPattern::All(steps);
        }
        for (attribute, pattern) in named {
            let value = Expr::Attribute {
                value: Box::new(subject.clone()),
                attribute: Identifier {
                    name: attribute.to_owned(),
                    span,
                },
                span,
            };
            self.part_pattern(pattern, &value, &mut steps);
        }
        TypedPattern::All(steps)
    }

    /// Returns what the positional sub-patterns of a pattern of class `tested` match, or the
    /// `TypeError` CPython raises for too many of them.
    fn positional(
        &self,
        tested: &TypeObjectOf,
        name: &str,
        given: usize,
        span: Span,
    ) -> Result<Positional, Box<Diagnostic>> {
        let accepted = match tested {
            TypeObjectOf::Bool
            | TypeObjectOf::Bytes
            | TypeObjectOf::Dict
            | TypeObjectOf::Float
            | TypeObjectOf::Int
            | TypeObjectOf::List
            | TypeObjectOf::Set
            | TypeObjectOf::Str
            | TypeObjectOf::Tuple => {
                if given <= 1 {
                    return Ok(Positional::Subject);
                }
                1
            }
            TypeObjectOf::Instance(class, _) => match self.match_args(*class) {
                Some(Ok(names)) if given <= names.len() => {
                    return Ok(Positional::Attributes(
                        names.into_iter().take(given).collect(),
                    ));
                }
                Some(Ok(names)) => names.len(),
                Some(Err(at)) if given > 0 => {
                    return Err(Box::new(
                        Diagnostic::new(
                            DiagnosticCode::error(201),
                            at,
                            "a `__match_args__` other than a tuple of `str` literals is \
                             planned for TyPy 1.0",
                        )
                        .with_rule(RuleId::CST_06),
                    ));
                }
                Some(Err(_)) | None => 0,
            },
            _ => 0,
        };
        if given == 0 {
            return Ok(Positional::Attributes(Vec::new()));
        }
        let noun = if accepted == 1 {
            "sub-pattern"
        } else {
            "sub-patterns"
        };
        Err(Box::new(type_error(
            span,
            &format!("{name}() accepts {accepted} positional {noun} ({given} given)"),
        )))
    }

    /// Returns the `__match_args__` of a class of the program, its own or its nearest base's.
    fn match_args(&self, class: ClassId) -> Option<Result<Vec<String>, Span>> {
        let mut current = Some(class);
        while let Some(class) = current {
            let info = &self.program.classes[class.0 as usize];
            if let Some(match_args) = &info.match_args {
                return Some(match_args.clone());
            }
            current = info.base;
        }
        None
    }
}

/// Returns the first attribute two sub-patterns name.
fn repeated<'name>(named: &[(&'name str, &Pattern)]) -> Option<&'name str> {
    named.iter().enumerate().find_map(|(position, (name, _))| {
        named[..position]
            .iter()
            .any(|(earlier, _)| earlier == name)
            .then_some(*name)
    })
}

/// Returns the name CPython gives the class in its messages: the last name of `class`.
fn class_name(class: &Expr) -> String {
    match class {
        Expr::Attribute { attribute, .. } => attribute.name.clone(),
        Expr::Name { id, .. } => id.clone(),
        _ => String::new(),
    }
}

/// Returns the error for a pattern that raises CPython's `TypeError` whenever the subject is
/// an instance of the class.
fn type_error(span: Span, message: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::error(401),
        span,
        format!("{message}: CPython raises `TypeError`"),
    )
    .with_rule(RuleId::CST_06)
}
