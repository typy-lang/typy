//! Resolves names read as values: locals, `__name__`, functions, builtins and names TyPy does not provide.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};

use super::BodyChecker;
use super::flow::Fact;
use crate::builtin::Builtin;
use crate::typed::{SymbolId, SymbolKind, TypedExpr, TypedExprKind};
use crate::types::Type;
use crate::unprovided::{Unprovided, unprovided};

/// Module-level names that CPython defines and TyPy 0.1 does not provide.
const MODULE_DUNDERS: &[&str] = &[
    "__annotations__",
    "__builtins__",
    "__doc__",
    "__file__",
    "__loader__",
    "__package__",
    "__spec__",
];

impl BodyChecker<'_> {
    /// Resolves a name that is read as a value (LR 4.2.2).
    pub(super) fn name(&mut self, id: &str, span: Span) -> TypedExpr {
        // A structure class statement binds its name to this hidden name (ADR-0049).
        if let Some(class) = self.program.foreign.structure_named(id) {
            return Self::cdata_class(class, span);
        }
        // A nested function reads its own name and its captures from its function object
        // (FUN-04).
        if let Some(read) = self.closure_read(id, span) {
            return read;
        }
        if self.in_function()
            && !self.global_names.contains(id)
            && !self.locals.contains_key(id)
            && let Some(symbol) = self.program.module_symbols.get(id).copied()
        {
            return self.module_variable(id, symbol, span);
        }
        if let Some(symbol) = self.locals.get(id).copied() {
            if self.global_names.contains(id) {
                return self.module_variable(id, symbol, span);
            }
            if let Some(assigned) = &self.assigned
                && !assigned.contains(&Fact::Assigned(symbol))
                && self.reported_unassigned.insert(symbol)
            {
                self.diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::error(306),
                        span,
                        format!("`{id}` may be used before it is assigned"),
                    )
                    .with_rule(RuleId::TYP_12)
                    .with_help(format!(
                        "assign `{id}` on every path that reaches this line"
                    )),
                );
            }
            // The instance used other than through its attributes may reach a method that an
            // override in a subclass defines, so `super().__init__()` in a subclass must come
            // after the subclass's own attributes are assigned (ADR-0021).
            if let Some(initializing) = self.initializing
                && initializing.instance == symbol
            {
                self.program.classes[initializing.class.0 as usize].init_uses_instance = true;
            }
            if let Some(initializing) = self.initializing
                && initializing.instance == symbol
                && !self.reported_incomplete
                && !self.instance_is_complete(initializing)
            {
                self.reported_incomplete = true;
                self.incomplete_instance(initializing, span, false);
            }
            if let Some(refused) = self.refused_nested_value(symbol, id, span) {
                return refused;
            }
            let ty = self.types.get(&symbol).cloned().unwrap_or(Type::Error);
            let kind = self.read_kind(id, symbol);
            let read = Self::typed(kind, ty.clone(), span);
            let not_none = self
                .assigned
                .as_ref()
                .is_some_and(|facts| facts.contains(&Fact::NotNone(symbol)));
            if let Some(narrowed) = self.narrowed_type(symbol, id) {
                // A narrowed `T | None` a test showed is not `None` is a `T`.
                let narrowed = match narrowed {
                    Type::Optional(inner) if not_none => *inner,
                    other => other,
                };
                return Self::narrowed_read(read, narrowed, span);
            }
            // A test before this point showed the optional variable is not `None` (TYP-13);
            // a call may assign a variable a function declares `global` or `nonlocal`, so it
            // is not narrowed.
            if let Type::Optional(inner) = ty
                && !self.changed_by_calls(id)
                && self
                    .assigned
                    .as_ref()
                    .is_some_and(|facts| facts.contains(&Fact::NotNone(symbol)))
            {
                return Self::typed(TypedExprKind::Unwrap(Box::new(read)), *inner, span);
            }
            return read;
        }
        if id == "NotImplemented" {
            return Self::typed(TypedExprKind::NotImplemented, Type::NotImplemented, span);
        }
        if id == "__name__" {
            return Self::typed(TypedExprKind::ModuleName, Type::Str, span);
        }
        if let Some(read) = self.rebound_read(id, span) {
            return read;
        }
        if let Some(refused) = self.annotation_only(id, span) {
            return refused;
        }
        if let Some(type_object) = self.type_object_named(id, span) {
            return self.through_lazy(id, type_object);
        }
        if let Some(refused) = self.function_as_value(id, span) {
            return refused;
        }
        self.unknown_name(id, span)
    }

    /// Returns how the variable `symbol`, named `id`, is read: as a module variable, through
    /// the cell a nested function shares it in (FUN-04), or from its slot.
    fn read_kind(&mut self, id: &str, symbol: SymbolId) -> TypedExprKind {
        if self.program.symbols[symbol.0 as usize].kind == SymbolKind::ModuleVariable {
            return TypedExprKind::ModuleVariable(symbol);
        }
        if self.cell_symbol(id) == Some(symbol) {
            self.used_cells.insert(symbol);
            return TypedExprKind::CellRead(symbol);
        }
        TypedExprKind::Local(symbol)
    }

    /// Returns a scalar class of `ctypes` read as a value (FFI-03), the function object of a
    /// function of the program (FUN-05), or the error for a built-in function read as one,
    /// which is planned for 0.10 (FUN-08).
    fn function_as_value(&mut self, id: &str, span: Span) -> Option<TypedExpr> {
        if let Some(super::builtin_module::ModuleFunction::CtypesType(ty)) =
            self.program.module_functions.get(id)
        {
            return Some(Self::cdata_class(crate::ctypes::CData::Scalar(*ty), span));
        }
        if let Some(function) = self.program.function_ids.get(id).copied() {
            let value = self.function_value(function, id, span);
            return Some(self.through_lazy(id, value));
        }
        if let Some(builtin) = self.unbound_builtin_function(id, span) {
            return Some(self.builtin_without_context(builtin, span));
        }
        let read = typy_source::syntax::Expr::Name {
            id: id.to_owned(),
            span,
        };
        if self.module_function_named(&read).is_some() {
            return Some(self.module_function_without_context(id, span));
        }
        if !self.program.module_functions.contains_key(id) && Builtin::named(id).is_none() {
            return None;
        }
        Some(
            self.error(
                Diagnostic::new(
                    DiagnosticCode::error(201),
                    span,
                    format!("using `{id}` as a value is planned for TyPy 0.10 (FUN-08)"),
                )
                .with_rule(RuleId::FUN_08),
                span,
            ),
        )
    }

    /// Reads a module variable from a function: its value as it is now, which may be unbound
    /// and then raises `NameError`, and is never narrowed, since a call may assign it (SMT-08).
    fn module_variable(&mut self, id: &str, symbol: SymbolId, span: Span) -> TypedExpr {
        let ty = self.program.symbols[symbol.0 as usize].ty.clone();
        if ty == Type::Error && self.program.provisional {
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(424),
                    span,
                    format!("an `__init__` reads `{id}` before the module's code gives it a type"),
                )
                .with_rule(RuleId::SMT_08)
                .with_help(format!(
                    "declare its type at module level, such as `{id}: int = ...`"
                )),
                span,
            );
        }
        Self::typed(TypedExprKind::ModuleVariable(symbol), ty, span)
    }

    pub(super) fn unknown_name(&mut self, id: &str, span: Span) -> TypedExpr {
        let diagnostic = match unprovided(id) {
            Some(Unprovided::Planned { release, rule }) => Diagnostic::new(
                DiagnosticCode::error(201),
                span,
                format!("`{id}` is planned for TyPy {release} ({rule})"),
            )
            .with_rule(rule),
            Some(Unprovided::Excluded) => Diagnostic::new(
                DiagnosticCode::error(202),
                span,
                format!("`{id}` is a Python builtin that TyPy does not provide"),
            )
            .with_rule(RuleId::STD_04),
            None if MODULE_DUNDERS.contains(&id) => Diagnostic::new(
                DiagnosticCode::error(202),
                span,
                format!("`{id}` is a module attribute that TyPy does not provide"),
            )
            .with_rule(RuleId::STD_04),
            None => Diagnostic::new(
                DiagnosticCode::error(205),
                span,
                format!("`{id}` is not defined"),
            )
            .with_rule(RuleId::STD_04),
        };
        self.error(diagnostic, span)
    }
}

impl BodyChecker<'_> {
    /// Returns the narrowest type `isinstance` tests showed the variable holds here, if any;
    /// a variable a function declares `global` may be assigned by a call, so it is not
    /// narrowed (TYP-07).
    /// Returns true when a call may assign the variable `id`: a module variable a function
    /// declares `global` (SMT-08), or a variable a nested function assigns through
    /// `nonlocal` (SMT-09), so no test or assignment narrows it.
    pub(super) fn changed_by_calls(&self, id: &str) -> bool {
        self.program.assigned_in_functions.contains(id)
            || self.closure_scope.rebound.contains(id)
            || self.nonlocal_names.contains_key(id)
    }

    pub(super) fn narrowed_type(&self, symbol: crate::typed::SymbolId, id: &str) -> Option<Type> {
        if let Some((_, member)) = self
            .splitting
            .iter()
            .rev()
            .find(|(each, _)| *each == super::each_member::Split::variable(symbol))
        {
            return Some(member.clone());
        }
        if self.changed_by_calls(id) {
            return None;
        }
        let facts = self.assigned.as_ref()?;
        let mut narrowest: Option<&Type> = None;
        for fact in facts {
            let Fact::Narrowed(each, position) = fact else {
                continue;
            };
            let Some(ty) = self
                .narrowings
                .get(*position as usize)
                .filter(|_| *each == symbol)
            else {
                continue;
            };
            if narrowest.is_none_or(|current| ty.is_assignable_to(current)) {
                narrowest = Some(ty);
            }
        }
        narrowest.cloned()
    }

    /// Returns the read of a variable as the type a test narrowed it to: the value an
    /// optional holds, then the instance as one of a derived class.
    fn narrowed_read(read: TypedExpr, narrowed: Type, span: Span) -> TypedExpr {
        let read = match &read.ty {
            Type::Optional(inner) if !matches!(narrowed, Type::Optional(_)) => {
                let inner = inner.as_ref().clone();
                Self::typed(TypedExprKind::Unwrap(Box::new(read)), inner, span)
            }
            _ => read,
        };
        if read.ty == narrowed {
            return read;
        }
        Self::typed(TypedExprKind::Downcast(Box::new(read)), narrowed, span)
    }

    /// Reports a name that only annotations read: a type alias of the module, or a name of
    /// `typing`, `collections.abc` or `types`.
    fn annotation_only(&mut self, id: &str, span: Span) -> Option<TypedExpr> {
        if let Some(alias) = self.type_alias_value(id, span) {
            return Some(alias);
        }
        if self.program.annotation_names.contains_key(id)
            || self.program.annotation_modules.contains_key(id)
        {
            return Some(
                self.error(
                    Diagnostic::new(
                        DiagnosticCode::error(201),
                        span,
                        format!("using `{id}` as a value is planned for TyPy 0.10 (TYP-18)"),
                    )
                    .with_rule(RuleId::TYP_18)
                    .with_help("TyPy reads it in annotations, `isinstance` and class patterns"),
                    span,
                ),
            );
        }
        None
    }
}
