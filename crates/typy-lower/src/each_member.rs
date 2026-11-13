//! Lowers an operation checked for each member of a union: the runtime tests the object
//! against each member's class in turn, and the first that holds runs (TYP-14, ADR-0022).

use typy_check::EachMember;
use typy_diagnostics::Span;
use typy_tyir::{Constant, RuntimeFunction, Terminator, TyirType};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

impl FunctionLowerer<'_> {
    /// Returns the operation for the member the variable's object is; the members are
    /// distinct classes, so one test holds, and the last member needs none.
    pub(crate) fn each_member(&mut self, each: &EachMember, ty: TyirType, span: Span) -> Lowered {
        let value = self.expression(&each.value);
        let join = self.builder.create_block();
        let gives = ty != TyirType::Unit;
        let result = gives.then(|| self.builder.add_block_parameter(join, ty));
        let last = each.arms.len() - 1;
        for (position, (class, body)) in each.arms.iter().enumerate() {
            if position < last {
                let object = self.bits(Self::borrowed(value.value, value.ty), span);
                let class = self.type_object(class, span).value;
                let found = self
                    .call_runtime(RuntimeFunction::IsInstance, vec![object, class], span)
                    .value;
                let this = self.builder.create_block();
                let next = self.builder.create_block();
                self.builder.terminate(Terminator::Branch {
                    condition: found,
                    then: this,
                    otherwise: next,
                });
                self.builder.switch_to(this);
                self.member_arm(body, join, gives, span);
                self.builder.switch_to(next);
            } else {
                self.member_arm(body, join, gives, span);
            }
        }
        self.builder.switch_to(join);
        self.release(value, span);
        if let Some(result) = result {
            return self.produced(result, ty);
        }
        let unit = self.constant(Constant::Unit, span);
        Self::borrowed(unit, TyirType::Unit)
    }

    /// Lowers one member's operation and passes its result, with its reference, to `join`.
    fn member_arm(
        &mut self,
        body: &typy_check::TypedExpr,
        join: typy_tyir::BlockId,
        gives: bool,
        span: Span,
    ) {
        let outcome = self.expression(body);
        let arguments = if gives {
            vec![self.take_reference(outcome, span)]
        } else {
            self.release(outcome, span);
            Vec::new()
        };
        self.builder.terminate(Terminator::Jump {
            target: join,
            arguments,
        });
    }
}
