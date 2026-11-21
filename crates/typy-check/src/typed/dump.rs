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

    fn pattern(&mut self, depth: usize, pattern: &crate::typed::TypedPattern) {
        use crate::typed::TypedPattern;
        match pattern {
            TypedPattern::Test(test) => {
                self.line(depth, "Test", test.span);
                self.expression(depth + 1, test);
            }
            TypedPattern::Bind(symbol, value) => {
                let name = self.program.symbol(*symbol).name.clone();
                self.line(depth, &format!("Bind {name}"), value.span);
                self.expression(depth + 1, value);
            }
            TypedPattern::All(patterns) | TypedPattern::Any(patterns) => {
                let label = if matches!(pattern, TypedPattern::All(_)) {
                    "All"
                } else {
                    "Any"
                };
                let _ = writeln!(self.text, "{}{label}", "  ".repeat(depth));
                for inner in patterns {
                    self.pattern(depth + 1, inner);
                }
            }
        }
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
            AssignTarget::Unpack {
                before,
                rest,
                after,
                ..
            } => {
                let mut parts: Vec<String> =
                    before.iter().map(|target| self.target(target)).collect();
                if let Some(rest) = rest {
                    parts.push(format!("*{}", self.target(rest)));
                }
                parts.extend(after.iter().map(|target| self.target(target)));
                format!("unpack ({})", parts.join(", "))
            }
            AssignTarget::Item { list, .. } => format!("item of {}", list.ty),
            AssignTarget::Slice { list, .. } => format!("slice of {}", list.ty),
            AssignTarget::Entry { dict, .. } => format!("entry of {}", dict.ty),
            AssignTarget::Attribute { object, index } => self.attribute(&object.ty, *index),
            AssignTarget::Property { object, .. } => format!("property of {}", object.ty),
            AssignTarget::CData { object, place } => match place {
                crate::typed::CDataPlace::Value => format!("value of {}", object.ty),
                crate::typed::CDataPlace::Item(_) => format!("item of {}", object.ty),
                crate::typed::CDataPlace::Field(field) => format!("field {field} of {}", object.ty),
            },
            AssignTarget::Lazy { target, .. } => format!("lazy {}", self.target(target)),
            AssignTarget::Converted { value, target, .. } => {
                format!("{} as {}", self.target(target), value.ty)
            }
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
            TypedStmt::Yield {
                value,
                received,
                span,
            } => {
                self.line(depth, "Yield", *span);
                self.expression(depth + 1, value);
                if let Some(received) = received {
                    self.statement(depth + 1, received);
                }
            }
            TypedStmt::YieldFrom {
                source,
                item,
                received,
                span,
            } => {
                self.line(depth, "YieldFrom", *span);
                self.expression(depth + 1, source);
                self.expression(depth + 1, item);
                if let Some(received) = received {
                    self.statement(depth + 1, received);
                }
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
            TypedStmt::Match {
                subject,
                cases,
                span,
            } => {
                self.line(depth, "Match", *span);
                if let Some((symbol, value)) = subject {
                    let name = self.program.symbol(*symbol).name.clone();
                    self.line(depth + 1, &format!("subject {name}"), value.span);
                    self.expression(depth + 2, value);
                }
                for case in cases {
                    self.line(depth + 1, "case", *span);
                    self.pattern(depth + 2, &case.pattern);
                    if let Some(guard) = &case.guard {
                        self.line(depth + 2, "guard", guard.span);
                        self.expression(depth + 3, guard);
                    }
                    self.block(depth + 2, "body", &case.body);
                }
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
                pending,
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
                        crate::typed::ImportBinding::Expression { target, .. } => {
                            format!(
                                "{} = a value of a built-in module",
                                self.program.symbol(*target).name
                            )
                        }
                    })
                    .collect();
                let runs = pending.iter().fold(String::new(), |mut runs, reify| {
                    let _ = write!(runs, " runs lazy {}", reify.0);
                    runs
                });
                self.line(
                    depth,
                    &format!(
                        "Import {}{runs} binds {}",
                        modules.join(" "),
                        names.join(", ")
                    ),
                    *span,
                );
            }
            TypedStmt::LazyImport {
                reifies,
                targets,
                span,
            } => {
                let reifies: Vec<String> = reifies.iter().map(|id| id.0.to_string()).collect();
                let names: Vec<&str> = targets
                    .iter()
                    .map(|target| self.program.symbol(*target).name.as_str())
                    .collect();
                self.line(
                    depth,
                    &format!(
                        "LazyImport {} binds {}",
                        reifies.join(" "),
                        names.join(", ")
                    ),
                    *span,
                );
            }
            TypedStmt::FunctionDef {
                function,
                target,
                captures,
                span,
            } => {
                let line = format!(
                    "FunctionDef {} as {}",
                    self.program.function(*function).qualified_name,
                    self.program.symbol(*target).name
                );
                self.line(depth, &line, *span);
                for capture in captures {
                    self.expression(depth + 1, capture);
                }
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

    #[allow(clippy::too_many_lines, reason = "one arm per expression kind")]
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
            TypedExprKind::TypeAliasObject(index) => format!("TypeAliasObject {index}"),
            TypedExprKind::FunctionValue(function) => format!(
                "FunctionValue {}",
                self.program.function(*function).qualified_name
            ),
            TypedExprKind::CallValue { .. } => "CallValue".to_owned(),
            TypedExprKind::Capture(index) => format!("Capture {index}"),
            TypedExprKind::RunningFunction => "RunningFunction".to_owned(),
            TypedExprKind::Received => "Received".to_owned(),
            TypedExprKind::DelegatedItem => "DelegatedItem".to_owned(),
            TypedExprKind::Spread { kind, .. } => format!("Spread {kind:?}"),
            TypedExprKind::DictSpread(_) => "DictSpread".to_owned(),
            TypedExprKind::DelegatedResult => "DelegatedResult".to_owned(),
            TypedExprKind::CellRead(symbol) => {
                format!("CellRead {}", self.program.symbol(*symbol).name)
            }
            TypedExprKind::CellObject(symbol) => {
                format!("CellObject {}", self.program.symbol(*symbol).name)
            }
            TypedExprKind::CapturedCell(index) => format!("CapturedCell {index}"),
            TypedExprKind::CapturedCellObject(index) => format!("CapturedCellObject {index}"),
            TypedExprKind::Lambda { function, .. } => {
                format!("Lambda {}", self.program.function(*function).qualified_name)
            }
            TypedExprKind::CaptureOf {
                function, index, ..
            } => format!(
                "CaptureOf {} {index}",
                self.program.function(*function).qualified_name
            ),
            TypedExprKind::CallNested { function, .. } => {
                format!(
                    "CallNested {}",
                    self.program.function(*function).qualified_name
                )
            }
            TypedExprKind::ChosenFunction { functions, .. } => {
                let names: Vec<&str> = functions
                    .iter()
                    .map(|(_, function)| self.program.function(*function).qualified_name.as_str())
                    .collect();
                format!("ChosenFunction {}", names.join(" "))
            }
            TypedExprKind::BoundMethod { methods, .. } => {
                let names: Vec<&str> = methods
                    .iter()
                    .map(|(_, method)| self.program.function(*method).qualified_name.as_str())
                    .collect();
                format!("BoundMethod {}", names.join(" "))
            }
            TypedExprKind::ProtocolMethod { name, .. } => format!("ProtocolMethod {name}"),
            TypedExprKind::VersionRead { .. } => "VersionRead".to_owned(),
            TypedExprKind::TypeAliasPart { module, .. } => if *module {
                "TypeAliasModule"
            } else {
                "TypeAliasName"
            }
            .to_owned(),
            TypedExprKind::AsIterable(_) => "AsIterable".to_owned(),
            TypedExprKind::AsIterator(_) => "AsIterator".to_owned(),
            TypedExprKind::MathCall { function, .. } => format!("MathCall {}", function.name()),
            TypedExprKind::SysValue(value) => format!("SysValue {value:?}"),
            TypedExprKind::ForeignLibrary(index) => {
                let name = self.program.foreign_libraries[*index as usize]
                    .name
                    .as_deref();
                format!("ForeignLibrary {}", name.unwrap_or("None"))
            }
            TypedExprKind::ForeignCall {
                function,
                arguments,
            } => {
                let name = &self.program.foreign_functions[*function as usize].name;
                self.line(depth, &format!("ForeignCall {name}: {ty}"), expression.span);
                for argument in arguments {
                    self.expression(depth + 1, argument);
                }
                return;
            }
            TypedExprKind::CData(cdata) => {
                let name = format!("{cdata:?}");
                let name = name.split(['(', ' ']).next().unwrap_or_default();
                self.line(depth, &format!("CData {name}: {ty}"), expression.span);
                for part in cdata.expressions() {
                    self.expression(depth + 1, part);
                }
                return;
            }
            TypedExprKind::SysExit(_) => "SysExit".to_owned(),
            TypedExprKind::RecursionLimit(_) => "RecursionLimit".to_owned(),
            TypedExprKind::ExceptionPart { part, .. } => format!("ExceptionPart({part:?})"),
            TypedExprKind::FileAttribute { attribute, .. } => {
                format!("FileAttribute({attribute:?})")
            }
            TypedExprKind::PercentDynamic { .. } => "PercentDynamic".to_owned(),
            TypedExprKind::Input(_) => "Input".to_owned(),
            TypedExprKind::Open { mode, access, .. } => format!("Open {mode:?} {access:?}"),
            TypedExprKind::PercentFormat { parts, .. } => {
                let fields = parts
                    .iter()
                    .filter(|part| matches!(part, super::percent::PercentPart::Field(_)))
                    .count();
                format!("PercentFormat {fields} fields")
            }
            TypedExprKind::Lazy { reify, .. } => format!("Lazy {}", reify.0),
            TypedExprKind::ModuleName => "ModuleName".to_owned(),
            TypedExprKind::Binary { operator, .. } => format!("Binary {operator:?}"),
            TypedExprKind::Unary { operator, .. } => format!("Unary {operator:?}"),
            TypedExprKind::BoolOp { operator, .. } => format!("BoolOp {operator:?}"),
            TypedExprKind::Compare { operators, .. } => format!("Compare {operators:?}"),
            TypedExprKind::IfExp { .. } => "IfExp".to_owned(),
            TypedExprKind::NewException { class, .. } => format!("NewException {class}"),
            TypedExprKind::SetArguments { .. } => "SetArguments".to_owned(),
            TypedExprKind::Iter(_) => "Iter".to_owned(),
            TypedExprKind::Reversed(_) => "Reversed".to_owned(),
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
            TypedExprKind::Print(call) => {
                format!(
                    "Print sep={} end={}",
                    call.sep.is_some(),
                    call.end.is_some()
                )
            }
            TypedExprKind::Builtin { builtin, .. } => format!("Builtin {builtin:?}"),
            TypedExprKind::Keyed { operation, .. } => format!("Keyed {operation:?}"),
            TypedExprKind::IterableBuiltin { builtin, .. } => {
                format!("IterableBuiltin {builtin:?}")
            }
            TypedExprKind::Index { .. } => "Index".to_owned(),
            TypedExprKind::Tuple(_) => "Tuple".to_owned(),
            TypedExprKind::List(_) => "List".to_owned(),
            TypedExprKind::ListOf(_) => "ListOf".to_owned(),
            TypedExprKind::IterContains { .. } => "IterContains".to_owned(),
            TypedExprKind::SetOf(_) => "SetOf".to_owned(),
            TypedExprKind::Set(_) => "Set".to_owned(),
            TypedExprKind::Dict { .. } => "Dict".to_owned(),
            TypedExprKind::Wrap(_) => "Wrap".to_owned(),
            TypedExprKind::Named { target, .. } => {
                format!("Named {}", self.program.symbol(*target).name)
            }
            TypedExprKind::Upcast(_) => "Upcast".to_owned(),
            TypedExprKind::Downcast(_) => "Downcast".to_owned(),
            TypedExprKind::IsInstance(_) => "IsInstance".to_owned(),
            TypedExprKind::IsSubclass { .. } => "IsSubclass".to_owned(),
            TypedExprKind::EachMember(_) => "EachMember".to_owned(),
            TypedExprKind::CheckedLength(_) => "CheckedLength".to_owned(),
            TypedExprKind::OperatorCall(_) => "OperatorCall".to_owned(),
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
            TypedExprKind::Raise(_) => "Raise".to_owned(),
            TypedExprKind::Hash(_) => "Hash".to_owned(),
            TypedExprKind::Sequence { .. } => "Sequence".to_owned(),
            TypedExprKind::NotImplemented => "NotImplemented".to_owned(),
            TypedExprKind::SliceBound { part, .. } => format!("SliceBound {part}"),
            TypedExprKind::ClassVariable(holders) => format!("ClassVariable {}", holders.len()),
            TypedExprKind::ExceptionArguments { raw, .. } => {
                format!("ExceptionArguments{}", if *raw { " raw" } else { "" })
            }
            TypedExprKind::TupleSlice { positions, .. } => format!("TupleSlice {positions:?}"),
        };
        self.line(depth, &format!("{label} : {ty}"), expression.span);
        for child in TypedExpr::operands(expression) {
            self.expression(depth + 1, child);
        }
    }
}

/// Returns the values an f-string formats, in order, those of its specifications included.
pub(super) fn fstring_values(parts: &[super::expression::FStringPart]) -> Vec<&TypedExpr> {
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
