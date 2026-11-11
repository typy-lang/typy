//! Writes a syntax tree as indented text, one node per line, for `--emit=ast`.

use std::fmt::Write as _;

use typy_diagnostics::Span;

use super::expression::{Comprehension, Expr, FStringPart};
use super::statement::Stmt;
use super::tree::SyntaxTree;
use crate::source_file::SourceFile;

/// Returns the tree as indented text, one node per line with its `LINE:COLUMN`, for `--emit=ast`.
#[must_use]
pub fn dump(tree: &SyntaxTree, file: &SourceFile) -> String {
    let mut writer = Dump {
        file,
        text: String::new(),
    };
    writer.line(0, "Module", None);
    for statement in &tree.body {
        writer.statement(1, statement);
    }
    writer.text
}

struct Dump<'file> {
    file: &'file SourceFile,
    text: String,
}

impl Dump<'_> {
    fn line(&mut self, depth: usize, label: &str, span: Option<Span>) {
        let indent = "  ".repeat(depth);
        match span {
            Some(span) => {
                let at = self.file.position(span.start());
                let _ = writeln!(self.text, "{indent}{label} @{}:{}", at.line, at.column);
            }
            None => {
                let _ = writeln!(self.text, "{indent}{label}");
            }
        }
    }

    fn block(&mut self, depth: usize, label: &str, statements: &[Stmt]) {
        if statements.is_empty() {
            return;
        }
        self.line(depth, label, None);
        for statement in statements {
            self.statement(depth + 1, statement);
        }
    }

    #[allow(clippy::too_many_lines, reason = "one arm per statement kind")]
    fn statement(&mut self, depth: usize, statement: &Stmt) {
        let span = Some(statement.span());
        match statement {
            Stmt::FunctionDef {
                name,
                parameters,
                returns,
                body,
                decorators,
                type_parameter_count,
                ..
            } => {
                self.line(depth, &format!("FunctionDef {}", name.name), span);
                for decorator in decorators {
                    self.line(depth + 1, "decorator", None);
                    self.expression(depth + 2, decorator);
                }
                if *type_parameter_count > 0 {
                    self.line(
                        depth + 1,
                        &format!("type_params {type_parameter_count}"),
                        None,
                    );
                }
                for parameter in parameters {
                    let label = format!("Parameter {} {:?}", parameter.name.name, parameter.kind);
                    self.line(depth + 1, &label, Some(parameter.span));
                    if let Some(annotation) = &parameter.annotation {
                        self.expression(depth + 2, annotation);
                    }
                    if let Some(default) = &parameter.default {
                        self.line(depth + 2, "default", None);
                        self.expression(depth + 3, default);
                    }
                }
                if let Some(returns) = returns {
                    self.line(depth + 1, "returns", None);
                    self.expression(depth + 2, returns);
                }
                self.block(depth + 1, "body", body);
            }
            Stmt::ClassDef {
                name,
                bases,
                keyword_count,
                body,
                decorators,
                type_parameter_count,
                ..
            } => {
                self.line(depth, &format!("ClassDef {}", name.name), span);
                for decorator in decorators {
                    self.line(depth + 1, "decorator", None);
                    self.expression(depth + 2, decorator);
                }
                if *type_parameter_count > 0 {
                    let label = format!("type_params {type_parameter_count}");
                    self.line(depth + 1, &label, None);
                }
                for base in bases {
                    self.line(depth + 1, "base", None);
                    self.expression(depth + 2, base);
                }
                if *keyword_count > 0 {
                    self.line(depth + 1, &format!("keywords {keyword_count}"), None);
                }
                self.block(depth + 1, "body", body);
            }
            Stmt::Return { value, .. } => {
                self.line(depth, "Return", span);
                if let Some(value) = value {
                    self.expression(depth + 1, value);
                }
            }
            Stmt::Assign { targets, value, .. } => {
                self.line(depth, "Assign", span);
                for target in targets {
                    self.expression(depth + 1, target);
                }
                self.expression(depth + 1, value);
            }
            Stmt::AugAssign {
                target,
                operator,
                value,
                ..
            } => {
                self.line(depth, &format!("AugAssign {operator:?}"), span);
                self.expression(depth + 1, target);
                self.expression(depth + 1, value);
            }
            Stmt::AnnAssign {
                target,
                annotation,
                value,
                ..
            } => {
                self.line(depth, "AnnAssign", span);
                self.expression(depth + 1, target);
                self.expression(depth + 1, annotation);
                if let Some(value) = value {
                    self.expression(depth + 1, value);
                }
            }
            Stmt::For {
                target,
                iter,
                body,
                orelse,
                ..
            } => {
                self.line(depth, "For", span);
                self.expression(depth + 1, target);
                self.expression(depth + 1, iter);
                self.block(depth + 1, "body", body);
                self.block(depth + 1, "orelse", orelse);
            }
            Stmt::While {
                test, body, orelse, ..
            }
            | Stmt::If {
                test, body, orelse, ..
            } => {
                let label = if matches!(statement, Stmt::While { .. }) {
                    "While"
                } else {
                    "If"
                };
                self.line(depth, label, span);
                self.expression(depth + 1, test);
                self.block(depth + 1, "body", body);
                self.block(depth + 1, "orelse", orelse);
            }
            Stmt::Import { names, is_lazy, .. } => {
                let names: Vec<&str> = names.iter().map(|alias| alias.name.name.as_str()).collect();
                let lazy = if *is_lazy { " lazy" } else { "" };
                self.line(depth, &format!("Import{lazy} {}", names.join(", ")), span);
            }
            Stmt::ImportFrom {
                module,
                names,
                level,
                is_lazy,
                ..
            } => {
                let names: Vec<&str> = names.iter().map(|alias| alias.name.name.as_str()).collect();
                let module = module.as_ref().map_or("", |module| module.name.as_str());
                let lazy = if *is_lazy { " lazy" } else { "" };
                let dots = ".".repeat(*level as usize);
                self.line(
                    depth,
                    &format!("ImportFrom{lazy} {dots}{module} {}", names.join(", ")),
                    span,
                );
            }
            Stmt::Expr { value, .. } => {
                self.line(depth, "Expr", span);
                self.expression(depth + 1, value);
            }
            Stmt::Raise {
                exception, cause, ..
            } => {
                let from = if cause.is_some() { " from" } else { "" };
                self.line(depth, &format!("Raise{from}"), span);
                for value in exception.iter().chain(cause) {
                    self.expression(depth + 1, value);
                }
            }
            Stmt::Try {
                body,
                handlers,
                orelse,
                finalbody,
                ..
            } => {
                self.line(depth, "Try", span);
                self.block(depth + 1, "body", body);
                for handler in handlers {
                    let name = handler
                        .name
                        .as_ref()
                        .map_or(String::new(), |name| format!(" as {}", name.name));
                    self.line(depth + 1, &format!("except{name}"), Some(handler.span));
                    if let Some(class) = &handler.class {
                        self.expression(depth + 2, class);
                    }
                    self.block(depth + 2, "body", &handler.body);
                }
                self.block(depth + 1, "orelse", orelse);
                self.block(depth + 1, "finally", finalbody);
            }
            Stmt::Assert { test, message, .. } => {
                self.line(depth, "Assert", span);
                for value in std::iter::once(test).chain(message) {
                    self.expression(depth + 1, value);
                }
            }
            Stmt::Pass { .. } => self.line(depth, "Pass", span),
            Stmt::Break { .. } => self.line(depth, "Break", span),
            Stmt::Continue { .. } => self.line(depth, "Continue", span),
            Stmt::Other { kind, .. } => self.line(depth, &kind.to_string(), span),
        }
    }

    /// Writes a comprehension: what it makes, then each clause with its target, its source and
    /// its conditions.
    fn comprehension(
        &mut self,
        depth: usize,
        label: &str,
        made: &[&Expr],
        generators: &[Comprehension],
        span: Option<Span>,
    ) {
        self.line(depth, label, span);
        for expression in made {
            self.expression(depth + 1, expression);
        }
        for generator in generators {
            let label = if generator.is_async {
                "AsyncComprehension"
            } else {
                "Comprehension"
            };
            self.line(depth + 1, label, Some(generator.span));
            self.expression(depth + 2, &generator.target);
            self.expression(depth + 2, &generator.iter);
            for condition in &generator.ifs {
                self.expression(depth + 2, condition);
            }
        }
    }

    /// Writes the literal text and replacement fields of an f-string or a format spec.
    fn fstring_parts(&mut self, depth: usize, parts: &[FStringPart]) {
        for part in parts {
            match part {
                FStringPart::Literal(text) => self.line(depth, &format!("Literal {text:?}"), None),
                FStringPart::Field(field) => {
                    let conversion = field
                        .conversion
                        .map_or(String::new(), |conversion| format!(" !{conversion}"));
                    let debug = field
                        .debug
                        .as_ref()
                        .map_or(String::new(), |debug| format!(" debug {debug:?}"));
                    let label = format!("Field{conversion}{debug}");
                    self.line(depth, &label, Some(field.span));
                    self.expression(depth + 1, &field.value);
                    if let Some(spec) = &field.spec {
                        self.line(depth + 1, "spec", None);
                        self.fstring_parts(depth + 2, spec);
                    }
                }
            }
        }
    }

    /// Writes a dict display: each key, or `Unpack` for a `**` item, then its value.
    fn dict(&mut self, depth: usize, keys: &[Option<Expr>], values: &[Expr], span: Option<Span>) {
        self.line(depth, "Dict", span);
        for (key, value) in keys.iter().zip(values) {
            match key {
                Some(key) => self.expression(depth + 1, key),
                None => self.line(depth + 1, "Unpack", Some(value.span())),
            }
            self.expression(depth + 1, value);
        }
    }

    /// Writes a tuple, list or set display and its items.
    fn display(&mut self, depth: usize, label: &str, elements: &[Expr], span: Option<Span>) {
        self.line(depth, label, span);
        for element in elements {
            self.expression(depth + 1, element);
        }
    }

    #[allow(clippy::too_many_lines, reason = "one arm per expression kind")]
    fn expression(&mut self, depth: usize, expression: &Expr) {
        let span = Some(expression.span());
        match expression {
            Expr::BoolOp {
                operator, values, ..
            } => {
                self.line(depth, &format!("BoolOp {operator:?}"), span);
                for value in values {
                    self.expression(depth + 1, value);
                }
            }
            Expr::BinOp {
                left,
                operator,
                right,
                ..
            } => {
                self.line(depth, &format!("BinOp {operator:?}"), span);
                self.expression(depth + 1, left);
                self.expression(depth + 1, right);
            }
            Expr::UnaryOp {
                operator, operand, ..
            } => {
                self.line(depth, &format!("UnaryOp {operator:?}"), span);
                self.expression(depth + 1, operand);
            }
            Expr::Tuple { elements, .. } => self.display(depth, "Tuple", elements, span),
            Expr::List { elements, .. } => self.display(depth, "List", elements, span),
            Expr::Set { elements, .. } => self.display(depth, "Set", elements, span),
            Expr::Dict { keys, values, .. } => self.dict(depth, keys, values, span),
            Expr::ListComp {
                element,
                generators,
                ..
            }
            | Expr::SetComp {
                element,
                generators,
                ..
            } => {
                let label = if matches!(expression, Expr::ListComp { .. }) {
                    "ListComp"
                } else {
                    "SetComp"
                };
                self.comprehension(depth, label, &[element], generators, span);
            }
            Expr::DictComp {
                key,
                value,
                generators,
                ..
            } => self.comprehension(depth, "DictComp", &[key, value], generators, span),
            Expr::Attribute {
                value, attribute, ..
            } => {
                self.line(depth, &format!("Attribute {}", attribute.name), span);
                self.expression(depth + 1, value);
            }
            Expr::Slice {
                lower, upper, step, ..
            } => {
                self.line(depth, "Slice", span);
                for bound in [lower, upper, step].into_iter().flatten() {
                    self.expression(depth + 1, bound);
                }
            }
            Expr::IfExp {
                test, body, orelse, ..
            } => {
                self.line(depth, "IfExp", span);
                self.expression(depth + 1, test);
                self.expression(depth + 1, body);
                self.expression(depth + 1, orelse);
            }
            Expr::Compare {
                left,
                operators,
                comparators,
                ..
            } => {
                self.line(depth, &format!("Compare {operators:?}"), span);
                self.expression(depth + 1, left);
                for comparator in comparators {
                    self.expression(depth + 1, comparator);
                }
            }
            Expr::Call {
                function,
                arguments,
                keywords,
                ..
            } => {
                self.line(depth, "Call", span);
                self.expression(depth + 1, function);
                for argument in arguments {
                    self.expression(depth + 1, argument);
                }
                for keyword in keywords {
                    let name = keyword
                        .name
                        .as_ref()
                        .map_or("**", |name| name.name.as_str());
                    self.line(depth + 1, &format!("keyword {name}"), Some(keyword.span));
                    self.expression(depth + 2, &keyword.value);
                }
            }
            Expr::Constant { value, .. } => self.line(depth, &format!("Constant {value:?}"), span),
            Expr::Subscript { value, index, .. } => {
                self.line(depth, "Subscript", span);
                self.expression(depth + 1, value);
                self.expression(depth + 1, index);
            }
            Expr::Name { id, .. } => self.line(depth, &format!("Name {id}"), span),
            Expr::FString { parts, .. } => {
                self.line(depth, "FString", span);
                self.fstring_parts(depth + 1, parts);
            }
            Expr::Quoted { annotation, .. } => {
                self.line(depth, "Quoted", span);
                self.expression(depth + 1, annotation);
            }
            Expr::Other { kind, .. } => self.line(depth, &kind.to_string(), span),
        }
    }
}
