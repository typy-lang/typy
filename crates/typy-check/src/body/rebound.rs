//! Module-level names that imports bind to more than one function or class, or that an import
//! inside a block binds: a hidden module variable holds the number of the binding import that
//! ran last, and a call or a read of the name tests it (MOD-12, ADR-0036).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{CompareOperator, Expr, Keyword};

use super::{BodyChecker, ModuleFunction, Program};
use crate::exception::{BuiltinException, ExceptionClass};
use crate::typed::{
    ClassId, FunctionId, ImportBinding, SymbolId, TypedConstant, TypedExpr, TypedExprKind,
};
use crate::types::Type;

/// What an import binds a name to that a call reads when compiling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Bound {
    Function(FunctionId),
    Class(ClassId),
    ModuleFunction(ModuleFunction),
}

/// One import that binds a name the binding variable tracks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Binding {
    /// What the import binds the name to.
    pub(crate) target: Bound,
    /// Where the import names it, which tells the import apart.
    pub(crate) span: Span,
    /// Whether the import is lazy, so a call runs it first.
    pub(crate) lazy: bool,
}

/// A name whose binding variable a call tests.
#[derive(Clone, Debug)]
pub(crate) struct Rebound {
    /// The module variable that holds the number of the binding import that ran last, from 1,
    /// and 0 while none has run.
    pub(crate) tag: SymbolId,
    /// The imports that bind the name, in source order.
    pub(crate) bindings: Vec<Binding>,
}

impl Rebound {
    /// Returns true when the imports bind the name to more than one function or class.
    pub(super) fn several(&self) -> bool {
        self.bindings
            .iter()
            .any(|binding| binding.target != self.bindings[0].target)
    }
}

/// Returns the name of the binding variable of `name`.
pub(crate) fn binding_variable(name: &str) -> String {
    format!("<bound {name}>")
}

impl Program {
    /// Notes the names of the module being checked whose binding variables calls test. Each
    /// variable is an `int`, and its `NameError` names the name itself.
    pub(crate) fn set_rebound(&mut self, names: Vec<(String, Vec<Binding>)>) {
        self.rebound.clear();
        self.rebound_reify.clear();
        for (name, bindings) in names {
            let Some(tag) = self.module_symbols.get(&binding_variable(&name)).copied() else {
                continue;
            };
            let symbol = &mut self.symbols[tag.0 as usize];
            symbol.ty = Type::Int;
            symbol.name.clone_from(&name);
            self.rebound.insert(name, Rebound { tag, bindings });
        }
    }

    /// Returns the statements that start each binding variable at 0, when the module body
    /// starts.
    pub(super) fn rebound_starts(&self, span: Span) -> Vec<crate::typed::TypedStmt> {
        self.rebound
            .values()
            .map(|rebound| crate::typed::TypedStmt::Assign {
                targets: vec![crate::typed::AssignTarget::Name(rebound.tag)],
                value: number(0, span),
                span,
            })
            .collect()
    }
}

/// Returns the `int` literal `value`.
fn number(value: usize, span: Span) -> TypedExpr {
    BodyChecker::typed(
        TypedExprKind::Constant(TypedConstant::Int(value.to_string())),
        Type::Int,
        span,
    )
}

impl BodyChecker<'_> {
    /// Returns the binding that stores the number of the import naming `name` at `span` in
    /// the name's binding variable, when the module tracks the name.
    pub(super) fn rebound_binding(&mut self, name: &str, span: Span) -> Option<ImportBinding> {
        if self.in_function() {
            return None;
        }
        let rebound = self.program.rebound.get(name)?;
        let position = rebound
            .bindings
            .iter()
            .position(|binding| binding.span == span)?;
        let (tag, value) = (rebound.tag, number(position + 1, span));
        let variable = Expr::Name {
            id: binding_variable(name),
            span,
        };
        self.assign(&variable, Type::Int, &value);
        Some(ImportBinding::Expression { target: tag, value })
    }

    /// Notes the function that runs the lazy import naming `name` at `span`, which a call
    /// through that import runs first.
    pub(super) fn note_rebound_reify(&mut self, name: &str, span: Span, reify: FunctionId) {
        if self.program.rebound.contains_key(name) {
            self.program.rebound_reify.insert(span, reify);
        }
    }

    /// Returns the tracked name `name` unless a variable or a function's own import hides it.
    fn tracked(&self, name: &str) -> Option<Rebound> {
        if self.locals.contains_key(name) {
            return None;
        }
        self.program.rebound.get(name).cloned()
    }

    /// Returns `name(arguments)` of a tracked name: a test of its binding variable for each
    /// binding import, whose arm calls what that import binds, and `NameError` when none ran.
    pub(super) fn rebound_call(
        &mut self,
        name: &str,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> Option<TypedExpr> {
        let rebound = self.tracked(name)?;
        let arms: Vec<TypedExpr> = rebound
            .bindings
            .iter()
            .map(|binding| {
                let call = match binding.target {
                    Bound::Function(function) => {
                        self.call_function(function, name, arguments, keywords, span)
                    }
                    Bound::Class(class) => self.new_instance(class, arguments, keywords, span),
                    Bound::ModuleFunction(function) => {
                        self.module_function_call(function, arguments, keywords, span)
                    }
                };
                self.through_binding(binding, call)
            })
            .collect();
        Some(self.chosen(name, &rebound, arms, span))
    }

    /// Returns the tracked name `name` read as a value: what the binding import that ran last
    /// binds, guarded as a call is, when every import binds one function or class. A name
    /// bound to several is planned (MOD-14).
    pub(super) fn rebound_read(&mut self, name: &str, span: Span) -> Option<TypedExpr> {
        let rebound = self.tracked(name)?;
        if rebound.several() {
            return Some(self.error(rebound_planned(name, "read as a value", span), span));
        }
        let read = match rebound.bindings[0].target {
            Bound::Class(_) => self.type_object_named(name, span)?,
            Bound::Function(_) | Bound::ModuleFunction(_) => return None,
        };
        let arms = rebound
            .bindings
            .iter()
            .map(|binding| self.through_binding(binding, read.clone()))
            .collect();
        Some(self.chosen(name, &rebound, arms, span))
    }

    /// Wraps `value` so it first runs the lazy import of `binding`, when it is lazy.
    fn through_binding(&self, binding: &Binding, value: TypedExpr) -> TypedExpr {
        match self.program.rebound_reify.get(&binding.span) {
            Some(reify) if binding.lazy => {
                let (ty, span) = (value.ty.clone(), value.span);
                Self::typed(
                    TypedExprKind::Lazy {
                        reify: *reify,
                        value: Box::new(value),
                    },
                    ty,
                    span,
                )
            }
            _ => value,
        }
    }

    /// Returns the arm of the binding import that ran last, by the binding variable's
    /// number, or `NameError` when none has run.
    fn chosen(
        &mut self,
        name: &str,
        rebound: &Rebound,
        arms: Vec<TypedExpr>,
        span: Span,
    ) -> TypedExpr {
        if arms.iter().any(|arm| arm.ty.has_error()) {
            return Self::typed(
                TypedExprKind::Constant(TypedConstant::None),
                Type::Error,
                span,
            );
        }
        let ty = Type::union(arms.iter().map(|arm| arm.ty.clone()).collect());
        let message = format!("name '{name}' is not defined");
        let class = ExceptionClass::Builtin(BuiltinException::NameError);
        self.program.record_creation(&class, &[Type::Str]);
        let exception = Self::typed(
            TypedExprKind::NewException {
                class: class.clone(),
                arguments: vec![Self::typed(
                    TypedExprKind::Constant(TypedConstant::Str(message)),
                    Type::Str,
                    span,
                )],
                init: None,
            },
            Type::Exception(class),
            span,
        );
        let mut chosen = Self::typed(TypedExprKind::Raise(Box::new(exception)), ty.clone(), span);
        let tag = Self::typed(TypedExprKind::ModuleVariable(rebound.tag), Type::Int, span);
        for (position, arm) in arms.into_iter().enumerate().rev() {
            let test = Self::typed(
                TypedExprKind::Compare {
                    left: Box::new(tag.clone()),
                    operators: vec![CompareOperator::Eq],
                    comparators: vec![number(position + 1, span)],
                },
                Type::Bool,
                span,
            );
            chosen = Self::typed(
                TypedExprKind::IfExp {
                    test: Box::new(test),
                    body: Box::new(Self::converted(arm, Some(&ty))),
                    orelse: Box::new(chosen),
                },
                ty.clone(),
                span,
            );
        }
        chosen
    }
}

/// Returns the error for a use of a name bound to several functions or classes that only a
/// call has.
pub(crate) fn rebound_planned(name: &str, what: &str, span: Span) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::error(201),
        span,
        format!(
            "`{name}`, which imports bind to more than one function or class, {what} is \
             planned for TyPy 1.0 (MOD-14)"
        ),
    )
    .with_rule(RuleId::MOD_14)
    .with_help("import each under a name of its own")
}
