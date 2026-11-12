//! Checks what a body does with classes: makes instances, reads and assigns their attributes,
//! and calls their methods (CLS-01, CLS-02). Inside `__init__`, the instance is used only to
//! assign and read attributes until every attribute is assigned, so no attribute is ever read
//! before it holds a value.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Expr, Identifier, Keyword};

use super::BodyChecker;
use super::builtin_module::is_builtin_module;
use super::flow::Fact;
use crate::typed::{Argument, AssignTarget, ClassId, InitCall, SymbolId, TypedExpr, TypedExprKind};
use crate::types::Type;

/// The `__init__` being checked: its class, and the parameter that holds the new instance.
#[derive(Clone, Copy, Debug)]
pub(super) struct Initializing {
    pub(super) class: ClassId,
    pub(super) instance: SymbolId,
}

impl BodyChecker<'_> {
    /// Returns the instance an attribute expression names when it is the instance `__init__`
    /// is building, read without the check that it is complete.
    fn initialized_instance(&self, value: &Expr) -> Option<(Initializing, Span)> {
        let initializing = self.initializing?;
        let Expr::Name { id, span } = value else {
            return None;
        };
        (self.locals.get(id) == Some(&initializing.instance)).then_some((initializing, *span))
    }

    /// Returns true when every attribute of the class `__init__` builds is assigned here.
    pub(super) fn instance_is_complete(&self, initializing: Initializing) -> bool {
        let count = self.program.classes[initializing.class.0 as usize]
            .attributes
            .len();
        self.assigned
            .as_ref()
            .is_none_or(|facts| (0..count).all(|index| facts.contains(&Fact::Attribute(index))))
    }

    /// Reports a use of the instance `__init__` builds before every attribute is assigned, or,
    /// when `ending`, a way out of `__init__` that leaves one unassigned.
    pub(super) fn incomplete_instance(
        &mut self,
        initializing: Initializing,
        span: Span,
        ending: bool,
    ) {
        let class = &self.program.classes[initializing.class.0 as usize];
        let missing: Vec<String> = class
            .attributes
            .iter()
            .enumerate()
            .filter(|(index, _)| {
                self.assigned
                    .as_ref()
                    .is_some_and(|facts| !facts.contains(&Fact::Attribute(*index)))
            })
            .map(|(_, attribute)| format!("`{}`", attribute.name))
            .collect();
        let message = if ending {
            format!(
                "`__init__` can end without assigning {}",
                missing.join(", ")
            )
        } else {
            format!(
                "the instance is used before `__init__` assigns {}",
                missing.join(", ")
            )
        };
        self.diagnostics.push(
            Diagnostic::new(DiagnosticCode::error(309), span, message)
                .with_rule(RuleId::CLS_01)
                .with_help("assign every attribute first"),
        );
    }

    /// Checks the instance an attribute expression reads or assigns, and returns it with its
    /// class.
    fn instance(&mut self, value: &Expr) -> Option<(TypedExpr, ClassId)> {
        let object = self.instance_object(value);
        self.as_instance(object)
    }

    /// Checks the value an attribute expression reads or assigns; the instance `__init__` is
    /// building is read without the check that it is complete.
    fn instance_object(&mut self, value: &Expr) -> TypedExpr {
        match self.initialized_instance(value) {
            Some((initializing, span)) => Self::typed(
                TypedExprKind::Local(initializing.instance),
                self.program.instance_type(initializing.class),
                span,
            ),
            None => self.expression(value),
        }
    }

    /// Returns an instance with its class, after reporting a value that is not an instance.
    fn as_instance(&mut self, object: TypedExpr) -> Option<(TypedExpr, ClassId)> {
        match &object.ty {
            _ if let Some(class) = object.ty.class_id() => Some((object, class)),
            Type::Error => None,
            other => {
                let message = format!("{} has no attributes in TyPy", other.with_article());
                self.diagnostics.push(
                    Diagnostic::new(DiagnosticCode::error(412), object.span, message)
                        .with_rule(RuleId::LIB_03),
                );
                None
            }
        }
    }

    /// Returns the position of an attribute of a class, after reporting a name the class
    /// does not have, or names as a method.
    fn attribute_index(&mut self, class: ClassId, attribute: &Identifier) -> Option<usize> {
        let info = &self.program.classes[class.0 as usize];
        if let Some(index) = info.attribute(&attribute.name) {
            return Some(index);
        }
        let (message, rule) = if self.program.method(class, &attribute.name).is_some() {
            (
                format!(
                    "using the method `{}.{}` as a value is planned for TyPy 1.0 (FUN-05)",
                    info.name, attribute.name
                ),
                RuleId::FUN_05,
            )
        } else {
            (
                format!("`{}` has no attribute `{}`", info.name, attribute.name),
                RuleId::CLS_02,
            )
        };
        let code = if rule == RuleId::FUN_05 { 201 } else { 414 };
        self.diagnostics.push(
            Diagnostic::new(DiagnosticCode::error(code), attribute.span, message).with_rule(rule),
        );
        None
    }

    /// Checks `value.attribute`, read as a value.
    pub(super) fn attribute(
        &mut self,
        value: &Expr,
        attribute: &Identifier,
        span: Span,
    ) -> TypedExpr {
        if let Some(module) = self.module_of(value) {
            if is_builtin_module(&module) {
                return self.builtin_attribute(&module, attribute, span);
            }
            let read = self.module_attribute(&module, &attribute.name, span);
            return self.through_lazy_module(value, read);
        }
        let object = self.instance_object(value);
        if let Type::TypeObject(_) = object.ty {
            return self.type_object_attribute(object, attribute, span);
        }
        let Some((object, class)) = self.as_instance(object) else {
            return Self::invalid_call(span);
        };
        let Some(index) = self.attribute_index(class, attribute) else {
            return Self::invalid_call(span);
        };
        if self.initialized_instance(value).is_some()
            && self
                .assigned
                .as_ref()
                .is_some_and(|facts| !facts.contains(&Fact::Attribute(index)))
        {
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(309),
                    span,
                    format!("`{}` is read before `__init__` assigns it", attribute.name),
                )
                .with_rule(RuleId::CLS_01),
                span,
            );
        }
        let info = &self.program.classes[class.0 as usize];
        let Some(ty) = self.program.attribute_ty(class, index) else {
            let message = format!(
                "the type of `{}.{}` is not known here, because `{}.__init__` gives it and is \
                 checked later",
                info.name, attribute.name, info.name
            );
            let help = format!(
                "declare it in the class body, such as `{}: int`",
                attribute.name
            );
            return self.error(
                Diagnostic::new(DiagnosticCode::error(310), attribute.span, message)
                    .with_rule(RuleId::CLS_01)
                    .with_help(help),
                span,
            );
        };
        Self::typed(
            TypedExprKind::Attribute {
                object: Box::new(object),
                index,
            },
            ty,
            span,
        )
    }

    /// Returns the type an attribute target already has, which a value without a type of its
    /// own, such as `[]`, takes.
    pub(super) fn attribute_type(&self, value: &Expr, attribute: &Identifier) -> Option<Type> {
        let Expr::Name { id, .. } = value else {
            return None;
        };
        let class = match self.initialized_instance(value) {
            Some((initializing, _)) => initializing.class,
            None => self.types.get(self.locals.get(id)?)?.class_id()?,
        };
        let index = self.program.classes[class.0 as usize].attribute(&attribute.name)?;
        self.program.attribute_ty(class, index)
    }

    /// Checks `value.attribute = ...` with a value of type `ty`, or with the annotation
    /// `declared`. In `__init__` the first assignment of an attribute the class body does not
    /// declare gives it its type; anywhere else only an attribute of the class is assigned
    /// (CLS-02).
    pub(super) fn attribute_target(
        &mut self,
        value: &Expr,
        attribute: &Identifier,
        ty: &Type,
        declared: Option<Type>,
        span: Span,
    ) -> Option<AssignTarget> {
        let (object, class) = self.instance(value)?;
        let index = self.attribute_index(class, attribute)?;
        let initializing = self.initialized_instance(value).is_some();
        // The attribute counts as assigned even when the value is wrong, so one cause gives
        // one error.
        if initializing && let Some(facts) = &mut self.assigned {
            facts.insert(Fact::Attribute(index));
        }
        // An inherited attribute takes its type from the class that declares it.
        let owner = self.program.declaring(class, index);
        let info = &mut self.program.classes[owner.0 as usize];
        let attribute_type = match (&info.attributes[index].ty, declared) {
            (None, Some(declared)) if initializing => {
                info.attributes[index].ty = Some(declared.clone());
                declared
            }
            (None, None) if initializing => {
                if matches!(ty, Type::NoneType) {
                    let message = format!(
                        "`{}` is first assigned `None`, which does not give it a type",
                        attribute.name
                    );
                    let help = format!(
                        "annotate it, such as `self.{}: int | None = None`",
                        attribute.name
                    );
                    self.diagnostics.push(
                        Diagnostic::new(DiagnosticCode::error(410), span, message)
                            .with_rule(RuleId::TYP_05)
                            .with_help(help),
                    );
                    return None;
                }
                info.attributes[index].ty = Some(ty.clone());
                ty.clone()
            }
            (Some(existing), declared) => {
                let existing = existing.clone();
                if let Some(declared) = declared
                    && !declared.matches(&existing)
                {
                    self.diagnostics.push(
                        Diagnostic::new(
                            DiagnosticCode::error(402),
                            span,
                            format!(
                                "`{}` is {} and is declared again as {declared}",
                                attribute.name,
                                existing.with_article()
                            ),
                        )
                        .with_rule(RuleId::TYP_03),
                    );
                }
                existing
            }
            (None, _) => {
                let message = format!(
                    "the type of `{}.{}` is not known here, because `{}.__init__` gives it and \
                     is checked later",
                    info.name, attribute.name, info.name
                );
                self.diagnostics.push(
                    Diagnostic::new(DiagnosticCode::error(310), attribute.span, message)
                        .with_rule(RuleId::CLS_01),
                );
                return None;
            }
        };
        if !ty.is_assignable_to(&attribute_type) {
            self.diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(402),
                    span,
                    format!(
                        "`{}` is {} and is assigned {}",
                        attribute.name,
                        attribute_type.with_article(),
                        ty.with_article()
                    ),
                )
                .with_rule(RuleId::TYP_03),
            );
        }
        Some(AssignTarget::Attribute {
            object: Box::new(object),
            index,
        })
    }

    /// Checks `Class(arguments)`: the arguments of `__init__`, without the instance, or none
    /// for a class without `__init__`. An exception class without `__init__` takes positional
    /// arguments of any type, as its built-in base does.
    pub(super) fn new_instance(
        &mut self,
        class: ClassId,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        let name = self.program.classes[class.0 as usize].name.clone();
        let init = self.program.method(class, "__init__");
        let ty = self.program.instance_type(class);
        if let Type::Exception(exception) = &ty {
            let exception = exception.clone();
            return match init {
                Some(function) => {
                    match self.bind_arguments(function, 1, &name, arguments, keywords, span) {
                        Ok(bound) => Self::typed(
                            TypedExprKind::NewException {
                                class: exception,
                                arguments: Vec::new(),
                                init: Some(Box::new(InitCall {
                                    function,
                                    arguments: bound,
                                    positional: arguments.len(),
                                })),
                            },
                            ty,
                            span,
                        ),
                        Err(error) => error,
                    }
                }
                None => self.exception_with_arguments(exception, arguments, keywords, span),
            };
        }
        let arguments = match init {
            Some(init) => match self.bind_arguments(init, 1, &name, arguments, keywords, span) {
                Ok(arguments) => arguments,
                Err(error) => return error,
            },
            None if arguments.is_empty() && keywords.is_empty() => Vec::new(),
            None => {
                for argument in arguments {
                    self.expression(argument);
                }
                return self.error(
                    Diagnostic::new(
                        DiagnosticCode::error(404),
                        span,
                        format!("`{name}` has no `__init__`, so it takes no arguments"),
                    )
                    .with_rule(RuleId::TYP_01),
                    span,
                );
            }
        };
        Self::typed(
            TypedExprKind::New {
                class,
                init,
                arguments,
            },
            ty,
            span,
        )
    }

    /// Checks `receiver.name(arguments)` on an instance: a call of the method with the
    /// instance as its first argument.
    pub(super) fn instance_method_call(
        &mut self,
        receiver: TypedExpr,
        class: ClassId,
        name: &Identifier,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        let found = self.program.method(class, &name.name);
        let info = &self.program.classes[class.0 as usize];
        let Some(method) = found else {
            let message = if info.attribute(&name.name).is_some() {
                format!(
                    "`{}.{}` is an attribute, and calling the value it holds is planned for \
                     TyPy 1.0 (FUN-05)",
                    info.name, name.name
                )
            } else {
                format!("`{}` has no method `{}`", info.name, name.name)
            };
            for argument in arguments {
                self.expression(argument);
            }
            return self.error(
                Diagnostic::new(DiagnosticCode::error(414), name.span, message)
                    .with_rule(RuleId::CLS_02),
                span,
            );
        };
        let qualified = format!("{}.{}", info.name, name.name);
        let mut bound = vec![Argument {
            parameter: 0,
            value: receiver,
        }];
        match self.bind_arguments(method, 1, &qualified, arguments, keywords, span) {
            Ok(arguments) => bound.extend(arguments),
            Err(error) => return error,
        }
        let returns = self.program.signatures[method.0 as usize].returns.clone();
        Self::typed(
            TypedExprKind::Call {
                function: method,
                arguments: bound,
            },
            returns,
            span,
        )
    }
}
