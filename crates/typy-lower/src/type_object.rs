//! Lowers type objects as values: `type(x)`, a type's name read as a value, and `__name__`
//! (CLS-12). Each type object is immortal, so a type object is never counted.

use typy_check::{Type, TypeObjectOf, TypedExpr, View};
use typy_diagnostics::Span;
use typy_tyir::{Constant, RuntimeType, TyirType, TypeObjectRef};

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
        TypeObjectOf::Instance(class, _) => return TypeObjectRef::Class(class.0),
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
        let result = match (&value.ty, TypeObjectOf::of(&value.ty)) {
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
        };
        self.release(lowered, span);
        result
    }
}
