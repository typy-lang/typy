//! Nested functions (FUN-04, ADR-0061): a `def` inside a function makes a new function object
//! each time it runs, which holds a copy of each variable of the functions around it that its
//! body reads. Its body is checked after the function's own, when each variable it captures
//! has its type.

use std::collections::{BTreeMap, BTreeSet};

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Expr, Keyword, ParameterKind, Stmt};

use super::BodyChecker;
use crate::check::{Definition, FunctionParts, function_definition};
use crate::typed::{
    CapturedVariable, FunctionId, FunctionSignature, SymbolId, TypedConstant, TypedExpr,
    TypedExprKind, TypedStmt,
};
use crate::types::Type;

/// What the names of a nested function's body read of the functions around it.
#[derive(Clone, Debug, Default)]
pub(super) struct Closure {
    /// The function's own name and the function, which its name reads inside it: the
    /// running function object.
    own: Option<(String, FunctionId)>,
    /// Each captured variable, by name.
    captures: BTreeMap<String, Captured>,
    /// The names of the functions of the function's group, empty for one in none: nested
    /// functions that call one another by name, each holding what any of them captures.
    group: BTreeSet<String>,
    /// The other functions of the group, by name, once the body that defines them is
    /// checked: a call by name runs one with the running function object (ADR-0061).
    siblings: BTreeMap<String, FunctionId>,
}

impl Closure {
    /// Returns the names of the variables the function captures.
    pub(super) fn names(&self) -> BTreeSet<String> {
        self.captures.keys().cloned().collect()
    }

    /// Returns the captured variables, in the order the function object holds them.
    pub(super) fn variables(&self) -> Vec<CapturedVariable> {
        let mut captures: Vec<(u32, CapturedVariable)> = self
            .captures
            .iter()
            .map(|(name, captured)| {
                let variable = CapturedVariable {
                    name: name.clone(),
                    ty: captured.ty.clone(),
                    cell: captured.cell,
                };
                (captured.position, variable)
            })
            .collect();
        captures.sort_by_key(|(position, _)| *position);
        captures.into_iter().map(|(_, captured)| captured).collect()
    }

    /// Returns the position and the value type of the captured cell of the variable `name`.
    pub(super) fn captured_cell(&self, name: &str) -> Option<(u32, Type)> {
        let captured = self.captures.get(name).filter(|captured| captured.cell)?;
        Some((captured.position, captured.ty.clone()))
    }
}

/// One captured variable: its position among the captures, its type, whether the capture
/// is the cell it lives in, and the nested function it holds when a `def` alone binds it,
/// which a call by name runs directly.
#[derive(Clone, Debug)]
struct Captured {
    position: u32,
    ty: Type,
    cell: bool,
    function: Option<FunctionId>,
}

/// A nested `def` whose body is checked after the body around it.
pub(super) struct Pending {
    function: FunctionId,
    statement: Stmt,
    definition: OwnedDefinition,
    closure: Closure,
}

/// The parts of a nested function's definition, kept while its body waits.
struct OwnedDefinition {
    name: String,
    qualified_name: String,
    signature: FunctionSignature,
    parameters: Vec<(String, Span)>,
    span: Span,
}

impl BodyChecker<'_> {
    /// Checks a `def` in a function: its signature now, in the annotations of the function
    /// around it, and its body once that function's is checked. A nested function of a
    /// generic function, or with type parameters of its own, has a copy for each set of
    /// types, which is planned for 0.9 (FUN-10).
    pub(super) fn nested_def(&mut self, statement: &Stmt) -> Option<TypedStmt> {
        let parts = FunctionParts::of(statement)?;
        let name = &parts.name.name;
        let symbol = *self.locals.get(name)?;
        if !parts.type_parameters.is_empty() || !self.program.type_arguments.is_empty() {
            self.diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(201),
                    parts.name.span,
                    "a nested function with type parameters, or inside a generic function, is \
                     planned for TyPy 0.9 (FUN-10)",
                )
                .with_rule(RuleId::FUN_10),
            );
            return self.refused_def(symbol);
        }
        if self.types.contains_key(&symbol) {
            self.diagnostics.push(also_assigned(name, parts.name.span));
            return None;
        }
        // Only its `def` binds a nested function's name, so a call by name runs its code.
        if self.closure_scope.rebound.contains(name) {
            self.diagnostics.push(also_assigned(name, parts.name.span));
        }
        let names = self.captured_names(parts.span);
        let group = self
            .closure_scope
            .groups
            .get(&parts.span)
            .cloned()
            .unwrap_or_default();
        let mut found = self.definition_of(&parts);
        found.qualified_name = format!("{}.<locals>.{name}", self.qualified_name);
        let function = self.program.next_function();
        self.program.signatures.push(found.signature.clone());
        self.program.functions.push(None);
        self.nested_functions.insert(symbol, function);
        self.types.insert(symbol, callable_of(&found.signature));
        let (mut closure, captures) = match self.group_captures(name, function, &group, parts.span)
        {
            Some(found) => found,
            None => self.capture_values(name, function, &names, parts.span),
        };
        closure.group = group;
        if let Some(facts) = &mut self.assigned {
            facts.insert(super::flow::Fact::Assigned(symbol));
        }
        self.pending.push(Pending {
            function,
            statement: statement.clone(),
            definition: OwnedDefinition {
                name: found.name,
                qualified_name: found.qualified_name,
                signature: found.signature,
                parameters: found.parameters,
                span: found.span,
            },
            closure,
        });
        Some(TypedStmt::FunctionDef {
            function,
            target: symbol,
            captures,
            span: parts.span,
        })
    }

    /// Gives the variable of a refused `def` the error type, assigned, so its uses report
    /// nothing more: one cause gives one error (DIA-04).
    fn refused_def(&mut self, symbol: SymbolId) -> Option<TypedStmt> {
        self.types.insert(symbol, Type::Error);
        if let Some(facts) = &mut self.assigned {
            facts.insert(super::flow::Fact::Assigned(symbol));
        }
        None
    }

    /// Returns the definition of a nested `def`, its annotations read where it is.
    fn definition_of<'tree>(&mut self, parts: &FunctionParts<'tree>) -> Definition<'tree> {
        let scope = super::locals::annotation_scope(self.program);
        let found = function_definition(parts, &scope, self.diagnostics);
        // An annotation may name a copy of a generic class, which the program then holds.
        self.program.make_class_copies();
        found
    }

    /// Returns the variables the nested `def` at `span` captures.
    pub(super) fn captured_names(&self, span: Span) -> BTreeSet<String> {
        self.closure_scope
            .captures
            .get(&span)
            .cloned()
            .unwrap_or_default()
    }

    /// Returns the closure of the nested function `function`, named `name`, with the reads
    /// of the variables it captures, `captured`, in order: the value it copies, or the cell a
    /// variable lives in, which it shares.
    pub(super) fn capture_values(
        &mut self,
        name: &str,
        function: FunctionId,
        captured: &BTreeSet<String>,
        span: Span,
    ) -> (Closure, Vec<TypedExpr>) {
        let mut closure = Closure {
            own: Some((name.to_owned(), function)),
            ..Closure::default()
        };
        let mut values = Vec::new();
        // The function's own name reads the running object inside it, so it is no capture.
        for (position, each) in captured.iter().filter(|each| *each != name).enumerate() {
            let (read, cell) = match self.cell_of(each, span) {
                Some(cell) => (cell, true),
                None => (self.name(each, span), false),
            };
            closure.captures.insert(
                each.clone(),
                Captured {
                    position: u32::try_from(position).unwrap_or(u32::MAX),
                    ty: read.ty.clone(),
                    cell,
                    function: if cell { None } else { self.function_held(each) },
                },
            );
            values.push(read);
        }
        (closure, values)
    }

    /// Returns, for a function of `group` after its first, a closure like the first's and
    /// the reads of its captures from the first's function object, which hold the same
    /// values with the types they had where the first `def` ran; `None` for any other.
    fn group_captures(
        &mut self,
        name: &str,
        function: FunctionId,
        group: &BTreeSet<String>,
        span: Span,
    ) -> Option<(Closure, Vec<TypedExpr>)> {
        if group.is_empty() {
            return None;
        }
        let first = &self
            .pending
            .iter()
            .find(|pending| pending.closure.group == *group)?
            .closure;
        let mut closure = first.clone();
        let (first_name, first_function) = closure.own.replace((name.to_owned(), function))?;
        let object = self.local_function(&first_name, span);
        let mut captures: Vec<&Captured> = closure.captures.values().collect();
        captures.sort_by_key(|captured| captured.position);
        let values = captures
            .into_iter()
            .map(|captured| {
                Self::typed(
                    TypedExprKind::CaptureOf {
                        function: first_function,
                        closure: Box::new(object.clone()),
                        index: captured.position,
                    },
                    captured.ty.clone(),
                    span,
                )
            })
            .collect();
        Some((closure, values))
    }

    /// Returns the cell the variable `name` lives in, of the type of its value, when it is
    /// a variable of this function that lives in one, or a captured cell.
    fn cell_of(&mut self, name: &str, span: Span) -> Option<TypedExpr> {
        if let Some(symbol) = self.cell_symbol(name) {
            self.used_cells.insert(symbol);
            let ty = self.types.get(&symbol).cloned().unwrap_or(Type::Error);
            return Some(Self::typed(TypedExprKind::CellObject(symbol), ty, span));
        }
        let captured = self.closure.as_ref()?.captures.get(name)?;
        captured.cell.then(|| {
            Self::typed(
                TypedExprKind::CapturedCellObject(captured.position),
                captured.ty.clone(),
                span,
            )
        })
    }

    /// Returns the variable of this function named `name` when it lives in a cell, which its
    /// nested functions share (ADR-0061).
    pub(super) fn cell_symbol(&self, name: &str) -> Option<SymbolId> {
        if let Some(symbol) = self.nonlocal_names.get(name) {
            return Some(*symbol);
        }
        if !self.closure_scope.cells.contains(name) || self.global_names.contains(name) {
            return None;
        }
        self.locals
            .get(name)
            .copied()
            .filter(|symbol| !self.is_module_variable(*symbol))
    }

    /// Returns true when `symbol` is a module variable, which a function reads as it is and
    /// never captures.
    pub(super) fn is_module_variable(&self, symbol: SymbolId) -> bool {
        self.program.symbols[symbol.0 as usize].kind == crate::typed::SymbolKind::ModuleVariable
    }

    /// Returns the nested function the variable `name` holds when a `def` alone binds it.
    fn function_held(&self, name: &str) -> Option<FunctionId> {
        if let Some(symbol) = self.locals.get(name) {
            return self.nested_functions.get(symbol).copied();
        }
        if self.global_names.contains(name) {
            return None;
        }
        let closure = self.closure.as_ref()?;
        match &closure.own {
            Some((own, function)) if own == name => Some(*function),
            _ => match closure.siblings.get(name) {
                Some(sibling) => Some(*sibling),
                None => closure.captures.get(name)?.function,
            },
        }
    }

    /// Returns the running function object, of the running function's `Callable` type.
    fn running_object(&self, function: FunctionId, span: Span) -> TypedExpr {
        let ty = callable_of(&self.program.signatures[function.0 as usize]);
        Self::typed(TypedExprKind::RunningFunction, ty, span)
    }

    /// Returns the read of `name` in a nested function's body when it names the function
    /// itself or a variable it captures.
    pub(super) fn closure_read(&self, name: &str, span: Span) -> Option<TypedExpr> {
        if self.locals.contains_key(name) || self.global_names.contains(name) {
            return None;
        }
        let closure = self.closure.as_ref()?;
        if let Some((own, function)) = &closure.own
            && own == name
        {
            return Some(self.running_object(*function, span));
        }
        let captured = closure.captures.get(name)?;
        let kind = if captured.cell {
            TypedExprKind::CapturedCell(captured.position)
        } else {
            TypedExprKind::Capture(captured.position)
        };
        Some(Self::typed(kind, captured.ty.clone(), span))
    }

    /// Returns the call of `name` when it names a nested function in reach: one a `def` of
    /// this body binds, the running function, or one this function captures; a call by name
    /// runs its code directly, given its function object.
    pub(super) fn nested_call(
        &mut self,
        name: &str,
        name_span: Span,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> Option<TypedExpr> {
        if self.refused_sibling(name) {
            for argument in arguments
                .iter()
                .chain(keywords.iter().map(|keyword| &keyword.value))
            {
                self.expression(argument);
            }
            return Some(Self::typed(
                TypedExprKind::Constant(TypedConstant::None),
                Type::Error,
                span,
            ));
        }
        let function = self.function_held(name)?;
        let sibling = self.closure.as_ref().and_then(|closure| {
            let (_, own) = closure.own.as_ref()?;
            closure.siblings.contains_key(name).then_some(*own)
        });
        let closure = if self.locals.contains_key(name) {
            self.local_function(name, name_span)
        } else if let Some(own) = sibling {
            // Each function of a group holds the captures of the whole group, in one order,
            // so the running object serves the other's code.
            self.running_object(own, name_span)
        } else {
            self.closure_read(name, name_span)?
        };
        Some(
            match self.bind_arguments(function, 0, name, arguments, keywords, span) {
                Ok(bound) => Self::typed(
                    TypedExprKind::CallNested {
                        function,
                        closure: Box::new(closure),
                        arguments: bound,
                    },
                    self.program.signatures[function.0 as usize].returns.clone(),
                    span,
                ),
                Err(error) => error,
            },
        )
    }

    /// Returns true when `name` names a function of the running function's group whose
    /// `def` was refused, so a call of it reports nothing more: one cause gives one error
    /// (DIA-04).
    fn refused_sibling(&self, name: &str) -> bool {
        !self.locals.contains_key(name)
            && !self.global_names.contains(name)
            && self.closure.as_ref().is_some_and(|closure| {
                closure.group.contains(name)
                    && !closure.siblings.contains_key(name)
                    && closure.own.as_ref().is_none_or(|(own, _)| own != name)
            })
    }

    /// Returns the read of the variable `symbol`, named `name`, that holds a nested function,
    /// for a call of it: an error when the call may come before its `def` runs (TYP-12).
    fn local_function(&mut self, name: &str, span: Span) -> TypedExpr {
        self.reading_function = true;
        let read = self.name(name, span);
        self.reading_function = false;
        read
    }

    /// Returns E0202 for a nested function with a keyword-only parameter read as a value,
    /// since a `Callable` passes every argument by position.
    pub(super) fn refused_nested_value(
        &mut self,
        symbol: SymbolId,
        name: &str,
        span: Span,
    ) -> Option<TypedExpr> {
        if self.reading_function {
            return None;
        }
        let function = *self.nested_functions.get(&symbol)?;
        let parameter = self.program.signatures[function.0 as usize]
            .parameters
            .iter()
            .find(|parameter| parameter.kind == ParameterKind::KeywordOnly)?
            .name
            .clone();
        let message = format!(
            "`{name}` takes `{parameter}` by keyword only, and a `Callable` passes every argument \
             by position, so `{name}` has no `Callable` type"
        );
        Some(self.error(
            Diagnostic::new(DiagnosticCode::error(202), span, message).with_rule(RuleId::FUN_05),
            span,
        ))
    }

    /// Reports an assignment to the variable of a nested function, which only its `def`
    /// binds; returns true when it reports one.
    pub(super) fn assigns_nested_function(
        &mut self,
        symbol: SymbolId,
        name: &str,
        span: Span,
    ) -> bool {
        if !self.nested_functions.contains_key(&symbol) {
            return false;
        }
        self.diagnostics.push(also_assigned(name, span));
        true
    }

    /// Gives each cell the nested functions of this body capture the type of its variable,
    /// which an assignment after a `def` may give it, and each function of a group the
    /// functions of the others, once the whole body is checked.
    pub(super) fn settle_closures(&mut self) {
        let mut pending = std::mem::take(&mut self.pending);
        for nested in &mut pending {
            let own = nested.closure.own.as_ref().map(|(own, _)| own.clone());
            nested.closure.siblings = nested
                .closure
                .group
                .iter()
                .filter(|name| Some(*name) != own.as_ref())
                .filter_map(|name| {
                    let symbol = self.locals.get(name)?;
                    Some((name.clone(), *self.nested_functions.get(symbol)?))
                })
                .collect();
            for (name, captured) in &mut nested.closure.captures {
                if captured.cell
                    && let Some(symbol) = self.cell_symbol(name)
                    && let Some(ty) = self.types.get(&symbol)
                {
                    captured.ty = ty.clone();
                }
            }
        }
        self.pending = pending;
    }

    /// Checks the body of each nested function this body defined, in order, with what it
    /// captures, and stores the types of its captures.
    pub(super) fn check_nested(
        program: &mut super::Program,
        pending: Vec<Pending>,
        diagnostics: &mut typy_diagnostics::Diagnostics,
    ) {
        for nested in pending {
            let Stmt::FunctionDef { body, .. } = &nested.statement else {
                continue;
            };
            program
                .closures
                .insert(nested.function, nested.closure.variables());
            let definition = Definition {
                name: nested.definition.name,
                qualified_name: nested.definition.qualified_name,
                class: None,
                takes_instance: false,
                copy_of: None,
                home: None,
                signature: nested.definition.signature,
                type_parameters: Vec::new(),
                parameters: nested.definition.parameters,
                body,
                span: nested.definition.span,
            };
            Self::check_body_of(
                program,
                nested.function,
                &definition,
                Some(nested.closure),
                diagnostics,
            );
        }
    }
}

/// Returns the `Callable` type of a signature: its parameters by position, and its result.
fn callable_of(signature: &FunctionSignature) -> Type {
    Type::Callable {
        parameters: signature
            .parameters
            .iter()
            .map(|parameter| parameter.ty.clone())
            .collect(),
        returns: Box::new(signature.returns.clone()),
    }
}

/// Returns E0202 for a name both a `def` and another statement bind.
fn also_assigned(name: &str, span: Span) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::error(202),
        span,
        format!("`{name}` is a function and is also assigned as a variable"),
    )
    .with_rule(RuleId::STD_04)
}
