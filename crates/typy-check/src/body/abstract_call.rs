//! Calls that meet abstract methods (CLS-45, ADR-0093, ADR-0115). An instance is made only of
//! a class that overrides every abstract method it has: a call that names another class is
//! refused, and one through a `type[C]` value raises CPython's `TypeError`. No call runs an
//! abstract method whose body gives no value directly, where its result is not `None`.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::Expr;

use super::{BodyChecker, Program};
use crate::exception::{BuiltinException, ExceptionClass};
use crate::typed::{ClassId, RaiseCause, TypedConstant, TypedExpr, TypedExprKind, TypedStmt};
use crate::types::Type;

impl Program {
    /// Returns the abstract methods `class` has without an override, in sorted order.
    pub(crate) fn abstract_methods(&self, class: ClassId) -> &[String] {
        &self.classes[class.0 as usize].abstract_methods
    }

    /// Returns whether the method `name` of `class`, its own or that of the nearest base that
    /// defines it, is abstract with a body that gives no value.
    pub(crate) fn runs_empty_abstract(&self, class: ClassId, name: &str) -> bool {
        let mut current = Some(class);
        while let Some(class) = current {
            let info = &self.classes[class.0 as usize];
            if info.empty_abstract.iter().any(|each| each == name) {
                return true;
            }
            if info.method(name).is_some() {
                return false;
            }
            current = info.base;
        }
        false
    }
}

impl BodyChecker<'_> {
    /// Returns E0404 for a call that names a class with abstract methods without an override,
    /// after checking the arguments; `None` for any other class.
    pub(super) fn abstract_called(
        &mut self,
        class: ClassId,
        arguments: &[Expr],
        span: Span,
    ) -> Option<TypedExpr> {
        let methods = self.program.abstract_methods(class);
        if methods.is_empty() {
            return None;
        }
        let name = self.program.classes[class.0 as usize].name.clone();
        let (noun, them) = if methods.len() == 1 {
            ("method", "it")
        } else {
            ("methods", "them")
        };
        let names: Vec<String> = methods.iter().map(|method| format!("`{method}`")).collect();
        let message = format!(
            "`{name}` leaves the abstract {noun} {} without an override, so it has no \
             instances; CPython raises `TypeError`",
            names.join(", ")
        );
        for argument in arguments {
            self.expression(argument);
        }
        let diagnostic = Diagnostic::new(DiagnosticCode::error(404), span, message)
            .with_rule(RuleId::CLS_45)
            .with_help(format!(
                "make an instance of a class derived from `{name}` that overrides {them}"
            ));
        Some(self.error(diagnostic, span))
    }

    /// Returns `made`, a new instance of `class` made through a `type[C]` value, as CPython's
    /// `TypeError` raised once the arguments are evaluated, when `class` has abstract methods
    /// without an override; `made` itself otherwise.
    pub(super) fn abstract_made(&mut self, class: ClassId, made: TypedExpr) -> TypedExpr {
        let methods = self.program.abstract_methods(class);
        if methods.is_empty() || !matches!(made.kind, TypedExprKind::New { .. }) {
            return made;
        }
        let quoted: Vec<String> = methods.iter().map(|method| format!("'{method}'")).collect();
        let message = format!(
            "Can't instantiate abstract class {} without an implementation for abstract {} {}",
            self.program.classes[class.0 as usize].name,
            if methods.len() == 1 {
                "method"
            } else {
                "methods"
            },
            quoted.join(", ")
        );
        let (span, ty) = (made.span, made.ty.clone());
        let TypedExprKind::New { arguments, .. } = made.kind else {
            return made;
        };
        let text = Self::typed(
            TypedExprKind::Constant(TypedConstant::Str(message)),
            Type::Str,
            span,
        );
        let mut raised = self.raised(text, &ty, span);
        for argument in arguments.into_iter().rev() {
            raised = Self::typed(
                TypedExprKind::Sequence {
                    first: Box::new(argument.value),
                    then: Box::new(raised),
                },
                ty.clone(),
                span,
            );
        }
        raised
    }

    /// Returns whether `definition` is an abstract method whose body gives no value.
    pub(super) fn is_empty_abstract(&self, definition: &crate::check::Definition<'_>) -> bool {
        definition.class.is_some_and(|class| {
            self.program.classes[class.0 as usize]
                .empty_abstract
                .contains(&definition.name)
        })
    }

    /// Returns `raise NotImplementedError` for the end of an abstract method whose body gives
    /// no value where its result is not `None`. No call runs it, as an instance is made only
    /// of a class that overrides it and a direct call is refused; should one run it, it raises
    /// rather than return no value.
    pub(super) fn abstract_end(
        &mut self,
        definition: &crate::check::Definition<'_>,
        returns: &Type,
    ) -> Option<TypedStmt> {
        if self.assigned.is_none()
            || matches!(returns, Type::NoneType | Type::Optional(_) | Type::Error)
            || !self.is_empty_abstract(definition)
        {
            return None;
        }
        let class = ExceptionClass::Builtin(BuiltinException::NotImplementedError);
        self.program.record_creation(&class, &[]);
        let exception = Self::typed(
            TypedExprKind::NewException {
                class: class.clone(),
                arguments: Vec::new(),
                init: None,
            },
            Type::Exception(class),
            definition.span,
        );
        Some(TypedStmt::Raise {
            exception: Some(exception),
            cause: RaiseCause::Absent,
            span: definition.span,
        })
    }

    /// Returns E0434 for a call that runs the method `name` of `class` directly, through
    /// `super()` or the class, when it is abstract with a body that gives no value and its
    /// result `returns` is not `None`; CPython's call returns `None` in its place.
    pub(super) fn empty_abstract_run(
        &self,
        class: ClassId,
        name: &str,
        returns: &Type,
        span: Span,
    ) -> Option<Diagnostic> {
        if matches!(returns, Type::NoneType | Type::Optional(_))
            || !self.program.runs_empty_abstract(class, name)
        {
            return None;
        }
        let owner = &self.program.classes[class.0 as usize].name;
        Some(
            Diagnostic::new(
                DiagnosticCode::error(434),
                span,
                format!(
                    "`{owner}.{name}` is abstract and its body gives no value, so running it \
                     directly returns `None` in CPython where {} is declared",
                    returns.with_article()
                ),
            )
            .with_rule(RuleId::CLS_45)
            .with_help(
                "call the method through an instance, which runs its override, or give the \
                 abstract method a body that returns a value",
            ),
        )
    }
}
