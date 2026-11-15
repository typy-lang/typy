//! Moves every node of a tree to one place, so code a tool writes for the program, such as
//! the methods `@dataclass` gives a class, is reported where the program asks for it.

use typy_diagnostics::Span;

use super::expression::{Comprehension, Expr, FStringPart, Keyword};
use super::statement::{ExceptHandler, Parameter, Stmt, WithItem};
use super::tree::Identifier;

impl Stmt {
    /// Gives the statement, and every node inside it, the place `span`.
    #[allow(clippy::too_many_lines, reason = "one arm per statement kind")]
    pub fn relocate(&mut self, span: Span) {
        match self {
            Self::FunctionDef {
                name,
                parameters,
                returns,
                body,
                decorators,
                span: at,
                ..
            } => {
                *at = span;
                name.span = span;
                for parameter in parameters {
                    parameter.relocate(span);
                }
                relocate_optional(returns.as_mut(), span);
                relocate_statements(body, span);
                relocate_expressions(decorators, span);
            }
            Self::ClassDef {
                name,
                bases,
                body,
                decorators,
                span: at,
                ..
            } => {
                *at = span;
                name.span = span;
                relocate_expressions(bases, span);
                relocate_statements(body, span);
                relocate_expressions(decorators, span);
            }
            Self::Return { value, span: at } => {
                *at = span;
                relocate_optional(value.as_mut(), span);
            }
            Self::Assign {
                targets,
                value,
                span: at,
            } => {
                *at = span;
                relocate_expressions(targets, span);
                value.relocate(span);
            }
            Self::AugAssign {
                target,
                value,
                span: at,
                ..
            } => {
                *at = span;
                target.relocate(span);
                value.relocate(span);
            }
            Self::AnnAssign {
                target,
                annotation,
                value,
                span: at,
                ..
            } => {
                *at = span;
                target.relocate(span);
                annotation.relocate(span);
                relocate_optional(value.as_mut(), span);
            }
            Self::For {
                target,
                iter,
                body,
                orelse,
                span: at,
            } => {
                *at = span;
                target.relocate(span);
                iter.relocate(span);
                relocate_statements(body, span);
                relocate_statements(orelse, span);
            }
            Self::While {
                test,
                body,
                orelse,
                span: at,
            }
            | Self::If {
                test,
                body,
                orelse,
                span: at,
            } => {
                *at = span;
                test.relocate(span);
                relocate_statements(body, span);
                relocate_statements(orelse, span);
            }
            Self::Match {
                subject,
                cases,
                span: at,
            } => {
                *at = span;
                subject.relocate(span);
                for case in cases {
                    case.span = span;
                    case.pattern.relocate(span);
                    relocate_optional(case.guard.as_mut(), span);
                    relocate_statements(&mut case.body, span);
                }
            }
            Self::Import {
                names, span: at, ..
            } => {
                *at = span;
                for alias in names {
                    alias.name.span = span;
                    relocate_identifier(alias.as_name.as_mut(), span);
                }
            }
            Self::ImportFrom {
                module,
                names,
                span: at,
                ..
            } => {
                *at = span;
                relocate_identifier(module.as_mut(), span);
                for alias in names {
                    alias.name.span = span;
                    relocate_identifier(alias.as_name.as_mut(), span);
                }
            }
            Self::Expr { value, span: at } => {
                *at = span;
                value.relocate(span);
            }
            Self::Try {
                body,
                handlers,
                orelse,
                finalbody,
                span: at,
            } => {
                *at = span;
                relocate_statements(body, span);
                for handler in handlers {
                    handler.relocate(span);
                }
                relocate_statements(orelse, span);
                relocate_statements(finalbody, span);
            }
            Self::With {
                items,
                body,
                span: at,
            } => {
                *at = span;
                for item in items {
                    item.relocate(span);
                }
                relocate_statements(body, span);
            }
            Self::Delete { targets, span: at } => {
                *at = span;
                relocate_expressions(targets, span);
            }
            Self::Assert {
                test,
                message,
                span: at,
            } => {
                *at = span;
                test.relocate(span);
                relocate_optional(message.as_mut(), span);
            }
            Self::TypeAlias {
                name,
                value,
                span: at,
                ..
            } => {
                *at = span;
                name.span = span;
                value.relocate(span);
            }
            Self::Global { names, span: at } => {
                *at = span;
                for name in names {
                    name.span = span;
                }
            }
            Self::Raise {
                exception,
                cause,
                span: at,
            } => {
                *at = span;
                relocate_optional(exception.as_mut(), span);
                relocate_optional(cause.as_mut(), span);
            }
            Self::Pass { span: at }
            | Self::Break { span: at }
            | Self::Continue { span: at }
            | Self::Other { span: at, .. } => *at = span,
        }
    }
}

impl Expr {
    /// Gives the expression, and every node inside it, the place `span`.
    #[allow(clippy::too_many_lines, reason = "one arm per expression kind")]
    pub fn relocate(&mut self, span: Span) {
        match self {
            Self::BoolOp {
                values, span: at, ..
            }
            | Self::Tuple {
                elements: values,
                span: at,
            }
            | Self::List {
                elements: values,
                span: at,
            }
            | Self::Set {
                elements: values,
                span: at,
            } => {
                *at = span;
                relocate_expressions(values, span);
            }
            Self::BinOp {
                left,
                right,
                span: at,
                ..
            } => {
                *at = span;
                left.relocate(span);
                right.relocate(span);
            }
            Self::UnaryOp {
                operand, span: at, ..
            }
            | Self::Quoted {
                annotation: operand,
                span: at,
            } => {
                *at = span;
                operand.relocate(span);
            }
            Self::IfExp {
                test,
                body,
                orelse,
                span: at,
            } => {
                *at = span;
                test.relocate(span);
                body.relocate(span);
                orelse.relocate(span);
            }
            Self::Named {
                target,
                value,
                span: at,
            } => {
                *at = span;
                target.span = span;
                value.relocate(span);
            }
            Self::Compare {
                left,
                comparators,
                span: at,
                ..
            } => {
                *at = span;
                left.relocate(span);
                relocate_expressions(comparators, span);
            }
            Self::Call {
                function,
                arguments,
                keywords,
                span: at,
            } => {
                *at = span;
                function.relocate(span);
                relocate_expressions(arguments, span);
                for keyword in keywords {
                    keyword.relocate(span);
                }
            }
            Self::Subscript {
                value,
                index,
                span: at,
            } => {
                *at = span;
                value.relocate(span);
                index.relocate(span);
            }
            Self::FString { parts, span: at } => {
                *at = span;
                relocate_parts(parts, span);
            }
            Self::Dict {
                keys,
                values,
                span: at,
            } => {
                *at = span;
                for key in keys.iter_mut().flatten() {
                    key.relocate(span);
                }
                relocate_expressions(values, span);
            }
            Self::ListComp {
                element,
                generators,
                span: at,
            }
            | Self::SetComp {
                element,
                generators,
                span: at,
            } => {
                *at = span;
                element.relocate(span);
                for generator in generators {
                    generator.relocate(span);
                }
            }
            Self::DictComp {
                key,
                value,
                generators,
                span: at,
            } => {
                *at = span;
                key.relocate(span);
                value.relocate(span);
                for generator in generators {
                    generator.relocate(span);
                }
            }
            Self::Attribute {
                value,
                attribute,
                span: at,
            } => {
                *at = span;
                value.relocate(span);
                attribute.span = span;
            }
            Self::Slice {
                lower,
                upper,
                step,
                span: at,
            } => {
                *at = span;
                for part in [lower, upper, step].into_iter().flatten() {
                    part.relocate(span);
                }
            }
            Self::Constant { span: at, .. }
            | Self::Name { span: at, .. }
            | Self::Other { span: at, .. } => *at = span,
        }
    }
}

impl Parameter {
    fn relocate(&mut self, span: Span) {
        self.span = span;
        self.name.span = span;
        relocate_optional(self.annotation.as_mut(), span);
        relocate_optional(self.default.as_mut(), span);
    }
}

impl Keyword {
    fn relocate(&mut self, span: Span) {
        self.span = span;
        relocate_identifier(self.name.as_mut(), span);
        self.value.relocate(span);
    }
}

impl Comprehension {
    fn relocate(&mut self, span: Span) {
        self.span = span;
        self.target.relocate(span);
        self.iter.relocate(span);
        relocate_expressions(&mut self.ifs, span);
    }
}

impl ExceptHandler {
    fn relocate(&mut self, span: Span) {
        self.span = span;
        relocate_optional(self.class.as_mut(), span);
        relocate_identifier(self.name.as_mut(), span);
        relocate_statements(&mut self.body, span);
    }
}

impl WithItem {
    fn relocate(&mut self, span: Span) {
        self.context.relocate(span);
        relocate_optional(self.target.as_mut(), span);
    }
}

fn relocate_statements(statements: &mut [Stmt], span: Span) {
    for statement in statements {
        statement.relocate(span);
    }
}

fn relocate_expressions(expressions: &mut [Expr], span: Span) {
    for expression in expressions {
        expression.relocate(span);
    }
}

fn relocate_optional(expression: Option<&mut Expr>, span: Span) {
    if let Some(expression) = expression {
        expression.relocate(span);
    }
}

fn relocate_identifier(identifier: Option<&mut Identifier>, span: Span) {
    if let Some(identifier) = identifier {
        identifier.span = span;
    }
}

fn relocate_parts(parts: &mut [FStringPart], span: Span) {
    for part in parts {
        if let FStringPart::Field(field) = part {
            field.span = span;
            field.value.relocate(span);
            if let Some(spec) = &mut field.spec {
                relocate_parts(spec, span);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{PythonFloor, SourceFile, parse};
    use camino::Utf8PathBuf;
    use typy_diagnostics::{FileId, Span, TextOffset};

    #[test]
    fn every_node_moves_to_the_place() {
        let text = "import a.b as c\nimport d\nfrom e import g as h, i\nglobal j\ntype T = list[int]\ndef f(self, other: int = 1) -> bool:\n    if not (self.x, f'{self.y!r:>{w}}') < [o for o in other if o]:\n        return {1: 2} == {3}\n    try:\n        del items[1:2]\n    except E as error:\n        raise X from error\n    with a as b:\n        assert c, d\n    for i in r:\n        x += lambda: 1\n    while y:\n        break\n";
        let file = SourceFile::from_text(FileId(7), Utf8PathBuf::from("generated.py"), text);
        let mut tree = parse(&file, PythonFloor::default()).unwrap();
        let place = Span::empty(FileId(3), TextOffset(5));
        for statement in &mut tree.body {
            statement.relocate(place);
        }
        let dumped = format!("{tree:?}");
        assert!(!dumped.contains("FileId(7)"), "{dumped}");
    }
}
