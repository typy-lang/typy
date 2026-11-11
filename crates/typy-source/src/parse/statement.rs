//! Converts Ruff's statements, parameters and import names.

use ruff_python_ast as ast;
use ruff_text_size::{Ranged, TextRange};

use super::Converter;
use super::operator::binary_operator;
use crate::syntax::{Alias, ExceptHandler, NodeKind, Parameter, ParameterKind, Stmt, WithItem};

impl Converter<'_> {
    pub(super) fn statements(&self, statements: &[ast::Stmt]) -> Vec<Stmt> {
        statements
            .iter()
            .map(|statement| self.statement(statement))
            .collect()
    }

    pub(super) fn other_statement(&self, kind: NodeKind, range: TextRange) -> Stmt {
        Stmt::Other {
            kind,
            span: self.span(range),
        }
    }

    #[allow(
        clippy::too_many_lines,
        reason = "one arm per statement kind of Python's grammar"
    )]
    pub(super) fn statement(&self, statement: &ast::Stmt) -> Stmt {
        let span = self.span(statement.range());
        match statement {
            ast::Stmt::FunctionDef(node) if node.is_async => {
                self.other_statement(NodeKind::AsyncFunctionDef, node.range)
            }
            ast::Stmt::FunctionDef(node) => Stmt::FunctionDef {
                name: self.identifier(&node.name),
                parameters: self.parameters(&node.parameters),
                returns: node
                    .returns
                    .as_deref()
                    .map(|returns| self.annotation(returns)),
                body: self.statements(&node.body),
                decorators: node
                    .decorator_list
                    .iter()
                    .map(|decorator| self.expression(&decorator.expression))
                    .collect(),
                type_parameter_count: node
                    .type_params
                    .as_ref()
                    .map_or(0, |parameters| parameters.type_params.len()),
                span,
            },
            ast::Stmt::ClassDef(node) => Stmt::ClassDef {
                name: self.identifier(&node.name),
                bases: node
                    .arguments
                    .as_deref()
                    .map(|arguments| {
                        arguments
                            .args
                            .iter()
                            .map(|base| self.expression(base))
                            .collect()
                    })
                    .unwrap_or_default(),
                keyword_count: node
                    .arguments
                    .as_deref()
                    .map_or(0, |arguments| arguments.keywords.len()),
                body: self.statements(&node.body),
                decorators: node
                    .decorator_list
                    .iter()
                    .map(|decorator| self.expression(&decorator.expression))
                    .collect(),
                type_parameter_count: node
                    .type_params
                    .as_ref()
                    .map_or(0, |parameters| parameters.type_params.len()),
                span,
            },
            ast::Stmt::Return(node) => Stmt::Return {
                value: node.value.as_deref().map(|value| self.expression(value)),
                span,
            },
            ast::Stmt::Delete(node) => Stmt::Delete {
                targets: node
                    .targets
                    .iter()
                    .map(|target| self.expression(target))
                    .collect(),
                span,
            },
            ast::Stmt::TypeAlias(node) => self.other_statement(NodeKind::TypeAlias, node.range),
            ast::Stmt::Assign(node) => Stmt::Assign {
                targets: node
                    .targets
                    .iter()
                    .map(|target| self.expression(target))
                    .collect(),
                value: self.expression(&node.value),
                span,
            },
            ast::Stmt::AugAssign(node) => Stmt::AugAssign {
                target: self.expression(&node.target),
                operator: binary_operator(node.op),
                value: self.expression(&node.value),
                span,
            },
            ast::Stmt::AnnAssign(node) => Stmt::AnnAssign {
                target: self.expression(&node.target),
                annotation: self.annotation(&node.annotation),
                value: node.value.as_deref().map(|value| self.expression(value)),
                simple: node.simple,
                span,
            },
            ast::Stmt::For(node) if node.is_async => {
                self.other_statement(NodeKind::AsyncFor, node.range)
            }
            ast::Stmt::For(node) => Stmt::For {
                target: self.expression(&node.target),
                iter: self.expression(&node.iter),
                body: self.statements(&node.body),
                orelse: self.statements(&node.orelse),
                span,
            },
            ast::Stmt::While(node) => Stmt::While {
                test: self.expression(&node.test),
                body: self.statements(&node.body),
                orelse: self.statements(&node.orelse),
                span,
            },
            ast::Stmt::If(node) => Stmt::If {
                test: self.expression(&node.test),
                body: self.statements(&node.body),
                orelse: self.elif_else(&node.elif_else_clauses),
                span,
            },
            ast::Stmt::With(node) if node.is_async => {
                self.other_statement(NodeKind::AsyncWith, node.range)
            }
            ast::Stmt::With(node) => Stmt::With {
                items: node
                    .items
                    .iter()
                    .map(|item| WithItem {
                        context: self.expression(&item.context_expr),
                        target: item
                            .optional_vars
                            .as_deref()
                            .map(|target| self.expression(target)),
                    })
                    .collect(),
                body: self.statements(&node.body),
                span,
            },
            ast::Stmt::Match(node) => self.other_statement(NodeKind::Match, node.range),
            ast::Stmt::Raise(node) => Stmt::Raise {
                exception: node.exc.as_deref().map(|value| self.expression(value)),
                cause: node.cause.as_deref().map(|value| self.expression(value)),
                span,
            },
            ast::Stmt::Try(node) if node.is_star => {
                self.other_statement(NodeKind::TryStar, node.range)
            }
            ast::Stmt::Try(node) => Stmt::Try {
                body: self.statements(&node.body),
                handlers: node
                    .handlers
                    .iter()
                    .map(|handler| {
                        let ast::ExceptHandler::ExceptHandler(handler) = handler;
                        ExceptHandler {
                            class: handler.type_.as_deref().map(|class| self.expression(class)),
                            name: handler.name.as_ref().map(|name| self.identifier(name)),
                            body: self.statements(&handler.body),
                            span: self.span(handler.range),
                        }
                    })
                    .collect(),
                orelse: self.statements(&node.orelse),
                finalbody: self.statements(&node.finalbody),
                span,
            },
            ast::Stmt::Assert(node) => Stmt::Assert {
                test: self.expression(&node.test),
                message: node.msg.as_deref().map(|message| self.expression(message)),
                span,
            },
            ast::Stmt::Import(node) => Stmt::Import {
                names: node.names.iter().map(|alias| self.alias(alias)).collect(),
                is_lazy: node.is_lazy,
                span,
            },
            ast::Stmt::ImportFrom(node) => Stmt::ImportFrom {
                module: node.module.as_ref().map(|module| self.identifier(module)),
                names: node.names.iter().map(|alias| self.alias(alias)).collect(),
                level: node.level,
                is_lazy: node.is_lazy,
                span,
            },
            ast::Stmt::Global(node) => self.other_statement(NodeKind::Global, node.range),
            ast::Stmt::Nonlocal(node) => self.other_statement(NodeKind::Nonlocal, node.range),
            ast::Stmt::Expr(node) => Stmt::Expr {
                value: self.expression(&node.value),
                span,
            },
            ast::Stmt::Pass(_) => Stmt::Pass { span },
            ast::Stmt::Break(_) => Stmt::Break { span },
            ast::Stmt::Continue(_) => Stmt::Continue { span },
            ast::Stmt::IpyEscapeCommand(node) => {
                self.other_statement(NodeKind::IpyEscapeCommand, node.range)
            }
        }
    }

    /// Turns Ruff's flat list of `elif` and `else` clauses into nested `If` nodes, as CPython's `ast` has them.
    pub(super) fn elif_else(&self, clauses: &[ast::ElifElseClause]) -> Vec<Stmt> {
        let Some((first, rest)) = clauses.split_first() else {
            return Vec::new();
        };
        match &first.test {
            Some(test) => vec![Stmt::If {
                test: self.expression(test),
                body: self.statements(&first.body),
                orelse: self.elif_else(rest),
                span: self.span(first.range),
            }],
            None => self.statements(&first.body),
        }
    }

    pub(super) fn alias(&self, alias: &ast::Alias) -> Alias {
        Alias {
            name: self.identifier(&alias.name),
            as_name: alias.asname.as_ref().map(|name| self.identifier(name)),
        }
    }

    pub(super) fn parameters(&self, parameters: &ast::Parameters) -> Vec<Parameter> {
        let with_default = |kind| {
            move |parameter: &ast::ParameterWithDefault| Parameter {
                name: self.identifier(&parameter.parameter.name),
                annotation: parameter
                    .parameter
                    .annotation
                    .as_deref()
                    .map(|annotation| self.annotation(annotation)),
                default: parameter
                    .default
                    .as_deref()
                    .map(|default| self.expression(default)),
                kind,
                span: self.span(parameter.range),
            }
        };
        let variadic = |kind| {
            move |parameter: &ast::Parameter| Parameter {
                name: self.identifier(&parameter.name),
                annotation: parameter
                    .annotation
                    .as_deref()
                    .map(|annotation| self.annotation(annotation)),
                default: None,
                kind,
                span: self.span(parameter.range),
            }
        };
        let mut result: Vec<Parameter> = parameters
            .posonlyargs
            .iter()
            .map(with_default(ParameterKind::PositionalOnly))
            .collect();
        result.extend(
            parameters
                .args
                .iter()
                .map(with_default(ParameterKind::Normal)),
        );
        result.extend(
            parameters
                .vararg
                .as_deref()
                .into_iter()
                .map(variadic(ParameterKind::VarPositional)),
        );
        result.extend(
            parameters
                .kwonlyargs
                .iter()
                .map(with_default(ParameterKind::KeywordOnly)),
        );
        result.extend(
            parameters
                .kwarg
                .as_deref()
                .into_iter()
                .map(variadic(ParameterKind::VarKeyword)),
        );
        result
    }
}
