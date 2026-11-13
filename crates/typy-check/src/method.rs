//! The methods of built-in types that a program can call (LIB-03).

mod signature;

pub(crate) use signature::{MethodSignature, Parameter, ParameterKind, ParameterType};

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
    /// `set.add(item)`: adds an item unless an equal one is there.
    SetAdd,
    /// `set.remove(item)`: removes an equal item, or raises `KeyError`.
    SetRemove,
    /// `set.discard(item)`: removes an equal item, if there is one.
    SetDiscard,
    /// `set.pop()`: removes and returns an item, or raises `KeyError` on an empty set.
    SetPop,
    /// `set.clear()`: removes every item.
    SetClear,
    /// `set.copy()`: returns a new set with the same items.
    SetCopy,
    /// `set.union(other)`: a new set of the items of both.
    SetUnion,
    /// `set.intersection(other)`: a new set of the items in both.
    SetIntersection,
    /// `set.difference(other)`: a new set of the items not in `other`.
    SetDifference,
    /// `set.symmetric_difference(other)`: a new set of the items in exactly one.
    SetSymmetricDifference,
    /// `set.update(other)`: adds the items of `other`.
    SetUpdate,
    /// `set.intersection_update(other)`: keeps the items `other` holds.
    SetIntersectionUpdate,
    /// `set.difference_update(other)`: removes the items of `other`.
    SetDifferenceUpdate,
    /// `set.symmetric_difference_update(other)`: keeps the items in exactly one.
    SetSymmetricDifferenceUpdate,
    /// `set.issubset(other)`: whether `other` holds every item.
    SetIssubset,
    /// `set.issuperset(other)`: whether every item of `other` is held.
    SetIssuperset,
    /// `set.isdisjoint(other)`: whether no item is in both.
    SetIsdisjoint,
    /// `dict.get(key, default)`: the value of `key`, or `default`.
    DictGet,
    /// `dict.pop(key)` or `dict.pop(key, default)`: removes `key` and returns its value.
    DictPop,
    /// `dict.popitem()`: removes and returns the last `(key, value)` pair.
    DictPopitem,
    /// `dict.setdefault(key, default)`: the value of `key`, set to `default` when missing.
    DictSetdefault,
    /// `dict.keys()`: a view of the keys.
    DictKeys,
    /// `dict.values()`: a view of the values.
    DictValues,
    /// `dict.items()`: a view of the `(key, value)` pairs.
    DictItems,
    /// `dict.update(other)`: sets each pair of a dict or an iterable of pairs.
    DictUpdate,
    /// `dict.clear()`: removes every entry.
    DictClear,
    /// `dict.copy()`: a new dict with the same entries.
    DictCopy,
    /// `tuple.index(item)`: returns the position of the first equal item, or raises
    /// `ValueError`.
    TupleIndex,
    /// `tuple.count(item)`: returns how many items are equal to `item`.
    TupleCount,
    /// `str.find(sub, start, end)`: the first position of `sub`, or -1.
    StrFind,
    /// `str.rfind(sub, start, end)`: the last position of `sub`, or -1.
    StrRfind,
    /// `str.index(sub, start, end)`: the first position of `sub`, or `ValueError`.
    StrIndex,
    /// `str.rindex(sub, start, end)`: the last position of `sub`, or `ValueError`.
    StrRindex,
    /// `str.count(sub, start, end)`: the occurrences of `sub` that do not overlap.
    StrCount,
    /// `str.startswith(prefix, start, end)`, with a `str` or a tuple of them.
    StrStartswith,
    /// `str.endswith(suffix, start, end)`, with a `str` or a tuple of them.
    StrEndswith,
    /// `str.split(sep=None, maxsplit=-1)`.
    StrSplit,
    /// `str.rsplit(sep=None, maxsplit=-1)`.
    StrRsplit,
    /// `str.splitlines(keepends=False)`.
    StrSplitlines,
    /// `str.partition(sep)`: the parts before, of and after the first `sep`.
    StrPartition,
    /// `str.rpartition(sep)`: the parts before, of and after the last `sep`.
    StrRpartition,
    /// `str.join(iterable)`: the strings, with the receiver between each two.
    StrJoin,
    /// `str.strip(chars=None)`.
    StrStrip,
    /// `str.lstrip(chars=None)`.
    StrLstrip,
    /// `str.rstrip(chars=None)`.
    StrRstrip,
    /// `str.removeprefix(prefix)`.
    StrRemoveprefix,
    /// `str.removesuffix(suffix)`.
    StrRemovesuffix,
    /// `str.replace(old, new, count=-1)`.
    StrReplace,
    /// `str.center(width, fillchar=' ')`.
    StrCenter,
    /// `str.ljust(width, fillchar=' ')`.
    StrLjust,
    /// `str.rjust(width, fillchar=' ')`.
    StrRjust,
    /// `str.zfill(width)`.
    StrZfill,
    /// `str.expandtabs(tabsize=8)`.
    StrExpandtabs,
    /// `str.isascii()`.
    StrIsascii,
    /// `str.upper()`.
    StrUpper,
    /// `str.lower()`.
    StrLower,
    /// `str.casefold()`.
    StrCasefold,
    /// `str.swapcase()`.
    StrSwapcase,
    /// `str.capitalize()`.
    StrCapitalize,
    /// `str.title()`.
    StrTitle,
    /// `str.isalpha()`.
    StrIsalpha,
    /// `str.isalnum()`.
    StrIsalnum,
    /// `str.isdecimal()`.
    StrIsdecimal,
    /// `str.isdigit()`.
    StrIsdigit,
    /// `str.isnumeric()`.
    StrIsnumeric,
    /// `str.isspace()`.
    StrIsspace,
    /// `str.isupper()`.
    StrIsupper,
    /// `str.islower()`.
    StrIslower,
    /// `str.istitle()`.
    StrIstitle,
    /// `str.isidentifier()`.
    StrIsidentifier,
    /// `str.isprintable()`.
    StrIsprintable,
    /// `str.encode(encoding='utf-8', errors='strict')`.
    StrEncode,
    /// `bytes.decode(encoding='utf-8', errors='strict')`.
    BytesDecode,
    /// `bytes.hex()`.
    BytesHex,
    /// `file.write(text)`: writes the text and returns the number of code points (LIB-04).
    TextWrite,
    /// `file.flush()`: writes out what the file holds.
    TextFlush,
    /// `file.read(size=-1)`: `size` characters, or the rest of the file (ADR-0019).
    TextRead,
    /// `file.readline(size=-1)`: the next line with its `\n`.
    TextReadline,
    /// `file.readlines()`: the list of the lines left.
    TextReadlines,
    /// `file.writelines(lines)`: writes each `str` of an iterable.
    TextWritelines,
    /// `file.close()`: writes out what the file holds and closes it.
    TextClose,
}

/// The methods of `str` that `bytes` has too, with ASCII meaning.
const BYTES_SHARED: [Method; 37] = [
    Method::StrFind,
    Method::StrRfind,
    Method::StrIndex,
    Method::StrRindex,
    Method::StrCount,
    Method::StrStartswith,
    Method::StrEndswith,
    Method::StrSplit,
    Method::StrRsplit,
    Method::StrSplitlines,
    Method::StrPartition,
    Method::StrRpartition,
    Method::StrJoin,
    Method::StrStrip,
    Method::StrLstrip,
    Method::StrRstrip,
    Method::StrRemoveprefix,
    Method::StrRemovesuffix,
    Method::StrReplace,
    Method::StrCenter,
    Method::StrLjust,
    Method::StrRjust,
    Method::StrZfill,
    Method::StrExpandtabs,
    Method::StrIsascii,
    Method::StrUpper,
    Method::StrLower,
    Method::StrSwapcase,
    Method::StrCapitalize,
    Method::StrTitle,
    Method::StrIsalpha,
    Method::StrIsalnum,
    Method::StrIsdigit,
    Method::StrIsspace,
    Method::StrIsupper,
    Method::StrIslower,
    Method::StrIstitle,
];

/// The `str` methods, by name, in the order of the variants.
const STR_METHODS: [(&str, Method); 43] = [
    ("find", Method::StrFind),
    ("rfind", Method::StrRfind),
    ("index", Method::StrIndex),
    ("rindex", Method::StrRindex),
    ("count", Method::StrCount),
    ("startswith", Method::StrStartswith),
    ("endswith", Method::StrEndswith),
    ("split", Method::StrSplit),
    ("rsplit", Method::StrRsplit),
    ("splitlines", Method::StrSplitlines),
    ("partition", Method::StrPartition),
    ("rpartition", Method::StrRpartition),
    ("join", Method::StrJoin),
    ("strip", Method::StrStrip),
    ("lstrip", Method::StrLstrip),
    ("rstrip", Method::StrRstrip),
    ("removeprefix", Method::StrRemoveprefix),
    ("removesuffix", Method::StrRemovesuffix),
    ("replace", Method::StrReplace),
    ("center", Method::StrCenter),
    ("ljust", Method::StrLjust),
    ("rjust", Method::StrRjust),
    ("zfill", Method::StrZfill),
    ("expandtabs", Method::StrExpandtabs),
    ("isascii", Method::StrIsascii),
    ("upper", Method::StrUpper),
    ("lower", Method::StrLower),
    ("casefold", Method::StrCasefold),
    ("swapcase", Method::StrSwapcase),
    ("capitalize", Method::StrCapitalize),
    ("title", Method::StrTitle),
    ("isalpha", Method::StrIsalpha),
    ("isalnum", Method::StrIsalnum),
    ("isdecimal", Method::StrIsdecimal),
    ("isdigit", Method::StrIsdigit),
    ("isnumeric", Method::StrIsnumeric),
    ("isspace", Method::StrIsspace),
    ("isupper", Method::StrIsupper),
    ("islower", Method::StrIslower),
    ("istitle", Method::StrIstitle),
    ("isidentifier", Method::StrIsidentifier),
    ("isprintable", Method::StrIsprintable),
    ("encode", Method::StrEncode),
];

/// The methods of `dict`.
const DICT_METHODS: [Method; 10] = [
    Method::DictGet,
    Method::DictPop,
    Method::DictPopitem,
    Method::DictSetdefault,
    Method::DictKeys,
    Method::DictValues,
    Method::DictItems,
    Method::DictUpdate,
    Method::DictClear,
    Method::DictCopy,
];

/// The methods of `set`.
const SET_METHODS: [Method; 17] = [
    Method::SetAdd,
    Method::SetRemove,
    Method::SetDiscard,
    Method::SetPop,
    Method::SetClear,
    Method::SetCopy,
    Method::SetUnion,
    Method::SetIntersection,
    Method::SetDifference,
    Method::SetSymmetricDifference,
    Method::SetUpdate,
    Method::SetIntersectionUpdate,
    Method::SetDifferenceUpdate,
    Method::SetSymmetricDifferenceUpdate,
    Method::SetIssubset,
    Method::SetIssuperset,
    Method::SetIsdisjoint,
];

impl Method {
    /// Returns the method `name` of a value of type `receiver`, if the release provides it.
    #[must_use]
    pub fn of(receiver: &Type, name: &str) -> Option<Self> {
        if *receiver == Type::Str {
            return STR_METHODS
                .iter()
                .find(|(each, _)| *each == name)
                .map(|(_, method)| *method);
        }
        if *receiver == Type::Bytes {
            return match name {
                "decode" => Some(Self::BytesDecode),
                "hex" => Some(Self::BytesHex),
                _ => STR_METHODS
                    .iter()
                    .find(|(each, method)| *each == name && BYTES_SHARED.contains(method))
                    .map(|(_, method)| *method),
            };
        }
        if let Type::Dict(..) = receiver {
            return DICT_METHODS
                .iter()
                .find(|method| method.name() == name)
                .copied();
        }
        if let Type::Set(_) = receiver {
            return SET_METHODS
                .iter()
                .find(|method| method.name() == name)
                .copied();
        }
        if *receiver == Type::TextFile {
            return match name {
                "write" => Some(Self::TextWrite),
                "flush" => Some(Self::TextFlush),
                "read" => Some(Self::TextRead),
                "readline" => Some(Self::TextReadline),
                "readlines" => Some(Self::TextReadlines),
                "writelines" => Some(Self::TextWritelines),
                "close" => Some(Self::TextClose),
                _ => None,
            };
        }
        if let Type::TupleOf(_) = receiver {
            return match name {
                "index" => Some(Self::TupleIndex),
                "count" => Some(Self::TupleCount),
                _ => None,
            };
        }
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
            Self::ListPop | Self::SetPop | Self::DictPop => "pop",
            Self::ListInsert => "insert",
            Self::ListExtend => "extend",
            Self::ListRemove | Self::SetRemove => "remove",
            Self::ListIndex | Self::TupleIndex | Self::StrIndex => "index",
            Self::ListCount | Self::TupleCount | Self::StrCount => "count",
            Self::ListClear | Self::SetClear | Self::DictClear => "clear",
            Self::ListCopy | Self::SetCopy | Self::DictCopy => "copy",
            Self::DictGet => "get",
            Self::DictPopitem => "popitem",
            Self::DictSetdefault => "setdefault",
            Self::DictKeys => "keys",
            Self::DictValues => "values",
            Self::DictItems => "items",
            Self::SetAdd => "add",
            Self::SetDiscard => "discard",
            Self::SetUnion => "union",
            Self::SetIntersection => "intersection",
            Self::SetDifference => "difference",
            Self::SetSymmetricDifference => "symmetric_difference",
            Self::SetUpdate | Self::DictUpdate => "update",
            Self::SetIntersectionUpdate => "intersection_update",
            Self::SetDifferenceUpdate => "difference_update",
            Self::SetSymmetricDifferenceUpdate => "symmetric_difference_update",
            Self::SetIssubset => "issubset",
            Self::SetIssuperset => "issuperset",
            Self::SetIsdisjoint => "isdisjoint",
            Self::ListReverse => "reverse",
            Self::ListSort => "sort",
            Self::StrFind => "find",
            Self::StrRfind => "rfind",
            Self::StrRindex => "rindex",
            Self::StrStartswith => "startswith",
            Self::StrEndswith => "endswith",
            Self::StrSplit => "split",
            Self::StrRsplit => "rsplit",
            Self::StrSplitlines => "splitlines",
            Self::StrPartition => "partition",
            Self::StrRpartition => "rpartition",
            Self::StrJoin => "join",
            Self::StrStrip => "strip",
            Self::StrLstrip => "lstrip",
            Self::StrRstrip => "rstrip",
            Self::StrRemoveprefix => "removeprefix",
            Self::StrRemovesuffix => "removesuffix",
            Self::StrReplace => "replace",
            Self::StrCenter => "center",
            Self::StrLjust => "ljust",
            Self::StrRjust => "rjust",
            Self::StrZfill => "zfill",
            Self::StrExpandtabs => "expandtabs",
            Self::StrIsascii => "isascii",
            Self::StrUpper => "upper",
            Self::StrLower => "lower",
            Self::StrCasefold => "casefold",
            Self::StrSwapcase => "swapcase",
            Self::StrCapitalize => "capitalize",
            Self::StrTitle => "title",
            Self::StrIsalpha => "isalpha",
            Self::StrIsalnum => "isalnum",
            Self::StrIsdecimal => "isdecimal",
            Self::StrIsdigit => "isdigit",
            Self::StrIsnumeric => "isnumeric",
            Self::StrIsspace => "isspace",
            Self::StrIsupper => "isupper",
            Self::StrIslower => "islower",
            Self::StrIstitle => "istitle",
            Self::StrIsidentifier => "isidentifier",
            Self::StrIsprintable => "isprintable",
            Self::StrEncode => "encode",
            Self::BytesDecode => "decode",
            Self::BytesHex => "hex",
            Self::TextWrite => "write",
            Self::TextFlush => "flush",
            Self::TextRead => "read",
            Self::TextReadline => "readline",
            Self::TextReadlines => "readlines",
            Self::TextWritelines => "writelines",
            Self::TextClose => "close",
        }
    }
}

#[cfg(test)]
mod tests {
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
}
