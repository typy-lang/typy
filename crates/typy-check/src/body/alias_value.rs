//! What a type alias read as a value holds (TYP-18, ADR-0087): its `__type_params__`, a tuple
//! of its type parameters, and its `__value__`, the value its `type` statement writes, a class
//! or a typing value read in the names of the alias's module. Both are read from the alias the
//! expression names; one held in a variable is no alias TyPy knows the source of.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Constant, Expr};

use super::BodyChecker;
use super::typing_value::AliasRendering;
use crate::typed::{TypedConstant, TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Returns the module and qualified name of the alias `alias` names: one read by its name,
    /// or the variable a function's or a class's `type` statement binds.
    fn alias_named(&self, alias: &TypedExpr) -> Option<(String, String)> {
        let index = match &alias.kind {
            TypedExprKind::TypeAliasObject(index) => *index,
            // A class's alias is its class value, the module variable named after it.
            TypedExprKind::ModuleVariable(symbol) => {
                let name = self.program.symbols[symbol.0 as usize].name.clone();
                let key = (self.program.module.clone(), name);
                return self.program.alias_sources.contains_key(&key).then_some(key);
            }
            TypedExprKind::Local(symbol) | TypedExprKind::CellRead(symbol) => {
                *self.program.alias_symbols.get(symbol)?
            }
            _ => return None,
        };
        let found = self.program.type_alias_of(index);
        Some((found.module.clone(), found.qualified_name.clone()))
    }

    /// Returns the error for `alias[...]` that is no typing value: E0401 for an alias of a
    /// module without type parameters, as CPython raises `TypeError`, and E0201 for any other,
    /// planned for 1.0 (TYP-22).
    pub(super) fn subscripted_alias(&mut self, alias: &TypedExpr, span: Span) -> TypedExpr {
        if let TypedExprKind::TypeAliasObject(_) = alias.kind {
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(401),
                    span,
                    "Only generic type aliases are subscriptable: CPython raises `TypeError`",
                )
                .with_rule(RuleId::SMT_12),
                span,
            );
        }
        self.error(
            Diagnostic::new(
                DiagnosticCode::error(201),
                span,
                "an alias a function or a class defines, or one held in a variable, given type \
                 arguments as a value is planned for TyPy 1.0 (TYP-22)",
            )
            .with_rule(RuleId::TYP_22),
            span,
        )
    }

    /// Returns `alias.__type_params__` or `alias.__value__` of the alias `alias` names.
    pub(super) fn alias_source_part(
        &mut self,
        alias: &TypedExpr,
        (attribute, at): (&str, Span),
        span: Span,
    ) -> TypedExpr {
        let Some((module, name)) = self.alias_named(alias) else {
            let message = format!(
                "`{attribute}` is read only through the name of an alias, whose `type` \
                 statement TyPy reads"
            );
            return self.error(
                Diagnostic::new(DiagnosticCode::error(202), at, message)
                    .with_rule(RuleId::TYP_18)
                    .with_help("read it through the alias's own name"),
                span,
            );
        };
        let source = self
            .program
            .alias_sources
            .get(&(module.clone(), name.clone()))
            .cloned();
        let Some(source) = source else {
            return Self::typed(
                TypedExprKind::Constant(TypedConstant::None),
                Type::Error,
                span,
            );
        };
        let owner = format!("{module}.{name}");
        if attribute == "__type_params__" {
            return self.alias_parameters_value(&owner, &source.parameters, span);
        }
        // The value reads the names of the alias's module, and its type parameters.
        let elsewhere = (module != self.program.module)
            .then(|| self.program.context_of(&module))
            .flatten()
            .map(|context| self.program.swap_module_context(context));
        let outer = self.alias_rendering.replace(AliasRendering {
            owner,
            parameters: source.parameters.clone(),
        });
        let value = self.alias_value_of(&source.value, span);
        self.alias_rendering = outer;
        if let Some(current) = elsewhere {
            self.program.swap_module_context(current);
        }
        value
    }

    /// Returns the tuple of the type parameters `parameters` of the alias `owner`, each a
    /// typing value written by its name, which CPython keeps one object of.
    fn alias_parameters_value(
        &mut self,
        owner: &str,
        parameters: &[String],
        span: Span,
    ) -> TypedExpr {
        let items: Vec<TypedExpr> = parameters
            .iter()
            .map(|parameter| self.type_parameter_value(owner, parameter, span))
            .collect();
        let ty = Type::Tuple(items.iter().map(|item| item.ty.clone()).collect());
        Self::typed(TypedExprKind::Tuple(items), ty, span)
    }

    /// Returns the type parameter `parameter` of the alias `owner`, a typing value written by
    /// its name, which CPython keeps one object of.
    fn type_parameter_value(&mut self, owner: &str, parameter: &str, span: Span) -> TypedExpr {
        let key = format!("typevar:alias:{owner}.{parameter}");
        let index = self.program.typing_value_object(parameter, &key);
        self.program.single_typing_values.insert(index);
        Self::typed(
            TypedExprKind::TypingValue(index),
            Type::Typing(Type::TYPING_ALIKE),
            span,
        )
    }

    /// Returns the value an alias's `type` statement writes, `value`, read at `span`: a
    /// typing value, a class, another alias or `None`.
    fn alias_value_of(&mut self, value: &Expr, span: Span) -> TypedExpr {
        if let Some(typing) = self.typing_value(value) {
            return TypedExpr { span, ..typing };
        }
        match value {
            Expr::Constant {
                value: Constant::None,
                ..
            } => Self::typed(
                TypedExprKind::Constant(TypedConstant::None),
                Type::NoneType,
                span,
            ),
            Expr::Name { id, .. } => {
                // The alias's own type parameter, as in `type Id[T] = T`.
                if let Some(rendering) = &self.alias_rendering
                    && rendering.parameters.contains(id)
                {
                    let owner = rendering.owner.clone();
                    return self.type_parameter_value(&owner, id, span);
                }
                if let Some((module, name)) = self.module_alias(id, span) {
                    return self.type_alias_object(&module, &name, span);
                }
                if let Some(class) = self.type_object_named(id, span) {
                    return class;
                }
                self.unwritten_value(span)
            }
            _ => self.unwritten_value(span),
        }
    }

    /// Returns E0202 for an alias whose value is no type TyPy writes, such as a name of
    /// `typing` that is no value.
    fn unwritten_value(&mut self, span: Span) -> TypedExpr {
        self.error(
            Diagnostic::new(
                DiagnosticCode::error(202),
                span,
                "this alias's value is no typing value TyPy reads, so `__value__` is not read",
            )
            .with_rule(RuleId::TYP_18),
            span,
        )
    }
}
