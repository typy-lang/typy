//! Parses a source file with `ruff_python_parser` and converts the result to TyPy's syntax tree.
//!
//! No type of `ruff_python_parser` or `ruff_python_ast` leaves this module (ADR-0005).

use ruff_python_ast::{self as ast, PythonVersion};
use ruff_python_parser::{Mode, ParseOptions, parse_unchecked};
use ruff_text_size::{Ranged, TextRange};
use typy_diagnostics::{Diagnostic, DiagnosticCode, FileId, RuleId, Span, TextOffset};

use crate::int_literal::decimal_digits;
use crate::settings::PythonFloor;
use crate::source_file::SourceFile;
use crate::syntax::{
    Alias, BinaryOperator, BoolOperator, CompareOperator, Constant, Expr, Identifier, Keyword,
    NodeKind, Parameter, ParameterKind, Stmt, SyntaxTree, UnaryOperator,
};

/// Parses `file` with `floor` as the target version; any syntax error, or syntax newer than the floor, is an E0103 (STD-01).
pub fn parse(file: &SourceFile, floor: PythonFloor) -> Result<SyntaxTree, Vec<Diagnostic>> {
    let version = PythonVersion {
        major: 3,
        minor: floor.minor(),
    };
    let options = ParseOptions::from(Mode::Module).with_target_version(version);
    let parsed = parse_unchecked(file.text(), options);
    let converter = Converter {
        file: file.id(),
        text: file.text(),
    };

    let mut errors: Vec<Diagnostic> = parsed
        .errors()
        .iter()
        .map(|error| converter.syntax_error(error.location, &error.error.to_string()))
        .collect();
    errors.extend(
        parsed
            .unsupported_syntax_errors()
            .iter()
            .map(|error| converter.syntax_error(error.range, &error.to_string())),
    );
    if !errors.is_empty() {
        return Err(errors);
    }
    let ast::Mod::Module(module) = parsed.into_syntax() else {
        unreachable!("Mode::Module always produces a module");
    };
    Ok(SyntaxTree {
        body: converter.statements(&module.body),
    })
}

/// Rewrites a parser message as one sentence that starts in lower case and has no final period (DIA-01).
fn sentence(message: &str) -> String {
    let trimmed = message.trim().trim_end_matches('.');
    let mut characters = trimmed.chars();
    match characters.next() {
        Some(first) => first.to_lowercase().chain(characters).collect(),
        None => "invalid syntax".to_owned(),
    }
}

struct Converter<'text> {
    file: FileId,
    text: &'text str,
}

impl Converter<'_> {
    fn span(&self, range: TextRange) -> Span {
        let start = TextOffset(range.start().to_u32());
        let end = TextOffset(range.end().to_u32());
        Span::new(self.file, start, end).expect("the parser produces ordered ranges")
    }

    fn syntax_error(&self, range: TextRange, message: &str) -> Diagnostic {
        Diagnostic::new(
            DiagnosticCode::error(103),
            self.span(range),
            sentence(message),
        )
        .with_rule(RuleId::STD_01)
    }

    fn identifier(&self, identifier: &ast::Identifier) -> Identifier {
        Identifier {
            name: identifier.as_str().to_owned(),
            span: self.span(identifier.range()),
        }
    }

    fn statements(&self, statements: &[ast::Stmt]) -> Vec<Stmt> {
        statements
            .iter()
            .map(|statement| self.statement(statement))
            .collect()
    }

    fn other_statement(&self, kind: NodeKind, range: TextRange) -> Stmt {
        Stmt::Other {
            kind,
            span: self.span(range),
        }
    }

    #[allow(
        clippy::too_many_lines,
        reason = "one arm per statement kind of Python's grammar"
    )]
    fn statement(&self, statement: &ast::Stmt) -> Stmt {
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
                    .map(|returns| self.expression(returns)),
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
            ast::Stmt::ClassDef(node) => self.other_statement(NodeKind::ClassDef, node.range),
            ast::Stmt::Return(node) => Stmt::Return {
                value: node.value.as_deref().map(|value| self.expression(value)),
                span,
            },
            ast::Stmt::Delete(node) => self.other_statement(NodeKind::Delete, node.range),
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
                annotation: self.expression(&node.annotation),
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
            ast::Stmt::With(node) => self.other_statement(NodeKind::With, node.range),
            ast::Stmt::Match(node) => self.other_statement(NodeKind::Match, node.range),
            ast::Stmt::Raise(node) => self.other_statement(NodeKind::Raise, node.range),
            ast::Stmt::Try(node) if node.is_star => {
                self.other_statement(NodeKind::TryStar, node.range)
            }
            ast::Stmt::Try(node) => self.other_statement(NodeKind::Try, node.range),
            ast::Stmt::Assert(node) => self.other_statement(NodeKind::Assert, node.range),
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
    fn elif_else(&self, clauses: &[ast::ElifElseClause]) -> Vec<Stmt> {
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

    fn alias(&self, alias: &ast::Alias) -> Alias {
        Alias {
            name: self.identifier(&alias.name),
            as_name: alias.asname.as_ref().map(|name| self.identifier(name)),
        }
    }

    fn parameters(&self, parameters: &ast::Parameters) -> Vec<Parameter> {
        let with_default = |kind| {
            move |parameter: &ast::ParameterWithDefault| Parameter {
                name: self.identifier(&parameter.parameter.name),
                annotation: parameter
                    .parameter
                    .annotation
                    .as_deref()
                    .map(|annotation| self.expression(annotation)),
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
                    .map(|annotation| self.expression(annotation)),
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

    fn other(&self, kind: NodeKind, range: TextRange) -> Expr {
        Expr::Other {
            kind,
            span: self.span(range),
        }
    }

    fn boxed(&self, expression: &ast::Expr) -> Box<Expr> {
        Box::new(self.expression(expression))
    }

    fn constant(&self, value: Constant, range: TextRange) -> Expr {
        Expr::Constant {
            value,
            span: self.span(range),
        }
    }

    #[allow(
        clippy::too_many_lines,
        reason = "one arm per expression kind of Python's grammar"
    )]
    fn expression(&self, expression: &ast::Expr) -> Expr {
        let span = self.span(expression.range());
        match expression {
            ast::Expr::BoolOp(node) => Expr::BoolOp {
                operator: match node.op {
                    ast::BoolOp::And => BoolOperator::And,
                    ast::BoolOp::Or => BoolOperator::Or,
                },
                values: node
                    .values
                    .iter()
                    .map(|value| self.expression(value))
                    .collect(),
                span,
            },
            ast::Expr::BinOp(node) => Expr::BinOp {
                left: self.boxed(&node.left),
                operator: binary_operator(node.op),
                right: self.boxed(&node.right),
                span,
            },
            ast::Expr::UnaryOp(node) => Expr::UnaryOp {
                operator: match node.op {
                    ast::UnaryOp::Invert => UnaryOperator::Invert,
                    ast::UnaryOp::Not => UnaryOperator::Not,
                    ast::UnaryOp::UAdd => UnaryOperator::UAdd,
                    ast::UnaryOp::USub => UnaryOperator::USub,
                },
                operand: self.boxed(&node.operand),
                span,
            },
            ast::Expr::If(node) => Expr::IfExp {
                test: self.boxed(&node.test),
                body: self.boxed(&node.body),
                orelse: self.boxed(&node.orelse),
                span,
            },
            ast::Expr::Compare(node) => {
                let (left, comparators) = node
                    .operands
                    .split_first()
                    .expect("a comparison has at least two operands");
                Expr::Compare {
                    left: self.boxed(left),
                    operators: node.ops.iter().copied().map(compare_operator).collect(),
                    comparators: comparators
                        .iter()
                        .map(|operand| self.expression(operand))
                        .collect(),
                    span,
                }
            }
            ast::Expr::Call(node) => Expr::Call {
                function: self.boxed(&node.func),
                arguments: node
                    .arguments
                    .args
                    .iter()
                    .map(|argument| self.expression(argument))
                    .collect(),
                keywords: node
                    .arguments
                    .keywords
                    .iter()
                    .map(|keyword| Keyword {
                        name: keyword.arg.as_ref().map(|name| self.identifier(name)),
                        value: self.expression(&keyword.value),
                        span: self.span(keyword.range),
                    })
                    .collect(),
                span,
            },
            ast::Expr::StringLiteral(node) => {
                self.constant(Constant::Str(node.value.to_str().to_owned()), node.range)
            }
            ast::Expr::BytesLiteral(node) => {
                self.constant(Constant::Bytes(node.value.bytes().collect()), node.range)
            }
            ast::Expr::NumberLiteral(node) => {
                let value = match &node.value {
                    ast::Number::Int(integer) => {
                        let source =
                            &self.text[node.range.start().to_usize()..node.range.end().to_usize()];
                        Constant::Int(decimal_digits(source).unwrap_or_else(|| integer.to_string()))
                    }
                    ast::Number::Float(value) => Constant::Float(*value),
                    ast::Number::Complex { imag, .. } => Constant::Complex { imaginary: *imag },
                };
                self.constant(value, node.range)
            }
            ast::Expr::BooleanLiteral(node) => {
                self.constant(Constant::Bool(node.value), node.range)
            }
            ast::Expr::NoneLiteral(node) => self.constant(Constant::None, node.range),
            ast::Expr::EllipsisLiteral(node) => self.constant(Constant::Ellipsis, node.range),
            ast::Expr::Subscript(node) => Expr::Subscript {
                value: self.boxed(&node.value),
                index: self.boxed(&node.slice),
                span,
            },
            ast::Expr::Name(node) => Expr::Name {
                id: node.id.as_str().to_owned(),
                span,
            },
            ast::Expr::Named(node) => self.other(NodeKind::NamedExpr, node.range),
            ast::Expr::Lambda(node) => self.other(NodeKind::Lambda, node.range),
            ast::Expr::Dict(node) => self.other(NodeKind::Dict, node.range),
            ast::Expr::Set(node) => self.other(NodeKind::Set, node.range),
            ast::Expr::ListComp(node) => self.other(NodeKind::ListComp, node.range),
            ast::Expr::SetComp(node) => self.other(NodeKind::SetComp, node.range),
            ast::Expr::DictComp(node) => self.other(NodeKind::DictComp, node.range),
            ast::Expr::Generator(node) => self.other(NodeKind::GeneratorExp, node.range),
            ast::Expr::Await(node) => self.other(NodeKind::Await, node.range),
            ast::Expr::Yield(node) => self.other(NodeKind::Yield, node.range),
            ast::Expr::YieldFrom(node) => self.other(NodeKind::YieldFrom, node.range),
            ast::Expr::FString(node) => self.other(NodeKind::JoinedStr, node.range),
            ast::Expr::TString(node) => self.other(NodeKind::TemplateStr, node.range),
            ast::Expr::Attribute(node) => self.other(NodeKind::Attribute, node.range),
            ast::Expr::Starred(node) => self.other(NodeKind::Starred, node.range),
            ast::Expr::List(node) => self.other(NodeKind::List, node.range),
            ast::Expr::Tuple(node) => self.other(NodeKind::Tuple, node.range),
            ast::Expr::Slice(node) => self.other(NodeKind::Slice, node.range),
            ast::Expr::IpyEscapeCommand(node) => self.other(NodeKind::IpyEscapeCommand, node.range),
        }
    }
}

fn binary_operator(operator: ast::Operator) -> BinaryOperator {
    match operator {
        ast::Operator::Add => BinaryOperator::Add,
        ast::Operator::Sub => BinaryOperator::Sub,
        ast::Operator::Mult => BinaryOperator::Mult,
        ast::Operator::MatMult => BinaryOperator::MatMult,
        ast::Operator::Div => BinaryOperator::Div,
        ast::Operator::Mod => BinaryOperator::Mod,
        ast::Operator::Pow => BinaryOperator::Pow,
        ast::Operator::LShift => BinaryOperator::LShift,
        ast::Operator::RShift => BinaryOperator::RShift,
        ast::Operator::BitOr => BinaryOperator::BitOr,
        ast::Operator::BitXor => BinaryOperator::BitXor,
        ast::Operator::BitAnd => BinaryOperator::BitAnd,
        ast::Operator::FloorDiv => BinaryOperator::FloorDiv,
    }
}

fn compare_operator(operator: ast::CmpOp) -> CompareOperator {
    match operator {
        ast::CmpOp::Eq => CompareOperator::Eq,
        ast::CmpOp::NotEq => CompareOperator::NotEq,
        ast::CmpOp::Lt => CompareOperator::Lt,
        ast::CmpOp::LtE => CompareOperator::LtE,
        ast::CmpOp::Gt => CompareOperator::Gt,
        ast::CmpOp::GtE => CompareOperator::GtE,
        ast::CmpOp::Is => CompareOperator::Is,
        ast::CmpOp::IsNot => CompareOperator::IsNot,
        ast::CmpOp::In => CompareOperator::In,
        ast::CmpOp::NotIn => CompareOperator::NotIn,
    }
}

#[cfg(test)]
mod tests;
