//! Operands, results and terminators have the types their operations take.

use super::VerifyError;
use crate::block::BlockId;
use crate::function::TyirFunction;
use crate::module::TyirModule;
use crate::operation::{Operation, ScalarKind};
use crate::terminator::{Callee, Terminator};
use crate::types::{TupleTypeId, TyirType};
use crate::value::ValueId;

struct Checker<'module> {
    module: &'module TyirModule,
    function: &'module TyirFunction,
    block: BlockId,
}

pub(super) fn check(module: &TyirModule, function: &TyirFunction) -> Result<(), VerifyError> {
    for block in &function.blocks {
        let checker = Checker {
            module,
            function,
            block: block.id,
        };
        for instruction in &block.instructions {
            let result = checker.operation(&instruction.operation)?;
            match (result, instruction.result) {
                (Some(expected), Some(value)) => checker.expect(value, expected, "the result")?,
                (None, None) => {}
                (Some(_), None) => {
                    return Err(checker.error("an instruction's result is dropped".to_owned()));
                }
                (None, Some(_)) => {
                    return Err(
                        checker.error("an instruction without a result defines a value".to_owned())
                    );
                }
            }
        }
        checker.terminator(&block.terminator)?;
    }
    Ok(())
}

fn scalar(kind: ScalarKind) -> TyirType {
    match kind {
        ScalarKind::Int => TyirType::Int,
        ScalarKind::Float => TyirType::Float,
        ScalarKind::Bool => TyirType::Bool,
        ScalarKind::Str => TyirType::STR,
    }
}

impl Checker<'_> {
    fn error(&self, message: String) -> VerifyError {
        VerifyError {
            function: self.function.symbol.clone(),
            block: Some(self.block),
            message,
        }
    }

    fn ty(&self, value: ValueId) -> Result<TyirType, VerifyError> {
        self.function
            .values
            .get(value.0 as usize)
            .map(|info| info.ty)
            .ok_or_else(|| self.error(format!("v{} has no type", value.0)))
    }

    fn expect(&self, value: ValueId, expected: TyirType, what: &str) -> Result<(), VerifyError> {
        let actual = self.ty(value)?;
        if actual == expected {
            Ok(())
        } else {
            Err(self.error(format!("{what} v{} is {actual}, not {expected}", value.0)))
        }
    }

    fn arguments(
        &self,
        arguments: &[ValueId],
        parameters: &[TyirType],
        callee: &str,
    ) -> Result<(), VerifyError> {
        if arguments.len() != parameters.len() {
            return Err(self.error(format!(
                "{callee} takes {} arguments, not {}",
                parameters.len(),
                arguments.len()
            )));
        }
        for (argument, parameter) in arguments.iter().zip(parameters) {
            self.expect(*argument, *parameter, &format!("an argument of {callee}"))?;
        }
        Ok(())
    }

    fn callee_signature(
        &self,
        callee: Callee,
    ) -> Result<(Vec<TyirType>, TyirType, bool, String), VerifyError> {
        match callee {
            Callee::Runtime(function) => {
                let signature = function.signature();
                Ok((
                    signature.parameters.to_vec(),
                    signature.result,
                    signature.may_raise,
                    format!("rt.{}", function.name()),
                ))
            }
            Callee::Function(id) => {
                let function = self
                    .module
                    .functions
                    .get(id.0 as usize)
                    .ok_or_else(|| self.error(format!("fn{} does not exist", id.0)))?;
                let parameters = function
                    .parameters
                    .iter()
                    .map(|value| function.value(*value).ty)
                    .collect();
                Ok((
                    parameters,
                    function.return_type,
                    function.may_raise,
                    function.symbol.clone(),
                ))
            }
        }
    }

    /// Returns the result type of an operation after checking its operands.
    fn operation(&self, operation: &Operation) -> Result<Option<TyirType>, VerifyError> {
        Ok(match operation {
            Operation::Constant(constant) => Some(constant.ty()),
            Operation::Load(slot) => Some(
                self.function
                    .slots
                    .get(slot.0 as usize)
                    .ok_or_else(|| self.error(format!("slot s{} does not exist", slot.0)))?
                    .ty,
            ),
            Operation::Store { slot, value } => {
                let slot = self
                    .function
                    .slots
                    .get(slot.0 as usize)
                    .ok_or_else(|| self.error(format!("slot s{} does not exist", slot.0)))?;
                self.expect(*value, slot.ty, "the stored value")?;
                None
            }
            Operation::IntBinary { left, right, .. } => {
                self.expect(*left, TyirType::Int, "an int operand")?;
                self.expect(*right, TyirType::Int, "an int operand")?;
                Some(TyirType::Int)
            }
            Operation::IntUnary { operand, .. } => {
                self.expect(*operand, TyirType::Int, "an int operand")?;
                Some(TyirType::Int)
            }
            Operation::FloatBinary { left, right, .. } => {
                self.expect(*left, TyirType::Float, "a float operand")?;
                self.expect(*right, TyirType::Float, "a float operand")?;
                Some(TyirType::Float)
            }
            Operation::FloatNegate(operand) => {
                self.expect(*operand, TyirType::Float, "a float operand")?;
                Some(TyirType::Float)
            }
            Operation::BoolBinary { left, right, .. } => {
                self.expect(*left, TyirType::Bool, "a bool operand")?;
                self.expect(*right, TyirType::Bool, "a bool operand")?;
                Some(TyirType::Bool)
            }
            Operation::BoolNot(operand) => {
                self.expect(*operand, TyirType::Bool, "a bool operand")?;
                Some(TyirType::Bool)
            }
            Operation::Compare {
                kind, left, right, ..
            } => {
                self.expect(*left, scalar(*kind), "a compared value")?;
                self.expect(*right, scalar(*kind), "a compared value")?;
                Some(TyirType::Bool)
            }
            Operation::IsTrue { kind, value } => {
                self.expect(*value, scalar(*kind), "a tested value")?;
                Some(TyirType::Bool)
            }
            Operation::CallRuntime {
                function,
                arguments,
            } => self.call(Callee::Runtime(*function), arguments)?,
            Operation::Call {
                function,
                arguments,
            } => self.call(Callee::Function(*function), arguments)?,
            Operation::MakeTuple { ty, items } => Some(self.make_tuple(*ty, items)?),
            Operation::TupleItem { tuple, index } => Some(self.tuple_item(*tuple, *index)?),
            Operation::Increment(value) | Operation::Decrement(value) => {
                if !self.module.tuple_types.is_counted(self.ty(*value)?) {
                    return Err(self.error(format!("v{} is not counted", value.0)));
                }
                None
            }
        })
    }

    /// A tuple's items have the types of its tuple type, one for each.
    fn make_tuple(&self, ty: TupleTypeId, items: &[ValueId]) -> Result<TyirType, VerifyError> {
        let Some(expected) = self.module.tuple_types.items(ty) else {
            return Err(self.error(format!("tuple#{} is not a type of the module", ty.0)));
        };
        if expected.len() != items.len() {
            return Err(self.error(format!(
                "tuple#{} has {} items, and {} are given",
                ty.0,
                expected.len(),
                items.len()
            )));
        }
        for (item, item_type) in items.iter().zip(expected) {
            self.expect(*item, *item_type, "a tuple item")?;
        }
        Ok(TyirType::Tuple(ty))
    }

    /// An item is read from a tuple that has it.
    fn tuple_item(&self, tuple: ValueId, index: u32) -> Result<TyirType, VerifyError> {
        let TyirType::Tuple(id) = self.ty(tuple)? else {
            return Err(self.error(format!("v{} is not a tuple", tuple.0)));
        };
        self.module
            .tuple_types
            .items(id)
            .and_then(|items| items.get(index as usize))
            .copied()
            .ok_or_else(|| self.error(format!("tuple#{} has no item {index}", id.0)))
    }

    fn call(&self, callee: Callee, arguments: &[ValueId]) -> Result<Option<TyirType>, VerifyError> {
        let (parameters, result, may_raise, name) = self.callee_signature(callee)?;
        if may_raise {
            return Err(self.error(format!("{name} may raise, so it is called with invoke")));
        }
        self.arguments(arguments, &parameters, &name)?;
        Ok((result != TyirType::Unit).then_some(result))
    }

    fn block_parameters(&self, block: BlockId) -> Result<Vec<TyirType>, VerifyError> {
        let target = self
            .function
            .blocks
            .get(block.0 as usize)
            .ok_or_else(|| self.error(format!("b{} does not exist", block.0)))?;
        target
            .parameters
            .iter()
            .map(|value| self.ty(*value))
            .collect()
    }

    fn terminator(&self, terminator: &Terminator) -> Result<(), VerifyError> {
        match terminator {
            Terminator::Jump { target, arguments } => {
                let parameters = self.block_parameters(*target)?;
                self.arguments(arguments, &parameters, &format!("b{}", target.0))
            }
            Terminator::Branch {
                condition,
                then,
                otherwise,
            } => {
                self.expect(*condition, TyirType::Bool, "the condition")?;
                for block in [then, otherwise] {
                    if !self.block_parameters(*block)?.is_empty() {
                        return Err(
                            self.error(format!("b{} is a branch target with parameters", block.0))
                        );
                    }
                }
                Ok(())
            }
            Terminator::Invoke {
                callee,
                arguments,
                normal,
                error,
                ..
            } => {
                let (parameters, result, may_raise, name) = self.callee_signature(*callee)?;
                if !may_raise {
                    return Err(self.error(format!(
                        "{name} cannot raise, so it is called without invoke"
                    )));
                }
                self.arguments(arguments, &parameters, &name)?;
                if self.block_parameters(*normal)? != [result] {
                    return Err(self.error(format!(
                        "b{} must take the {result} result of {name}",
                        normal.0
                    )));
                }
                if self.block_parameters(*error)? != [TyirType::EXCEPTION] {
                    return Err(
                        self.error(format!("b{} must take the exception of {name}", error.0))
                    );
                }
                Ok(())
            }
            Terminator::Return(value) => {
                self.expect(*value, self.function.return_type, "the returned value")
            }
            Terminator::ReturnError(value) => {
                if !self.function.may_raise {
                    return Err(
                        self.error("a function that cannot raise returns an exception".to_owned())
                    );
                }
                self.expect(*value, TyirType::EXCEPTION, "the returned exception")
            }
            Terminator::Unreachable => Ok(()),
        }
    }
}
