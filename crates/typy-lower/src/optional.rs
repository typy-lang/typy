//! Lowers optional values: `None` held in a value each type never takes (TYP-06, TYP-13,
//! ADR-0011).

use typy_check::{Type, TypedExpr};
use typy_diagnostics::Span;
use typy_source::syntax::CompareOperator;
use typy_tyir::{Constant, Operation, Terminator, TyirType, ValueId};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

impl FunctionLowerer<'_> {
    /// Returns `None` of an optional TyIR type.
    pub(crate) fn none_of(&mut self, ty: TyirType, span: Span) -> ValueId {
        let TyirType::Optional(base) = ty else {
            unreachable!("only an optional type holds `None` as a value")
        };
        self.constant(Constant::NoneOf(base), span)
    }

    /// Returns `inner`, a value of the base or `None`, as an optional value of type `ty`; the
    /// optional takes over the value's reference.
    pub(crate) fn wrap(&mut self, inner: &TypedExpr, ty: TyirType, span: Span) -> Lowered {
        if inner.ty == Type::NoneType {
            return Self::borrowed(self.none_of(ty, span), ty);
        }
        let value = self.expression(inner);
        let wrapped = self.builder.push_value(
            Operation::Wrap {
                value: value.value,
                ty,
            },
            ty,
            span,
        );
        self.transfer(value, wrapped, ty)
    }

    /// Returns an instance, or an optional one, as an instance of a base, of type `ty`; the
    /// result takes over the value's reference.
    pub(crate) fn upcast(&mut self, inner: &TypedExpr, ty: TyirType, span: Span) -> Lowered {
        let value = self.expression(inner);
        // A union whose members are held alike is held as they are.
        if value.ty == ty {
            return value;
        }
        let upcast = self.builder.push_value(
            Operation::Upcast {
                value: value.value,
                ty,
            },
            ty,
            span,
        );
        self.transfer(value, upcast, ty)
    }

    /// Returns `value` as a variable of type `ty` holds it: a value of the base of an optional
    /// `ty` is wrapped, as a `for` loop binds an item to a `T | None` variable, and an instance
    /// of a class derived from the base is held as the base first; any other value is returned
    /// as it is. The result takes over the value's reference.
    pub(crate) fn held_as(&mut self, value: Lowered, ty: TyirType, span: Span) -> Lowered {
        if value.ty == ty || !matches!(ty, TyirType::Optional(_)) {
            return value;
        }
        let base = self.base_type(ty);
        // The checker lets an instance of a derived class fill a `Base | None` parameter
        // from a tuple `*` unpacks (EXP-17), so it is held as the base before it is wrapped.
        let value = match (value.ty, base) {
            (TyirType::Object(from), TyirType::Object(to))
                if from != to && from.class_index().is_some() && to.class_index().is_some() =>
            {
                let upcast = self.builder.push_value(
                    Operation::Upcast {
                        value: value.value,
                        ty: base,
                    },
                    base,
                    span,
                );
                self.transfer(value, upcast, base)
            }
            _ => value,
        };
        if value.ty != base {
            return value;
        }
        let wrapped = self.builder.push_value(
            Operation::Wrap {
                value: value.value,
                ty,
            },
            ty,
            span,
        );
        self.transfer(value, wrapped, ty)
    }

    /// Returns the value an optional holds, which a test showed is not `None`; it takes over
    /// the optional's reference.
    pub(crate) fn unwrap(&mut self, inner: &TypedExpr, ty: TyirType, span: Span) -> Lowered {
        let value = self.expression(inner);
        let unwrapped = self
            .builder
            .push_value(Operation::Unwrap(value.value), ty, span);
        self.transfer(value, unwrapped, ty)
    }

    /// Returns whether an optional value is `None`; the value is not released.
    pub(crate) fn is_none(&mut self, value: Lowered, span: Span) -> ValueId {
        self.builder
            .push_value(Operation::IsNone(value.value), TyirType::Bool, span)
    }

    /// Returns a value of type `ty` that depends on whether an optional value is `None`:
    /// `when_none` gives it for `None`, and `when_some` from the value the optional holds,
    /// borrowed from it. The optional is not released.
    pub(crate) fn optional_branch(
        &mut self,
        optional: Lowered,
        ty: TyirType,
        span: Span,
        when_none: impl FnOnce(&mut Self) -> Lowered,
        when_some: impl FnOnce(&mut Self, Lowered) -> Lowered,
    ) -> Lowered {
        let condition = self.is_none(optional, span);
        let none = self.builder.create_block();
        let some = self.builder.create_block();
        let join = self.builder.create_block();
        let result = self.builder.add_block_parameter(join, ty);
        self.builder.terminate(Terminator::Branch {
            condition,
            then: none,
            otherwise: some,
        });
        self.builder.switch_to(none);
        let given = when_none(self);
        let given = self.take_reference(given, span);
        self.builder.terminate(Terminator::Jump {
            target: join,
            arguments: vec![given],
        });
        self.builder.switch_to(some);
        let base = self.base_type(optional.ty);
        let held = self
            .builder
            .push_value(Operation::Unwrap(optional.value), base, span);
        let given = when_some(self, Self::borrowed(held, base));
        let given = self.take_reference(given, span);
        self.builder.terminate(Terminator::Jump {
            target: join,
            arguments: vec![given],
        });
        self.builder.switch_to(join);
        self.produced(result, ty)
    }

    /// Returns `left == right`, or `left != right`, where either is optional: `None` equals
    /// only `None`, and two values compare as their types do (EXP-06). The operands are not
    /// released.
    pub(crate) fn optional_equality(
        &mut self,
        operator: CompareOperator,
        left: Lowered,
        right: Lowered,
        span: Span,
    ) -> ValueId {
        let equal = operator == CompareOperator::Eq;
        let (held, other, reversed) = if matches!(left.ty, TyirType::Optional(_)) {
            (left, right, false)
        } else {
            (right, left, true)
        };
        let result = self.optional_branch(
            held,
            TyirType::Bool,
            span,
            |this| {
                // `None` on this side: equal exactly when the other side is `None` too.
                let other_none = this.none_test(other, span);
                Self::borrowed(this.equal_or_not(equal, other_none, span), TyirType::Bool)
            },
            |this, value| {
                let compared = if matches!(other.ty, TyirType::Optional(_)) {
                    this.optional_branch(
                        other,
                        TyirType::Bool,
                        span,
                        |this| {
                            let never = this.constant(Constant::Bool(!equal), span);
                            Self::borrowed(never, TyirType::Bool)
                        },
                        |this, other| {
                            let compared =
                                this.ordered_compare(operator, (value, other), reversed, span);
                            Self::borrowed(compared, TyirType::Bool)
                        },
                    )
                    .value
                } else if other.ty == TyirType::Unit {
                    this.constant(Constant::Bool(!equal), span)
                } else {
                    this.ordered_compare(operator, (value, other), reversed, span)
                };
                Self::borrowed(compared, TyirType::Bool)
            },
        );
        result.value
    }

    /// Returns whether a value is `None`: an optional's test, true for `None` itself, and
    /// false for a value of any other type.
    fn none_test(&mut self, value: Lowered, span: Span) -> ValueId {
        match value.ty {
            TyirType::Optional(_) => self.is_none(value, span),
            TyirType::Unit => self.constant(Constant::Bool(true), span),
            _ => self.constant(Constant::Bool(false), span),
        }
    }

    /// Returns `truth` for `==`, and its negation for `!=`.
    fn equal_or_not(&mut self, equal: bool, truth: ValueId, span: Span) -> ValueId {
        if equal {
            return truth;
        }
        self.builder
            .push_value(Operation::BoolNot(truth), TyirType::Bool, span)
    }

    /// Compares two values with `operator`, keeping their order in the source: `reversed`
    /// when `first` was written on the right.
    fn ordered_compare(
        &mut self,
        operator: CompareOperator,
        (first, second): (Lowered, Lowered),
        reversed: bool,
        span: Span,
    ) -> ValueId {
        if reversed {
            self.compare(operator, second, first, span)
        } else {
            self.compare(operator, first, second, span)
        }
    }

    /// Returns the type an optional value of type `ty` holds when it is not `None`.
    pub(crate) fn base_type(&self, ty: TyirType) -> TyirType {
        let TyirType::Optional(base) = ty else {
            unreachable!("an optional type has a base")
        };
        self.tuple_types.optional_value(base)
    }

    /// Returns the truth of an optional value: `None` is false, and any other value is tested
    /// as its base is.
    pub(crate) fn optional_truth(&mut self, optional: Lowered, span: Span) -> ValueId {
        let result = self.optional_branch(
            optional,
            TyirType::Bool,
            span,
            |lowerer| {
                Self::borrowed(
                    lowerer.constant(Constant::Bool(false), span),
                    TyirType::Bool,
                )
            },
            |lowerer, value| {
                let truth = lowerer.truth(value, span);
                Self::borrowed(truth, TyirType::Bool)
            },
        );
        result.value
    }

    /// Returns `str` or, with `repr`, `repr` of an optional value: `None`, or the text of the
    /// value it holds.
    pub(crate) fn optional_text(&mut self, optional: Lowered, repr: bool, span: Span) -> Lowered {
        self.optional_branch(
            optional,
            TyirType::STR,
            span,
            |lowerer| Self::borrowed(lowerer.string("None", span), TyirType::STR),
            |lowerer, value| {
                if repr {
                    lowerer.repr_of(value, span)
                } else {
                    lowerer.string_of(value, span)
                }
            },
        )
    }
}
