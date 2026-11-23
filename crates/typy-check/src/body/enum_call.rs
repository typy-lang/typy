//! What an enum class does as a value (CLS-44, ADR-0116): `Color(value)` finds a member by
//! its value and `Color[name]` by its name, and a loop over the class, `len` and `in` read the
//! list of its members, each through a hidden static method its expansion writes. The
//! variables of its members make each one through `__init__`.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{CompareOperator, Expr, Identifier, Keyword};

use super::{BodyChecker, Program};
use crate::class_type::{ENUM_LOOKUP, ENUM_MEMBER, ENUM_MEMBERS, ENUM_NAMED};
use crate::typed::{ClassId, FunctionId, TypedExpr};
use crate::types::Type;

impl Program {
    /// Returns the hidden static method `name` an enum class's expansion writes, or `None`
    /// for any other class.
    pub(crate) fn enum_method(&self, class: ClassId, name: &str) -> Option<FunctionId> {
        self.classes[class.0 as usize].method(name)
    }
}

impl BodyChecker<'_> {
    /// Returns a member made where its enum class's statement runs, for the hidden name its
    /// variable calls; `None` for any other name.
    pub(super) fn enum_member_made(
        &mut self,
        name: &str,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> Option<TypedExpr> {
        let class = *self
            .program
            .class_ids
            .get(name.strip_prefix(ENUM_MEMBER)?)?;
        Some(self.instance_made(class, arguments, keywords, span))
    }

    /// Returns `Class(value)` of an enum class, which finds the member whose value equals
    /// `value`; `None` for any other class.
    pub(super) fn enum_lookup(
        &mut self,
        class: ClassId,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> Option<TypedExpr> {
        let lookup = self.program.enum_method(class, ENUM_LOOKUP)?;
        let by_keyword = matches!(keywords, [keyword]
            if keyword.name.as_ref().is_some_and(|name| name.name == "value"));
        // One value, by its position or as the keyword `value`.
        let one_value = match arguments.len() {
            1 => keywords.is_empty(),
            0 => by_keyword,
            _ => false,
        };
        if !one_value {
            for argument in arguments
                .iter()
                .chain(keywords.iter().map(|keyword| &keyword.value))
            {
                self.expression(argument);
            }
            let class_name = &self.program.classes[class.0 as usize].name;
            let message = format!(
                "`{class_name}` is called with one value, which finds the member that has it"
            );
            return Some(
                self.error(
                    Diagnostic::new(DiagnosticCode::error(404), span, message)
                        .with_rule(RuleId::CLS_44),
                    span,
                ),
            );
        }
        let name = Identifier {
            name: ENUM_LOOKUP.to_owned(),
            span,
        };
        Some(self.static_call(lookup, class, &name, arguments, keywords, span))
    }

    /// Returns `Class[name]` of an enum class, which finds the member `name` names; `None`
    /// when `value` is no enum class.
    pub(super) fn enum_named(
        &mut self,
        value: &TypedExpr,
        index: &Expr,
        span: Span,
    ) -> Option<TypedExpr> {
        let class = enum_class(self.program, &value.ty)?;
        let named = self.program.enum_method(class, ENUM_NAMED)?;
        let name = Identifier {
            name: ENUM_NAMED.to_owned(),
            span,
        };
        Some(self.static_call(named, class, &name, std::slice::from_ref(index), &[], span))
    }

    /// Returns `value`, or the list of its members when it is an enum class, which a loop,
    /// `len` and `in` read.
    pub(super) fn enum_members(&mut self, value: TypedExpr) -> TypedExpr {
        let Some(class) = enum_class(self.program, &value.ty) else {
            return value;
        };
        let Some(members) = self.program.enum_method(class, ENUM_MEMBERS) else {
            return value;
        };
        let name = Identifier {
            name: ENUM_MEMBERS.to_owned(),
            span: value.span,
        };
        self.static_call(members, class, &name, &[], &[], value.span)
    }

    /// Returns the operands of a comparison with each enum class that `in` or `not in` reads
    /// as the list of its members.
    pub(super) fn enum_containers(
        &mut self,
        operators: &[CompareOperator],
        operands: Vec<TypedExpr>,
    ) -> Vec<TypedExpr> {
        operands
            .into_iter()
            .enumerate()
            .map(|(position, operand)| {
                let contains = position
                    .checked_sub(1)
                    .and_then(|before| operators.get(before))
                    .is_some_and(|operator| {
                        matches!(operator, CompareOperator::In | CompareOperator::NotIn)
                    });
                if contains {
                    self.enum_members(operand)
                } else {
                    operand
                }
            })
            .collect()
    }
}

/// Returns the enum class a value of type `ty` is, when it is one.
fn enum_class(program: &Program, ty: &Type) -> Option<ClassId> {
    let Type::TypeObject(Some(of)) = ty else {
        return None;
    };
    let class = BodyChecker::program_class(of)?;
    program.enum_method(class, ENUM_LOOKUP).map(|_| class)
}
