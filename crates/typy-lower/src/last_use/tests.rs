//! Tests of `last_use`.

use super::movable_type;
use typy_check::{Type, View};

#[test]
fn a_type_holding_a_view_of_a_dict_is_not_moved() {
    let view = Type::DictView(View::Keys, Box::new(Type::Str), Box::new(Type::Int));
    for (ty, movable) in [
        (Type::Str, true),
        (Type::List(Box::new(Type::Int)), true),
        (Type::Tuple(vec![Type::Str, Type::Int]), true),
        (Type::Optional(Box::new(Type::Str)), true),
        (view.clone(), false),
        (Type::Tuple(vec![Type::Int, view.clone()]), false),
        (Type::Optional(Box::new(view)), false),
    ] {
        assert_eq!(movable_type(&ty), movable, "{ty:?}");
    }
}
