//! Reads a method as a value (FUN-05, ADR-0060): through an instance, a bound method of the
//! method of the object's class; through a class, the function itself, as in CPython.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Identifier, ParameterKind};

use super::BodyChecker;
use super::callable_adapter::Expected;
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
        if self.class_method_named(class, &name.name) {
            return self.class_method_value_of(object, class, name, span);
        }
        if let Some(refused) = self.refused_method_value((class, true), name, span) {
            return refused;
        }
        let method = self
            .program
            .method(class, &name.name)
            .unwrap_or(FunctionId(0));
        let is_static = self.is_static_method(class, &name.name);
        if let Some(expected) = self.adapted_callable(method, usize::from(!is_static), span) {
            let read = (name.name.as_str(), span);
            return self.adapted_instance_method(object, (class, method), read, &expected);
        }
        let mut overrides = self.overrides_of(class, &name.name, false);
        // A subclass whose override is held otherwise is bound through an adapter to the
        // signature of the method read, which takes what the method takes (FUN-09, CLS-04).
        for (each, function) in &mut overrides {
            if self.is_adapted(*each, &name.name) {
                *function = match self.adapter_to(*each, &name.name, method) {
                    Some(adapter) => adapter,
                    None => self
                        .program
                        .value_adapter(*each, &name.name, (*function, method)),
                };
            }
        }
        if is_static && overrides.is_empty() {
            let value = self.function_value(method, &name.name, span);
            return Self::after(object, value);
        }
        let signature = &self.program.signatures[method.0 as usize];
        let ty = Type::Callable {
            parameters: signature
                .parameters
                .iter()
                .skip(usize::from(!is_static))
                .map(|parameter| parameter.ty.clone())
                .collect(),
            returns: Box::new(signature.returns.clone()),
        };
        if is_static
            || overrides
                .iter()
                .any(|(each, _)| self.is_static_method(*each, &name.name))
        {
            // The method of each class that defines it as a method binds the object, and a
            // static method is the function itself; an override that is neither held
            // otherwise nor adapted takes and gives values held as the method's (FUN-09,
            // CLS-35, ADR-0079).
            let binds = std::iter::once(class)
                .chain(overrides.iter().map(|(each, _)| *each))
                .filter(|each| !self.is_static_method(*each, &name.name))
                .collect();
            let functions = std::iter::once((class, method)).chain(overrides).collect();
            return Self::typed(
                TypedExprKind::ChosenFunction {
                    object: Box::new(object),
                    functions,
                    binds,
                },
                ty,
                span,
            );
        }
        let methods = std::iter::once((class, method)).chain(overrides).collect();
        Self::typed(
            TypedExprKind::BoundMethod {
                object: Box::new(object),
                methods,
                binds_class: false,
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
        if let Some(refused) = self.refused_method_value((class, false), name, span) {
            return refused;
        }
        if self.class_method_named(class, &name.name) {
            return self.class_method_value_of(object, class, name, span);
        }
        let method = self
            .program
            .method(class, &name.name)
            .unwrap_or(FunctionId(0));
        let value = match self.adapted_callable(method, 0, span) {
            Some((parameters, returns)) => self.adapted_function_value(
                (class, method),
                &name.name,
                (&parameters, returns.as_ref()),
                span,
            ),
            None => self.function_value(method, &name.name, span),
        };
        Self::after(object, value)
    }

    /// Returns `object.name` of `method`, the method `name` of `class`, through an adapter to
    /// the `Callable` `expected` of the method of each class the object may be an instance of
    /// (FUN-19): one that binds the object for a method, and one called directly for a static
    /// method.
    fn adapted_instance_method(
        &mut self,
        object: TypedExpr,
        (class, method): (ClassId, FunctionId),
        (name, span): (&str, Span),
        expected: &Expected,
    ) -> TypedExpr {
        let methods: Vec<(ClassId, FunctionId)> = std::iter::once((class, method))
            .chain(self.overrides_of(class, name, false))
            .collect();
        let binds: Box<[ClassId]> = methods
            .iter()
            .map(|(each, _)| *each)
            .filter(|each| !self.is_static_method(*each, name))
            .collect();
        if binds.len() == methods.len() {
            return self.adapted_bound_method(object, (methods, false), name, expected, span);
        }
        self.adapted_chosen_function(object, (methods, binds), name, expected, span)
    }

    /// Returns the error for a method that is no value of one `Callable` type: a property,
    /// whose value through its class is a `property` object, planned for 1.0 (FUN-17); a method
    /// with type parameters of its own, E0202; and, where no `Callable` its context expects
    /// takes it through an adapter (FUN-19), one with a keyword-only parameter, E0202, or with
    /// `*args` or `**kwargs`, E0203, as for a function.
    fn refused_method_value(
        &mut self,
        (class, through_instance): (ClassId, bool),
        name: &Identifier,
        span: Span,
    ) -> Option<TypedExpr> {
        if self.property_getter(class, &name.name).is_some() {
            let class_name = &self.program.classes[class.0 as usize].name;
            let message = format!(
                "`{class_name}.{}` is a property, and a property read through its class as a \
                 value is planned for TyPy 1.0 (FUN-17)",
                name.name
            );
            return Some(
                self.error(
                    Diagnostic::new(DiagnosticCode::error(201), name.span, message)
                        .with_rule(RuleId::FUN_17),
                    span,
                ),
            );
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
        let skip = usize::from(through_instance && !self.is_static_method(class, &name.name));
        if self.adapted_callable(method, skip, span).is_some() {
            return None;
        }
        if let Some(refused) = self.refused_variadic_value(method, &qualified, name.span) {
            return Some(refused);
        }
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

    /// Returns each class derived from `class` that defines the method `name` itself, with
    /// its method, each after the classes it derives from, so the last whose test holds is
    /// the object's own class's.
    fn overrides_of(
        &self,
        class: ClassId,
        name: &str,
        class_methods: bool,
    ) -> Vec<(ClassId, FunctionId)> {
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
            .filter_map(|info| {
                let method = if class_methods {
                    info.class_methods
                        .iter()
                        .find(|(each, _)| each == name)
                        .map(|(_, copy)| *copy)
                } else {
                    info.method(name)
                };
                method.map(|method| (info.id, method))
            })
            .collect();
        found.sort_by_key(|(each, _)| depth(*each));
        found
    }

    /// Returns the copy of the class method `name` that `class` has (ADR-0025).
    fn class_method_copy(&self, class: ClassId, name: &str) -> Option<FunctionId> {
        self.program.classes[class.0 as usize]
            .class_methods
            .iter()
            .find(|(each, _)| each == name)
            .map(|(_, copy)| *copy)
    }

    /// Returns the class method `name` of `class` as a value (FUN-09, ADR-0079): a bound method
    /// of the copy of the class it is read through, which binds that class; read through an
    /// instance, of the copy of the instance's own class, which binds `type(instance)`. A
    /// class method of a generic class read through an instance is planned for 0.14 (CLS-30).
    fn class_method_value_of(
        &mut self,
        object: TypedExpr,
        class: ClassId,
        name: &Identifier,
        span: Span,
    ) -> TypedExpr {
        let through_instance = !matches!(object.ty, Type::TypeObject(_));
        if through_instance
            && let Type::Instance(instance) = &object.ty
            && instance.generic.is_some()
        {
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(201),
                    name.span,
                    format!(
                        "`{}` is a class method of a generic class, and reading one through an \
                         instance is planned for TyPy 0.14 (CLS-30)",
                        name.name
                    ),
                )
                .with_rule(RuleId::CLS_30),
                span,
            );
        }
        let copy = self
            .class_method_copy(class, &name.name)
            .unwrap_or(FunctionId(0));
        let signature = &self.program.signatures[copy.0 as usize];
        let ty = Type::Callable {
            parameters: signature
                .parameters
                .iter()
                .skip(1)
                .map(|parameter| parameter.ty.clone())
                .collect(),
            returns: Box::new(signature.returns.clone()),
        };
        let overrides = if through_instance {
            self.overrides_of(class, &name.name, true)
        } else {
            Vec::new()
        };
        let methods: Vec<(ClassId, FunctionId)> =
            std::iter::once((class, copy)).chain(overrides).collect();
        if let Some(expected) = self.adapted_callable(copy, 1, span) {
            let bound = (methods, through_instance);
            return self.adapted_bound_method(object, bound, &name.name, &expected, span);
        }
        let qualified = format!(
            "{}.{}",
            self.program.classes[class.0 as usize].name, name.name
        );
        if let Some(refused) = self.refused_variadic_value(copy, &qualified, name.span) {
            return refused;
        }
        Self::typed(
            TypedExprKind::BoundMethod {
                object: Box::new(object),
                methods,
                binds_class: through_instance,
            },
            ty,
            span,
        )
    }

    /// Returns true when `class` defines `name` as a class method.
    fn class_method_named(&self, class: ClassId, name: &str) -> bool {
        self.program.classes[class.0 as usize]
            .class_methods
            .iter()
            .any(|(each, _)| each == name)
    }

    /// Returns the adapter of the method `name` of `class` when it takes what `method`, the
    /// method read, takes: when it adapts the override to `method` itself or to a method of
    /// the same signature.
    fn adapter_to(&self, class: ClassId, name: &str, method: FunctionId) -> Option<FunctionId> {
        let info = &self.program.classes[class.0 as usize];
        let (_, base, _) = info.adapted.iter().find(|(each, ..)| each == name)?;
        let same = |left: FunctionId, right: FunctionId| {
            let (left, right) = (
                &self.program.signatures[left.0 as usize],
                &self.program.signatures[right.0 as usize],
            );
            left.returns == right.returns
                && left.parameters.len() == right.parameters.len()
                && left
                    .parameters
                    .iter()
                    .zip(&right.parameters)
                    .skip(1)
                    .all(|(left, right)| left.ty == right.ty)
        };
        if *base != method && !same(*base, method) {
            return None;
        }
        info.adapters
            .iter()
            .find(|(each, _)| each == name)
            .map(|(_, adapter)| *adapter)
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
