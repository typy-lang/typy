//! Lowers expressions: constants, locals, `and`, `or`, conditional expressions, truth tests and calls.

use typy_check::{FunctionId, SignatureParameter, TypedConstant, TypedExpr, TypedExprKind};
use typy_diagnostics::Span;
use typy_source::syntax::BoolOperator;
use typy_tyir::{Callee, Constant, Operation, ScalarKind, Terminator, TyirType, ValueId};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

impl FunctionLowerer<'_> {
    /// Lowers an expression; the caller releases or moves the result.
    #[allow(clippy::too_many_lines, reason = "one arm per expression kind")]
    pub(crate) fn expression(&mut self, expression: &TypedExpr) -> Lowered {
        let span = expression.span;
        let ty = self.lowered_type(&expression.ty);
        match &expression.kind {
            TypedExprKind::Constant(constant) => self.typed_constant(constant, span),
            TypedExprKind::Local(symbol) => {
                let slot = self.slots[symbol];
                self.load(slot, span)
            }
            TypedExprKind::Tuple(items) => self.tuple_display(items, ty, span),
            TypedExprKind::List(items) => {
                let item = match &expression.ty {
                    typy_check::Type::List(item) => self.lowered_type(item),
                    _ => unreachable!("a list display has a list type"),
                };
                self.list_display(items, item, span)
            }
            TypedExprKind::ListOf(source) => self.list_of(source, span),
            TypedExprKind::ListItem { list, index } => {
                let list = self.expression(list);
                let index = self.expression(index);
                self.list_item(list, index, ty, span)
            }
            TypedExprKind::Slice {
                value,
                lower,
                upper,
                step,
            } => self.slice(
                value,
                [lower.as_deref(), upper.as_deref(), step.as_deref()],
                ty,
                span,
            ),
            TypedExprKind::Method {
                receiver,
                method,
                arguments,
            } => self.method(receiver, *method, arguments, ty, span),
            TypedExprKind::TupleSlice { tuple, positions } => {
                let tuple = self.expression(tuple);
                self.tuple_slice(tuple, positions, ty, span)
            }
            TypedExprKind::TupleItem { tuple, index } => {
                let tuple = self.expression(tuple);
                self.tuple_item(tuple, *index, ty, span)
            }
            TypedExprKind::ModuleName => {
                let value = self.string("__main__", span);
                Self::borrowed(value, TyirType::STR)
            }
            TypedExprKind::Binary {
                left,
                operator,
                right,
            } => {
                let left = self.expression(left);
                let right = self.expression(right);
                self.binary(*operator, left, right, &expression.ty, span)
            }
            TypedExprKind::Unary { operator, operand } => {
                let operand = self.expression(operand);
                self.unary(*operator, operand, span)
            }
            TypedExprKind::BoolOp { operator, values } => {
                self.bool_operation(*operator, values, ty, span)
            }
            TypedExprKind::Compare {
                left,
                operators,
                comparators,
            } => self.comparison_chain(left, operators, comparators, span),
            TypedExprKind::IfExp { test, body, orelse } => {
                self.conditional(test, body, orelse, ty, span)
            }
            TypedExprKind::Call {
                function,
                arguments,
            } => {
                let values: Vec<(usize, Lowered)> = arguments
                    .iter()
                    .map(|argument| (argument.parameter, self.expression(&argument.value)))
                    .collect();
                self.call_function(*function, values, ty, span)
            }
            TypedExprKind::Print { values, sep, end } => {
                self.print(values, sep.as_deref(), end.as_deref(), span)
            }
            TypedExprKind::IterableBuiltin {
                builtin,
                iterable,
                option,
            } => self.iterable_builtin(*builtin, iterable, option.as_deref(), ty, span),
            TypedExprKind::Builtin { builtin, arguments } => {
                let types: Vec<typy_check::Type> = arguments
                    .iter()
                    .map(|argument| argument.ty.clone())
                    .collect();
                let values: Vec<Lowered> = arguments
                    .iter()
                    .map(|argument| self.expression(argument))
                    .collect();
                self.builtin(*builtin, values, &types, ty, span)
            }
            TypedExprKind::Index { value, index } => self.str_index(value, index, span),
        }
    }

    /// Lowers a literal; constants are immortal, so they are borrowed.
    pub(crate) fn typed_constant(&mut self, constant: &TypedConstant, span: Span) -> Lowered {
        let (value, ty) = match constant {
            TypedConstant::None => (self.constant(Constant::Unit, span), TyirType::Unit),
            TypedConstant::Bool(value) => {
                (self.constant(Constant::Bool(*value), span), TyirType::Bool)
            }
            TypedConstant::Int(digits) => (
                self.constant(Constant::Int(digits.clone()), span),
                TyirType::Int,
            ),
            TypedConstant::Float(value) => (
                self.constant(Constant::Float(*value), span),
                TyirType::Float,
            ),
            TypedConstant::Str(text) => (self.string(text, span), TyirType::STR),
        };
        Self::borrowed(value, ty)
    }

    /// Returns the truth value of a lowered value (EXP-10); the value is not released.
    pub(crate) fn truth(&mut self, lowered: Lowered, span: Span) -> ValueId {
        let kind = match lowered.ty {
            TyirType::Bool => return lowered.value,
            TyirType::Unit => return self.constant(Constant::Bool(false), span),
            TyirType::Int => ScalarKind::Int,
            TyirType::Float => ScalarKind::Float,
            TyirType::Object(typy_tyir::TypeId::LIST) => return self.list_truth(lowered, span),
            TyirType::Object(_) => ScalarKind::Str,
            TyirType::Bits => unreachable!("list items are converted before they are tested"),
            // A tuple's length is fixed by its type, so its truth is too.
            TyirType::Tuple(_) => {
                let filled = self.tuple_length(lowered.ty) > 0;
                return self.constant(Constant::Bool(filled), span);
            }
        };
        self.builder.push_value(
            Operation::IsTrue {
                kind,
                value: lowered.value,
            },
            TyirType::Bool,
            span,
        )
    }

    /// `a or b` and `a and b` return one of their operands, evaluating each only while the result is undecided (EXP-09).
    fn bool_operation(
        &mut self,
        operator: BoolOperator,
        values: &[TypedExpr],
        ty: TyirType,
        span: Span,
    ) -> Lowered {
        let join = self.builder.create_block();
        let result = self.builder.add_block_parameter(join, ty);
        for (position, value) in values.iter().enumerate() {
            let lowered = self.expression(value);
            if position + 1 == values.len() {
                let owned = self.take_reference(lowered, span);
                self.builder.terminate(Terminator::Jump {
                    target: join,
                    arguments: vec![owned],
                });
                break;
            }
            let truth = self.truth(lowered, span);
            let owned = self.take_reference(lowered, span);
            let decided = self.builder.create_block();
            let next = self.builder.create_block();
            let (then, otherwise) = match operator {
                BoolOperator::Or => (decided, next),
                BoolOperator::And => (next, decided),
            };
            self.builder.terminate(Terminator::Branch {
                condition: truth,
                then,
                otherwise,
            });
            self.builder.switch_to(decided);
            self.builder.terminate(Terminator::Jump {
                target: join,
                arguments: vec![owned],
            });
            self.builder.switch_to(next);
            if self.counted(ty) {
                self.builder.push(Operation::Decrement(owned), span);
            }
        }
        self.builder.switch_to(join);
        self.produced(result, ty)
    }

    fn conditional(
        &mut self,
        test: &TypedExpr,
        body: &TypedExpr,
        orelse: &TypedExpr,
        ty: TyirType,
        span: Span,
    ) -> Lowered {
        let tested = self.expression(test);
        let condition = self.truth(tested, span);
        self.release(tested, span);
        let then = self.builder.create_block();
        let otherwise = self.builder.create_block();
        let join = self.builder.create_block();
        let result = self.builder.add_block_parameter(join, ty);
        self.builder.terminate(Terminator::Branch {
            condition,
            then,
            otherwise,
        });
        for (block, branch) in [(then, body), (otherwise, orelse)] {
            self.builder.switch_to(block);
            let lowered = self.expression(branch);
            let owned = self.take_reference(lowered, span);
            self.builder.terminate(Terminator::Jump {
                target: join,
                arguments: vec![owned],
            });
        }
        self.builder.switch_to(join);
        self.produced(result, ty)
    }

    /// Calls a program function: arguments were evaluated in source order; missing ones take their constant defaults (EXP-16).
    fn call_function(
        &mut self,
        function: FunctionId,
        arguments: Vec<(usize, Lowered)>,
        ty: TyirType,
        span: Span,
    ) -> Lowered {
        let parameters: Vec<SignatureParameter> =
            self.program.function(function).signature.parameters.clone();
        let mut ordered: Vec<ValueId> = Vec::with_capacity(parameters.len());
        for (index, parameter) in parameters.iter().enumerate() {
            let value = if let Some((_, lowered)) =
                arguments.iter().find(|(position, _)| *position == index)
            {
                lowered.value
            } else {
                let default = parameter.default.clone().unwrap_or(TypedConstant::None);
                self.typed_constant(&default, span).value
            };
            ordered.push(value);
        }
        let callee = typy_tyir::FunctionId(function.0);
        let value = if self
            .may_raise
            .get(function.0 as usize)
            .copied()
            .unwrap_or(false)
        {
            self.invoke(Callee::Function(callee), ordered, ty, span)
        } else if ty == TyirType::Unit {
            self.builder.push(
                Operation::Call {
                    function: callee,
                    arguments: ordered,
                },
                span,
            );
            self.constant(Constant::Unit, span)
        } else {
            self.builder.push_value(
                Operation::Call {
                    function: callee,
                    arguments: ordered,
                },
                ty,
                span,
            )
        };
        let result = self.produced(value, ty);
        for (_, lowered) in arguments.into_iter().rev() {
            self.release(lowered, span);
        }
        result
    }
}
