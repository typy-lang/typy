//! Lowers comparisons that call comparison methods, choosing the reflected one when the right
//! operand's class derives from the left's and defines it otherwise, as CPython does
//! (CLS-07, ADR-0026).

use typy_check::OperatorCall;
use typy_diagnostics::Span;
use typy_tyir::{BoolOperator, Operation, RuntimeFunction, Terminator, TyirType, TypeId};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

impl FunctionLowerer<'_> {
    /// Returns `left op right` through its comparison method.
    pub(crate) fn operator_call(
        &mut self,
        call: &OperatorCall,
        ty: TyirType,
        span: Span,
    ) -> Lowered {
        let left = self.expression(&call.left);
        let left = self.kept_for_call(&call.left, left, span);
        let right = self.expression(&call.right);
        let right = self.kept_for_call(&call.right, right, span);
        let borrowed = |value: Lowered| Self::borrowed(value.value, value.ty);
        let forward_call = |this: &mut Self, method| {
            this.call_method(
                method,
                vec![(0, borrowed(left)), (1, borrowed(right))],
                ty,
                span,
            )
        };
        let reflected_call = |this: &mut Self, method| {
            this.call_method(
                method,
                vec![(0, borrowed(right)), (1, borrowed(left))],
                ty,
                span,
            )
        };
        // A reflected method no class overrides is one function for every class, so when the
        // left object's class has the same one, CPython never calls it first.
        let swaps = call.reflected != call.left_reflected
            || call
                .left_reflected
                .is_some_and(|method| self.methods.dispatch(method).is_some());
        let result = match (call.forward, call.reflected) {
            (Some(forward), Some(_)) if !swaps => forward_call(self, forward),
            (Some(forward), Some(reflected)) => {
                let swap = self.reflected_first(call, left, right, span);
                let join = self.builder.create_block();
                let value = self.builder.add_block_parameter(join, ty);
                let first = self.builder.create_block();
                let second = self.builder.create_block();
                self.builder.terminate(Terminator::Branch {
                    condition: swap,
                    then: first,
                    otherwise: second,
                });
                for (block, reflect) in [(first, true), (second, false)] {
                    self.builder.switch_to(block);
                    let outcome = if reflect {
                        reflected_call(self, reflected)
                    } else {
                        forward_call(self, forward)
                    };
                    // The result leaves its branch with its reference, as `optional_branch`
                    // passes one on.
                    let given = self.take_reference(outcome, span);
                    self.builder.terminate(Terminator::Jump {
                        target: join,
                        arguments: vec![given],
                    });
                }
                self.builder.switch_to(join);
                self.produced(value, ty)
            }
            (Some(forward), None) => forward_call(self, forward),
            (None, Some(reflected)) => reflected_call(self, reflected),
            (None, None) => unreachable!("the checker gives a comparison call a method"),
        };
        let result = if call.negated {
            let negated =
                self.builder
                    .push_value(Operation::BoolNot(result.value), TyirType::Bool, span);
            Self::borrowed(negated, TyirType::Bool)
        } else {
            result
        };
        self.release(right, span);
        self.release(left, span);
        result
    }

    /// Returns whether the right operand's reflected method runs first: its class derives
    /// from the left operand's, is another class, and has another reflected method there.
    fn reflected_first(
        &mut self,
        call: &OperatorCall,
        left: Lowered,
        right: Lowered,
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
