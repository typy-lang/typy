//! Checks calls through `super()`: a method of the base of the class whose method encloses
//! the call, called directly on the instance (CLS-14, ADR-0021).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Expr, Identifier, Keyword};

use super::BodyChecker;
use super::class::Initializing;
use super::flow::Fact;
use crate::typed::{Argument, TypedConstant, TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Checks `super().name(arguments)`, or `super(Class, self).name(arguments)` naming the
    /// enclosing class and its instance, in a method of a class. It calls the method of the
    /// nearest base that has one, directly, since the base is known when the program is
    /// compiled. In `__init__`, `super().__init__(...)` also counts the inherited attributes
    /// as assigned.
    pub(super) fn super_call(
        &mut self,
        receiver: &Expr,
        name: &Identifier,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        let Some(method) = self.method else {
            let diagnostic = Diagnostic::new(
                DiagnosticCode::error(414),
                span,
                "`super()` is used in a method of a class",
            )
            .with_rule(RuleId::CLS_14);
            return self.refused_super(diagnostic, arguments, span);
        };
        let start = match self.super_start(receiver, method) {
            Ok(start) => start,
            Err(diagnostic) => return self.refused_super(*diagnostic, arguments, span),
        };
        match self.initializing {
            Some(initializing) if name.name == "__init__" => {
                self.super_init(initializing, start, arguments, keywords, span)
            }
            _ => self.super_method(method, start, name, arguments, keywords, span),
        }
    }

    /// Reports a refused `super()` call after checking its arguments.
    fn refused_super(
        &mut self,
        diagnostic: Diagnostic,
        arguments: &[Expr],
        span: Span,
    ) -> TypedExpr {
        for argument in arguments {
            self.expression(argument);
        }
        self.error(diagnostic, span)
    }

    /// Returns the class whose base the lookup of `super(...)` starts at: the enclosing class
    /// for `super()`, and `C` for `super(C, self)`, where `C` is the enclosing class or one of
    /// its bases, as CPython's lookup starts after `C` in the instance's chain (CLS-26).
    fn super_start(
        &self,
        receiver: &Expr,
        method: Initializing,
    ) -> Result<crate::typed::ClassId, Box<Diagnostic>> {
        let arguments = match receiver {
            Expr::Call { arguments, .. } => arguments.as_slice(),
            _ => &[],
        };
        let chain = |class: crate::typed::ClassId| {
            std::iter::successors(Some(method.class), |each| {
                self.program.classes[each.0 as usize].base
            })
            .any(|each| each == class)
        };
        let named_class = |expression: &Expr| match expression {
            Expr::Name { id, .. } => self
                .program
                .class_ids
                .get(id)
                .copied()
                .filter(|class| chain(*class)),
            _ => None,
        };
        let names_instance = |expression: &Expr| {
            matches!(expression, Expr::Name { id, .. }
                if self.locals.get(id) == Some(&method.instance))
        };
        match arguments {
            [] => Ok(method.class),
            [class, instance] if names_instance(instance) => named_class(class).ok_or_else(|| {
                Box::new(
                    Diagnostic::new(
                        DiagnosticCode::error(404),
                        class.span(),
                        "super(type, obj): obj must be an instance or subtype of type",
                    )
                    .with_rule(RuleId::CLS_26),
                )
            }),
            _ => Err(Box::new(
                Diagnostic::new(
                    DiagnosticCode::error(201),
                    receiver.span(),
                    "`super()` with an instance other than the method's own is planned for \
                     TyPy 0.14 (CLS-29)",
                )
                .with_rule(RuleId::CLS_29)
                .with_help("write `super()`, or `super(Class, self)`"),
            )),
        }
    }

    /// Checks `super().name(arguments)` outside `__init__`: a direct call of the method of the
    /// nearest base of `start` that has it.
    fn super_method(
        &mut self,
        method: Initializing,
        start: crate::typed::ClassId,
        name: &Identifier,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        let base = self.program.classes[start.0 as usize].base;
        let Some((function, base)) =
            base.and_then(|base| Some((self.program.method(base, &name.name)?, base)))
        else {
            for argument in arguments {
                self.expression(argument);
            }
            let class = &self.program.classes[start.0 as usize].name;
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(414),
                    name.span,
                    format!(
                        "no base of `{class}` that is a class of the program has a method `{}`",
                        name.name
                    ),
                )
                .with_rule(RuleId::CLS_14),
                span,
            );
        };
        let qualified = format!("{}.{}", self.program.instance_type(base), name.name);
        let instance = self.enclosing_instance(method, span);
        let (target, signature) = match self.method_target(
            (base, &name.name),
            function,
            1,
            &qualified,
            (arguments, keywords),
            span,
        ) {
            Ok(found) => found,
            Err(error) => return error,
        };
        let bound = match self.bind_to(&signature, 1, &qualified, arguments, keywords, span) {
            Ok(bound) => bound,
            Err(error) => return error,
        };
        let mut all = vec![Argument {
            passed: crate::typed::Passed::Position,
            parameter: 0,
            value: instance,
        }];
        all.extend(bound);
        Self::typed(
            TypedExprKind::Call {
                function: target,
                arguments: all,
                dispatch: false,
            },
            signature.returns,
            span,
        )
    }

    /// Checks `super().__init__(arguments)` in `__init__`. With a base of the program that has
    /// `__init__`, it calls that `__init__` on the instance; for an exception class whose bases
    /// of the program have none, it sets the exception's arguments, as `BaseException.__init__`
    /// does; for a class whose bases have none, it is `object.__init__()`, which does nothing.
    /// The inherited attributes count as assigned after it. A base's `__init__` that uses the
    /// instance may run an override that reads the class's own attributes, so those are
    /// assigned first.
    fn super_init(
        &mut self,
        initializing: Initializing,
        start: crate::typed::ClassId,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        let info = &self.program.classes[start.0 as usize];
        let (base, inherited) = (info.base, info.inherited);
        self.base_init(initializing, (base, inherited), arguments, keywords, span)
    }

    /// Checks a call of the `__init__` of `base`, or of `object` or the exception class the
    /// class derives from when `base` is `None`, on the instance `__init__` is building, with
    /// the arguments after the instance; the first `inherited` attributes count as assigned
    /// after it.
    pub(super) fn base_init(
        &mut self,
        initializing: Initializing,
        (base, inherited): (Option<crate::typed::ClassId>, usize),
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        let base_init = base.and_then(|base| self.program.method(base, "__init__"));
        let call = if let (Some(function), Some(base)) = (base_init, base) {
            if let Some(error) = self.init_shares_too_early(initializing, base, span) {
                // The call is refused, but it names the base's `__init__`, which assigns the
                // inherited attributes, so their absence is not reported again.
                if let Some(facts) = &mut self.assigned {
                    facts.extend((0..inherited).map(Fact::Attribute));
                }
                return error;
            }
            let qualified = format!("{}.__init__", self.program.instance_type(base));
            let instance = self.enclosing_instance(initializing, span);
            match self.bind_arguments(function, 1, &qualified, arguments, keywords, span) {
                Ok(bound) => {
                    let mut all = vec![Argument {
                        passed: crate::typed::Passed::Position,
                        parameter: 0,
                        value: instance,
                    }];
                    all.extend(bound);
                    Self::typed(
                        TypedExprKind::Call {
                            function,
                            arguments: all,
                            dispatch: false,
                        },
                        Type::NoneType,
                        span,
                    )
                }
                Err(error) => return error,
            }
        } else if let Some(exception) = self.program.exception(initializing.class).cloned() {
            let instance = self.enclosing_instance(initializing, span);
            let values: Vec<TypedExpr> = arguments
                .iter()
                .map(|value| self.expression(value))
                .collect();
            if let Some(keyword) = keywords.first() {
                return self.error(
                    Diagnostic::new(
                        DiagnosticCode::error(404),
                        keyword.span,
                        "`BaseException.__init__` takes positional arguments only",
                    )
                    .with_rule(RuleId::CLS_14),
                    span,
                );
            }
            let types: Vec<Type> = values.iter().map(|value| value.ty.clone()).collect();
            self.program.record_initialization(&exception, &types);
            Self::typed(
                TypedExprKind::SetArguments {
                    exception: Box::new(instance),
                    arguments: values,
                },
                Type::NoneType,
                span,
            )
        } else {
            if !arguments.is_empty() || !keywords.is_empty() {
                for argument in arguments {
                    self.expression(argument);
                }
                return self.error(
                    Diagnostic::new(
                        DiagnosticCode::error(404),
                        span,
                        "`object.__init__` takes no arguments",
                    )
                    .with_rule(RuleId::CLS_14),
                    span,
                );
            }
            Self::typed(
                TypedExprKind::Constant(TypedConstant::None),
                Type::NoneType,
                span,
            )
        };
        if let Some(facts) = &mut self.assigned {
            facts.extend((0..inherited).map(Fact::Attribute));
        }
        call
    }

    /// Returns the error for `super().__init__(...)` before the class's own attributes are
    /// assigned when the base's `__init__` uses the instance, and notes that the class's
    /// `__init__` uses it when the base's does.
    fn init_shares_too_early(
        &mut self,
        initializing: Initializing,
        base: crate::typed::ClassId,
        span: Span,
    ) -> Option<TypedExpr> {
        let shares = std::iter::successors(Some(base), |class| {
            self.program.classes[class.0 as usize].base
        })
        .map(|class| &self.program.classes[class.0 as usize])
        .find(|info| info.init.is_some())
        .is_some_and(|info| info.init_uses_instance);
        if !shares {
            return None;
        }
        self.program.classes[initializing.class.0 as usize].init_uses_instance = true;
        let info = &self.program.classes[initializing.class.0 as usize];
        let missing: Vec<String> = info.attributes[info.inherited..]
            .iter()
            .enumerate()
            .filter(|(index, _)| {
                self.assigned
                    .as_ref()
                    .is_some_and(|facts| !facts.contains(&Fact::Attribute(info.inherited + index)))
            })
            .map(|(_, attribute)| format!("`{}`", attribute.name))
            .collect();
        if missing.is_empty() {
            return None;
        }
        let base_name = &self.program.classes[base.0 as usize].name;
        let diagnostic = Diagnostic::new(
            DiagnosticCode::error(309),
            span,
            format!(
                "this call runs an `__init__` of `{base_name}` that uses the instance, which \
                 may call a method that reads {}, before it is assigned",
                missing.join(", ")
            ),
        )
        .with_rule(RuleId::CLS_01)
        .with_help("assign the attributes of this class before calling the base's `__init__`");
        Some(self.error(diagnostic, span))
    }

    /// Returns the instance of the enclosing method, of its class's type.
    fn enclosing_instance(&self, method: Initializing, span: Span) -> TypedExpr {
        Self::typed(
            TypedExprKind::Local(method.instance),
            self.program.instance_type(method.class),
            span,
        )
    }
}
