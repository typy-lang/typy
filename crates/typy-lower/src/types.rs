//! Maps checked types to TyIR types.

use typy_check::Type;
use typy_tyir::{OptionalBase, TupleTypes, TyirType, TypeId};

/// Returns the TyIR type of a checked type; `None` maps to `unit`, and a tuple to the module's
/// tuple type with the same items. A checked program has no error types.
pub(crate) fn tyir_type(ty: &Type, tuples: &mut TupleTypes) -> TyirType {
    match ty {
        Type::Int => TyirType::Int,
        Type::Float => TyirType::Float,
        Type::Bool => TyirType::Bool,
        Type::Str => TyirType::STR,
        Type::Bytes => TyirType::BYTES,
        Type::NoneType | Type::Error => TyirType::Unit,
        // A `tuple[T, ...]` is a list object with the tuple type ID, so every list function
        // of the runtime reads it (DAT-14).
        Type::List(_) | Type::TupleOf(_) => TyirType::LIST,
        Type::Set(_) => TyirType::SET,
        Type::Dict(..) => TyirType::DICT,
        Type::DictView(..) => TyirType::VIEW,
        Type::Instance(class) => TyirType::Object(TypeId::class(class.id.0)),
        Type::TypeObject(_) => TyirType::TYPE,
        Type::Exception(_) => TyirType::EXCEPTION,
        Type::Iterator(_) => TyirType::ITERATOR,
        Type::Traceback => TyirType::TRACEBACK,
        Type::Range => TyirType::RANGE,
        Type::Module(_) | Type::AnyModule => TyirType::MODULE,
        Type::TextFile => TyirType::TEXT_FILE,
        Type::Iterable(_) => TyirType::ITERABLE,
        Type::Union(members) => union_type(ty, members, tuples),
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

/// Returns how a union of objects is held: as its members are when each is held alike, as an
/// instance of the class every member derives from, or as an object whose header says what
/// it is (ADR-0022).
fn union_type(ty: &Type, members: &[Type], tuples: &mut TupleTypes) -> TyirType {
    if let Some(base) = ty.union_base() {
        return TyirType::Object(TypeId::class(base.id.0));
    }
    let held: Vec<TyirType> = members
        .iter()
        .map(|member| tyir_type(member, tuples))
        .collect();
    if held.windows(2).all(|pair| pair[0] == pair[1]) {
        return held[0];
    }
    TyirType::UNION
}

#[cfg(test)]
mod tests {
    use super::tyir_type;
    use typy_check::Type;
    use typy_tyir::{TupleTypes, TyirType};

    #[test]
    fn each_checked_type_has_its_tyir_type() {
        let mut tuples = TupleTypes::default();
        assert_eq!(tyir_type(&Type::Int, &mut tuples), TyirType::Int);
        assert_eq!(tyir_type(&Type::Float, &mut tuples), TyirType::Float);
        assert_eq!(tyir_type(&Type::Bool, &mut tuples), TyirType::Bool);
        assert_eq!(tyir_type(&Type::Str, &mut tuples), TyirType::STR);
        assert_eq!(tyir_type(&Type::NoneType, &mut tuples), TyirType::Unit);
        let pair = tyir_type(&Type::Tuple(vec![Type::Int, Type::Str]), &mut tuples);
        assert_eq!(pair, tuples.intern(vec![TyirType::Int, TyirType::STR]));
    }

    #[test]
    fn a_union_is_held_as_its_members_share_or_as_any_object() {
        use typy_check::{ClassId, InstanceClass};
        use typy_tyir::TypeId;
        let mut tuples = TupleTypes::default();
        let mixed = Type::union(vec![Type::Str, Type::List(Box::new(Type::Int))]);
        assert_eq!(tyir_type(&mixed, &mut tuples), TyirType::UNION);
        let text = Type::union(vec![
            Type::List(Box::new(Type::Str)),
            Type::TupleOf(Box::new(Type::Int)),
        ]);
        assert_eq!(tyir_type(&text, &mut tuples), TyirType::LIST);
        let derived = |id: u32, name: &str| {
            let mut class = InstanceClass::root(ClassId(id), name);
            class.ancestors = vec![(ClassId(0), "Shape".to_owned())];
            Type::instance(class)
        };
        let shapes = Type::union(vec![derived(1, "Circle"), derived(2, "Square")]);
        assert_eq!(
            tyir_type(&shapes, &mut tuples),
            TyirType::Object(TypeId::class(0))
        );
    }
}
