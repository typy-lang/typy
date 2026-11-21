//! Lowers an operation checked for each member of a union: the runtime tests the object
//! against each member's class in turn, and the first that holds runs (TYP-14, ADR-0027).

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
        // A tagged union's tag names the member, in the order of the arms (TYP-14).
        if self.is_tagged(value.ty) {
            let result = self.tag_switch(value, ty, span, |this, position, _| {
                this.expression(&each.arms[position].1)
            });
            self.release(value, span);
            return result;
        }
        let join = self.builder.create_block();
        let gives = ty != TyirType::Unit;
        let result = gives.then(|| self.builder.add_block_parameter(join, ty));
        let last = each.arms.len() - 1;
        for (position, (class, body)) in each.arms.iter().enumerate() {
            if position < last {
                let found = self.member_found(value, class, span);
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
        let result = self.joined(join, result, ty, span);
        self.release(value, span);
        result
    }

    /// Returns whether the object a union of objects holds passes a member's test: an instance
    /// of its class, or of its kind of the runtime's own objects (TYP-14).
    pub(crate) fn member_found(
        &mut self,
        value: Lowered,
        test: &typy_check::MemberTest,
        span: Span,
    ) -> typy_tyir::ValueId {
        let object = self.bits(Self::borrowed(value.value, value.ty), span);
        match test {
            typy_check::MemberTest::Class(class) => {
                let class = self.type_object(class, span).value;
                self.call_runtime(RuntimeFunction::IsInstance, vec![object, class], span)
                    .value
            }
            // `type(x) is C`: the object's type object is the class's (TYP-23).
            typy_check::MemberTest::Exact(class) => {
                let own = self.call_runtime(RuntimeFunction::ObjectType, vec![object], span);
                let own = self.borrowed_result(own);
                let class = self.type_object(class, span);
                self.identical(own, class, span)
            }
            typy_check::MemberTest::Kind(kind) => {
                let id = match kind {
                    typy_check::RuntimeKind::Iterator => typy_tyir::TypeId::ITERATOR,
                    typy_check::RuntimeKind::Generator => typy_tyir::TypeId::GENERATOR,
                    typy_check::RuntimeKind::Traceback => typy_tyir::TypeId::TRACEBACK,
                    typy_check::RuntimeKind::Module => typy_tyir::TypeId::MODULE,
                    typy_check::RuntimeKind::File => typy_tyir::TypeId::TEXT_FILE,
                    typy_check::RuntimeKind::TypeAlias => typy_tyir::TypeId::TYPE_ALIAS,
                    typy_check::RuntimeKind::VersionInfo => typy_tyir::TypeId::VERSION_INFO,
                    typy_check::RuntimeKind::NotImplemented => typy_tyir::TypeId::NOT_IMPLEMENTED,
                };
                let id = self.constant(Constant::Int(id.0.to_string()), span);
                self.call_runtime(RuntimeFunction::ObjectHasTypeId, vec![object, id], span)
                    .value
            }
        }
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
        self.join_with(outcome, join, gives, span);
    }
}
