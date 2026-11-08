//! The methods of built-in types that a program can call (LIB-03).

use crate::types::Type;

/// A method of a built-in type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    /// `list.append(item)`: adds an item at the end and returns `None`.
    ListAppend,
    /// `list.pop()` or `list.pop(index)`: removes the item, the last by default, and returns it.
    ListPop,
    /// `list.insert(index, item)`: adds an item before `index`, clamped to the list.
    ListInsert,
    /// `list.extend(other)`: adds the items of another list at the end.
    ListExtend,
    /// `list.remove(item)`: removes the first equal item, or raises `ValueError`.
    ListRemove,
    /// `list.index(item)`: returns the position of the first equal item, or raises `ValueError`.
    ListIndex,
    /// `list.count(item)`: returns how many items are equal to `item`.
    ListCount,
    /// `list.clear()`: removes every item.
    ListClear,
    /// `list.copy()`: returns a new list with the same items.
    ListCopy,
    /// `list.reverse()`: reverses the items in place.
    ListReverse,
    /// `list.sort(reverse=False)`: sorts the items in place, stably, and returns `None`.
    ListSort,
}

/// The parameters a method takes, how many of them a call must give, and its result.
#[derive(Clone, Debug, PartialEq)]
pub struct MethodSignature {
    /// The parameter types, in order.
    pub parameters: Vec<Type>,
    /// The number of leading parameters a call must give; the rest have Python's defaults.
    pub required: usize,
    /// The result type.
    pub result: Type,
    /// The name of the last parameter when it can only be given by keyword, as `reverse` of
    /// `list.sort`.
    pub keyword: Option<&'static str>,
}

impl Method {
    /// Returns the method `name` of a value of type `receiver`, if the release provides it.
    #[must_use]
    pub fn of(receiver: &Type, name: &str) -> Option<Self> {
        let Type::List(_) = receiver else {
            return None;
        };
        Some(match name {
            "append" => Self::ListAppend,
            "pop" => Self::ListPop,
            "insert" => Self::ListInsert,
            "extend" => Self::ListExtend,
            "remove" => Self::ListRemove,
            "index" => Self::ListIndex,
            "count" => Self::ListCount,
            "clear" => Self::ListClear,
            "copy" => Self::ListCopy,
            "reverse" => Self::ListReverse,
            "sort" => Self::ListSort,
            _ => return None,
        })
    }

    /// Returns the method's Python name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::ListAppend => "append",
            Self::ListPop => "pop",
            Self::ListInsert => "insert",
            Self::ListExtend => "extend",
            Self::ListRemove => "remove",
            Self::ListIndex => "index",
            Self::ListCount => "count",
            Self::ListClear => "clear",
            Self::ListCopy => "copy",
            Self::ListReverse => "reverse",
            Self::ListSort => "sort",
        }
    }

    /// Returns the signature of the method on a receiver of type `receiver`, which is the type
    /// `of` found the method on.
    #[must_use]
    pub fn signature(self, receiver: &Type) -> MethodSignature {
        let item = match receiver {
            Type::List(item) => item.as_ref().clone(),
            _ => Type::Error,
        };
        let (parameters, required, result) = match self {
            Self::ListAppend | Self::ListRemove => (vec![item], 1, Type::NoneType),
            Self::ListPop => (vec![Type::Int], 0, item),
            Self::ListInsert => (vec![Type::Int, item], 2, Type::NoneType),
            Self::ListExtend => (vec![receiver.clone()], 1, Type::NoneType),
            Self::ListIndex | Self::ListCount => (vec![item], 1, Type::Int),
            Self::ListClear | Self::ListReverse => (Vec::new(), 0, Type::NoneType),
            Self::ListCopy => (Vec::new(), 0, receiver.clone()),
            Self::ListSort => (vec![Type::Bool], 0, Type::NoneType),
        };
        MethodSignature {
            parameters,
            required,
            result,
            keyword: (self == Self::ListSort).then_some("reverse"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Method;
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
        assert_eq!(
            (pop.parameters, pop.required, pop.result),
            (vec![Type::Int], 0, Type::Str)
        );
        let insert = Method::ListInsert.signature(&list);
        assert_eq!(insert.parameters, vec![Type::Int, Type::Str]);
        assert_eq!(Method::ListCopy.signature(&list).result, list);
        assert_eq!(
            Method::ListIndex.signature(&Type::Int).parameters,
            vec![Type::Error]
        );
    }
}
