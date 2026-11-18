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
            TypedExprKind::IterContains {
                value,
                held,
                source,
                each,
                test,
                negated,
            } => self.iterated_contains((value, *held), source, (*each, test), *negated, span),
            TypedExprKind::Set(items) => {
                let item = self.set_item_type(&expression.ty);
                self.set_display(items, item, span)
            }
            TypedExprKind::Wrap(inner) => self.wrap(inner, ty, span),
            TypedExprKind::Upcast(inner) => self.converted(inner, &expression.ty, ty, true, span),
            TypedExprKind::Downcast(inner) => {
                self.converted(inner, &expression.ty, ty, false, span)
            }
            TypedExprKind::IsInstance(test) => self.isinstance(test, span),
            TypedExprKind::EachMember(each) => self.each_member(each, ty, span),
            TypedExprKind::CheckedLength(call) => {
                let length = self.expression(call);
                let checked =
                    self.call_runtime(RuntimeFunction::IntLength, vec![length.value], span);
                self.release(length, span);
                checked
            }
            TypedExprKind::OperatorCall(call) => self.operator_call(call, &expression.ty, ty, span),
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
            TypedExprKind::Raise(exception) => self.raise_in_place(exception, ty, span),
            TypedExprKind::ClassVariable(holders) => self.class_variable(holders, ty, span),
            TypedExprKind::SliceBound { slice, part } => {
                let slice = self.expression(slice);
                let part = self.constant(Constant::Int(part.to_string()), span);
                let bound =
                    self.call_runtime(RuntimeFunction::SliceBound, vec![slice.value, part], span);
                self.release(slice, span);
                bound
            }
            TypedExprKind::NotImplemented => {
                let found = self.call_runtime(RuntimeFunction::NotImplemented, Vec::new(), span);
                self.borrowed_result(found)
            }
            TypedExprKind::Sequence { first, then } => {
                let first = self.expression(first);
                self.release(first, span);
                self.expression(then)
            }
            TypedExprKind::Hash(value) => {
                let value = self.expression(value);
                let (kind, descriptor) = self.item_layout(value.ty, span);
                let bits = self.bits(Self::borrowed(value.value, value.ty), span);
                let hash = self.call_runtime(
                    RuntimeFunction::HashValue,
                    vec![kind, descriptor, bits],
                    span,
                );
                self.release(value, span);
                hash
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
                dispatch,
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
                if *dispatch {
                    self.call_method(*function, values, ty, span)
                } else {
                    self.call_function(*function, values, ty, span)
                }
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
            TypedExprKind::FunctionValue(function) => self.function_value(*function, span),
            TypedExprKind::Capture(index) => self.capture(*index, ty, span),
            TypedExprKind::RunningFunction => self.running_function(),
            TypedExprKind::CallNested {
                function,
                closure,
                arguments,
            } => {
                let closure_value = self.expression(closure);
                let closure_value = self.kept_for_call(closure, closure_value, span);
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
                let result = self.call_nested(*function, closure_value.value, values, ty, span);
                self.release(closure_value, span);
                result
            }
            TypedExprKind::BoundMethod { object, methods } => {
                self.bound_method(object, methods, span)
            }
            TypedExprKind::Keyed {
                operation,
                list,
                key,
                option,
                option_first,
            } => self.keyed(
                *operation,
                list,
                key,
                (option.as_deref(), *option_first),
                span,
            ),
            TypedExprKind::CallValue { callee, arguments } => {
                self.call_value(callee, arguments, ty, span)
            }
            // A type alias object is immortal, so it is borrowed (SMT-12).
            TypedExprKind::TypeAliasObject(index) => {
                let value = self.constant(Constant::TypeAlias(*index), span);
                Self::borrowed(value, TyirType::TYPE_ALIAS)
            }
            // The value is evaluated for what it does, then read as its constant items (LIB-16).
            TypedExprKind::VersionRead { value, result } => {
                let value = self.expression(value);
                self.release(value, span);
                self.expression(result)
            }
            TypedExprKind::TypeAliasPart { alias, module } => {
                let alias = self.expression(alias);
                let function = if *module {
                    RuntimeFunction::TypeAliasModule
                } else {
                    RuntimeFunction::TypeAliasName
                };
                let part = self.call_runtime(function, vec![alias.value], span);
                self.release(alias, span);
                part
            }
            TypedExprKind::TypeName(object) => {
                let object = self.expression(object);
                let name = self.call_runtime(RuntimeFunction::TypeName, vec![object.value], span);
                self.release(object, span);
                name
            }
            TypedExprKind::Print(call) => self.print(call, span),
            TypedExprKind::SysValue(value) => self.sys_value(*value, span),
            // A library is linked when the program is built and holds no value (FFI-01).
            TypedExprKind::ForeignLibrary(_) => Self::borrowed(
                self.constant(typy_tyir::Constant::Unit, span),
                TyirType::Unit,
            ),
            TypedExprKind::ForeignCall {
                function,
                arguments,
            } => self.foreign_call(*function, arguments, span),
            TypedExprKind::CData(cdata) => self.cdata_expression(cdata, &expression.ty, span),
            TypedExprKind::Open {
                path,
                mode,
                access,
                options,
            } => self.open(path, mode, *access, options, &expression.ty, span),
            TypedExprKind::SysExit(code) => self.sys_exit(code.as_deref(), span),
            TypedExprKind::RecursionLimit(limit) => self.recursion_limit(limit.as_deref(), span),
            TypedExprKind::PercentDynamic { format, values } => {
                self.percent_dynamic(format, values, span)
            }
            TypedExprKind::Named { target, value } => {
                // The variable takes a reference of its own; the expression gives the value.
                let lowered = self.expression(value);
                self.assign_name(*target, Self::borrowed(lowered.value, lowered.ty), span);
                lowered
            }
            TypedExprKind::FileAttribute { file, attribute } => {
                self.file_attribute(file, *attribute, span)
            }
            TypedExprKind::ExceptionPart {
                exception,
                part,
                operation,
            } => self.exception_part(exception, *part, operation, span),
            TypedExprKind::ExceptionArguments { exception, raw } => {
                self.exception_arguments(exception, *raw, &expression.ty, ty, span)
            }
            TypedExprKind::Input(prompt) => self.input(prompt.as_deref(), span),
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
            TypedConstant::StrPoints(points) => {
                let index = self.strings.intern_points(points);
                (
                    self.constant(Constant::StrPoints(index), span),
                    TyirType::STR,
                )
            }
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
                let length = self.call_runtime(function, vec![lowered.value], span);
                let truth = self.builder.push_value(
                    Operation::IsTrue {
                        kind: ScalarKind::Int,
                        value: length.value,
                    },
                    TyirType::Bool,
                    span,
                );
                // The length is a new `int`, released once tested, as `list_truth` does.
                self.release(length, span);
                return truth;
            }
            // An instance's class decides its truth with `__bool__`, or with `__len__` when it
            // has no `__bool__`; without either, the instance is true (CLS-07), and so is a type
            // object.
            TyirType::Object(id) if id.class_index().is_some() => {
                return self.instance_truth(lowered, id, span);
            }
            TyirType::Object(
                typy_tyir::TypeId::TYPE | typy_tyir::TypeId::SLICE | typy_tyir::TypeId::FUNCTION,
            ) => {
                return self.constant(Constant::Bool(true), span);
            }
            // The object's type tests it, as CPython's truth test does (TYP-14); a `ctypes`
            // object's class decides (FFI-03).
            TyirType::UNION | TyirType::CDATA => {
                let object = self.bits(Self::borrowed(lowered.value, lowered.ty), span);
                return self
                    .call_runtime(RuntimeFunction::ObjectIsTrue, vec![object], span)
                    .value;
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
            // A tagged union is as true as the member it holds (TYP-14).
            TyirType::Tuple(_) if self.is_tagged(lowered.ty) => {
                return self
                    .tag_switch(lowered, TyirType::Bool, span, |this, _, slot| {
                        let truth = this.truth(slot, span);
                        Self::borrowed(truth, TyirType::Bool)
                    })
                    .value;
            }
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

    /// Returns the truth of an instance: its class's `__bool__`, or its `__len__` checked and
    /// compared with zero, or true (CLS-07); the instance is not released.
    fn instance_truth(&mut self, lowered: Lowered, id: typy_tyir::TypeId, span: Span) -> ValueId {
        let class = typy_check::ClassId(id.class_index().unwrap_or(0));
        let instance = Self::borrowed(lowered.value, lowered.ty);
        if let Some(method) = self.program.method(class, "__bool__") {
            return self
                .call_method(method, vec![(0, instance)], TyirType::Bool, span)
                .value;
        }
        // A class derived from this one may decide its truth otherwise: `__bool__` comes before
        // `__len__`, so the object's own class, through its type object, decides (CLS-24).
        let method = self.program.method(class, "__len__");
        if self.subclass_defines(class, "__bool__")
            || (method.is_none() && self.subclass_defines(class, "__len__"))
        {
            let object = self.bits(instance, span);
            return self
                .call_runtime(RuntimeFunction::ObjectIsTrue, vec![object], span)
                .value;
        }
        let Some(method) = method else {
            return self.constant(Constant::Bool(true), span);
        };
        let length = self.call_method(method, vec![(0, instance)], TyirType::Int, span);
        let checked = self.call_runtime(RuntimeFunction::IntLength, vec![length.value], span);
        self.release(length, span);
        let truth = self.builder.push_value(
            Operation::IsTrue {
                kind: ScalarKind::Int,
                value: checked.value,
            },
            TyirType::Bool,
            span,
        );
        // A large `int` is an object, so the checked length is released once tested.
        self.release(checked, span);
        truth
    }

    /// Returns true when a class derived from `class`, other than it, defines `name`.
    fn subclass_defines(&self, class: typy_check::ClassId, name: &str) -> bool {
        self.program.classes.iter().any(|other| {
            other.id != class
                && other.methods.iter().any(|(each, _)| each == name)
                && std::iter::successors(other.base, |base| self.program.class(*base).base)
                    .any(|base| base == class)
        })
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

    /// Calls a program function directly: arguments were evaluated in source order; missing ones take their constant defaults (EXP-16).
    pub(crate) fn call_function(
        &mut self,
        function: FunctionId,
        arguments: Vec<(usize, Lowered)>,
        ty: TyirType,
        span: Span,
    ) -> Lowered {
        self.call(function, None, (None, arguments), ty, span)
    }

    /// Calls the nested function `function` directly, with its function object `closure`
    /// first (FUN-04).
    pub(crate) fn call_nested(
        &mut self,
        function: FunctionId,
        closure: ValueId,
        arguments: Vec<(usize, Lowered)>,
        ty: TyirType,
        span: Span,
    ) -> Lowered {
        self.call(function, None, (Some(closure), arguments), ty, span)
    }

    /// Returns the value of the default `default` of `parameter`, held as the parameter is.
    fn default_value(
        &mut self,
        parameter: &SignatureParameter,
        default: Option<&TypedConstant>,
        span: Span,
    ) -> ValueId {
        let default = default.cloned().unwrap_or(TypedConstant::None);
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
    }

    /// Returns the default of parameter `index` of a call through the method table of the
    /// instance `instance`: that of the method of the instance's class, the nearest override
    /// of `function` whose class the instance is of, or `function`'s own.
    fn dispatched_default(
        &mut self,
        function: FunctionId,
        index: usize,
        instance: Lowered,
        span: Span,
    ) -> ValueId {
        let parameter = self.program.function(function).signature.parameters[index].clone();
        let overrides: Vec<(typy_check::ClassId, Option<TypedConstant>)> = self
            .methods
            .overrides(function)
            .into_iter()
            .map(|(class, method)| {
                let parameters = &self.program.function(method).signature.parameters;
                (
                    class,
                    parameters.get(index).and_then(|each| each.default.clone()),
                )
            })
            .collect();
        if overrides
            .iter()
            .all(|(_, default)| *default == parameter.default)
        {
            return self.default_value(&parameter, parameter.default.as_ref(), span);
        }
        let ty = self.lowered_type(&parameter.ty);
        self.default_of_class((&parameter, &overrides), instance, ty, span)
            .value
    }

    /// Returns the default of the first of `overrides` whose class the instance is of, or the
    /// parameter's own.
    fn default_of_class(
        &mut self,
        (parameter, overrides): (
            &SignatureParameter,
            &[(typy_check::ClassId, Option<TypedConstant>)],
        ),
        instance: Lowered,
        ty: TyirType,
        span: Span,
    ) -> Lowered {
        let Some(((class, default), rest)) = overrides.split_first() else {
            let value = self.default_value(parameter, parameter.default.as_ref(), span);
            return Self::borrowed(value, ty);
        };
        let object = self.bits(instance, span);
        let name = self.program.class(*class).name.clone();
        let of = self.type_object(&typy_check::TypeObjectOf::Instance(*class, name), span);
        let found = self
            .call_runtime(RuntimeFunction::IsInstance, vec![object, of.value], span)
            .value;
        self.either(
            found,
            ty,
            span,
            |this| {
                let value = this.default_value(parameter, default.as_ref(), span);
                Self::borrowed(value, ty)
            },
            |this| this.default_of_class((parameter, rest), instance, ty, span),
        )
    }

    /// Calls a method on the instance that is its first argument: through the method table
    /// when a class derived from the method's class defines it again, so the instance's
    /// class decides which runs, and directly otherwise (ADR-0021).
    pub(crate) fn call_method(
        &mut self,
        function: FunctionId,
        arguments: Vec<(usize, Lowered)>,
        ty: TyirType,
        span: Span,
    ) -> Lowered {
        let dispatch = self.methods.dispatch(function);
        self.call(function, dispatch, (None, arguments), ty, span)
    }

    /// Calls `function`, or the entry `dispatch` names of the table of the class of the first
    /// argument, which takes the arguments `function` takes.
    fn call(
        &mut self,
        function: FunctionId,
        dispatch: Option<(u32, u32)>,
        (closure, arguments): (Option<ValueId>, Vec<(usize, Lowered)>),
        ty: TyirType,
        span: Span,
    ) -> Lowered {
        let parameters: Vec<SignatureParameter> =
            self.program.function(function).signature.parameters.clone();
        let mut ordered: Vec<ValueId> = Vec::with_capacity(parameters.len() + 1);
        for (index, parameter) in parameters.iter().enumerate() {
            let value = if let Some((_, lowered)) =
                arguments.iter().find(|(position, _)| *position == index)
            {
                // A call the lowering makes itself, such as of an inherited `__init__` or
                // `__repr__`, may pass an instance of a subclass (ADR-0021).
                let wanted = self.lowered_type(&parameter.ty);
                if lowered.ty == wanted {
                    lowered.value
                } else {
                    self.builder.push_value(
                        Operation::Upcast {
                            value: lowered.value,
                            ty: wanted,
                        },
                        wanted,
                        span,
                    )
                }
            } else if dispatch.is_some() && index > 0 {
                // A method a class derived from this one defines again may give the parameter
                // another default, which the object's class decides (CLS-04).
                let held = self.lowered_type(&parameters[0].ty);
                self.dispatched_default(function, index, Self::borrowed(ordered[0], held), span)
            } else {
                self.default_value(parameter, parameter.default.as_ref(), span)
            };
            ordered.push(value);
        }
        if let Some(closure) = closure {
            ordered.insert(0, closure);
        }
        let function_id = typy_tyir::FunctionId(function.0);
        let (callee, operation) = match dispatch {
            Some((class, slot)) => (
                Callee::Method { class, slot },
                Operation::CallMethod {
                    class,
                    slot,
                    arguments: ordered.clone(),
                },
            ),
            None => (
                Callee::Function(function_id),
                Operation::Call {
                    function: function_id,
                    arguments: ordered.clone(),
                },
            ),
        };
        let value = if self
            .may_raise
            .get(function.0 as usize)
            .copied()
            .unwrap_or(false)
        {
            self.invoke(callee, ordered, ty, span)
        } else if ty == TyirType::Unit {
            self.builder.push(operation, span);
            self.constant(Constant::Unit, span)
        } else {
            self.builder.push_value(operation, ty, span)
        };
        let result = self.produced(value, ty);
        for (_, lowered) in arguments.into_iter().rev() {
            self.release(lowered, span);
        }
        result
    }
}
