//! Tests of `method`.

use super::{Method, ParameterKind, ParameterType};
use crate::types::Type;

#[test]
fn methods_belong_to_their_type() {
    let list = Type::List(Box::new(Type::Int));
    assert_eq!(Method::of(&list, "append"), Some(Method::ListAppend));
    assert_eq!(Method::of(&list, "push"), None);
    assert_eq!(Method::of(&Type::Str, "append"), None);
    for name in [
        "append", "pop", "insert", "extend", "remove", "index", "count", "clear", "copy",
        "reverse", "sort",
    ] {
        assert_eq!(Method::of(&list, name).map(Method::name), Some(name));
    }
}

#[test]
fn signatures_follow_the_item_type() {
    let list = Type::List(Box::new(Type::Str));
    let pop = Method::ListPop.signature(&list);
    assert_eq!(pop.result, Type::Str);
    assert_eq!(pop.parameters[0].ty, ParameterType::Exact(Type::Int));
    assert!(!pop.parameters[0].required);
    let insert = Method::ListInsert.signature(&list);
    assert_eq!(insert.parameters[1].ty, ParameterType::Exact(Type::Str));
    assert_eq!(Method::ListCopy.signature(&list).result, list);
    assert_eq!(
        Method::ListIndex.signature(&Type::Int).parameters[0].ty,
        ParameterType::Exact(Type::Error)
    );
    assert_eq!(
        Method::ListSort.signature(&list).parameters[0].kind,
        ParameterKind::Keyword
    );
}

#[test]
fn set_methods_take_items_and_sets_of_the_receiver_type() {
    let set = Type::Set(Box::new(Type::Int));
    assert_eq!(Method::of(&set, "add"), Some(Method::SetAdd));
    assert_eq!(Method::of(&set, "append"), None);
    for method in super::SET_METHODS {
        assert_eq!(Method::of(&set, method.name()), Some(method));
    }
    let add = Method::SetAdd.signature(&set);
    assert_eq!(add.parameters[0].ty, ParameterType::Exact(Type::Int));
    assert_eq!(add.result, Type::NoneType);
    assert_eq!(Method::SetPop.signature(&set).result, Type::Int);
    assert_eq!(Method::SetUnion.signature(&set).result, set);
    let update = Method::SetUpdate.signature(&set);
    assert_eq!(update.parameters[0].ty, ParameterType::Items(Type::Int));
    assert_eq!(update.parameters[0].kind, ParameterKind::Rest);
    assert!(!update.parameters[0].required);
    let other = &Method::SetIssubset.signature(&set).parameters[0];
    assert_eq!(other.kind, ParameterKind::Positional);
    assert!(other.required);
    assert_eq!(Method::SetIsdisjoint.signature(&set).result, Type::Bool);
    assert_eq!(Method::SetCopy.signature(&set).parameters, []);
}

#[test]
fn str_methods_take_python_s_parameters() {
    assert_eq!(Method::of(&Type::Str, "split"), Some(Method::StrSplit));
    assert_eq!(Method::of(&Type::Str, "upper"), Some(Method::StrUpper));
    assert_eq!(Method::of(&Type::Str, "format"), None);
    let find = Method::StrFind.signature(&Type::Str);
    let names: Vec<&str> = find
        .parameters
        .iter()
        .map(|parameter| parameter.name)
        .collect();
    assert_eq!(names, ["sub", "start", "end"]);
    let split = Method::StrSplit.signature(&Type::Str);
    assert_eq!(split.parameters[0].ty, ParameterType::StrOrNone);
    assert_eq!(split.parameters[1].kind, ParameterKind::Either);
    assert_eq!(
        Method::StrJoin.signature(&Type::Str).parameters[0].ty,
        ParameterType::Strings
    );
    assert_eq!(Method::StrIsascii.signature(&Type::Str).result, Type::Bool);
    assert_eq!(Method::StrIndex.name(), "index");
}
