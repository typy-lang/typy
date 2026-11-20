//! Static methods and properties: calls of a static method through its class or an instance,
//! reads of a property, which call its getter, and assignments, which call its setter
//! (CLS-06).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Expr, Identifier, Keyword};

use super::BodyChecker;
use crate::typed::{
    Argument, AssignTarget, ClassId, FunctionId, TypedConstant, TypedExpr, TypedExprKind, TypedStmt,
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
        // A static method of a generic class that names its type parameters is generic over
        // them, so a call through the generic class takes them from its arguments.
        let through_generic_class = self
            .program
            .class_copies
            .borrow()
            .roots
            .contains_key(&class)
            && self.is_static_method(class, name);
        if self.is_generic_method(class, name) {
            // A method with type parameters of its own takes the type arguments of the copy
            // of a generic class that holds it first (GEN-09).
            let known = self.holder_arguments(class, function, skip > 0);
            let given = (arguments, keywords);
            return self.call_target_with(function, (skip, &known), qualified, given, span);
        }
        if through_generic_class {
            return self.call_target(function, skip, qualified, arguments, keywords, span);
        }
        Ok((
            function,
            self.program.signatures[function.0 as usize].clone(),
        ))
    }

    /// Returns the type arguments of the class that holds the method `function`, from `class`
    /// up: a copy's; a generic class's own parameters for a call through its own instance,
    /// which only its body makes; none for any other class or call.
    fn holder_arguments(
        &self,
        class: ClassId,
        function: FunctionId,
        through_instance: bool,
    ) -> Vec<Type> {
        let holder = std::iter::successors(Some(class), |each| {
            self.program.classes[each.0 as usize].base
        })
        .find(|each| {
            self.program.classes[each.0 as usize]
                .methods
                .iter()
                .any(|(_, id)| *id == function)
        });
        let Some(holder) = holder else {
            return Vec::new();
        };
        let copies = self.program.class_copies.borrow();
        let root = copies.copy(holder).map_or(holder, |copy| copy.root);
        // A static method that names none of the class's type parameters is generic over its
        // own alone.
        let over_class = copies.roots.get(&root).is_some_and(|generic| {
            self.program
                .generic(function)
                .is_some_and(|method| method.parameters.starts_with(&generic.parameters))
        });
        if !over_class {
            return Vec::new();
        }
        if let Some(copy) = copies.copy(holder) {
            return copy.arguments.clone();
        }
        if !through_instance {
            return Vec::new();
        }
        copies.roots.get(&holder).map_or_else(Vec::new, |generic| {
            generic
                .parameters
                .iter()
                .cloned()
                .map(Type::parameter)
                .collect()
        })
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

    /// Checks `Class.name(arguments)`: a static method, called directly, or a method of the
    /// class or a base, called directly with the instance as its first argument, as CPython
    /// calls the function the class holds (CLS-06).
    pub(super) fn class_method_call(
        &mut self,
        class: ClassId,
        name: &Identifier,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        if let Some(call) = self.class_method_call_of(class, name, arguments, keywords, span) {
            return call;
        }
        if let Some(call) = self.base_init_through_class(class, name, arguments, keywords, span) {
            return call;
        }
        let class_name = self.program.classes[class.0 as usize].name.clone();
        let function = self.program.method(class, &name.name);
        let property = self.property_getter(class, &name.name).is_some();
        let Some(function) = function.filter(|_| !property) else {
            for argument in arguments {
                self.expression(argument);
            }
            let diagnostic = if property {
                Diagnostic::new(
                    DiagnosticCode::error(401),
                    name.span,
                    format!(
                        "`{class_name}.{}` is a property, which is read through an instance and \
                         cannot be called",
                        name.name
                    ),
                )
                .with_rule(RuleId::CLS_06)
            } else {
                Diagnostic::new(
                    DiagnosticCode::error(414),
                    name.span,
                    format!("`{class_name}` has no method `{}`", name.name),
                )
                .with_rule(RuleId::CLS_06)
            };
            return self.error(diagnostic, span);
        };
        if self.is_static_method(class, &name.name) {
            return self.static_call(function, class, name, arguments, keywords, span);
        }
        self.unbound_call(
            (class, &class_name),
            function,
            name,
            arguments,
            keywords,
            span,
        )
    }

    /// Checks `Class.name(instance, arguments)` for a method that takes the instance: a direct
    /// call of the method the class holds or inherits, whose first parameter takes the
    /// instance; a method of a generic class runs the copy the arguments give.
    fn unbound_call(
        &mut self,
        (class, class_name): (ClassId, &str),
        function: FunctionId,
        name: &Identifier,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        let qualified = format!("{class_name}.{}", name.name);
        let generic = self
            .program
            .class_copies
            .borrow()
            .roots
            .contains_key(&class);
        let instance_of = if generic {
            self.copy_of_instance(class, arguments)
        } else {
            None
        };
        if let Some(copy) = instance_of.filter(|copy| *copy != class) {
            // The instance's copy holds the method for its type arguments.
            let function = self.program.method(copy, &name.name).unwrap_or(function);
            return self.unbound_call(
                (copy, class_name),
                function,
                name,
                arguments,
                keywords,
                span,
            );
        }
        // An instance of the generic class itself is one its own body reads, which is checked,
        // never run, so the call goes to the method itself.
        let found = if generic && instance_of.is_none() {
            self.call_target(function, 0, &qualified, arguments, keywords, span)
        } else {
            self.method_target(
                (class, &name.name),
                function,
                0,
                &qualified,
                (arguments, keywords),
                span,
            )
        };
        let (target, signature) = match found {
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

    /// Returns the copy of the generic class `class` the first argument is an instance of, or
    /// `class` itself for an instance of the generic class, typed once with what that look
    /// reports dropped; `None` when it is neither.
    fn copy_of_instance(&mut self, class: ClassId, arguments: &[Expr]) -> Option<ClassId> {
        let first = arguments.first()?;
        let reported = self.diagnostics.len();
        let before = self.assigned.clone();
        let value = self.expression(first);
        self.diagnostics.truncate(reported);
        self.assigned = before;
        match &value.ty {
            Type::Instance(instance)
                if instance.id == class
                    || instance
                        .generic
                        .as_ref()
                        .is_some_and(|(root, _)| *root == class) =>
            {
                Some(instance.id)
            }
            _ => None,
        }
    }

    /// Checks `Base.__init__(self, arguments)` in the `__init__` of a class derived from
    /// `Base`, on the instance it is building: as `super().__init__(arguments)` does, it
    /// counts `Base`'s attributes as assigned; `None` for any other call.
    fn base_init_through_class(
        &mut self,
        class: ClassId,
        name: &Identifier,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> Option<TypedExpr> {
        let initializing = self.initializing?;
        let [Expr::Name { id, .. }, rest @ ..] = arguments else {
            return None;
        };
        let is_instance = self.locals.get(id) == Some(&initializing.instance);
        let derives = std::iter::successors(
            self.program.classes[initializing.class.0 as usize].base,
            |each| self.program.classes[each.0 as usize].base,
        )
        .any(|each| each == class);
        if name.name != "__init__" || !is_instance || !derives {
            return None;
        }
        let inherited = self.program.classes[class.0 as usize].attributes.len();
        Some(self.base_init(initializing, (Some(class), inherited), rest, keywords, span))
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
            passed: crate::typed::Passed::Position,
            parameter: 0,
            value: Self::converted(receiver.clone(), Some(&owner)),
        }];
        shifted.extend(arguments.iter().map(|argument| Argument {
            passed: crate::typed::Passed::Position,
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
                    passed: crate::typed::Passed::Position,
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
                        passed: crate::typed::Passed::Position,
                        parameter: 0,
                        value: Self::converted(object, owner.as_ref()),
                    },
                    Argument {
                        passed: crate::typed::Passed::Position,
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

    /// Checks `instance.name(arguments)` for a property: a call through the value its getter
    /// gives, as CPython calls it (FUN-05). A value that is no function is E0401, as CPython
    /// raises `TypeError`.
    pub(super) fn property_call(
        &mut self,
        receiver: TypedExpr,
        (class, getter): (ClassId, FunctionId),
        name: &Identifier,
        (arguments, keywords): (&[Expr], &[Keyword]),
        span: Span,
    ) -> TypedExpr {
        let value = self.property_read(receiver, getter, name.span);
        if value.ty.callable_parts().is_some() {
            return self.call_value(value, arguments, keywords, span);
        }
        for argument in arguments {
            self.expression(argument);
        }
        if value.ty.has_error() {
            return Self::typed(
                TypedExprKind::Constant(TypedConstant::None),
                Type::Error,
                span,
            );
        }
        let class_name = &self.program.classes[class.0 as usize].name;
        let message = format!(
            "`{class_name}.{}` is a property of {}, which cannot be called",
            name.name,
            value.ty.with_article()
        );
        self.error(
            Diagnostic::new(DiagnosticCode::error(401), name.span, message)
                .with_rule(RuleId::EXP_02)
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
        let qualified = format!("{class_name}.{}", name.name);
        let (class, function) =
            match self.through_class_copy(class, function, &qualified, (arguments, keywords), span)
            {
                Ok(found) => found,
                Err(error) => return Some(error),
            };
        let of = match self.program.exception(class) {
            Some(exception) => crate::types::TypeObjectOf::Exception(exception.clone()),
            None => crate::types::TypeObjectOf::Instance(class, class_name.clone()),
        };
        let class_object = Self::typed(
            TypedExprKind::TypeObject(Box::new(of.clone())),
            crate::types::TypeObjectOf::type_of(Some(of)),
            span,
        );
        // A class method with type parameters of its own runs the copy its arguments give,
        // with the type arguments of a copy of a generic class first (GEN-09).
        let known = self.copy_arguments_of(class, function);
        let given = (arguments, keywords);
        let (target, signature) =
            match self.call_target_with(function, (1, &known), &qualified, given, span) {
                Ok(found) => found,
                Err(error) => return Some(error),
            };
        let bound = match self.bind_to(&signature, 1, &qualified, arguments, keywords, span) {
            Ok(bound) => bound,
            Err(error) => return Some(error),
        };
        let mut all = vec![Argument {
            passed: crate::typed::Passed::Position,
            parameter: 0,
            value: class_object,
        }];
        all.extend(bound);
        Some(Self::typed(
            TypedExprKind::Call {
                function: target,
                arguments: all,
                dispatch: false,
            },
            signature.returns,
            span,
        ))
    }

    /// Returns the type arguments of the copy of a generic class `class`, when `function` is
    /// generic over that class's type parameters first; none otherwise.
    fn copy_arguments_of(&self, class: ClassId, function: FunctionId) -> Vec<Type> {
        let copies = self.program.class_copies.borrow();
        let Some(copy) = copies.copy(class) else {
            return Vec::new();
        };
        let over_class = copies.roots.get(&copy.root).is_some_and(|generic| {
            self.program
                .generic(function)
                .is_some_and(|method| method.parameters.starts_with(&generic.parameters))
        });
        if over_class {
            copy.arguments.clone()
        } else {
            Vec::new()
        }
    }

    /// Returns the class a call of the class method `function` through `class` passes as `cls`,
    /// with the method it runs: for a generic class, the copy for the type arguments the
    /// arguments give, and its copy of the method; `class` and `function` for any other.
    fn through_class_copy(
        &mut self,
        class: ClassId,
        function: FunctionId,
        qualified: &str,
        given: (&[Expr], &[Keyword]),
        span: Span,
    ) -> Result<(ClassId, FunctionId), TypedExpr> {
        let generic = self
            .program
            .class_copies
            .borrow()
            .roots
            .contains_key(&class);
        if !generic || self.program.generic(function).is_none() {
            return Ok((class, function));
        }
        let types = self.solved_arguments(function, (1, &[]), qualified, given, span)?;
        // Inside the generic class's own body a parameter may solve to another; that body is
        // checked, never run.
        if types.iter().any(|ty| ty.has_parameter() || ty.has_error()) {
            return Ok((class, function));
        }
        let copy = self.program.class_copies.borrow_mut().applied(class, types);
        self.program.make_class_copies();
        let Some(copy) = copy.class_id() else {
            return Ok((class, function));
        };
        let name = self.program.classes[class.0 as usize]
            .class_methods
            .iter()
            .find(|(_, each)| *each == function)
            .map(|(name, _)| name.clone());
        let made = name.and_then(|name| {
            self.program.classes[copy.0 as usize]
                .class_methods
                .iter()
                .find(|(each, _)| *each == name)
                .map(|(_, id)| *id)
        });
        Ok((copy, made.unwrap_or(function)))
    }

    /// Reports `instance.name(arguments)` for a class method of a generic class, which has no
    /// bridge to the copy of the instance's class; returns true when it reports one.
    pub(super) fn refuse_class_method_through_instance(
        &mut self,
        class: ClassId,
        name: &Identifier,
    ) -> bool {
        let info = &self.program.classes[class.0 as usize];
        let Some(copy) = info
            .class_methods
            .iter()
            .find(|(each, _)| *each == name.name)
            .map(|(_, copy)| *copy)
        else {
            return false;
        };
        let class_name = info.name.clone();
        let of_generic_class = self.program.is_opaque_class(class)
            || self.program.class_copies.borrow().copy(class).is_some();
        let (message, rule) = if of_generic_class || self.program.generic(copy).is_none() {
            (
                format!(
                    "calling the class method `{}` of a generic class through an instance is \
                     planned for TyPy 0.14 (CLS-30)",
                    name.name
                ),
                RuleId::CLS_30,
            )
        } else {
            (
                format!(
                    "calling the class method `{}`, which has type parameters, through an \
                     instance is planned for TyPy 0.13 (GEN-16)",
                    name.name
                ),
                RuleId::GEN_16,
            )
        };
        self.diagnostics.push(
            Diagnostic::new(DiagnosticCode::error(201), name.span, message)
                .with_rule(rule)
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
