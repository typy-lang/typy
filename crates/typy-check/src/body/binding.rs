//! Finds the parameter each argument of a call binds to, as CPython binds them (FUN-03,
//! EXP-17): a positional argument to the next parameter that takes one by position, then to
//! `*args`; a keyword argument to the parameter it names, unless that one is positional-only
//! or variadic, then to `**kwargs`.

use typy_source::syntax::ParameterKind;

use crate::typed::SignatureParameter;
use crate::types::Type;

/// The parameter an argument binds to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Slot<'signature> {
    /// The parameter at this index, of this type.
    Named(usize, &'signature Type),
    /// The `*args` or `**kwargs` parameter at this index, each of whose items has this type.
    Packed(usize, &'signature Type),
    /// The positional-only parameter at this index, which a keyword does not reach.
    PositionalOnly(usize),
}

impl<'signature> Slot<'signature> {
    /// Returns the type the argument must have, if it binds to a parameter.
    pub(super) const fn expected(self) -> Option<&'signature Type> {
        match self {
            Self::Named(_, ty) | Self::Packed(_, ty) => Some(ty),
            Self::PositionalOnly(_) => None,
        }
    }
}

/// The parameters of a signature after the instance, by how they take arguments.
pub(super) struct Parameters<'signature> {
    all: &'signature [SignatureParameter],
    /// How many come first and take an argument by position.
    positional: usize,
    /// The index of `*args`.
    star: Option<usize>,
    /// The index of `**kwargs`.
    double_star: Option<usize>,
}

impl<'signature> Parameters<'signature> {
    pub(super) fn of(parameters: &'signature [SignatureParameter]) -> Self {
        let position = |kind| {
            parameters
                .iter()
                .position(|parameter| parameter.kind == kind)
        };
        Self {
            all: parameters,
            positional: parameters
                .iter()
                .filter(|parameter| {
                    matches!(
                        parameter.kind,
                        ParameterKind::PositionalOnly | ParameterKind::Normal
                    )
                })
                .count(),
            star: position(ParameterKind::VarPositional),
            double_star: position(ParameterKind::VarKeyword),
        }
    }

    /// Returns how many parameters take an argument by position, `*args` aside.
    pub(super) const fn positional(&self) -> usize {
        self.positional
    }

    /// Returns true when a parameter is keyword-only.
    pub(super) fn has_keyword_only(&self) -> bool {
        self.all
            .iter()
            .any(|parameter| parameter.kind == ParameterKind::KeywordOnly)
    }

    /// Returns the parameter the positional argument at `index` binds to, or `None` past
    /// those the function takes.
    pub(super) fn of_position(&self, index: usize) -> Option<Slot<'signature>> {
        if index < self.positional {
            return Some(Slot::Named(index, &self.all[index].ty));
        }
        let star = self.star?;
        Some(Slot::Packed(star, packed_item(&self.all[star].ty)))
    }

    /// Returns the parameter the keyword argument `name` binds to, or `None` when no
    /// parameter takes it.
    pub(super) fn of_keyword(&self, name: &str) -> Option<Slot<'signature>> {
        let named = self.all.iter().position(|parameter| {
            parameter.name == name
                && !matches!(
                    parameter.kind,
                    ParameterKind::VarPositional | ParameterKind::VarKeyword
                )
        });
        match (named, self.double_star) {
            (Some(index), _) if self.all[index].kind != ParameterKind::PositionalOnly => {
                Some(Slot::Named(index, &self.all[index].ty))
            }
            // CPython gives `**kwargs` a keyword that names a positional-only parameter.
            (_, Some(double_star)) => Some(Slot::Packed(
                double_star,
                packed_item(&self.all[double_star].ty),
            )),
            (Some(index), None) => Some(Slot::PositionalOnly(index)),
            (None, None) => None,
        }
    }
}

impl<'signature> Parameters<'signature> {
    /// Returns the type of each parameter a positional value at `position` or later may fill,
    /// and the item type of `*args`.
    pub(super) fn reached_from(&self, position: usize) -> Vec<&'signature Type> {
        self.all
            .iter()
            .enumerate()
            .filter_map(|(index, parameter)| match parameter.kind {
                ParameterKind::PositionalOnly | ParameterKind::Normal if index >= position => {
                    Some(&parameter.ty)
                }
                ParameterKind::VarPositional => Some(packed_item(&parameter.ty)),
                _ => None,
            })
            .collect()
    }

    /// Returns the type of each parameter a keyword may fill, and the value type of
    /// `**kwargs`.
    pub(super) fn reached_by_name(&self) -> Vec<&'signature Type> {
        self.all
            .iter()
            .filter_map(|parameter| match parameter.kind {
                ParameterKind::Normal | ParameterKind::KeywordOnly => Some(&parameter.ty),
                ParameterKind::VarKeyword => Some(packed_item(&parameter.ty)),
                _ => None,
            })
            .collect()
    }
}

/// Returns the type of each item of a `*args` parameter, of type `tuple[T, ...]`, or of each
/// value of a `**kwargs` parameter, of type `dict[str, T]`: `T`.
fn packed_item(ty: &Type) -> &Type {
    match ty {
        Type::TupleOf(item) | Type::Dict(_, item) => item,
        other => other,
    }
}
