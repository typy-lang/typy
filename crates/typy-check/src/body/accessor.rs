//! Static methods and properties: calls of a static method through its class or an instance,
//! reads of a property, which call its getter, and assignments, which call its setter
//! (CLS-06).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Expr, Identifier, Keyword};

use super::BodyChecker;
use crate::typed::{Argument, ClassId, FunctionId, TypedExpr, TypedExprKind, TypedStmt};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Returns true when `class` or a base defines `name` as a static method.
    pub(super) fn is_static_method(&self, class: ClassId, name: &str) -> bool {
        let mut current = Some(class);
        while let Some(each) = current {
            let info = &self.program.classes[each.0 as usize];
            if info.method(name).is_some() {
                return info.static_methods.iter().any(|method| method == name);
            }
            current = info.base;
        }
        false
    }

    /// Returns the getter of the property `name` of `class` or a base, when it has one.
    pub(super) fn property_getter(&self, class: ClassId, name: &str) -> Option<FunctionId> {
        let mut current = Some(class);
        while let Some(each) = current {
            let info = &self.program.classes[each.0 as usize];
            if info.properties.iter().any(|property| property == name) {
                return self.program.method(class, name);
            }
            if info.method(name).is_some() || info.attribute(name).is_some() {
                return None;
            }
            current = info.base;
        }
        None
    }

    /// Checks `Class.name(arguments)`: a static method, called directly.
    pub(super) fn class_method_call(
        &mut self,
        class: ClassId,
        name: &Identifier,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        let class_name = self.program.classes[class.0 as usize].name.clone();
        let function = self.program.method(class, &name.name);
        let Some(function) = function.filter(|_| self.is_static_method(class, &name.name)) else {
            for argument in arguments {
                self.expression(argument);
            }
            let message = if function.is_some() {
                format!(
                    "calling the method `{class_name}.{}` through its class is planned for \
                     TyPy 0.5 (CLS-06)",
                    name.name
                )
            } else {
                format!("`{class_name}` has no static method `{}`", name.name)
            };
            let code = if function.is_some() { 201 } else { 414 };
            return self.error(
                Diagnostic::new(DiagnosticCode::error(code), name.span, message)
                    .with_rule(RuleId::CLS_06)
                    .with_help("call it on an instance"),
                span,
            );
        };
        self.static_call(function, &class_name, name, arguments, keywords, span)
    }

    /// Calls a static method with its arguments, without an instance.
    pub(super) fn static_call(
        &mut self,
        function: FunctionId,
        class_name: &str,
        name: &Identifier,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        let qualified = format!("{class_name}.{}", name.name);
        let bound = match self.bind_arguments(function, 0, &qualified, arguments, keywords, span) {
            Ok(bound) => bound,
            Err(error) => return error,
        };
        let returns = self.program.signatures[function.0 as usize].returns.clone();
        Self::typed(
            TypedExprKind::Call {
                function,
                arguments: bound,
                dispatch: false,
            },
            returns,
            span,
        )
    }

    /// Reads the property `name` of an instance: a call of its getter, which runs the getter
    /// of the instance's class when that class defines it again.
    pub(super) fn property_read(
        &mut self,
        object: TypedExpr,
        getter: FunctionId,
        span: Span,
    ) -> TypedExpr {
        let owner = self.program.signatures[getter.0 as usize]
            .parameters
            .first()
            .map(|parameter| parameter.ty.clone());
        let returns = self.program.signatures[getter.0 as usize].returns.clone();
        Self::typed(
            TypedExprKind::Call {
                function: getter,
                arguments: vec![Argument {
                    parameter: 0,
                    value: Self::converted(object, owner.as_ref()),
                }],
                dispatch: true,
            },
            returns,
            span,
        )
    }

    /// Checks `instance.name = value` for a property: a call of its setter. A property without
    /// one cannot be assigned, as CPython raises `AttributeError` for it. Returns `None` when
    /// `name` is not a property.
    pub(super) fn property_assignment(
        &mut self,
        object: &Expr,
        attribute: &Identifier,
        value: &Expr,
        span: Span,
    ) -> Option<TypedStmt> {
        let instance = self.peek_class(object)?;
        let getter = self.property_getter(instance, &attribute.name)?;
        let setter_name = format!("{}.setter", attribute.name);
        let class_name = self.program.classes[instance.0 as usize].name.clone();
        let Some(setter) = self.program.method(instance, &setter_name) else {
            self.expression(value);
            self.diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(414),
                    span,
                    format!(
                        "the property `{class_name}.{}` has no setter, so it is not assigned",
                        attribute.name
                    ),
                )
                .with_rule(RuleId::CLS_06),
            );
            return Some(TypedStmt::Pass { span });
        };
        let expected = self.program.signatures[getter.0 as usize].returns.clone();
        let object = self.expression(object);
        let value = self.expression_expecting(value, Some(&expected));
        if !value.ty.is_assignable_to(&expected) {
            self.diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(402),
                    value.span,
                    format!(
                        "the property `{class_name}.{}` is {expected} and is assigned {}",
                        attribute.name,
                        value.ty.with_article()
                    ),
                )
                .with_rule(RuleId::TYP_03),
            );
        }
        let owner = self.program.signatures[setter.0 as usize]
            .parameters
            .first()
            .map(|parameter| parameter.ty.clone());
        let value = Self::converted(value, Some(&expected));
        let call = Self::typed(
            TypedExprKind::Call {
                function: setter,
                arguments: vec![
                    Argument {
                        parameter: 0,
                        value: Self::converted(object, owner.as_ref()),
                    },
                    Argument {
                        parameter: 1,
                        value,
                    },
                ],
                dispatch: true,
            },
            Type::NoneType,
            span,
        );
        Some(TypedStmt::Expr { value: call, span })
    }

    /// Returns the class of the instance a variable holds, without checking the expression,
    /// when the expression reads a variable that holds one.
    fn peek_class(&self, object: &Expr) -> Option<ClassId> {
        let Expr::Name { id, .. } = object else {
            return None;
        };
        let symbol = self.locals.get(id)?;
        self.types.get(symbol)?.class_id()
    }

    /// Reports `instance.name(arguments)` for a property, whose value CPython would call.
    pub(super) fn uncallable_property(
        &mut self,
        class: ClassId,
        name: &Identifier,
        arguments: &[Expr],
        span: Span,
    ) -> TypedExpr {
        for argument in arguments {
            self.expression(argument);
        }
        let class_name = self.program.classes[class.0 as usize].name.clone();
        self.error(
            Diagnostic::new(
                DiagnosticCode::error(201),
                name.span,
                format!(
                    "`{class_name}.{}` is a property, and calling the value it gives is planned \
                     for TyPy 1.0 (FUN-05)",
                    name.name
                ),
            )
            .with_rule(RuleId::FUN_05)
            .with_help(format!("read it as `{}` without parentheses", name.name)),
            span,
        )
    }

    /// Reports an assignment to a property through an expression other than a variable,
    /// whose setter call is planned; returns true when it reports one.
    pub(super) fn refuse_property_target(
        &mut self,
        class: ClassId,
        attribute: &Identifier,
        span: Span,
    ) -> bool {
        if self.property_getter(class, &attribute.name).is_none() {
            return false;
        }
        self.diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(201),
                span,
                format!(
                    "assigning the property `{}` other than as `variable.{} = value` is planned \
                     for TyPy 0.5 (CLS-06)",
                    attribute.name, attribute.name
                ),
            )
            .with_rule(RuleId::CLS_06),
        );
        true
    }
}
