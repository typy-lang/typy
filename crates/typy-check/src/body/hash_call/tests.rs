//! Tests of the class name a `TypeError` gives for a value without a hash.

use super::unhashable_name;
use crate::types::{Type, View};

#[test]
fn the_name_is_of_the_part_that_has_no_hash() {
    let list = Type::List(Box::new(Type::Int));
    let cases = [
        (list.clone(), "list"),
        (Type::Set(Box::new(Type::Int)), "set"),
        (Type::Dict(Box::new(Type::Str), Box::new(Type::Int)), "dict"),
        (Type::Bytearray, "bytearray"),
        (
            Type::DictView(View::Items, Box::new(Type::Str), Box::new(Type::Int)),
            "dict_items",
        ),
        (Type::Tuple(vec![Type::Int, list.clone()]), "list"),
        (Type::TupleOf(Box::new(list.clone())), "list"),
        (Type::Optional(Box::new(list)), "list"),
    ];
    for (ty, name) in cases {
        assert_eq!(unhashable_name(&ty), name, "{ty}");
    }
}
