//! Reads a method as a value (FUN-05, ADR-0060): through an instance, a bound method of the
//! method of the object's class; through a class, the function itself, as in CPython.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Identifier, ParameterKind};

use super::BodyChecker;
use crate::typed::{ClassId, FunctionId, TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Returns true when `class` or a base defines `name` as a method, a static method or a
    /// class method, which a read of `name` gives as a value.
    pub(super) fn has_method(&self, class: ClassId, name: &str) -> bool {
        self.program.method(class, name).is_some() || self.class_method_named(class, name)
    }

    /// Returns `object.name` of the method `name` of `class`, read through an instance: a
    /// bound method of the method of the object's class, of the `Callable` type of the
    /// parameters after `self`. A static method a subclass does not define again is the
    /// function itself, after `object` is evaluated for what it does.
    pub(super) fn instance_method_value(
        &mut self,
        object: TypedExpr,
        class: ClassId,
        name: &Identifier,
        span: Span,
    ) -> TypedExpr {
        if let Some(refused) = self.refused_method_value(class, name, span) {
            return refused;
        }
        let method = self
            .program
            .method(class, &name.name)
            .unwrap_or(FunctionId(0));
        let overrides = self.overrides_of(class, &name.name);
        if self.is_static_method(class, &name.name) {
            if !overrides.is_empty() {
                return self.planned_method_value(
                    name,
                    "a static method a subclass defines again",
                    span,
                );
            }
            let value = self.function_value(method, &name.name, span);
            return Self::after(object, value);
        }
        if overrides.iter().any(|(each, _)| {
            self.is_static_method(*each, &name.name) || self.is_adapted(*each, &name.name)
        }) {
            return self.planned_method_value(
                name,
                "a method a subclass overrides with other parameters or a result held otherwise",
                span,
            );
        }
        let signature = &self.program.signatures[method.0 as usize];
        let ty = Type::Callable {
            parameters: signature
                .parameters
                .iter()
                .skip(1)
                .map(|parameter| parameter.ty.clone())
                .collect(),
            returns: Box::new(signature.returns.clone()),
        };
        let methods = std::iter::once((class, method)).chain(overrides).collect();
        Self::typed(
            TypedExprKind::BoundMethod {
                object: Box::new(object),
                methods,
            },
            ty,
            span,
        )
    }

    /// Returns `Class.name` of the method `name` of `class`, read through the class: the
    /// function itself, whose first parameter is the instance for a method that takes one.
    pub(super) fn class_method_value(
        &mut self,
        object: TypedExpr,
        class: ClassId,
        name: &Identifier,
        span: Span,
    ) -> TypedExpr {
        if let Some(refused) = self.refused_method_value(class, name, span) {
            return refused;
        }
        let method = self
            .program
            .method(class, &name.name)
            .unwrap_or(FunctionId(0));
        let value = self.function_value(method, &name.name, span);
        Self::after(object, value)
    }

    /// Returns the error for a method that is no value of one `Callable` type: a class
    /// method, which binds the class it is read through, planned for 0.9 (FUN-09), and a
    /// method with type parameters of its own or a keyword-only parameter, E0202, as for a
    /// function.
    fn refused_method_value(
        &mut self,
        class: ClassId,
        name: &Identifier,
        span: Span,
    ) -> Option<TypedExpr> {
        if self.class_method_named(class, &name.name) {
            return Some(self.planned_method_value(name, "a class method", span));
        }
        let info = &self.program.classes[class.0 as usize];
        let qualified = format!("{}.{}", info.name, name.name);
        if self.is_generic_method(class, &name.name) {
            let message = format!(
                "the method `{qualified}` has type parameters of its own, so it is a copy for \
                 each set of types and has no one value"
            );
            return Some(
                self.error(
                    Diagnostic::new(DiagnosticCode::error(202), name.span, message)
                        .with_rule(RuleId::FUN_05),
                    span,
                ),
            );
        }
        let method = self.program.method(class, &name.name)?;
        let keyword_only = self.program.signatures[method.0 as usize]
            .parameters
            .iter()
            .find(|parameter| parameter.kind == ParameterKind::KeywordOnly)?
            .name
            .clone();
        let message = format!(
            "`{qualified}` takes `{keyword_only}` by keyword only, and a `Callable` passes every \
             argument by position, so `{qualified}` has no `Callable` type"
        );
        Some(
            self.error(
                Diagnostic::new(DiagnosticCode::error(202), name.span, message)
                    .with_rule(RuleId::FUN_05),
                span,
            ),
        )
    }

    /// Returns E0201 for a method of a kind whose value is planned for 0.9 (FUN-09).
    fn planned_method_value(&mut self, name: &Identifier, what: &str, span: Span) -> TypedExpr {
        self.error(
            Diagnostic::new(
                DiagnosticCode::error(201),
                name.span,
                format!(
                    "`{}` is {what}, and using one as a value is planned for TyPy 0.9 (FUN-09)",
                    name.name
                ),
            )
            .with_rule(RuleId::FUN_09),
            span,
        )
    }

    /// Returns each class derived from `class` that defines the method `name` itself, with
    /// its method, each after the classes it derives from, so the last whose test holds is
    /// the object's own class's.
    fn overrides_of(&self, class: ClassId, name: &str) -> Vec<(ClassId, FunctionId)> {
        let depth = |each: ClassId| {
            std::iter::successors(Some(each), |current| {
                self.program.classes[current.0 as usize].base
            })
            .count()
        };
        let derives = |each: ClassId| {
            std::iter::successors(self.program.classes[each.0 as usize].base, |current| {
                self.program.classes[current.0 as usize].base
            })
            .any(|base| base == class)
        };
        let mut found: Vec<(ClassId, FunctionId)> = self
            .program
            .classes
            .iter()
            .filter(|info| derives(info.id))
            .filter_map(|info| info.method(name).map(|method| (info.id, method)))
            .collect();
        found.sort_by_key(|(each, _)| depth(*each));
        found
    }

    /// Returns true when `class` defines `name` as a class method.
    fn class_method_named(&self, class: ClassId, name: &str) -> bool {
        self.program.classes[class.0 as usize]
            .class_methods
            .iter()
            .any(|(each, _)| each == name)
    }

    /// Returns true when `class` overrides `name` with a method held otherwise than its
    /// base's, which only an adapter reaches through the base (CLS-04).
    fn is_adapted(&self, class: ClassId, name: &str) -> bool {
        self.program.classes[class.0 as usize]
            .adapted
            .iter()
            .any(|(each, _, _)| each == name)
    }

    /// Returns `value` after `object` is evaluated for what it does, as CPython evaluates
    /// the object a static method or a function of a class is read through; a variable or a
    /// type object is only named.
    fn after(object: TypedExpr, value: TypedExpr) -> TypedExpr {
        if value.ty.has_error()
            || matches!(
                object.kind,
                TypedExprKind::Local(_)
                    | TypedExprKind::ModuleVariable(_)
                    | TypedExprKind::TypeObject(_)
            )
        {
            return value;
        }
        let (ty, span) = (value.ty.clone(), value.span);
        Self::typed(
            TypedExprKind::Sequence {
                first: Box::new(object),
                then: Box::new(value),
            },
            ty,
            span,
        )
    }
}
