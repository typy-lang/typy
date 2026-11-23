//! Maps checked types to TyIR types.

use typy_check::Type;
use typy_tyir::{OptionalBase, TupleTypes, TyirType, TypeId};

/// Returns the TyIR type of a checked type; `None` maps to `unit`, and a tuple to the module's
/// tuple type with the same items. A checked program has no error types.
pub(crate) fn tyir_type(ty: &Type, tuples: &mut TupleTypes) -> TyirType {
    match ty {
        // A declared abstract collection is held as the built-in types it holds (LIB-20).
        Type::Abstract(collection) => tyir_type(&collection.members, tuples),
        // A literal type is held as its base type (TYP-28, ADR-0110).
        Type::Literal(value) => tyir_type(&value.base(), tuples),
        // `Self` is held as an instance of the method's class (TYP-30, ADR-0113).
        Type::SelfType(own) => tyir_type(own, tuples),
        Type::Int => TyirType::Int,
        Type::Float => TyirType::Float,
        Type::Bool => TyirType::Bool,
        Type::Str => TyirType::STR,
        Type::Bytes => TyirType::BYTES,
        Type::Bytearray => TyirType::BYTEARRAY,
        // A library is linked when the program is built and holds no value (FFI-01), and a
        // `ctypes` class is known from its type (FFI-03).
        Type::NoneType | Type::Error | Type::ForeignLibrary(_) | Type::CDataClass(_) => {
            TyirType::Unit
        }
        Type::CData(_) => TyirType::CDATA,
        Type::Callable { .. } => TyirType::FUNCTION,
        // A `tuple[T, ...]` is a list object with the tuple type ID, so every list function
        // of the runtime reads it (DAT-14).
        Type::List(_) | Type::TupleOf(_) => TyirType::LIST,
        Type::Set(_) => TyirType::SET,
        Type::Dict(..) => TyirType::DICT,
        Type::DictView(..) => TyirType::VIEW,
        // A protocol value is an instance of any class that satisfies it (ADR-0041), an
        // `object` an object of any type, a value held by value boxed (ADR-0090), and a typing
        // value an object whose header says what it is (TYP-18).
        Type::Instance(class) if class.protocol => TyirType::UNION,
        Type::Object | Type::Typing(_) => TyirType::UNION,
        Type::Instance(class) => TyirType::Object(TypeId::class(class.id.0)),
        Type::TypeObject(_) => TyirType::TYPE,
        Type::Exception(_) => TyirType::EXCEPTION,
        Type::Iterator(_) => TyirType::ITERATOR,
        Type::Generator { .. } => TyirType::GENERATOR,
        Type::Traceback => TyirType::TRACEBACK,
        Type::Range => TyirType::RANGE,
        Type::Slice => TyirType::SLICE,
        Type::Module(_) | Type::AnyModule => TyirType::MODULE,
        Type::TypeAlias(_) => TyirType::TYPE_ALIAS,
        Type::VersionInfo => TyirType::VERSION_INFO,
        Type::NotImplemented => TyirType::NOT_IMPLEMENTED,
        // A binary file is the same runtime object as a text file, with flags of its own.
        Type::TextFile | Type::BinaryFile(_) | Type::DescriptorFile(_) => TyirType::TEXT_FILE,
        Type::Iterable(_) => TyirType::ITERABLE,
        Type::Union(members) => union_type(ty, members, tuples),
        Type::Parameter(_) => unreachable!("a copy of a generic body holds no type parameter"),
        Type::Tuple(items) => {
            let items = items.iter().map(|item| tyir_type(item, tuples)).collect();
            tuples.intern(items)
        }
        Type::Optional(inner) => TyirType::Optional(match tyir_type(inner, tuples) {
            TyirType::Int => OptionalBase::Int,
            TyirType::Float => OptionalBase::Float,
            TyirType::Bool => OptionalBase::Bool,
            TyirType::Object(id) => OptionalBase::Object(id),
            // A tuple has no value to spare, so it is held beside a presence flag (ADR-0011).
            tuple @ TyirType::Tuple(_) => match tuples.intern(vec![TyirType::Bool, tuple]) {
                TyirType::Tuple(pair) => OptionalBase::Tuple(pair),
                _ => unreachable!("interning gives a tuple type"),
            },
            other => unreachable!("`{other} | None` is not an optional type the checker makes"),
        }),
    }
}

/// Returns how a union is held: one with a member held by value as a tuple of a tag and a
/// slot for each member; one of objects as its members are when each is held alike, as an
/// instance of the class every member derives from, or as an object whose header says what
/// it is (ADR-0027).
fn union_type(ty: &Type, members: &[Type], tuples: &mut TupleTypes) -> TyirType {
    if let Some(base) = ty.union_base() {
        return TyirType::Object(TypeId::class(base.id.0));
    }
    let held: Vec<TyirType> = members
        .iter()
        .map(|member| tyir_type(member, tuples))
        .collect();
    if !members.iter().all(Type::is_object) {
        return tuples.intern_union(&held);
    }
    if held.windows(2).all(|pair| pair[0] == pair[1]) {
        return held[0];
    }
    TyirType::UNION
}

#[cfg(test)]
mod tests;
