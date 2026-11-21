//! Lowers operators that call operator or comparison methods (CLS-07, CLS-21, ADR-0026,
//! ADR-0038). The methods run in CPython's order: the reflected method of a right operand whose
//! class derives from the left's may run first, and a method that returns `NotImplemented`
//! gives way to the next. After the last, `==` and `!=` compare identities, and any other
//! operator raises `TypeError`.

use typy_check::{FunctionId, OperatorCall, OperatorKind, Type};
use typy_diagnostics::Span;
use typy_tyir::{BoolOperator, Operation, RuntimeFunction, Terminator, TyirType, TypeId};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

/// One step of an operator call.
#[derive(Clone)]
enum Step {
    /// Calls the method, with the operands exchanged when `exchanged`.
    Call { method: FunctionId, exchanged: bool },
    /// Passes over the rest when both operands are of one class.
    UnlessSameClass,
    /// Goes on with `first` when the right operand's reflected method runs first, and with
    /// `second` otherwise.
    Choose { first: Vec<Step>, second: Vec<Step> },
    /// Calls the reflected method of the class derived from the right operand's that the
    /// right object is an instance of, when it is of one, with the operands exchanged.
    Below(Vec<(typy_check::ClassId, FunctionId)>),
}

/// The operands of an operator call, evaluated once, and its result.
#[derive(Clone, Copy)]
struct Operands<'call> {
    call: &'call OperatorCall,
    left: Lowered,
    right: Lowered,
    result: &'call Type,
    ty: TyirType,
}

impl FunctionLowerer<'_> {
    /// Returns `left op right` through its methods, a value of `result`, held as `ty`.
    pub(crate) fn operator_call(
        &mut self,
        call: &OperatorCall,
        result: &Type,
        ty: TyirType,
        span: Span,
    ) -> Lowered {
        let left = self.expression(&call.left);
        let left = self.kept_for_call(&call.left, left, span);
        let right = self.expression(&call.right);
        let right = self.kept_for_call(&call.right, right, span);
        let operands = Operands {
            call,
            left,
            right,
            result,
            ty,
        };
        let steps = self.operator_steps(call);
        let outcome = self.tried(operands, &steps, span);
        let outcome = if call.negated {
            let negated =
                self.builder
                    .push_value(Operation::BoolNot(outcome.value), TyirType::Bool, span);
            Self::borrowed(negated, TyirType::Bool)
        } else {
            outcome
        };
        self.release(right, span);
        self.release(left, span);
        outcome
    }

    /// Returns the steps of an operator call. An arithmetic operator calls the reflected
    /// method only for operands of different classes, and first only for a right operand of a
    /// class that derives from the left's and defines it again; a comparison calls it for any
    /// operands, and first for a right operand of a class that derives from the left's.
    fn operator_steps(&self, call: &OperatorCall) -> Vec<Step> {
        let mut steps: Vec<Step> = call
            .in_place
            .map(|method| Step::Call {
                method,
                exchanged: false,
            })
            .into_iter()
            .collect();
        let forward = |method| Step::Call {
            method,
            exchanged: false,
        };
        let reflected = |method| Step::Call {
            method,
            exchanged: true,
        };
        let both_instances = matches!(
            (&call.left.ty, &call.right.ty),
            (Type::Instance(_), Type::Instance(_))
        );
        // The reflected method of the right operand's class, or of the class derived from it
        // that the right object is of (CLS-07).
        let reflected_step = call.reflected.map(reflected).or_else(|| {
            (!call.reflected_below.is_empty()).then(|| Step::Below(call.reflected_below.clone()))
        });
        let rest = match (call.kind, call.forward, reflected_step) {
            (OperatorKind::Arithmetic(_), forward_method, Some(step)) => {
                let mut later = Vec::new();
                if both_instances {
                    later.push(Step::UnlessSameClass);
                }
                later.push(step.clone());
                match forward_method {
                    Some(first) if self.swaps(call) => vec![Step::Choose {
                        first: vec![step, forward(first)],
                        second: std::iter::once(forward(first)).chain(later).collect(),
                    }],
                    Some(first) => std::iter::once(forward(first)).chain(later).collect(),
                    None => later,
                }
            }
            (_, Some(first), Some(step)) if self.may_derive(call) => vec![Step::Choose {
                first: vec![step.clone(), forward(first)],
                second: vec![forward(first), step],
            }],
            (_, Some(first), Some(step)) => vec![forward(first), step],
            (_, Some(first), None) => vec![forward(first)],
            (_, None, Some(step)) => vec![step],
            (_, None, None) => Vec::new(),
        };
        steps.extend(rest);
        steps
    }

    /// Returns true when the right operand's reflected arithmetic method may run first: a
    /// reflected method no class overrides is one function for every class, so when the left
    /// object's class has the same one, CPython never calls it first.
    fn swaps(&self, call: &OperatorCall) -> bool {
        !call.reflected_below.is_empty()
            || call.reflected != call.left_reflected
            || call
                .left_reflected
                .is_some_and(|method| self.methods.dispatch(method).is_some())
    }

    /// Returns true when the right object's class may derive from the left object's, as it
    /// can only when one static class derives from the other and one has a subclass.
    fn may_derive(&self, call: &OperatorCall) -> bool {
        let (Type::Instance(left), Type::Instance(right)) = (&call.left.ty, &call.right.ty) else {
            return false;
        };
        (self.class_derives(right.id.0, left.id.0) || self.class_derives(left.id.0, right.id.0))
            && (self.program.has_subclass(left.id) || self.program.has_subclass(right.id))
    }

    /// Returns what the steps give: each method's result unless it is `NotImplemented`, which
    /// gives way to the next step, and after the last, what the operator's kind says.
    fn tried(&mut self, operands: Operands<'_>, steps: &[Step], span: Span) -> Lowered {
        let Some((step, rest)) = steps.split_first() else {
            return self.operator_fallback(operands, span);
        };
        match step {
            Step::Call { method, exchanged } => {
                let signature = self.program.function(*method).signature.clone();
                let own = signature.returns;
                let own_ty = self.lowered_type(&own);
                let (call, left, right) = (operands.call, operands.left, operands.right);
                let (first, second) = if *exchanged {
                    ((right, &call.right.ty), (left, &call.left.ty))
                } else {
                    ((left, &call.left.ty), (right, &call.right.ty))
                };
                let first = self.operand_for(first, signature.parameters.first(), span);
                let second = self.operand_for(second, signature.parameters.get(1), span);
                let borrowed = |value: Lowered| Self::borrowed(value.value, value.ty);
                let outcome = self.call_method(
                    *method,
                    vec![(0, borrowed(first)), (1, borrowed(second))],
                    own_ty,
                    span,
                );
                self.release(second, span);
                self.release(first, span);
                if !own.may_give_way() {
                    return self.settled(outcome, &own, operands, span);
                }
                let members = own.members().to_vec();
                let given =
                    self.each_member_of(outcome, &own, operands.ty, span, |this, index, held| {
                        if members[index] == Type::NotImplemented {
                            this.tried(operands, rest, span)
                        } else {
                            this.settled(held, &members[index], operands, span)
                        }
                    });
                self.release(outcome, span);
                given
            }
            Step::UnlessSameClass => {
                let same = self.same_class(operands.left, operands.right, span);
                self.either(
                    same,
                    operands.ty,
                    span,
                    |this| this.operator_fallback(operands, span),
                    |this| this.tried(operands, rest, span),
                )
            }
            Step::Below(candidates) => self.below(operands, candidates, rest, span),
            Step::Choose { first, second } => {
                let arithmetic = matches!(operands.call.kind, OperatorKind::Arithmetic(_));
                let swap = self.reflected_first(
                    operands.call,
                    (operands.left, operands.right),
                    arithmetic,
                    span,
                );
                self.either(
                    swap,
                    operands.ty,
                    span,
                    |this| this.tried(operands, first, span),
                    |this| this.tried(operands, second, span),
                )
            }
        }
    }

    /// Returns an operand, of its checked type, as the argument `parameter` takes: a value a
    /// tagged union holds is tagged, which the caller releases after the call; any other is
    /// borrowed, and the call casts an object to the class the parameter names.
    pub(crate) fn operand_for(
        &mut self,
        (value, from): (Lowered, &Type),
        parameter: Option<&typy_check::SignatureParameter>,
        span: Span,
    ) -> Lowered {
        let borrowed = Self::borrowed(value.value, value.ty);
        let Some(parameter) = parameter else {
            return borrowed;
        };
        let wanted = self.lowered_type(&parameter.ty);
        // A parameter of type `object`, such as `other` of `__eq__`, takes the operand boxed
        // when it is held by value (TYP-23).
        if parameter.ty == Type::Object && *from != Type::Object {
            return self.boxed(borrowed, from, span);
        }
        if value.ty == wanted || (!self.involves_tag(wanted) && !self.involves_tag(value.ty)) {
            return borrowed;
        }
        self.union_converted(borrowed, from, &parameter.ty, wanted, true, span)
    }

    /// Returns what the steps give after the reflected method of the first of `candidates`
    /// the right object is an instance of, called with the right object as an instance of its
    /// class; with none, what the steps give.
    fn below(
        &mut self,
        operands: Operands<'_>,
        candidates: &[(typy_check::ClassId, FunctionId)],
        rest: &[Step],
        span: Span,
    ) -> Lowered {
        let Some((&(class, method), others)) = candidates.split_first() else {
            return self.tried(operands, rest, span);
        };
        let right = operands.right;
        let object = self.bits(Self::borrowed(right.value, right.ty), span);
        let name = self.program.class(class).name.clone();
        let of = self.type_object(&typy_check::TypeObjectOf::Instance(class, name), span);
        let found = self
            .call_runtime(RuntimeFunction::IsInstance, vec![object, of.value], span)
            .value;
        self.either(
            found,
            operands.ty,
            span,
            |this| {
                let ty = TyirType::Object(TypeId::class(class.0));
                let narrowed = this.builder.push_value(
                    Operation::Downcast {
                        value: right.value,
                        ty,
                    },
                    ty,
                    span,
                );
                let narrowed = Operands {
                    right: Self::borrowed(narrowed, ty),
                    ..operands
                };
                let steps: Vec<Step> = std::iter::once(Step::Call {
                    method,
                    exchanged: true,
                })
                .chain(rest.iter().cloned())
                .collect();
                this.tried(narrowed, &steps, span)
            },
            |this| this.below(operands, others, rest, span),
        )
    }

    /// Returns `value`, of the checked type `from`, as a value of the call's result.
    fn settled(
        &mut self,
        value: Lowered,
        from: &Type,
        operands: Operands<'_>,
        span: Span,
    ) -> Lowered {
        if from == operands.result || value.ty == operands.ty {
            return value;
        }
        self.widened_to_union(value, from, operands.result, operands.ty, span)
    }

    /// Returns what the operator gives once every method gave way: `==` and `!=` compare
    /// identities, and any other operator raises CPython's `TypeError`.
    fn operator_fallback(&mut self, operands: Operands<'_>, span: Span) -> Lowered {
        let (function, symbol) = match operands.call.kind {
            OperatorKind::Equality { different } => {
                let same = self.identity(!different, operands.left, operands.right, span);
                let same = Self::borrowed(same, TyirType::Bool);
                return self.settled(same, &Type::Bool, operands, span);
            }
            OperatorKind::Arithmetic(symbol) => (RuntimeFunction::OperandsUnsupported, symbol),
            OperatorKind::Ordering(symbol) => (RuntimeFunction::OrderingUnsupported, symbol),
        };
        let symbol = self.string(symbol, span);
        let left = self.runtime_type(operands.left, &operands.call.left.ty, span);
        let right = self.runtime_type(operands.right, &operands.call.right.ty, span);
        self.call_runtime(function, vec![symbol, left.value, right.value], span);
        self.builder.terminate(Terminator::Unreachable);
        let after = self.builder.create_block();
        self.builder.switch_to(after);
        Self::borrowed(self.zero_value(operands.ty, span), operands.ty)
    }

    /// Returns whether two instances are of one class.
    fn same_class(&mut self, left: Lowered, right: Lowered, span: Span) -> typy_tyir::ValueId {
        let class = |this: &mut Self, value: Lowered| {
            let bits = this.bits(Self::borrowed(value.value, value.ty), span);
            // A class is immortal, so its type object is borrowed, as `type()` reads it.
            let class = this.call_runtime(RuntimeFunction::ObjectType, vec![bits], span);
            this.borrowed_result(class).value
        };
        let (left, right) = (class(self, left), class(self, right));
        self.builder
            .push_value(Operation::Identical { left, right }, TyirType::Bool, span)
    }

    /// Returns what `then` gives when `condition` holds and what `otherwise` gives when not, a
    /// value of `ty`.
    pub(crate) fn either(
        &mut self,
        condition: typy_tyir::ValueId,
        ty: TyirType,
        span: Span,
        then: impl FnOnce(&mut Self) -> Lowered,
        otherwise: impl FnOnce(&mut Self) -> Lowered,
    ) -> Lowered {
        let join = self.builder.create_block();
        let gives = ty != TyirType::Unit;
        let result = gives.then(|| self.builder.add_block_parameter(join, ty));
        let (first, second) = (self.builder.create_block(), self.builder.create_block());
        self.builder.terminate(Terminator::Branch {
            condition,
            then: first,
            otherwise: second,
        });
        self.builder.switch_to(first);
        let outcome = then(self);
        self.join_with(outcome, join, gives, span);
        self.builder.switch_to(second);
        let outcome = otherwise(self);
        self.join_with(outcome, join, gives, span);
        self.joined(join, result, ty, span)
    }

    /// Returns whether the right operand's reflected method runs first: its class derives
    /// from the left operand's and is another class, and for an arithmetic operator, when
    /// `overridden`, has another reflected method there.
    fn reflected_first(
        &mut self,
        call: &OperatorCall,
        (left, right): (Lowered, Lowered),
        overridden: bool,
        span: Span,
    ) -> typy_tyir::ValueId {
        let left_bits = self.bits(Self::borrowed(left.value, left.ty), span);
        let right_bits = self.bits(Self::borrowed(right.value, right.ty), span);
        // A class is immortal, so its type object is borrowed, as `type()` reads it.
        let left_class = self.call_runtime(RuntimeFunction::ObjectType, vec![left_bits], span);
        let left_class = self.borrowed_result(left_class).value;
        let right_class = self.call_runtime(RuntimeFunction::ObjectType, vec![right_bits], span);
        let right_class = self.borrowed_result(right_class).value;
        let derives = self
            .call_runtime(
                RuntimeFunction::IsInstance,
                vec![right_bits, left_class],
                span,
            )
            .value;
        let same_class = self.builder.push_value(
            Operation::Identical {
                left: left_class,
                right: right_class,
            },
            TyirType::Bool,
            span,
        );
        let other_class =
            self.builder
                .push_value(Operation::BoolNot(same_class), TyirType::Bool, span);
        let subclass = self.builder.push_value(
            Operation::BoolBinary {
                operator: BoolOperator::And,
                left: derives,
                right: other_class,
            },
            TyirType::Bool,
            span,
        );
        // With the left class's reflected method in a table, a subclass that inherits it is
        // no reason to swap; the entries are read only once the right class derives from the
        // left's, whose table is a prefix of its own.
        let Some((class, slot)) = call
            .left_reflected
            .filter(|_| overridden)
            .and_then(|method| self.methods.dispatch(method))
        else {
            return subclass;
        };
        self.another_method(subclass, class, slot, left, right, span)
    }

    /// Returns whether the right object has another method than the left one at `slot` of
    /// `class`'s table, read only when `subclass` shows its class derives from the left
    /// object's, whose table is a prefix of its own; false otherwise.
    fn another_method(
        &mut self,
        subclass: typy_tyir::ValueId,
        class: u32,
        slot: u32,
        left: Lowered,
        right: Lowered,
        span: Span,
    ) -> typy_tyir::ValueId {
        let join = self.builder.create_block();
        let swap = self.builder.add_block_parameter(join, TyirType::Bool);
        let check = self.builder.create_block();
        let no = self.builder.create_block();
        self.builder.terminate(Terminator::Branch {
            condition: subclass,
            then: check,
            otherwise: no,
        });
        self.builder.switch_to(no);
        let false_value = self.constant(typy_tyir::Constant::Bool(false), span);
        self.builder.terminate(Terminator::Jump {
            target: join,
            arguments: vec![false_value],
        });
        self.builder.switch_to(check);
        let of_class = TyirType::Object(TypeId::class(class));
        let as_class = |this: &mut Self, value: Lowered| {
            this.builder.push_value(
                Operation::Upcast {
                    value: value.value,
                    ty: of_class,
                },
                of_class,
                span,
            )
        };
        let left_object = if left.ty == of_class {
            left.value
        } else {
            as_class(self, left)
        };
        // The runtime showed the right operand is an instance of the left's class, so of
        // `class`, whichever side of it its static class is.
        let right_object = match right.ty {
            ty if ty == of_class => right.value,
            TyirType::Object(id)
                if id
                    .class_index()
                    .is_some_and(|index| self.class_derives(index, class)) =>
            {
                as_class(self, right)
            }
            _ => self.builder.push_value(
                Operation::Downcast {
                    value: right.value,
                    ty: of_class,
                },
                of_class,
                span,
            ),
        };
        let same = self.builder.push_value(
            Operation::SameMethod {
                class,
                slot,
                left: left_object,
                right: right_object,
            },
            TyirType::Bool,
            span,
        );
        let differs = self
            .builder
            .push_value(Operation::BoolNot(same), TyirType::Bool, span);
        self.builder.terminate(Terminator::Jump {
            target: join,
            arguments: vec![differs],
        });
        self.builder.switch_to(join);
        swap
    }

    /// Returns true when class `index` is class `base` or derives from it.
    fn class_derives(&self, index: u32, base: u32) -> bool {
        let mut current = Some(typy_check::ClassId(index));
        while let Some(class) = current {
            if class.0 == base {
                return true;
            }
            current = self.program.class(class).base;
        }
        false
    }
}
