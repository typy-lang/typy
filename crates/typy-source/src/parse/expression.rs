//! Converts Ruff's expressions and literals.

use ruff_python_ast as ast;
use ruff_python_ast::StringFlags as _;
use ruff_text_size::{Ranged, TextRange};

use super::Converter;
use super::operator::{binary_operator, compare_operator};
use crate::int_literal::decimal_digits;
use crate::syntax::{
    BoolOperator, Comprehension, Constant, Expr, FStringPart, Identifier, Keyword, NodeKind,
    ReplacementField, UnaryOperator,
};

impl Converter<'_> {
    /// Converts an annotation; a string that holds an expression becomes `Quoted`, so the
    /// checker can tell a forward reference from a name that is already bound.
    pub(super) fn annotation(&self, annotation: &ast::Expr) -> Expr {
        if let ast::Expr::StringLiteral(node) = annotation
            && let Some(literal) = node.as_single_part_string()
            && let Ok(parsed) = ruff_python_parser::parse_string_annotation(self.text, literal)
        {
            return Expr::Quoted {
                annotation: Box::new(self.expression(parsed.expr())),
                span: self.span(node.range),
            };
        }
        self.expression(annotation)
    }

    /// Returns the value of a string literal and the literals implicitly joined to it, as
    /// code points when one holds an escaped surrogate (LEX-10).
    fn string_constant(&self, node: &ast::ExprStringLiteral) -> Constant {
        let parts: Vec<Option<Vec<u32>>> = node
            .value
            .iter()
            .map(|part| self.literal_points(part))
            .collect();
        if parts.iter().all(Option::is_none) {
            return Constant::Str(node.value.to_str().to_owned());
        }
        let points = node
            .value
            .iter()
            .zip(parts)
            .flat_map(|(part, points)| {
                points.unwrap_or_else(|| part.value.chars().map(u32::from).collect())
            })
            .collect();
        Constant::StrPoints(points)
    }

    /// Returns the code points of one string literal when it holds an escaped surrogate.
    fn literal_points(&self, literal: &ast::StringLiteral) -> Option<Vec<u32>> {
        if literal.flags.prefix().is_raw() {
            return None;
        }
        let range = literal.range;
        let content = TextRange::new(
            range.start() + literal.flags.opener_len(),
            range.end() - literal.flags.closer_len(),
        );
        self.surrogate_points(&literal.value, content)
    }

    /// Returns literal text of an f-string, whose source is `range`. Text that holds an
    /// escaped surrogate becomes a replacement field of that string, which formats as the
    /// string itself, since an f-string's text is Rust text (LEX-10).
    fn fstring_text(&self, value: &str, range: TextRange, raw: bool) -> FStringPart {
        match (!raw)
            .then(|| self.surrogate_points(value, range))
            .flatten()
        {
            None => FStringPart::Literal(value.to_owned()),
            Some(points) => self.points_field(points, range),
        }
    }

    /// Returns the replacement field `{text}` of the string with code points `points`.
    fn points_field(&self, points: Vec<u32>, range: TextRange) -> FStringPart {
        let span = self.span(range);
        FStringPart::Field(Box::new(ReplacementField {
            value: Expr::Constant {
                value: Constant::StrPoints(points),
                span,
            },
            debug: None,
            conversion: None,
            spec: None,
            span,
        }))
    }

    /// Converts the elements of an f-string or of a format specification.
    fn fstring_parts(
        &self,
        elements: &ast::InterpolatedStringElements,
        raw: bool,
    ) -> Vec<FStringPart> {
        elements
            .iter()
            .map(|element| match element {
                ast::InterpolatedStringElement::Literal(literal) => {
                    self.fstring_text(&literal.value, literal.range, raw)
                }
                ast::InterpolatedStringElement::Interpolation(field) => {
                    FStringPart::Field(Box::new(ReplacementField {
                        value: self.expression(&field.expression),
                        debug: field
                            .debug_text
                            .as_ref()
                            .map(|text| text.as_str().to_owned()),
                        conversion: match field.conversion {
                            ast::ConversionFlag::None => None,
                            ast::ConversionFlag::Str => Some('s'),
                            ast::ConversionFlag::Repr => Some('r'),
                            ast::ConversionFlag::Ascii => Some('a'),
                        },
                        spec: field
                            .format_spec
                            .as_ref()
                            .map(|spec| self.fstring_parts(&spec.elements, raw)),
                        span: self.span(field.range),
                    }))
                }
            })
            .collect()
    }

    /// Converts the `for` clauses of a comprehension.
    fn generators(&self, generators: &[ast::Comprehension]) -> Vec<Comprehension> {
        generators
            .iter()
            .map(|generator| Comprehension {
                target: self.expression(&generator.target),
                iter: self.expression(&generator.iter),
                ifs: generator
                    .ifs
                    .iter()
                    .map(|condition| self.expression(condition))
                    .collect(),
                is_async: generator.is_async,
                span: self.span(generator.range),
            })
            .collect()
    }

    /// Converts an expression, one level deeper; past `MAX_DEPTH` it stands in `None`, and
    /// parsing fails.
    pub(super) fn expression(&self, expression: &ast::Expr) -> Expr {
        self.deeper(expression.range(), || self.expression_kind(expression))
            .unwrap_or_else(|| self.constant(Constant::None, expression.range()))
    }

    #[allow(
        clippy::too_many_lines,
        reason = "one arm per expression kind of Python's grammar"
    )]
    fn expression_kind(&self, expression: &ast::Expr) -> Expr {
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
            ast::Expr::StringLiteral(node) => self.constant(self.string_constant(node), node.range),
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
            // Any target but a name is a parse error, so the tree never holds one.
            ast::Expr::Named(node) => match node.target.as_ref() {
                ast::Expr::Name(name) => Expr::Named {
                    target: Identifier {
                        name: name.id.as_str().to_owned(),
                        span: self.span(name.range),
                    },
                    value: self.boxed(&node.value),
                    span,
                },
                _ => self.other(NodeKind::NamedExpr, node.range),
            },
            ast::Expr::Lambda(node) => Expr::Lambda {
                parameters: node
                    .parameters
                    .as_deref()
                    .map(|parameters| self.parameters(parameters))
                    .unwrap_or_default(),
                body: self.boxed(&node.body),
                span,
            },
            ast::Expr::Dict(node) => Expr::Dict {
                keys: node
                    .items
                    .iter()
                    .map(|item| item.key.as_ref().map(|key| self.expression(key)))
                    .collect(),
                values: node
                    .items
                    .iter()
                    .map(|item| self.expression(&item.value))
                    .collect(),
                span: self.span(node.range),
            },
            ast::Expr::ListComp(node) => Expr::ListComp {
                element: Box::new(self.expression(&node.elt)),
                generators: self.generators(&node.generators),
                span: self.span(node.range),
            },
            ast::Expr::SetComp(node) => Expr::SetComp {
                element: Box::new(self.expression(&node.elt)),
                generators: self.generators(&node.generators),
                span: self.span(node.range),
            },
            ast::Expr::DictComp(node) => Expr::DictComp {
                // Ruff leaves the key out only for `{**d for ...}`, a syntax error, and a tree
                // with errors is never converted.
                key: Box::new(match node.key.as_deref() {
                    Some(key) => self.expression(key),
                    None => self.expression(&node.value),
                }),
                value: Box::new(self.expression(&node.value)),
                generators: self.generators(&node.generators),
                span: self.span(node.range),
            },
            ast::Expr::Generator(node) => Expr::GeneratorExp {
                element: Box::new(self.expression(&node.elt)),
                generators: self.generators(&node.generators),
                span: self.span(node.range),
            },
            ast::Expr::Await(node) => self.other(NodeKind::Await, node.range),
            ast::Expr::Yield(node) => Expr::Yield {
                value: node.value.as_deref().map(|value| self.boxed(value)),
                span,
            },
            ast::Expr::YieldFrom(node) => Expr::YieldFrom {
                value: self.boxed(&node.value),
                span,
            },
            ast::Expr::FString(node) => Expr::FString {
                parts: node
                    .value
                    .iter()
                    .flat_map(|part| match part {
                        ast::FStringPartRef::Literal(literal) => {
                            vec![match self.literal_points(literal) {
                                None => FStringPart::Literal(literal.value.to_string()),
                                Some(points) => self.points_field(points, literal.range),
                            }]
                        }
                        ast::FStringPartRef::FString(fstring) => {
                            self.fstring_parts(&fstring.elements, fstring.flags.prefix().is_raw())
                        }
                    })
                    .collect(),
                span,
            },
            ast::Expr::TString(node) => self.other(NodeKind::TemplateStr, node.range),
            ast::Expr::Attribute(node) => Expr::Attribute {
                value: Box::new(self.expression(&node.value)),
                attribute: self.identifier(&node.attr),
                span: self.span(node.range),
            },
            ast::Expr::Starred(node) => Expr::Starred {
                value: Box::new(self.expression(&node.value)),
                span: self.span(node.range),
            },
            ast::Expr::List(node) => Expr::List {
                elements: node
                    .elts
                    .iter()
                    .map(|element| self.expression(element))
                    .collect(),
                span: self.span(node.range),
            },
            ast::Expr::Set(node) => Expr::Set {
                elements: node
                    .elts
                    .iter()
                    .map(|element| self.expression(element))
                    .collect(),
                span: self.span(node.range),
            },
            ast::Expr::Tuple(node) => Expr::Tuple {
                elements: node
                    .elts
                    .iter()
                    .map(|element| self.expression(element))
                    .collect(),
                span: self.span(node.range),
            },
            ast::Expr::Slice(node) => Expr::Slice {
                lower: node.lower.as_deref().map(|bound| self.boxed(bound)),
                upper: node.upper.as_deref().map(|bound| self.boxed(bound)),
                step: node.step.as_deref().map(|bound| self.boxed(bound)),
                span,
            },
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
