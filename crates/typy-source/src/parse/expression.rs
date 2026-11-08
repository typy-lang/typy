//! Converts Ruff's expressions and literals.

use ruff_python_ast as ast;
use ruff_text_size::{Ranged, TextRange};

use super::Converter;
use super::operator::{binary_operator, compare_operator};
use crate::int_literal::decimal_digits;
use crate::syntax::{BoolOperator, Constant, Expr, Keyword, NodeKind, UnaryOperator};

impl Converter<'_> {
    #[allow(
        clippy::too_many_lines,
        reason = "one arm per expression kind of Python's grammar"
    )]
    pub(super) fn expression(&self, expression: &ast::Expr) -> Expr {
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
            ast::Expr::Tuple(node) => Expr::Tuple {
                elements: node
                    .elts
                    .iter()
                    .map(|element| self.expression(element))
                    .collect(),
                span: self.span(node.range),
            },
            ast::Expr::Slice(node) => self.other(NodeKind::Slice, node.range),
            ast::Expr::IpyEscapeCommand(node) => self.other(NodeKind::IpyEscapeCommand, node.range),
        }
    }

    pub(super) fn other(&self, kind: NodeKind, range: TextRange) -> Expr {
        Expr::Other {
            kind,
            span: self.span(range),
        }
    }

    pub(super) fn boxed(&self, expression: &ast::Expr) -> Box<Expr> {
        Box::new(self.expression(expression))
    }

    pub(super) fn constant(&self, value: Constant, range: TextRange) -> Expr {
        Expr::Constant {
            value,
            span: self.span(range),
        }
    }
}
