//! Static methods and properties: calls of a static method through its class or an instance,
//! reads of a property, which call its getter, and assignments, which call its setter
//! (CLS-06).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Expr, Identifier, Keyword};

use super::BodyChecker;
use crate::typed::{
    Argument, AssignTarget, ClassId, FunctionId, TypedExpr, TypedExprKind, TypedStmt,
};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Returns true when `class` or a base defines `name` as a static method.
    pub(super) fn is_static_method(&self, class: ClassId, name: &str) -> bool {
        std::iter::successors(Some(class), |each| {
            self.program.classes[each.0 as usize].base
        })
        .map(|each| &self.program.classes[each.0 as usize])
        .find(|info| info.method(name).is_some())
        .is_some_and(|info| info.static_methods.iter().any(|method| method == name))
    }

    /// Returns true when `class` or a base defines `name` as a method with type parameters of
    /// its own, whose calls go to its copies (GEN-01).
    pub(super) fn is_generic_method(&self, class: ClassId, name: &str) -> bool {
        std::iter::successors(Some(class), |each| {
            self.program.classes[each.0 as usize].base
        })
        .map(|each| &self.program.classes[each.0 as usize])
        .find(|info| info.method(name).is_some())
        .is_some_and(|info| info.generic_methods.iter().any(|method| method == name))
    }

    /// Returns the function a call of the method `function`, named `name` in `class`, runs,
    /// with its signature: the copy for a method with type parameters of its own, solved from
    /// the arguments after the first `skip`; the method itself for any other, including one
    /// that is generic only through its class.
    pub(super) fn method_target(
        &mut self,
        (class, name): (ClassId, &str),
        function: FunctionId,
        skip: usize,
        qualified: &str,
        (arguments, keywords): (&[Expr], &[Keyword]),
        span: Span,
    ) -> Result<(FunctionId, crate::typed::FunctionSignature), TypedExpr> {
        if self.is_generic_method(class, name) {
            return self.call_target(function, skip, qualified, arguments, keywords, span);
        }
        Ok((
            function,
            self.program.signatures[function.0 as usize].clone(),
        ))
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
        if let Some(call) = self.class_method_call_of(class, name, arguments, keywords, span) {
            return call;
        }
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
        self.static_call(function, class, name, arguments, keywords, span)
    }

    /// Returns the call of a static method through an instance as a call of its bridge
    /// through the method table, when a class derived from `class` defines `name` again;
    /// `None` when the call goes to the static method itself.
    pub(super) fn through_static_bridge(
        &self,
        class: ClassId,
        name: &str,
        receiver: &TypedExpr,
        call: &TypedExpr,
    ) -> Option<TypedExpr> {
        let TypedExprKind::Call { arguments, .. } = &call.kind else {
            return None;
        };
        // A copy of a generic static method is called directly.
        if !self.defined_below(class, name) || self.is_generic_method(class, name) {
            return None;
        }
        let bridge = std::iter::successors(Some(class), |each| {
            self.program.classes[each.0 as usize].base
        })
        .find_map(|each| {
            self.program.classes[each.0 as usize]
                .static_bridges
                .iter()
                .find(|(each_name, _)| each_name == name)
                .map(|(_, bridge)| *bridge)
        })?;
        let owner = self.program.signatures[bridge.0 as usize]
            .parameters
            .first()?
            .ty
            .clone();
        // The bound arguments name the static method's parameters, which follow the instance.
        let mut shifted = vec![Argument {
            parameter: 0,
            value: Self::converted(receiver.clone(), Some(&owner)),
        }];
        shifted.extend(arguments.iter().map(|argument| Argument {
            parameter: argument.parameter + 1,
            value: argument.value.clone(),
        }));
        Some(Self::typed(
            TypedExprKind::Call {
                function: bridge,
                arguments: shifted,
                dispatch: true,
            },
            call.ty.clone(),
            call.span,
        ))
    }

    /// Calls a static method of `class` with its arguments, without an instance.
    pub(super) fn static_call(
        &mut self,
        function: FunctionId,
        class: ClassId,
        name: &Identifier,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        let class_name = &self.program.classes[class.0 as usize].name;
        let qualified = format!("{class_name}.{}", name.name);
        let (target, signature) = match self.method_target(
            (class, &name.name),
            function,
            0,
            &qualified,
            (arguments, keywords),
            span,
        ) {
            Ok(found) => found,
            Err(error) => return error,
        };
        let bound = match self.bind_to(&signature, 0, &qualified, arguments, keywords, span) {
            Ok(bound) => bound,
            Err(error) => return error,
        };
        Self::typed(
            TypedExprKind::Call {
                function: target,
                arguments: bound,
                dispatch: false,
            },
            signature.returns,
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

    /// Returns the target `object.name = value` of the property `name` through its setter,
    /// or `None` after reporting a property without one or a value of another type (CLS-06).
    pub(super) fn property_target(
        &mut self,
        (object, class, getter): (&TypedExpr, ClassId, FunctionId),
        attribute: &Identifier,
        ty: &Type,
        span: Span,
    ) -> Option<AssignTarget> {
        let class_name = self.program.classes[class.0 as usize].name.clone();
        let Some(setter) = self
            .program
            .method(class, &format!("{}.setter", attribute.name))
        else {
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
            return None;
        };
        let expected = self.program.signatures[getter.0 as usize].returns.clone();
        if !ty.is_assignable_to(&expected) {
            self.diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(402),
                    span,
                    format!(
                        "the property `{class_name}.{}` is {expected} and is assigned {}",
                        attribute.name,
                        ty.with_article()
                    ),
                )
                .with_rule(RuleId::TYP_03),
            );
            return None;
        }
        let owner = self.program.signatures[setter.0 as usize]
            .parameters
            .first()
            .map(|parameter| parameter.ty.clone());
        Some(AssignTarget::Property {
            object: Box::new(Self::converted(object.clone(), owner.as_ref())),
            setter,
            value: ty.clone(),
        })
    }

    /// Checks `Class.name(arguments)` for a class method: a call of the copy of `Class`, which
    /// takes the class first; `None` when `name` is no class method of the class (ADR-0025).
    fn class_method_call_of(
        &mut self,
        class: ClassId,
        name: &Identifier,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> Option<TypedExpr> {
        let info = &self.program.classes[class.0 as usize];
        let class_name = info.name.clone();
        let function = info
            .class_methods
            .iter()
            .find(|(each, _)| *each == name.name)
            .map(|(_, id)| *id)?;
        let of = match self.program.exception(class) {
            Some(exception) => crate::types::TypeObjectOf::Exception(exception.clone()),
            None => crate::types::TypeObjectOf::Instance(class, class_name.clone()),
        };
        let class_object = Self::typed(
            TypedExprKind::TypeObject(Box::new(of.clone())),
            crate::types::TypeObjectOf::type_of(Some(of)),
            span,
        );
        let qualified = format!("{class_name}.{}", name.name);
        let bound = match self.bind_arguments(function, 1, &qualified, arguments, keywords, span) {
            Ok(bound) => bound,
            Err(error) => return Some(error),
        };
        let mut all = vec![Argument {
            parameter: 0,
            value: class_object,
        }];
        all.extend(bound);
        let returns = self.program.signatures[function.0 as usize].returns.clone();
        Some(Self::typed(
            TypedExprKind::Call {
                function,
                arguments: all,
                dispatch: false,
            },
            returns,
            span,
        ))
    }

    /// Reports `instance.name(arguments)` for a class method of a generic class, which has no
    /// bridge to the copy of the instance's class; returns true when it reports one.
    pub(super) fn refuse_class_method_through_instance(
        &mut self,
        class: ClassId,
        name: &Identifier,
    ) -> bool {
        let info = &self.program.classes[class.0 as usize];
        let class_method = info
            .class_methods
            .iter()
            .any(|(each, _)| *each == name.name);
        if !class_method {
            return false;
        }
        let class_name = info.name.clone();
        self.diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(201),
                name.span,
                format!(
                    "calling the class method `{}` of a generic class through an instance is \
                     planned for TyPy 1.0 (CLS-30)",
                    name.name
                ),
            )
            .with_rule(RuleId::CLS_30)
            .with_help(format!(
                "call it through its class, `{class_name}.{}(...)`",
                name.name
            )),
        );
        true
    }

    /// Reports an assignment to a field of a frozen dataclass outside the `__init__`
    /// `@dataclass` writes, where CPython raises `FrozenInstanceError` (CLS-05); returns true
    /// when it reports one.
    pub(super) fn refuse_frozen_field(
        &mut self,
        class: ClassId,
        attribute: &Identifier,
        object: &Expr,
        span: Span,
    ) -> bool {
        let mut current = Some(class);
        let mut frozen = None;
        while let Some(each) = current {
            let info = &self.program.classes[each.0 as usize];
            if info.frozen {
                frozen = Some(each);
                break;
            }
            current = info.base;
        }
        let Some(frozen) = frozen else {
            return false;
        };
        let initializing = self.initializing.is_some_and(|initializing| {
            initializing.class == class
                && self.program.classes[class.0 as usize].generated_init
                && matches!(object, Expr::Name { id, .. }
                    if self.locals.get(id) == Some(&initializing.instance))
        });
        if initializing {
            return false;
        }
        let name = self.program.classes[frozen.0 as usize].name.clone();
        self.diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(414),
                span,
                format!(
                    "`{name}` is a frozen dataclass, so its field `{}` is not assigned, as CPython \
                     raises `FrozenInstanceError`",
                    attribute.name
                ),
            )
            .with_rule(RuleId::CLS_05)
            .with_help("make a new instance with the value instead"),
        );
        true
    }
}
