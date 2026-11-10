//! Lowers what a program does with its classes: making an instance, reading and assigning an
//! attribute, comparing instances by identity, and writing an instance as text (CLS-01,
//! CLS-08, ADR-0012).

use typy_check::{Argument, ClassId, FunctionId, TypedExpr};
use typy_diagnostics::Span;
use typy_tyir::{Operation, RuntimeFunction, TyirType, TypeId, ValueId};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

impl FunctionLowerer<'_> {
    /// Lowers `Class(arguments)`: the arguments are evaluated, then the instance is made, then
    /// `__init__` runs on it, as CPython does.
    pub(crate) fn new_instance(
        &mut self,
        class: ClassId,
        init: Option<FunctionId>,
        arguments: &[Argument],
        span: Span,
    ) -> Lowered {
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
        let ty = TyirType::Object(TypeId::class(class.0));
        let value = self
            .builder
            .push_value(Operation::NewInstance { class: class.0 }, ty, span);
        let instance = self.produced(value, ty);
        if let Some(init) = init {
            let mut values = values;
            values.insert(0, (0, Self::borrowed(instance.value, ty)));
            let done = self.call_function(init, values, TyirType::Unit, span);
            self.release(done, span);
        } else {
            for (_, lowered) in values.into_iter().rev() {
                self.release(lowered, span);
            }
        }
        instance
    }

    /// Reads the attribute at `index` of an instance, borrowed from it; an attribute of an
    /// instance this expression made keeps a reference of its own.
    pub(crate) fn attribute(&mut self, object: &TypedExpr, index: usize, span: Span) -> Lowered {
        let object = self.expression(object);
        let ty = self.field_type(object.ty, index);
        let value = self.builder.push_value(
            Operation::Field {
                object: object.value,
                index: field_index(index),
            },
            ty,
            span,
        );
        if !object.owned {
            return Self::borrowed(value, ty);
        }
        let kept = if self.counted(ty) {
            self.builder.push(Operation::Increment(value), span);
            self.produced(value, ty)
        } else {
            Self::borrowed(value, ty)
        };
        self.release(object, span);
        kept
    }

    /// Stores a value in the attribute at `index` of an instance; the value is evaluated
    /// first, then the instance, as CPython does. The attribute takes over a reference and
    /// releases the value it held, which is zero, and so nothing, before `__init__` assigns it.
    pub(crate) fn assign_attribute(
        &mut self,
        object: &TypedExpr,
        index: usize,
        value: Lowered,
        span: Span,
    ) {
        let object = self.expression(object);
        self.store_field(object, index, value, span);
        self.release(object, span);
    }

    /// Stores a value in the attribute at `index` of an instance that is already evaluated.
    fn store_field(&mut self, object: Lowered, index: usize, value: Lowered, span: Span) {
        let ty = self.field_type(object.ty, index);
        let stored = self.take_reference(value, span);
        let field = field_index(index);
        let old = self.counted(ty).then(|| {
            self.builder.push_value(
                Operation::Field {
                    object: object.value,
                    index: field,
                },
                ty,
                span,
            )
        });
        self.builder.push(
            Operation::SetField {
                object: object.value,
                index: field,
                value: stored,
            },
            span,
        );
        if let Some(old) = old {
            self.builder.push(Operation::Decrement(old), span);
        }
    }

    /// Lowers `object.attribute op= value`: the instance is evaluated once, and the attribute
    /// is read, with a reference of its own, before the value is evaluated, as CPython does.
    pub(crate) fn augmented_attribute(
        &mut self,
        object: &TypedExpr,
        index: usize,
        combine: impl FnOnce(&mut Self, Lowered) -> Lowered,
        span: Span,
    ) {
        let object = self.expression(object);
        let ty = self.field_type(object.ty, index);
        let value = self.builder.push_value(
            Operation::Field {
                object: object.value,
                index: field_index(index),
            },
            ty,
            span,
        );
        let current = if self.counted(ty) {
            self.builder.push(Operation::Increment(value), span);
            self.produced(value, ty)
        } else {
            Self::borrowed(value, ty)
        };
        let result = combine(self, current);
        self.store_field(object, index, result, span);
        self.release(object, span);
    }

    /// Returns whether two objects, or optional objects, are one object.
    pub(crate) fn identical(&mut self, left: Lowered, right: Lowered, span: Span) -> ValueId {
        self.builder.push_value(
            Operation::Identical {
                left: left.value,
                right: right.value,
            },
            TyirType::Bool,
            span,
        )
    }

    /// Returns `left == right`, or `left != right` when `equal` is false, for instances of a
    /// class without `__eq__`, which are equal only to themselves (CLS-07).
    pub(crate) fn instance_equality(
        &mut self,
        equal: bool,
        left: Lowered,
        right: Lowered,
        span: Span,
    ) -> ValueId {
        let same = self.identical(left, right, span);
        if equal {
            return same;
        }
        self.builder
            .push_value(Operation::BoolNot(same), TyirType::Bool, span)
    }

    /// Returns `str(instance)`, or `repr(instance)` when `repr` is set: the class's `__str__`,
    /// then its `__repr__`, then CPython's default text (CLS-08). The instance is not
    /// released.
    pub(crate) fn instance_text(&mut self, lowered: Lowered, repr: bool, span: Span) -> Lowered {
        let TyirType::Object(id) = lowered.ty else {
            unreachable!("an instance is an object");
        };
        let class = self.program.class(ClassId(id.class_index().unwrap_or(0)));
        let method = |name: &str| {
            class
                .methods
                .iter()
                .find(|(each, _)| each == name)
                .map(|(_, id)| *id)
        };
        let method = if repr {
            method("__repr__")
        } else {
            method("__str__").or_else(|| method("__repr__"))
        };
        if let Some(method) = method {
            let instance = Self::borrowed(lowered.value, lowered.ty);
            return self.call_function(method, vec![(0, instance)], TyirType::STR, span);
        }
        let bits = self
            .builder
            .push_value(Operation::ToBits(lowered.value), TyirType::Bits, span);
        self.call_runtime(RuntimeFunction::InstanceText, vec![bits], span)
    }

    /// Returns the type of the attribute at `index` of an instance of type `ty`.
    fn field_type(&mut self, ty: TyirType, index: usize) -> TyirType {
        let TyirType::Object(id) = ty else {
            unreachable!("only an instance has attributes");
        };
        let program = self.program;
        let class = program.class(ClassId(id.class_index().unwrap_or(0)));
        self.lowered_type(&class.attributes[index].ty)
    }
}

/// Returns an attribute's position as TyIR numbers it.
fn field_index(index: usize) -> u32 {
    u32::try_from(index).unwrap_or(u32::MAX)
}
