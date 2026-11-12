//! Lowers expressions: constants, locals, `and`, `or`, conditional expressions, truth tests and calls.

use typy_check::{FunctionId, SignatureParameter, TypedConstant, TypedExpr, TypedExprKind};
use typy_diagnostics::Span;
use typy_source::syntax::BoolOperator;
use typy_tyir::{
    Callee, Constant, Operation, RuntimeFunction, ScalarKind, Terminator, TyirType, ValueId,
};

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
            TypedExprKind::Local(symbol) | TypedExprKind::ModuleVariable(symbol) => {
                self.read_name(*symbol, span)
            }
            TypedExprKind::Tuple(items) => self.tuple_display(items, ty, span),
            TypedExprKind::MathCall {
                function,
                arguments,
            } => self.math_call(*function, arguments, &expression.ty, span),
            TypedExprKind::PercentFormat { values, parts } => {
                self.percent_format(values, parts, span)
            }
            TypedExprKind::Lazy { reify, value } => {
                self.run_lazy_import(*reify, span);
                self.expression(value)
            }
            TypedExprKind::NewException {
                class,
                arguments,
                init,
            } => self.new_exception(class, arguments, init.as_deref(), span),
            TypedExprKind::SetArguments {
                exception,
                arguments,
            } => self.set_arguments(exception, arguments, span),
            TypedExprKind::Iter(source) => self.iter_of(source, span),
            TypedExprKind::Next {
                iterator,
                method,
                default,
            } => self.next_of(iterator, *method, default.as_deref(), ty, span),
            TypedExprKind::List(items) => {
                let item = match &expression.ty {
                    typy_check::Type::List(item) => self.lowered_type(item),
                    _ => unreachable!("a list display has a list type"),
                };
                self.list_display(items, item, span)
            }
            TypedExprKind::ListOf(source) => self.list_of(source, span),
            TypedExprKind::Set(items) => {
                let item = self.set_item_type(&expression.ty);
                self.set_display(items, item, span)
            }
            TypedExprKind::Wrap(inner) => self.wrap(inner, ty, span),
            TypedExprKind::Unwrap(inner) => self.unwrap(inner, ty, span),
            TypedExprKind::Dict { keys, values } => {
                self.dict_display(keys, values, &expression.ty, span)
            }
            TypedExprKind::DictOf(source) => self.dict_of(source, &expression.ty, span),
            TypedExprKind::DictWith { source, entries } => {
                self.dict_with(source.as_deref(), entries, &expression.ty, span)
            }
            TypedExprKind::DictFromKeys { source, value } => {
                self.dict_from_keys(source, value.as_deref(), &expression.ty, span)
            }
            TypedExprKind::Comprehension {
                element,
                value,
                generators,
            } => self.comprehension(element, value.as_deref(), generators, &expression.ty, span),
            TypedExprKind::DictItem { dict, key } => {
                let dict = self.expression(dict);
                let key = self.expression(key);
                self.dict_item(dict, key, ty, span)
            }
            TypedExprKind::SetOf(source) => {
                let item = self.set_item_type(&expression.ty);
                self.set_of(source, item, span)
            }
            TypedExprKind::TupleOf(inner) => {
                let item = match &expression.ty {
                    typy_check::Type::TupleOf(item) => self.lowered_type(item),
                    _ => unreachable!("a tuple of a sequence has a `tuple[T, ...]` type"),
                };
                self.tuple_of(inner, item, span)
            }
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
                let module = self.module.clone();
                let value = self.string(&module, span);
                Self::borrowed(value, TyirType::STR)
            }
            TypedExprKind::ModuleAttribute { module, symbol } => {
                self.module_attribute(module, *symbol, span)
            }
            TypedExprKind::Submodule(module) => self.submodule(module, span),
            // An iterable is the same object, so only its type changes.
            TypedExprKind::AsIterable(inner) => {
                let lowered = self.expression(inner);
                let bits = self.bits(lowered, span);
                let value = self.builder.push_value(
                    Operation::FromBits {
                        bits,
                        ty: TyirType::ITERABLE,
                    },
                    TyirType::ITERABLE,
                    span,
                );
                self.transfer(lowered, value, TyirType::ITERABLE)
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
                    .map(|argument| {
                        let lowered = self.expression(&argument.value);
                        (
                            argument.parameter,
                            self.kept_for_call(&argument.value, lowered, span),
                        )
                    })
                    .collect();
                self.call_function(*function, values, ty, span)
            }
            TypedExprKind::New {
                class,
                init,
                arguments,
            } => self.new_instance(*class, *init, arguments, span),
            TypedExprKind::Attribute { object, index } => self.attribute(object, *index, span),
            TypedExprKind::TypeOf(value) => self.type_of(value, span),
            TypedExprKind::FString(parts) => self.fstring(parts, span),
            TypedExprKind::TypeObject(of) => self.type_object(of, span),
            TypedExprKind::TypeName(object) => {
                let object = self.expression(object);
                let name = self.call_runtime(RuntimeFunction::TypeName, vec![object.value], span);
                self.release(object, span);
                name
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

    /// Returns the TyIR type of the items of a set type.
    fn set_item_type(&mut self, ty: &typy_check::Type) -> TyirType {
        match ty {
            typy_check::Type::Set(item) => self.lowered_type(item),
            _ => unreachable!("a set expression has a set type"),
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
            TypedConstant::Bytes(data) => (self.bytes_constant(data, span), TyirType::BYTES),
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
            TyirType::Object(
                typy_tyir::TypeId::BYTES
                | typy_tyir::TypeId::SET
                | typy_tyir::TypeId::DICT
                | typy_tyir::TypeId::DICT_VIEW,
            ) => {
                let function = match lowered.ty {
                    TyirType::SET => RuntimeFunction::SetLength,
                    TyirType::DICT => RuntimeFunction::DictLength,
                    TyirType::VIEW => RuntimeFunction::DictViewLength,
                    _ => RuntimeFunction::BytesLength,
                };
                let length = self.call_runtime(function, vec![lowered.value], span).value;
                return self.builder.push_value(
                    Operation::IsTrue {
                        kind: ScalarKind::Int,
                        value: length,
                    },
                    TyirType::Bool,
                    span,
                );
            }
            // An instance of a class without `__bool__` or `__len__` is true (CLS-07), and so
            // is a type object.
            TyirType::Object(id) if id.class_index().is_some() || id == typy_tyir::TypeId::TYPE => {
                return self.constant(Constant::Bool(true), span);
            }
            // A range is true when it holds its own start; its length may pass `sys.maxsize`,
            // where CPython's truth test still answers.
            TyirType::RANGE => {
                let which = self.constant(Constant::Int("0".to_owned()), span);
                let start = self.call_runtime(
                    RuntimeFunction::RangeBound,
                    vec![lowered.value, which],
                    span,
                );
                let inside = self.call_runtime(
                    RuntimeFunction::RangeContains,
                    vec![lowered.value, start.value],
                    span,
                );
                self.release(start, span);
                return inside.value;
            }
            TyirType::Object(_) => ScalarKind::Str,
            TyirType::Bits => unreachable!("list items are converted before they are tested"),
            TyirType::Optional(_) => return self.optional_truth(lowered, span),
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
    pub(crate) fn call_function(
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
                let constant = self.typed_constant(&default, span).value;
                match self.lowered_type(&parameter.ty) {
                    // A default of an optional parameter is `None` or a value of its base.
                    optional @ TyirType::Optional(_) if default == TypedConstant::None => {
                        self.none_of(optional, span)
                    }
                    optional @ TyirType::Optional(_) => self.builder.push_value(
                        Operation::Wrap {
                            value: constant,
                            ty: optional,
                        },
                        optional,
                        span,
                    ),
                    _ => constant,
                }
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
