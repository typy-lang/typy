//! Lowers function values and the calls through them (FUN-05, ADR-0060).

use typy_check::{ClassId, FunctionId, MemberTest, SymbolId, TypeObjectOf, TypedExpr};
use typy_diagnostics::Span;
use typy_tyir::{Callee, Constant, IntOperator, Operation, RuntimeFunction, TyirType};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

impl FunctionLowerer<'_> {
    /// Returns the function object of `function`, which captures nothing: one immortal
    /// object, so it is borrowed and using it allocates nothing.
    pub(crate) fn function_value(&mut self, function: FunctionId, span: Span) -> Lowered {
        let value = self.constant(Constant::Function(typy_tyir::FunctionId(function.0)), span);
        Self::borrowed(value, TyirType::FUNCTION)
    }

    /// Lowers a nested `def`: a new function object of `function`, stored in the variable
    /// `target` (FUN-04, ADR-0061).
    pub(crate) fn nested_def(
        &mut self,
        function: FunctionId,
        target: SymbolId,
        captures: &[TypedExpr],
        span: Span,
    ) {
        let closure = self.new_closure(function, captures, span);
        self.assign_name(target, closure, span);
    }

    /// Returns a new function object of the nested function or `lambda` `function`, which
    /// takes over a reference to the value of each capture, in order (FUN-04, EXP-14).
    pub(crate) fn new_closure(
        &mut self,
        function: FunctionId,
        captures: &[TypedExpr],
        span: Span,
    ) -> Lowered {
        let variables = self
            .program
            .closures
            .get(&function)
            .cloned()
            .unwrap_or_default();
        let tuple = crate::function::captures_layout(&variables, self.tuple_types);
        let closure = self.builder.push_value(
            Operation::NewClosure {
                function: typy_tyir::FunctionId(function.0),
                captures: tuple,
            },
            TyirType::FUNCTION,
            span,
        );
        let closure = self.produced(closure, TyirType::FUNCTION);
        for (index, capture) in captures.iter().enumerate() {
            let value = self.expression(capture);
            let value = self.take_reference(value, span);
            self.builder.push(
                Operation::SetCapture {
                    closure: closure.value,
                    captures: tuple,
                    index: u32::try_from(index).unwrap_or(u32::MAX),
                    value,
                },
                span,
            );
        }
        closure
    }

    /// Returns the capture at `index` of the running nested function, of type `ty`, borrowed
    /// from its function object (FUN-04).
    pub(crate) fn capture(&mut self, index: u32, ty: TyirType, span: Span) -> Lowered {
        let (closure, captures) = self
            .closure
            .unwrap_or_else(|| unreachable!("only a nested function reads a capture"));
        let value = self.builder.push_value(
            Operation::Capture {
                closure,
                captures,
                index,
            },
            ty,
            span,
        );
        Self::borrowed(value, ty)
    }

    /// Returns the capture at `index` of the function object `closure` of the nested function
    /// `function`, with a reference of its own, since the object may be released first.
    pub(crate) fn capture_of(
        &mut self,
        function: FunctionId,
        closure: &TypedExpr,
        index: u32,
        span: Span,
    ) -> Lowered {
        let variables = self
            .program
            .closures
            .get(&function)
            .cloned()
            .unwrap_or_default();
        let captures = crate::function::captures_layout(&variables, self.tuple_types);
        let ty = self
            .tuple_types
            .items(captures)
            .and_then(|items| items.get(index as usize).copied())
            .unwrap_or(TyirType::Unit);
        let object = self.expression(closure);
        let value = self.builder.push_value(
            Operation::Capture {
                closure: object.value,
                captures,
                index,
            },
            ty,
            span,
        );
        let owned = self.take_reference(Self::borrowed(value, ty), span);
        self.release(object, span);
        self.produced(owned, ty)
    }

    /// Returns the running nested function's function object, borrowed from its caller.
    pub(crate) fn running_function(&self) -> Lowered {
        let (closure, _) = self
            .closure
            .unwrap_or_else(|| unreachable!("only a nested function reads its own object"));
        Self::borrowed(closure, TyirType::FUNCTION)
    }

    /// Returns a new bound method of the instance `object` gives, of the method of its
    /// class among `methods`: the first, of its static class, unless the object is an
    /// instance of a class of a later entry, each after the classes it derives from, so the
    /// last test that holds names the object's own class's method.
    pub(crate) fn bound_method(
        &mut self,
        object: &TypedExpr,
        methods: &[(ClassId, FunctionId)],
        binds_class: bool,
        span: Span,
    ) -> Lowered {
        let instance = self.expression(object);
        let descriptor = |this: &mut Self, method: FunctionId| {
            this.constant(
                Constant::MethodDescriptor(typy_tyir::FunctionId(method.0)),
                span,
            )
        };
        let mut chosen = descriptor(self, methods[0].1);
        for (class, method) in &methods[1..] {
            let name = self.program.class(*class).name.clone();
            let test = MemberTest::Class(TypeObjectOf::Instance(*class, name));
            let found = self.member_found(instance, &test, span);
            let own = descriptor(self, *method);
            chosen = self.select(found, own, chosen, TyirType::Bits);
        }
        // A class method binds the object's class, which it holds as CPython's `cls` (FUN-09).
        let bound = if binds_class {
            self.runtime_type(instance, &object.ty, span)
        } else {
            Self::borrowed(instance.value, instance.ty)
        };
        let bits = self.bits(Self::borrowed(bound.value, bound.ty), span);
        // A method of a built-in type read through a value is written as CPython writes a
        // built-in method (FUN-08).
        let new = if matches!(
            self.program.builtin_values.get(&methods[0].1),
            Some(typy_check::BuiltinValue::Method)
        ) {
            RuntimeFunction::BuiltinMethodNew
        } else {
            RuntimeFunction::MethodNew
        };
        let made = self.call_runtime(new, vec![chosen, bits], span);
        self.release(instance, span);
        made
    }

    /// Returns the function among `functions` of the class of the instance `object` gives,
    /// chosen by the same tests as `bound_method`, after evaluating the object: the static
    /// method of the instance's own class (FUN-09).
    pub(crate) fn chosen_function(
        &mut self,
        object: &TypedExpr,
        functions: &[(ClassId, FunctionId)],
        span: Span,
    ) -> Lowered {
        let instance = self.expression(object);
        let constant = |this: &mut Self, function: FunctionId| {
            this.constant(Constant::Function(typy_tyir::FunctionId(function.0)), span)
        };
        let mut chosen = constant(self, functions[0].1);
        for (class, function) in &functions[1..] {
            let name = self.program.class(*class).name.clone();
            let test = MemberTest::Class(TypeObjectOf::Instance(*class, name));
            let found = self.member_found(instance, &test, span);
            let own = constant(self, *function);
            chosen = self.select(found, own, chosen, TyirType::FUNCTION);
        }
        self.release(instance, span);
        Self::borrowed(chosen, TyirType::FUNCTION)
    }

    /// Returns the value of a static method read through `object` when a class in `binds`
    /// defines it again as a method: for such a class, a bound method of its method and the
    /// object; for any other, the static method of its class, chosen by the same tests as
    /// `bound_method` (FUN-09).
    pub(crate) fn chosen_value(
        &mut self,
        object: &TypedExpr,
        functions: &[(ClassId, FunctionId)],
        binds: &[ClassId],
        span: Span,
    ) -> Lowered {
        let instance = self.expression(object);
        let choice = |this: &mut Self, (class, function): (ClassId, FunctionId)| {
            let id = typy_tyir::FunctionId(function.0);
            let bound = binds.contains(&class);
            let constant = if bound {
                Constant::MethodDescriptor(id)
            } else {
                Constant::Function(id)
            };
            let value = this.constant(constant, span);
            // A descriptor is an address already; a function object is read as one.
            let bits = if bound {
                value
            } else {
                this.bits(Self::borrowed(value, TyirType::FUNCTION), span)
            };
            let binds = this.constant(Constant::Bool(bound), span);
            (bits, binds)
        };
        let (mut chosen, mut bound) = choice(self, functions[0]);
        for (class, function) in &functions[1..] {
            let name = self.program.class(*class).name.clone();
            let test = MemberTest::Class(TypeObjectOf::Instance(*class, name));
            let found = self.member_found(instance, &test, span);
            let (own, own_bound) = choice(self, (*class, *function));
            chosen = self.select(found, own, chosen, TyirType::Bits);
            bound = self.select(found, own_bound, bound, TyirType::Bool);
        }
        let bits = self.bits(Self::borrowed(instance.value, instance.ty), span);
        let made = self.call_runtime(
            RuntimeFunction::MethodOrFunction,
            vec![chosen, bits, bound],
            span,
        );
        self.release(instance, span);
        made
    }

    /// Calls the code of the function object `callee` gives with `arguments`, and returns
    /// the result as an owned value. A `Callable` type does not say whether its function
    /// raises, so the call always takes the raising form. The function object and each
    /// argument are held for the call as those of a call by name are, so the call cannot
    /// free the object it runs, nor an argument it reads, by assigning where it was read.
    pub(crate) fn call_value(
        &mut self,
        callee: &TypedExpr,
        arguments: &[TypedExpr],
        ty: TyirType,
        span: Span,
    ) -> Lowered {
        let function = self.expression(callee);
        let function = self.kept_for_call(callee, function, span);
        let lowered: Vec<Lowered> = arguments
            .iter()
            .map(|argument| {
                let value = self.expression(argument);
                self.kept_for_call(argument, value, span)
            })
            .collect();
        let values = std::iter::once(function.value)
            .chain(lowered.iter().map(|argument| argument.value))
            .collect();
        let value = self.invoke(Callee::Value, values, ty, span);
        let result = self.produced(value, ty);
        for argument in lowered.into_iter().rev() {
            self.release(argument, span);
        }
        self.release(function, span);
        result
    }

    /// Lowers a call through the function value `callee` with `*` (EXP-22): the values
    /// before the first `*`, then the list `rest` of the others, whose length the runtime
    /// checks against the `Callable`'s parameters left, raising `TypeError` otherwise; then
    /// the call takes the list's items as its last arguments.
    pub(crate) fn call_value_unpacked(
        &mut self,
        callee: &TypedExpr,
        (fixed, rest): (&[TypedExpr], &TypedExpr),
        ty: TyirType,
        span: Span,
    ) -> Lowered {
        let parameters: Vec<typy_check::Type> = callee
            .ty
            .callable_parts()
            .map(|(parameters, _)| parameters.to_vec())
            .unwrap_or_default();
        let function = self.expression(callee);
        let function = self.kept_for_call(callee, function, span);
        let lowered: Vec<Lowered> = fixed
            .iter()
            .map(|argument| {
                let value = self.expression(argument);
                self.kept_for_call(argument, value, span)
            })
            .collect();
        let list = self.expression(rest);
        let length = self.call_runtime(RuntimeFunction::ListLength, vec![list.value], span);
        let before = self.constant(Constant::Int(fixed.len().to_string()), span);
        let given = self.builder.push_value(
            Operation::IntBinary {
                operator: IntOperator::Add,
                left: before,
                right: length.value,
            },
            TyirType::Int,
            span,
        );
        self.release(length, span);
        let expected = self.constant(Constant::Int(parameters.len().to_string()), span);
        let object = self.bits(Self::borrowed(function.value, function.ty), span);
        self.call_runtime(
            RuntimeFunction::ValueArity,
            vec![object, expected, given],
            span,
        );
        let items: Vec<Lowered> = parameters[fixed.len().min(parameters.len())..]
            .iter()
            .enumerate()
            .map(|(offset, parameter)| {
                let wanted = self.lowered_type(parameter);
                let index = self.constant(Constant::Int(offset.to_string()), span);
                let bits =
                    self.call_runtime(RuntimeFunction::ListGet, vec![list.value, index], span);
                let value = self.value_of_bits(bits.value, wanted, span);
                Self::borrowed(value, wanted)
            })
            .collect();
        let values = std::iter::once(function.value)
            .chain(lowered.iter().map(|argument| argument.value))
            .chain(items.iter().map(|item| item.value))
            .collect();
        let value = self.invoke(Callee::Value, values, ty, span);
        let result = self.produced(value, ty);
        self.release(list, span);
        for argument in lowered.into_iter().rev() {
            self.release(argument, span);
        }
        self.release(function, span);
        result
    }
}
