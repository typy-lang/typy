//! Checks calls of exception classes and `raise` statements (SMT-06, EXC-07, EXC-12,
//! ADR-0013).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Constant, ExceptHandler, Expr, Identifier, Keyword, Stmt};

use super::BodyChecker;
use super::flow::{Fact, meet};
use crate::exception::{BuiltinException, ExceptionClass, common_base};
use crate::typed::{
    Argument, RaiseCause, SymbolKind, TypedExpr, TypedExprKind, TypedHandler, TypedStmt,
};
use crate::types::{Type, TypeObjectOf};

impl BodyChecker<'_> {
    /// Checks a call of a built-in exception class: positional arguments of any type, which
    /// the exception keeps.
    pub(super) fn new_exception(
        &mut self,
        class: BuiltinException,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        if !class.can_be_called() {
            for argument in arguments {
                self.expression(argument);
            }
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(417),
                    span,
                    format!(
                        "`{}` cannot be called in TyPy; its five arguments are not checked",
                        class.name()
                    ),
                )
                .with_rule(RuleId::EXC_12)
                .with_help("raise `ValueError` or a class of your own, and catch this one"),
                span,
            );
        }
        self.exception_with_arguments(ExceptionClass::Builtin(class), arguments, keywords, span)
    }

    /// Checks a call of an exception class without `__init__` of the program's: positional
    /// arguments of any type, which the exception keeps.
    pub(super) fn exception_with_arguments(
        &mut self,
        class: ExceptionClass,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        let values: Vec<TypedExpr> = arguments
            .iter()
            .map(|value| self.expression(value))
            .collect();
        if let Some(keyword) = keywords.first() {
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    keyword.span,
                    format!("`{}` takes positional arguments only", class.name()),
                )
                .with_rule(RuleId::EXC_01),
                span,
            );
        }
        Self::typed(
            TypedExprKind::NewException {
                class: class.clone(),
                arguments: values,
                init: None,
            },
            Type::Exception(class),
            span,
        )
    }

    /// Checks `raise`, `raise exception` and `raise exception from cause`. Nothing after it
    /// runs, so the point after it cannot be reached.
    pub(super) fn raise_statement(
        &mut self,
        exception: Option<&Expr>,
        cause: Option<&Expr>,
        span: Span,
    ) -> TypedStmt {
        let Some(exception) = exception else {
            if self.handling == 0 {
                self.diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::error(416),
                        span,
                        "a bare `raise` is accepted only inside an `except` clause",
                    )
                    .with_rule(RuleId::EXC_07)
                    .with_help("name the exception to raise, such as `raise ValueError(...)`"),
                );
            }
            self.assigned = None;
            return TypedStmt::Raise {
                exception: None,
                cause: RaiseCause::Absent,
                span,
            };
        };
        let exception = self.exception_operand(exception, "`raise`");
        let cause = match cause {
            None => RaiseCause::Absent,
            Some(Expr::Constant {
                value: Constant::None,
                ..
            }) => RaiseCause::None,
            Some(cause) => RaiseCause::Exception(self.exception_operand(cause, "`from`")),
        };
        self.assigned = None;
        TypedStmt::Raise {
            exception: Some(exception),
            cause,
            span,
        }
    }

    /// Checks what `raise` or `from` names: an exception, or an exception class, which is
    /// called with no arguments, as CPython does.
    fn exception_operand(&mut self, operand: &Expr, what: &str) -> TypedExpr {
        let value = self.expression(operand);
        match &value.ty {
            Type::Exception(_) | Type::Error => value,
            Type::TypeObject(Some(of)) if matches!(**of, TypeObjectOf::Exception(_)) => {
                let TypeObjectOf::Exception(class) = (**of).clone() else {
                    unreachable!("matched above");
                };
                self.called_class(&class, value.span)
            }
            other => {
                let message = format!(
                    "{what} takes an exception or an exception class, and this is {}",
                    other.with_article()
                );
                self.error(
                    Diagnostic::new(DiagnosticCode::error(415), value.span, message)
                        .with_rule(RuleId::SMT_06),
                    value.span,
                )
            }
        }
    }

    /// Returns a new exception of `class`, called with no arguments, in place of the class.
    fn called_class(&mut self, class: &ExceptionClass, span: Span) -> TypedExpr {
        match class {
            ExceptionClass::Builtin(builtin) => self.new_exception(*builtin, &[], &[], span),
            ExceptionClass::Class(class) => self.new_instance(class.id, &[], &[], span),
        }
    }

    /// Checks `try` with its `except` clauses, its `else` block and its `finally` block. The
    /// body may stop at any statement, so each clause starts from what held before the body,
    /// less what the body may have set to `None`; `else` follows the body. `finally` runs on
    /// every way out, so it starts from what held before the `try`, and what it assigns holds
    /// after the `try` too.
    pub(super) fn try_statement(
        &mut self,
        body: &[Stmt],
        handlers: &[ExceptHandler],
        orelse: &[Stmt],
        finalbody: &[Stmt],
        span: Span,
    ) -> TypedStmt {
        let before = self.assigned.clone();
        let typed_body = self.block(body);
        let orelse_typed = self.block(orelse);
        let mut ends = self.assigned.take();
        let mut typed_handlers = Vec::new();
        for handler in handlers {
            self.assigned.clone_from(&before);
            self.forget_not_none(body);
            typed_handlers.push(self.handler(handler));
            ends = meet(ends, self.assigned.take());
        }
        self.assigned.clone_from(&before);
        for block in [body, orelse] {
            self.forget_not_none(block);
        }
        for handler in handlers {
            self.forget_not_none(&handler.body);
        }
        self.finally_loops.push(self.breaks.len());
        let finalbody = self.block(finalbody);
        self.finally_loops.pop();
        let after_finally = self.assigned.take();
        self.assigned = match (ends, after_finally, before) {
            // A `finally` that always raises leaves nothing after the `try` reachable.
            (_, None, _) => None,
            (Some(ends), Some(finally), Some(before)) => {
                Some(ends.union(&(&finally - &before)).copied().collect())
            }
            (ends, _, _) => ends,
        };
        TypedStmt::Try {
            body: typed_body,
            handlers: typed_handlers,
            orelse: orelse_typed,
            finalbody,
            span,
        }
    }

    /// Reports a `return`, `break` or `continue` that would leave a `finally` block, which
    /// would drop an exception on its way out (EXC-06).
    pub(super) fn check_stays_in_finally(&mut self, statement: &str, span: Span) {
        let Some(loops) = self.finally_loops.last() else {
            return;
        };
        if statement != "return" && *loops < self.breaks.len() {
            return;
        }
        self.diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(419),
                span,
                format!("`{statement}` cannot leave a `finally` block"),
            )
            .with_rule(RuleId::EXC_06)
            .with_help("move it after the `try` statement, as CPython 3.14 warns about it"),
        );
    }

    /// Checks one `except` clause. Its name is a variable of the clause alone, as CPython
    /// deletes it when the clause ends, so a variable of that name outside is unassigned after.
    fn handler(&mut self, handler: &ExceptHandler) -> TypedHandler {
        let classes = self.handler_classes(handler.class.as_ref());
        let bound = handler.name.as_ref().map(|name| {
            let ty = Type::Exception(common_base(&classes));
            let symbol =
                self.program
                    .add_symbol(&name.name, SymbolKind::Local, ty.clone(), name.span);
            self.local_order.push(symbol);
            self.types.insert(symbol, ty);
            if let Some(known) = &mut self.assigned {
                known.insert(Fact::Assigned(symbol));
            }
            let outer = self.locals.insert(name.name.clone(), symbol);
            (name.name.clone(), symbol, outer)
        });
        self.handling += 1;
        let body = self.block(&handler.body);
        self.handling -= 1;
        let name = bound.map(|(name, symbol, outer)| {
            match outer {
                Some(outer) => {
                    self.locals.insert(name, outer);
                    if let Some(known) = &mut self.assigned {
                        known.remove(&Fact::Assigned(outer));
                    }
                }
                None => {
                    self.locals.remove(&name);
                }
            }
            symbol
        });
        TypedHandler {
            classes,
            name,
            body,
            span: handler.span,
        }
    }

    /// Returns the classes an `except` clause names: one, a tuple of them, or `BaseException`
    /// for a bare `except`. Anything else is E0418 (EXC-09).
    fn handler_classes(&mut self, class: Option<&Expr>) -> Vec<ExceptionClass> {
        let Some(class) = class else {
            return vec![ExceptionClass::Builtin(BuiltinException::BaseException)];
        };
        let named: Vec<&Expr> = match class {
            Expr::Tuple { elements, .. } => elements.iter().collect(),
            other => vec![other],
        };
        named
            .into_iter()
            .filter_map(|expression| {
                let value = self.expression(expression);
                if let Type::TypeObject(Some(of)) = &value.ty
                    && let TypeObjectOf::Exception(class) = of.as_ref()
                {
                    return Some(class.clone());
                }
                if value.ty == Type::Error {
                    return None;
                }
                let message = format!(
                    "an `except` clause names exception classes, and this is {}",
                    value.ty.with_article()
                );
                self.diagnostics.push(
                    Diagnostic::new(DiagnosticCode::error(418), value.span, message)
                        .with_rule(RuleId::EXC_09)
                        .with_help("name a class such as `ValueError`, or a tuple of classes"),
                );
                None
            })
            .collect()
    }

    /// Checks `super().name(arguments)`, accepted as `super().__init__(...)` in the `__init__`
    /// of an exception class (CLS-14). With a base of the program that has `__init__`, it calls
    /// that `__init__` on the instance; otherwise it sets the exception's arguments, as
    /// `BaseException.__init__` does. Either way the inherited attributes are assigned after.
    pub(super) fn super_call(
        &mut self,
        name: &Identifier,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        let exception_init = self.initializing.filter(|initializing| {
            name.name == "__init__" && self.program.exceptions.contains_key(&initializing.class)
        });
        let Some(initializing) = exception_init else {
            for argument in arguments {
                self.expression(argument);
            }
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(201),
                    span,
                    "`super()` is planned for TyPy 0.4, beyond `super().__init__(...)` in the \
                     `__init__` of an exception class (CLS-14)",
                )
                .with_rule(RuleId::CLS_14),
                span,
            );
        };
        let instance = Self::typed(
            TypedExprKind::Local(initializing.instance),
            self.program.instance_type(initializing.class),
            span,
        );
        let info = &self.program.classes[initializing.class.0 as usize];
        let (base, inherited) = (info.base, info.inherited);
        let base_init = base.and_then(|base| self.program.method(base, "__init__"));
        let call = if let (Some(function), Some(base)) = (base_init, base) {
            let qualified = format!("{}.__init__", self.program.instance_type(base));
            match self.bind_arguments(function, 1, &qualified, arguments, keywords, span) {
                Ok(bound) => {
                    let mut all = vec![Argument {
                        parameter: 0,
                        value: instance,
                    }];
                    all.extend(bound);
                    Self::typed(
                        TypedExprKind::Call {
                            function,
                            arguments: all,
                        },
                        Type::NoneType,
                        span,
                    )
                }
                Err(error) => return error,
            }
        } else {
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
            Self::typed(
                TypedExprKind::SetArguments {
                    exception: Box::new(instance),
                    arguments: values,
                },
                Type::NoneType,
                span,
            )
        };
        if let Some(facts) = &mut self.assigned {
            facts.extend((0..inherited).map(Fact::Attribute));
        }
        call
    }

    /// Checks `assert test, message`, which raises `AssertionError(message)` when the test is
    /// false, in every build mode (SMT-04). What the test shows holds after it, as it does for
    /// an `if` whose other branch raises.
    pub(super) fn assert_statement(
        &mut self,
        test: &Expr,
        message: Option<&Expr>,
        span: Span,
    ) -> TypedStmt {
        let test = self.truth_test(test);
        let (holds, _) = super::flow::test_facts(&test);
        let exception = self.new_exception(
            BuiltinException::AssertionError,
            message.map(std::slice::from_ref).unwrap_or_default(),
            &[],
            span,
        );
        self.add_facts(holds);
        TypedStmt::Assert {
            test,
            exception,
            span,
        }
    }
}
