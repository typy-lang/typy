//! `frozendict`, a class of the `typy-stdlib` module `_frozendict`, read where CPython reads
//! any mapping: as the dict it holds (DAT-18, ADR-0123).

use typy_diagnostics::Span;
use typy_source::syntax::{Expr, Keyword};

use super::{BodyChecker, Program};
use crate::typed::{ClassId, TypedExpr, TypedExprKind};
use crate::types::Type;

/// The module of `typy-stdlib` that writes `frozendict`.
pub(crate) const FROZENDICT_MODULE: &str = "_frozendict";

/// The field a `frozendict` holds its items in.
const ITEMS: &str = "_items";

/// The methods of a `frozendict` that read its items as a dict's do, so they take the dict's
/// signatures, such as `get` with a default of the value type.
const READ_AS_DICT: [&str; 4] = ["get", "keys", "values", "items"];

/// Returns true for a module of `typy-stdlib` whose classes are built-ins, which show without
/// a module, as `<class 'frozendict'>` does.
#[must_use]
pub fn is_built_in_module(module: &str) -> bool {
    module == FROZENDICT_MODULE
}

impl Program {
    /// Returns true when `class` is `frozendict` or one of its copies.
    pub(crate) fn is_frozendict(&self, class: ClassId) -> bool {
        let root = self
            .class_copies
            .borrow()
            .copies
            .iter()
            .find(|copy| copy.id == class)
            .map_or(class, |copy| copy.root);
        self.classes[root.0 as usize].name == "frozendict"
            && self
                .class_modules
                .get(root.0 as usize)
                .is_some_and(|module| module == FROZENDICT_MODULE)
    }
}

impl BodyChecker<'_> {
    /// Returns `value` as the dict it holds when it is a `frozendict`, which iterates, unpacks
    /// and merges as that dict does; any other value as it is.
    pub(super) fn frozendict_items(&self, value: TypedExpr) -> TypedExpr {
        let Type::Instance(class) = &value.ty else {
            return value;
        };
        let class = class.id;
        if !self.program.is_frozendict(class) {
            return value;
        }
        let Some(index) = self.program.classes[class.0 as usize].attribute(ITEMS) else {
            return value;
        };
        let Some(ty) = self.program.attribute_ty(class, index) else {
            return value;
        };
        let span = value.span;
        Self::typed(
            TypedExprKind::Attribute {
                object: Box::new(value),
                index,
            },
            ty,
            span,
        )
    }

    /// Returns `frozendict(arguments)` made from `dict(arguments)`, since `frozendict` takes
    /// what `dict` takes: a mapping, `(key, value)` pairs, keywords, or nothing; `None` for a
    /// class other than `frozendict`, or a call that already passes `dict(...)`.
    pub(super) fn frozendict_made(
        &mut self,
        class: ClassId,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> Option<TypedExpr> {
        if !self.program.is_frozendict(class) {
            return None;
        }
        if let ([Expr::Call { function, .. }], []) = (arguments, keywords)
            && matches!(function.as_ref(), Expr::Name { id, .. } if id == "dict")
        {
            return None;
        }
        let items = Expr::Call {
            function: Box::new(Expr::Name {
                id: "dict".to_owned(),
                span,
            }),
            arguments: arguments.to_vec(),
            keywords: keywords.to_vec(),
            span,
        };
        Some(self.instance_made(class, &[items], &[], span))
    }

    /// Returns `frozendict(...)` checked as the copy `expected` holds, so an empty call, or
    /// one of an empty display, takes the copy's item types; `None` for any other call.
    pub(super) fn frozendict_expecting(
        &mut self,
        expression: &Expr,
        expected: Option<&Type>,
    ) -> Option<TypedExpr> {
        let Expr::Call {
            function,
            arguments,
            keywords,
            span,
        } = expression
        else {
            return None;
        };
        let Some(Type::Instance(copy)) = expected else {
            return None;
        };
        let named = self.class_named_by(function)?;
        if !self.program.is_frozendict(named) || !self.program.is_frozendict(copy.id) {
            return None;
        }
        self.frozendict_made(copy.id, arguments, keywords, *span)
    }

    /// Returns `receiver` as the dict it holds when it is a `frozendict` and `method` reads
    /// its items as a dict's method does; any other receiver as it is.
    pub(super) fn frozendict_receiver(&self, receiver: TypedExpr, method: &str) -> TypedExpr {
        if READ_AS_DICT.contains(&method) {
            self.frozendict_items(receiver)
        } else {
            receiver
        }
    }
}
