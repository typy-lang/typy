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
    /// module or one it imports, or the error for an alias a function or a class defines.
    pub(super) fn type_alias_value(&mut self, id: &str, span: Span) -> Option<TypedExpr> {
        if self.local_aliases.contains(id) {
            return Some(self.error(
                Diagnostic::new(
                    DiagnosticCode::error(201),
                    span,
                    format!(
                        "reading the type alias `{id}`, which a function or a class defines, as \
                         a value is planned for TyPy 0.9 (TYP-18)"
                    ),
                )
                .with_rule(RuleId::TYP_18)
                .with_help("define the alias at the top of the module"),
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

    /// Returns the object of the type alias `name` of `module`.
    pub(super) fn type_alias_object(&mut self, module: &str, name: &str, span: Span) -> TypedExpr {
        let index = self.program.type_alias_object(module, name);
        Self::typed(TypedExprKind::TypeAliasObject(index), Type::TypeAlias, span)
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
                return self.error(
                    Diagnostic::new(
                        DiagnosticCode::error(201),
                        attribute.span,
                        format!(
                            "`{}` of a type alias is planned for TyPy 0.9 (TYP-18)",
                            attribute.name
                        ),
                    )
                    .with_rule(RuleId::TYP_18),
                    span,
                );
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
