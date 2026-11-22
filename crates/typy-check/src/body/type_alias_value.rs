//! Type aliases read as values (SMT-12): the name of a module's alias, or of one imported
//! from another module, gives a `typing.TypeAliasType` object, which `print`, `str`, `repr`,
//! `type()` and `is` read as CPython's does, with its `__name__` and `__module__`.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::Identifier;

use super::BodyChecker;
use crate::typed::{TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Returns the value of `id` when it names a type alias: the object of an alias of the
    /// module or one it imports, or of one a class defines, whose class value reads it by a
    /// hidden name (TYP-18).
    pub(super) fn type_alias_value(&mut self, id: &str, span: Span) -> Option<TypedExpr> {
        if let Some(qualified) = crate::check::class_alias_name(id) {
            let name = qualified.rsplit('.').next().unwrap_or(qualified).to_owned();
            let module = self.program.module.clone();
            let index = self.program.type_alias_object(&module, (&name, qualified));
            return Some(Self::typed(
                TypedExprKind::TypeAliasObject(index),
                Type::TypeAlias(self.local_alias_floor()),
                span,
            ));
        }
        let (module, name) = if let Some(alias) = self.program.imported_aliases.get(id) {
            (alias.module.clone(), alias.name.clone())
        } else if self.program.type_aliases.contains(id) {
            (self.program.module.clone(), id.to_owned())
        } else {
            return None;
        };
        Some(self.type_alias_object(&module, &name, span))
    }

    /// Returns the floor from which every oracle from the program's floor writes an alias a
    /// function or a class defines alike: its qualified name only from 3.15.
    pub(super) const fn local_alias_floor(&self) -> u8 {
        if self.program.floor >= 15 {
            Type::TYPING_ALIKE
        } else {
            15
        }
    }

    /// Checks a function's `type` statement (TYP-18): its variable takes a new alias, a copy
    /// of the alias's description, each time the statement runs.
    pub(super) fn local_type_alias(
        &mut self,
        (name, parameters, value): (
            &Identifier,
            &[typy_source::syntax::TypeParameter],
            &typy_source::syntax::Expr,
        ),
        span: Span,
    ) -> Option<crate::typed::TypedStmt> {
        let qualified = format!(
            "{}.<locals>.{}",
            super::typing_value::without_type_arguments(&self.qualified_name),
            name.name
        );
        let module = self.program.module.clone();
        self.program.alias_sources.insert(
            (module.clone(), qualified.clone()),
            super::program::AliasSource::of(parameters, value),
        );
        let index = self
            .program
            .type_alias_object(&module, (&name.name, &qualified));
        let symbol = *self.locals.get(&name.name)?;
        self.program.alias_symbols.insert(symbol, index);
        let made = Self::typed(
            TypedExprKind::NewTypeAlias(index),
            Type::TypeAlias(self.local_alias_floor()),
            span,
        );
        let target = typy_source::syntax::Expr::Name {
            id: name.name.clone(),
            span: name.span,
        };
        let targets = self
            .assign_target(&target, made.ty.clone(), &made)
            .into_iter()
            .collect();
        Some(crate::typed::TypedStmt::Assign {
            targets,
            value: made,
            span,
        })
    }

    /// Returns the object of the type alias `name` of `module`.
    pub(super) fn type_alias_object(&mut self, module: &str, name: &str, span: Span) -> TypedExpr {
        let index = self.program.type_alias_object(module, (name, name));
        Self::typed(
            TypedExprKind::TypeAliasObject(index),
            Type::TypeAlias(Type::TYPING_ALIKE),
            span,
        )
    }

    /// Checks `alias.attribute` of a type alias: `__name__` and `__module__`.
    pub(super) fn type_alias_attribute(
        &mut self,
        alias: TypedExpr,
        attribute: &Identifier,
        span: Span,
    ) -> TypedExpr {
        let module = match attribute.name.as_str() {
            "__name__" => false,
            "__module__" => true,
            "__value__" | "__type_params__" => {
                return self.alias_source_part(&alias, (&attribute.name, attribute.span), span);
            }
            other => {
                return self.error(
                    Diagnostic::new(
                        DiagnosticCode::error(412),
                        attribute.span,
                        format!(
                            "a TypeAliasType has no attribute `{other}` in TyPy; it has \
                             `__name__` and `__module__`"
                        ),
                    )
                    .with_rule(RuleId::SMT_12),
                    span,
                );
            }
        };
        Self::typed(
            TypedExprKind::TypeAliasPart {
                alias: Box::new(alias),
                module,
            },
            Type::Str,
            span,
        )
    }
}
