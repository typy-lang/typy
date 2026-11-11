//! Writes a typed program as indented text, one node per line with its position and type, for
//! `--emit=typed-ast`.

use std::fmt::Write as _;

use typy_diagnostics::{SourceLookup, Span};

use super::expression::{TypedExpr, TypedExprKind};
use super::function::TypedFunction;
use super::program::TypedProgram;
use super::statement::{AssignTarget, LoopSource, RaiseCause, TypedStmt};
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
    for class in &program.classes {
        let attributes: Vec<String> = class
            .attributes
            .iter()
            .map(|attribute| format!("{}: {}", attribute.name, attribute.ty))
            .collect();
        let methods: Vec<String> = class
            .methods
            .iter()
            .map(|(name, id)| format!("{name} fn{}", id.0))
            .collect();
        let _ = writeln!(
            writer.text,
            "Class {}.{}({}) methods [{}]",
            class.module,
            class.name,
            attributes.join(", "),
            methods.join(", ")
        );
    }
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

    /// Returns `attribute NAME: TYPE` for the attribute at `index` of an instance of `ty`.
    fn attribute(&self, ty: &crate::types::Type, index: usize) -> String {
        let Some(class) = ty.class_id() else {
            return format!("attribute {index}");
        };
        let attribute = &self.program.class(class).attributes[index];
        format!("attribute {}: {}", attribute.name, attribute.ty)
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
            function.qualified_name,
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
            AssignTarget::Slice { list, .. } => format!("slice of {}", list.ty),
            AssignTarget::Entry { dict, .. } => format!("entry of {}", dict.ty),
            AssignTarget::Attribute { object, index } => self.attribute(&object.ty, *index),
        }
    }

    #[allow(clippy::too_many_lines, reason = "one arm per statement kind")]
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
                    LoopSource::Protocol { .. } => " protocol",
                    LoopSource::Enumerate { .. } => " enumerate",
                    LoopSource::Zip { .. } => " zip",
                    LoopSource::Reversed(_) | LoopSource::Range { reversed: true, .. } => {
                        " reversed"
                    }
                    LoopSource::Range { .. } => " range",
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
            TypedStmt::Raise {
                exception,
                cause,
                span,
            } => self.raise_statement(depth, exception.as_ref(), cause, *span),
            TypedStmt::Try {
                body,
                handlers,
                orelse,
                finalbody,
                span,
            } => {
                self.line(depth, "Try", *span);
                self.block(depth + 1, "body", body);
                for handler in handlers {
                    let classes: Vec<String> =
                        handler.classes.iter().map(ToString::to_string).collect();
                    let name = handler.name.map_or(String::new(), |symbol| {
                        format!(" as {}", self.program.symbol(symbol).name)
                    });
                    self.line(
                        depth + 1,
                        &format!("except {}{name}", classes.join(", ")),
                        handler.span,
                    );
                    self.block(depth + 2, "body", &handler.body);
                }
                self.block(depth + 1, "orelse", orelse);
                self.block(depth + 1, "finally", finalbody);
            }
            TypedStmt::With {
                manager,
                suppresses,
                target,
                body,
                span,
                ..
            } => {
                let target = target.as_ref().map_or(String::new(), |target| {
                    format!(" as {}", self.target(target))
                });
                let suppresses = if *suppresses { " suppresses" } else { "" };
                self.line(depth, &format!("With{target}{suppresses}"), *span);
                self.expression(depth + 1, manager);
                self.block(depth + 1, "body", body);
            }
            TypedStmt::Delete { targets, span } => {
                self.line(depth, "Delete", *span);
                for target in targets {
                    self.expression(depth + 1, target);
                }
            }
            TypedStmt::Assert {
                test,
                exception,
                span,
            } => {
                self.line(depth, "Assert", *span);
                self.expression(depth + 1, test);
                self.expression(depth + 1, exception);
            }
            TypedStmt::Import {
                modules,
                bindings,
                span,
            } => {
                let names: Vec<String> = bindings
                    .iter()
                    .map(|binding| match binding {
                        crate::typed::ImportBinding::Module { target, module } => {
                            format!("{} = {module}", self.program.symbol(*target).name)
                        }
                        crate::typed::ImportBinding::Value {
                            target,
                            module,
                            source,
                        } => format!(
                            "{} = {module}.{}",
                            self.program.symbol(*target).name,
                            self.program.symbol(*source).name
                        ),
                    })
                    .collect();
                self.line(
                    depth,
                    &format!("Import {} binds {}", modules.join(" "), names.join(", ")),
                    *span,
                );
            }
            TypedStmt::Pass { span } => self.line(depth, "Pass", *span),
            TypedStmt::Break { span } => self.line(depth, "Break", *span),
            TypedStmt::Continue { span } => self.line(depth, "Continue", *span),
        }
    }

    /// Writes `raise`, with `from` or `from None`, then the exception and the cause.
    fn raise_statement(
        &mut self,
        depth: usize,
        exception: Option<&TypedExpr>,
        cause: &RaiseCause,
        span: Span,
    ) {
        let (from, cause) = match cause {
            RaiseCause::Absent => ("", None),
            RaiseCause::None => (" from None", None),
            RaiseCause::Exception(cause) => (" from", Some(cause)),
        };
        self.line(depth, &format!("Raise{from}"), span);
        for value in exception.into_iter().chain(cause) {
            self.expression(depth + 1, value);
        }
    }

    fn expression(&mut self, depth: usize, expression: &TypedExpr) {
        let ty = &expression.ty;
        let label = match &expression.kind {
            TypedExprKind::Constant(constant) => format!("Constant {constant:?}"),
            TypedExprKind::Local(id) => format!("Local {}", self.program.symbol(*id).name),
            TypedExprKind::ModuleVariable(id) => {
                format!("ModuleVariable {}", self.program.symbol(*id).name)
            }
            TypedExprKind::ModuleAttribute { module, symbol } => {
                format!(
                    "ModuleAttribute {module}.{}",
                    self.program.symbol(*symbol).name
                )
            }
            TypedExprKind::Submodule(module) => format!("Submodule {module}"),
            TypedExprKind::AsIterable(_) => "AsIterable".to_owned(),
            TypedExprKind::ModuleName => "ModuleName".to_owned(),
            TypedExprKind::Binary { operator, .. } => format!("Binary {operator:?}"),
            TypedExprKind::Unary { operator, .. } => format!("Unary {operator:?}"),
            TypedExprKind::BoolOp { operator, .. } => format!("BoolOp {operator:?}"),
            TypedExprKind::Compare { operators, .. } => format!("Compare {operators:?}"),
            TypedExprKind::IfExp { .. } => "IfExp".to_owned(),
            TypedExprKind::NewException { class, .. } => format!("NewException {class}"),
            TypedExprKind::SetArguments { .. } => "SetArguments".to_owned(),
            TypedExprKind::Iter(_) => "Iter".to_owned(),
            TypedExprKind::Next { method, .. } => if method.is_some() {
                "Next __next__"
            } else {
                "Next"
            }
            .to_owned(),
            TypedExprKind::Call { function, .. } => {
                format!("Call {}", self.program.function(*function).qualified_name)
            }
            TypedExprKind::New { class, .. } => {
                format!("New {}", self.program.class(*class).name)
            }
            TypedExprKind::Attribute { object, index } => self.attribute(&object.ty, *index),
            TypedExprKind::TypeOf(_) => "TypeOf".to_owned(),
            TypedExprKind::FString(_) => "FString".to_owned(),
            TypedExprKind::TypeObject(of) => format!("TypeObject {}", of.name()),
            TypedExprKind::TypeName(_) => "TypeName".to_owned(),
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
            TypedExprKind::Wrap(_) => "Wrap".to_owned(),
            TypedExprKind::Unwrap(_) => "Unwrap".to_owned(),
            TypedExprKind::Comprehension { .. } => "Comprehension".to_owned(),
            TypedExprKind::DictOf(_) => "DictOf".to_owned(),
            TypedExprKind::DictWith { entries, .. } => {
                let names: Vec<&str> = entries.iter().map(|(name, _)| name.as_str()).collect();
                format!("DictWith {}", names.join(" "))
            }
            TypedExprKind::DictFromKeys { .. } => "DictFromKeys".to_owned(),
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
#[allow(clippy::too_many_lines, reason = "one arm per expression kind")]
fn children(expression: &TypedExpr) -> Vec<&TypedExpr> {
    match &expression.kind {
        TypedExprKind::Constant(_)
        | TypedExprKind::Local(_)
        | TypedExprKind::ModuleVariable(_)
        | TypedExprKind::ModuleAttribute { .. }
        | TypedExprKind::Submodule(_)
        | TypedExprKind::ModuleName
        | TypedExprKind::TypeObject(_) => Vec::new(),
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
        TypedExprKind::DictWith { source, entries } => source
            .iter()
            .flat_map(|source| source.expressions())
            .chain(entries.iter().map(|(_, value)| value))
            .collect(),
        TypedExprKind::DictFromKeys { source, value } => source
            .expressions()
            .into_iter()
            .chain(value.as_deref())
            .collect(),
        TypedExprKind::Dict { keys, values } => keys
            .iter()
            .zip(values)
            .flat_map(|(key, value)| [key, value])
            .collect(),
        TypedExprKind::DictItem { dict, key } => vec![dict, key],
        TypedExprKind::Wrap(inner)
        | TypedExprKind::Unwrap(inner)
        | TypedExprKind::AsIterable(inner) => vec![inner],
        TypedExprKind::Comprehension {
            element,
            value,
            generators,
        } => std::iter::once(element.as_ref())
            .chain(value.as_deref())
            .chain(generators.iter().flat_map(|generator| {
                generator
                    .source
                    .expressions()
                    .into_iter()
                    .chain(&generator.conditions)
            }))
            .collect(),
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
        TypedExprKind::Call { arguments, .. } | TypedExprKind::New { arguments, .. } => {
            arguments.iter().map(|argument| &argument.value).collect()
        }
        TypedExprKind::Attribute { object, .. }
        | TypedExprKind::TypeOf(object)
        | TypedExprKind::TypeName(object) => vec![object],
        TypedExprKind::NewException {
            arguments, init, ..
        } => arguments
            .iter()
            .chain(
                init.iter()
                    .flat_map(|init| init.arguments.iter().map(|argument| &argument.value)),
            )
            .collect(),
        TypedExprKind::Iter(source) => vec![source],
        TypedExprKind::Next {
            iterator, default, ..
        } => std::iter::once(iterator.as_ref())
            .chain(default.as_deref())
            .collect(),
        TypedExprKind::SetArguments {
            exception,
            arguments,
        } => std::iter::once(exception.as_ref())
            .chain(arguments)
            .collect(),
        TypedExprKind::FString(parts) => fstring_values(parts),
        TypedExprKind::Print { values, sep, end } => values
            .iter()
            .chain(sep.as_deref())
            .chain(end.as_deref())
            .collect(),
    }
}

/// Returns the values an f-string formats, in order, those of its specifications included.
fn fstring_values(parts: &[super::expression::FStringPart]) -> Vec<&TypedExpr> {
    let mut values = Vec::new();
    for part in parts {
        if let super::expression::FStringPart::Field(field) = part {
            values.push(&field.value);
            if let Some(spec) = &field.spec {
                values.extend(fstring_values(spec));
            }
        }
    }
    values
}
