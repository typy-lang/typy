//! Writes a typed program as indented text, one node per line with its position and type, for
//! `--emit=typed-ast`.

use std::fmt::Write as _;

use typy_diagnostics::{SourceLookup, Span};

use super::expression::{TypedExpr, TypedExprKind};
use super::function::TypedFunction;
use super::program::TypedProgram;
use super::statement::{AssignTarget, LoopSource, TypedStmt};
use super::symbol::SymbolId;

/// Returns the program as text: each function with its signature and locals, then its body.
#[must_use]
pub fn dump(program: &TypedProgram, lookup: &dyn SourceLookup) -> String {
    let mut writer = Dump {
        program,
        lookup,
        text: String::new(),
    };
    let _ = writeln!(writer.text, "Program entry fn{}", program.entry.0);
    for function in &program.functions {
        writer.function(function);
    }
    writer.text
}

struct Dump<'program> {
    program: &'program TypedProgram,
    lookup: &'program dyn SourceLookup,
    text: String,
}

impl Dump<'_> {
    fn line(&mut self, depth: usize, label: &str, span: Span) {
        let at = self.lookup.position(span.file(), span.start());
        let indent = "  ".repeat(depth);
        let _ = writeln!(self.text, "{indent}{label} @{}:{}", at.line, at.column);
    }

    fn symbol(&self, id: SymbolId) -> String {
        let symbol = self.program.symbol(id);
        format!("{}: {}", symbol.name, symbol.ty)
    }

    fn function(&mut self, function: &TypedFunction) {
        let parameters: Vec<String> = function
            .parameters
            .iter()
            .map(|id| self.symbol(*id))
            .collect();
        let label = format!(
            "Function fn{} {}.{}({}) -> {}",
            function.id.0,
            function.module,
            function.name,
            parameters.join(", "),
            function.signature.returns
        );
        self.line(0, &label, function.span);
        if !function.locals.is_empty() {
            let locals: Vec<String> = function.locals.iter().map(|id| self.symbol(*id)).collect();
            let _ = writeln!(self.text, "  locals {}", locals.join(", "));
        }
        self.block(1, "body", &function.body);
    }

    fn block(&mut self, depth: usize, label: &str, statements: &[TypedStmt]) {
        if statements.is_empty() {
            return;
        }
        let _ = writeln!(self.text, "{}{label}", "  ".repeat(depth));
        for statement in statements {
            self.statement(depth + 1, statement);
        }
    }

    /// Writes an assignment target: a variable with its type, or a tuple of targets.
    fn target(&self, target: &AssignTarget) -> String {
        match target {
            AssignTarget::Name(id) => self.symbol(*id),
            AssignTarget::Tuple(targets) => {
                let parts: Vec<String> = targets.iter().map(|target| self.target(target)).collect();
                format!("({})", parts.join(", "))
            }
            AssignTarget::Item { list, .. } => format!("item of {}", list.ty),
            AssignTarget::Entry { dict, .. } => format!("entry of {}", dict.ty),
        }
    }

    fn statement(&mut self, depth: usize, statement: &TypedStmt) {
        match statement {
            TypedStmt::Assign {
                targets,
                value,
                span,
            } => {
                let names: Vec<String> = targets.iter().map(|target| self.target(target)).collect();
                self.line(depth, &format!("Assign {}", names.join(", ")), *span);
                self.expression(depth + 1, value);
            }
            TypedStmt::AugAssign {
                target,
                operator,
                value,
                span,
            } => {
                let label = format!("AugAssign {operator:?} {}", self.target(target));
                self.line(depth, &label, *span);
                self.expression(depth + 1, value);
            }
            TypedStmt::Expr { value, span } => {
                self.line(depth, "Expr", *span);
                self.expression(depth + 1, value);
            }
            TypedStmt::Return { value, span } => {
                self.line(depth, "Return", *span);
                if let Some(value) = value {
                    self.expression(depth + 1, value);
                }
            }
            TypedStmt::If {
                test,
                body,
                orelse,
                span,
            }
            | TypedStmt::While {
                test,
                body,
                orelse,
                span,
            } => {
                let label = if matches!(statement, TypedStmt::If { .. }) {
                    "If"
                } else {
                    "While"
                };
                self.line(depth, label, *span);
                self.expression(depth + 1, test);
                self.block(depth + 1, "body", body);
                self.block(depth + 1, "orelse", orelse);
            }
            TypedStmt::ForEach {
                target,
                source,
                body,
                orelse,
                span,
            } => {
                let label = match source {
                    LoopSource::Items(_) => "",
                    LoopSource::Enumerate { .. } => " enumerate",
                    LoopSource::Zip { .. } => " zip",
                    LoopSource::Reversed(_) => " reversed",
                };
                self.line(
                    depth,
                    &format!("ForEach{label} {}", self.target(target)),
                    *span,
                );
                for expression in source.expressions() {
                    self.expression(depth + 1, expression);
                }
                self.block(depth + 1, "body", body);
                self.block(depth + 1, "orelse", orelse);
            }
            TypedStmt::ForRange {
                target,
                start,
                stop,
                step,
                body,
                orelse,
                span,
            } => {
                self.line(depth, &format!("ForRange {}", self.symbol(*target)), *span);
                for value in [start, stop, step] {
                    self.expression(depth + 1, value);
                }
                self.block(depth + 1, "body", body);
                self.block(depth + 1, "orelse", orelse);
            }
            TypedStmt::Pass { span } => self.line(depth, "Pass", *span),
            TypedStmt::Break { span } => self.line(depth, "Break", *span),
            TypedStmt::Continue { span } => self.line(depth, "Continue", *span),
        }
    }

    fn expression(&mut self, depth: usize, expression: &TypedExpr) {
        let ty = &expression.ty;
        let label = match &expression.kind {
            TypedExprKind::Constant(constant) => format!("Constant {constant:?}"),
            TypedExprKind::Local(id) => format!("Local {}", self.program.symbol(*id).name),
            TypedExprKind::ModuleName => "ModuleName".to_owned(),
            TypedExprKind::Binary { operator, .. } => format!("Binary {operator:?}"),
            TypedExprKind::Unary { operator, .. } => format!("Unary {operator:?}"),
            TypedExprKind::BoolOp { operator, .. } => format!("BoolOp {operator:?}"),
            TypedExprKind::Compare { operators, .. } => format!("Compare {operators:?}"),
            TypedExprKind::IfExp { .. } => "IfExp".to_owned(),
            TypedExprKind::Call { function, .. } => {
                format!("Call {}", self.program.function(*function).name)
            }
            TypedExprKind::Print { sep, end, .. } => {
                format!("Print sep={} end={}", sep.is_some(), end.is_some())
            }
            TypedExprKind::Builtin { builtin, .. } => format!("Builtin {builtin:?}"),
            TypedExprKind::IterableBuiltin { builtin, .. } => {
                format!("IterableBuiltin {builtin:?}")
            }
            TypedExprKind::Index { .. } => "Index".to_owned(),
            TypedExprKind::Tuple(_) => "Tuple".to_owned(),
            TypedExprKind::List(_) => "List".to_owned(),
            TypedExprKind::ListOf(_) => "ListOf".to_owned(),
            TypedExprKind::SetOf(_) => "SetOf".to_owned(),
            TypedExprKind::Set(_) => "Set".to_owned(),
            TypedExprKind::Dict { .. } => "Dict".to_owned(),
            TypedExprKind::DictOf(_) => "DictOf".to_owned(),
            TypedExprKind::DictItem { .. } => "DictItem".to_owned(),
            TypedExprKind::TupleOf(_) => "TupleOf".to_owned(),
            TypedExprKind::ListItem { .. } => "ListItem".to_owned(),
            TypedExprKind::Slice { .. } => "Slice".to_owned(),
            TypedExprKind::Method { method, .. } => format!("Method {}", method.name()),
            TypedExprKind::TupleItem { index, .. } => format!("TupleItem {index}"),
            TypedExprKind::TupleSlice { positions, .. } => format!("TupleSlice {positions:?}"),
        };
        self.line(depth, &format!("{label} : {ty}"), expression.span);
        for child in children(expression) {
            self.expression(depth + 1, child);
        }
    }
}

/// Returns an expression's operands in source order.
fn children(expression: &TypedExpr) -> Vec<&TypedExpr> {
    match &expression.kind {
        TypedExprKind::Constant(_) | TypedExprKind::Local(_) | TypedExprKind::ModuleName => {
            Vec::new()
        }
        TypedExprKind::Binary { left, right, .. }
        | TypedExprKind::Index {
            value: left,
            index: right,
        } => {
            vec![left, right]
        }
        TypedExprKind::Unary { operand, .. }
        | TypedExprKind::TupleOf(operand)
        | TypedExprKind::TupleItem { tuple: operand, .. }
        | TypedExprKind::TupleSlice { tuple: operand, .. } => {
            vec![operand]
        }
        TypedExprKind::Tuple(items) | TypedExprKind::List(items) | TypedExprKind::Set(items) => {
            items.iter().collect()
        }
        TypedExprKind::ListItem { list, index } => vec![list, index],
        TypedExprKind::ListOf(source)
        | TypedExprKind::SetOf(source)
        | TypedExprKind::DictOf(source) => source.expressions(),
        TypedExprKind::Dict { keys, values } => keys
            .iter()
            .zip(values)
            .flat_map(|(key, value)| [key, value])
            .collect(),
        TypedExprKind::DictItem { dict, key } => vec![dict, key],
        TypedExprKind::IterableBuiltin {
            iterable, option, ..
        } => std::iter::once(iterable.as_ref())
            .chain(option.as_deref())
            .collect(),
        TypedExprKind::Slice {
            value,
            lower,
            upper,
            step,
        } => std::iter::once(value.as_ref())
            .chain(lower.as_deref())
            .chain(upper.as_deref())
            .chain(step.as_deref())
            .collect(),
        TypedExprKind::Method {
            receiver,
            arguments,
            ..
        } => std::iter::once(receiver.as_ref())
            .chain(arguments.iter().map(|argument| &argument.value))
            .collect(),
        TypedExprKind::BoolOp { values, .. }
        | TypedExprKind::Builtin {
            arguments: values, ..
        } => values.iter().collect(),
        TypedExprKind::Compare {
            left, comparators, ..
        } => std::iter::once(left.as_ref()).chain(comparators).collect(),
        TypedExprKind::IfExp { test, body, orelse } => vec![test, body, orelse],
        TypedExprKind::Call { arguments, .. } => {
            arguments.iter().map(|argument| &argument.value).collect()
        }
        TypedExprKind::Print { values, sep, end } => values
            .iter()
            .chain(sep.as_deref())
            .chain(end.as_deref())
            .collect(),
    }
}
