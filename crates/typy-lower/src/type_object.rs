//! Lowers type objects as values: `type(x)`, a type's name read as a value, and `__name__`
//! (CLS-12). Each type object is immortal, so a type object is never counted.

use typy_check::{Type, TypeObjectOf, TypedExpr, View};
use typy_diagnostics::Span;
use typy_tyir::{Constant, RuntimeFunction, RuntimeType, TyirType, TypeObjectRef};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

/// Returns the type object a `TypeObjectOf` stands for.
fn reference(of: &TypeObjectOf) -> TypeObjectRef {
    TypeObjectRef::Runtime(match of {
        TypeObjectOf::Int => RuntimeType::Int,
        TypeObjectOf::Float => RuntimeType::Float,
        TypeObjectOf::Bool => RuntimeType::Bool,
        TypeObjectOf::Str => RuntimeType::Str,
        TypeObjectOf::Bytes => RuntimeType::Bytes,
        TypeObjectOf::NoneType => RuntimeType::NoneType,
        TypeObjectOf::List => RuntimeType::List,
        TypeObjectOf::Tuple => RuntimeType::Tuple,
        TypeObjectOf::Set => RuntimeType::Set,
        TypeObjectOf::Dict => RuntimeType::Dict,
        TypeObjectOf::View(View::Keys) => RuntimeType::DictKeys,
        TypeObjectOf::View(View::Values) => RuntimeType::DictValues,
        TypeObjectOf::View(View::Items) => RuntimeType::DictItems,
        TypeObjectOf::Object => RuntimeType::Object,
        TypeObjectOf::Type => RuntimeType::Type,
        TypeObjectOf::Range => RuntimeType::Range,
        TypeObjectOf::Instance(class, _) => return TypeObjectRef::Class(class.0),
        TypeObjectOf::Exception(class) => return crate::exception::exception_class(class),
    })
}

impl FunctionLowerer<'_> {
    /// Returns the type object `of` stands for.
    pub(crate) fn type_object(&mut self, of: &TypeObjectOf, span: Span) -> Lowered {
        let value = self.constant(Constant::TypeObject(reference(of)), span);
        Self::borrowed(value, TyirType::TYPE)
    }

    /// Lowers `type(value)`: the value is evaluated and released, and the result is its type
    /// object, which for an optional value depends on whether it is `None`.
    pub(crate) fn type_of(&mut self, value: &TypedExpr, span: Span) -> Lowered {
        let lowered = self.expression(value);
        let result = self.runtime_type(lowered, &value.ty, span);
        self.release(lowered, span);
        result
    }

    /// Returns the type object of `lowered`, a value of the checked type `ty`, borrowed: the
    /// class an exception or another object holds, or the type its checked type gives.
    pub(crate) fn runtime_type(&mut self, lowered: Lowered, ty: &Type, span: Span) -> Lowered {
        match (ty, TypeObjectOf::of(ty)) {
            (Type::Exception(_), _) => self.exception_type(&lowered, span),
            // These objects carry their type in their header.
            (
                Type::Iterator(_)
                | Type::Iterable(_)
                | Type::Module(_)
                | Type::AnyModule
                | Type::TextFile
                | Type::Traceback,
                _,
            ) => {
                let object = self.bits(Self::borrowed(lowered.value, lowered.ty), span);
                let class = self.call_runtime(RuntimeFunction::ObjectType, vec![object], span);
                self.borrowed_result(class)
            }
            (_, Some(of)) => self.type_object(&of, span),
            (Type::Optional(inner), None) => {
                let none = self.is_none(lowered, span);
                let absent = self.type_object(&TypeObjectOf::NoneType, span).value;
                let base = TypeObjectOf::of(inner).unwrap_or(TypeObjectOf::Object);
                let present = self.type_object(&base, span).value;
                let chosen = self.select(none, absent, present, TyirType::TYPE);
                Self::borrowed(chosen, TyirType::TYPE)
            }
            (_, None) => unreachable!("a checked program has no error types"),
        }
    }
}
