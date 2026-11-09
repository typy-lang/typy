//! The parameters of the methods of built-in types: their names, how a call may give them,
//! and their types (LIB-03).

use super::Method;
use crate::types::{Type, View};

/// How a call may give a parameter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParameterKind {
    /// Only by position, as most methods of built-in types take their arguments.
    Positional,
    /// By position or by its name.
    Either,
    /// Only by its name, as `reverse` of `list.sort`.
    Keyword,
    /// Every remaining positional argument, none or many, as `*others` of `set.union`.
    Rest,
}

/// What a parameter accepts.
#[derive(Clone, Debug, PartialEq)]
pub enum ParameterType {
    /// A value of this type.
    Exact(Type),
    /// A `str`, or `None` for the method's default, as `sep` of `str.split`.
    StrOrNone,
    /// A `str`, or a tuple of them, as `prefix` of `str.startswith`.
    Affixes,
    /// Any iterable of `str`, as the argument of `str.join`.
    Strings,
    /// Any iterable of this item type, as the argument of `set.update`.
    Items(Type),
    /// A dict of these key and value types, or any iterable of `(key, value)` pairs of them, as
    /// the argument of `dict.update`.
    Pairs(Type, Type),
    /// The name of a codec, a string literal the checker resolves.
    Codec,
    /// The name of an error handler, a string literal the checker resolves; one for decoding
    /// when `decoding` is set.
    Handler {
        /// True for `bytes.decode`.
        decoding: bool,
    },
}

/// One parameter of a method.
#[derive(Clone, Debug, PartialEq)]
pub struct Parameter {
    /// The name CPython gives it.
    pub name: &'static str,
    /// What it accepts.
    pub ty: ParameterType,
    /// How a call may give it.
    pub kind: ParameterKind,
    /// True when a call must give it; otherwise it has Python's default.
    pub required: bool,
}

/// The parameters of a method, in order, and its result.
#[derive(Clone, Debug, PartialEq)]
pub struct MethodSignature {
    /// The parameters.
    pub parameters: Vec<Parameter>,
    /// The result type.
    pub result: Type,
}

fn parameter(
    name: &'static str,
    ty: ParameterType,
    kind: ParameterKind,
    required: bool,
) -> Parameter {
    Parameter {
        name,
        ty,
        kind,
        required,
    }
}

/// A parameter given only by position.
fn positional(name: &'static str, ty: Type, required: bool) -> Parameter {
    parameter(
        name,
        ParameterType::Exact(ty),
        ParameterKind::Positional,
        required,
    )
}

/// `start` and `end`, which a search or a match may give after its first argument.
fn bounds() -> [Parameter; 2] {
    [
        positional("start", Type::Int, false),
        positional("end", Type::Int, false),
    ]
}

impl Method {
    /// Returns the signature of the method on a receiver of type `receiver`, which is the type
    /// `of` found the method on.
    #[must_use]
    pub fn signature(self, receiver: &Type) -> MethodSignature {
        if let Type::Dict(key, value) = receiver {
            return self.dict_signature(receiver, key, value);
        }
        let item = match receiver {
            Type::Set(item) => item.as_ref().clone(),
            _ => receiver.sequence_item().cloned().unwrap_or(Type::Error),
        };
        let other = || {
            vec![positional_as(
                "other",
                ParameterType::Items(item.clone()),
                true,
            )]
        };
        let others = || {
            vec![parameter(
                "others",
                ParameterType::Items(item.clone()),
                ParameterKind::Rest,
                false,
            )]
        };
        let (parameters, result) = match self {
            Self::SetAdd | Self::SetRemove | Self::SetDiscard => {
                (vec![positional("object", item, true)], Type::NoneType)
            }
            Self::SetPop => (Vec::new(), item),
            Self::SetUnion | Self::SetIntersection | Self::SetDifference => {
                (others(), receiver.clone())
            }
            Self::SetSymmetricDifference => (other(), receiver.clone()),
            Self::SetUpdate | Self::SetIntersectionUpdate | Self::SetDifferenceUpdate => {
                (others(), Type::NoneType)
            }
            Self::SetSymmetricDifferenceUpdate => (other(), Type::NoneType),
            Self::SetIssubset | Self::SetIssuperset | Self::SetIsdisjoint => (other(), Type::Bool),
            Self::ListAppend => (vec![positional("object", item, true)], Type::NoneType),
            Self::ListRemove => (vec![positional("value", item, true)], Type::NoneType),
            Self::ListPop => (vec![positional("index", Type::Int, false)], item),
            Self::ListInsert => (
                vec![
                    positional("index", Type::Int, true),
                    positional("object", item, true),
                ],
                Type::NoneType,
            ),
            Self::ListExtend => (
                vec![positional("iterable", receiver.clone(), true)],
                Type::NoneType,
            ),
            Self::ListIndex | Self::ListCount | Self::TupleIndex | Self::TupleCount => {
                (vec![positional("value", item, true)], Type::Int)
            }
            Self::ListClear | Self::ListReverse | Self::SetClear => (Vec::new(), Type::NoneType),
            Self::ListCopy | Self::SetCopy => (Vec::new(), receiver.clone()),
            Self::ListSort => (
                vec![parameter(
                    "reverse",
                    ParameterType::Exact(Type::Bool),
                    ParameterKind::Keyword,
                    false,
                )],
                Type::NoneType,
            ),
            _ => return self.str_signature(receiver),
        };
        MethodSignature { parameters, result }
    }

    /// Returns the signature of a method of a dict of `key` and `value`.
    fn dict_signature(self, receiver: &Type, key: &Type, value: &Type) -> MethodSignature {
        let view = |view| Type::DictView(view, Box::new(key.clone()), Box::new(value.clone()));
        let (parameters, result) = match self {
            Self::DictGet | Self::DictSetdefault => (
                vec![
                    positional("key", key.clone(), true),
                    positional("default", value.clone(), true),
                ],
                value.clone(),
            ),
            Self::DictPop => (
                vec![
                    positional("key", key.clone(), true),
                    positional("default", value.clone(), false),
                ],
                value.clone(),
            ),
            Self::DictPopitem => (Vec::new(), Type::Tuple(vec![key.clone(), value.clone()])),
            Self::DictKeys => (Vec::new(), view(View::Keys)),
            Self::DictValues => (Vec::new(), view(View::Values)),
            Self::DictItems => (Vec::new(), view(View::Items)),
            Self::DictUpdate => (
                vec![positional_as(
                    "other",
                    ParameterType::Pairs(key.clone(), value.clone()),
                    false,
                )],
                Type::NoneType,
            ),
            Self::DictCopy => (Vec::new(), receiver.clone()),
            _ => (Vec::new(), Type::NoneType),
        };
        MethodSignature { parameters, result }
    }

    /// Returns the signature of a method of `str`, or of `bytes` when `text` is `bytes`.
    fn str_signature(self, text: &Type) -> MethodSignature {
        let strings = Type::List(Box::new(text.clone()));
        let (parameters, result) = match self {
            Self::StrFind | Self::StrRfind | Self::StrIndex | Self::StrRindex | Self::StrCount => {
                (searched(positional("sub", text.clone(), true)), Type::Int)
            }
            Self::StrStartswith | Self::StrEndswith => (
                searched(positional_as("prefix", ParameterType::Affixes, true)),
                Type::Bool,
            ),
            Self::StrSplit | Self::StrRsplit => (
                vec![
                    either("sep", ParameterType::StrOrNone),
                    either("maxsplit", ParameterType::Exact(Type::Int)),
                ],
                strings,
            ),
            Self::StrSplitlines => (
                vec![either("keepends", ParameterType::Exact(Type::Bool))],
                strings,
            ),
            Self::StrPartition | Self::StrRpartition => (
                vec![positional("sep", text.clone(), true)],
                Type::Tuple(vec![text.clone(), text.clone(), text.clone()]),
            ),
            Self::StrJoin => (
                vec![positional_as("iterable", ParameterType::Strings, true)],
                text.clone(),
            ),
            Self::StrStrip | Self::StrLstrip | Self::StrRstrip => (
                vec![positional_as("chars", ParameterType::StrOrNone, false)],
                text.clone(),
            ),
            Self::StrRemoveprefix => (vec![positional("prefix", text.clone(), true)], text.clone()),
            Self::StrRemovesuffix => (vec![positional("suffix", text.clone(), true)], text.clone()),
            Self::StrReplace => (
                vec![
                    positional("old", text.clone(), true),
                    positional("new", text.clone(), true),
                    positional("count", Type::Int, false),
                ],
                text.clone(),
            ),
            Self::StrCenter | Self::StrLjust | Self::StrRjust => (
                vec![
                    positional("width", Type::Int, true),
                    positional("fillchar", text.clone(), false),
                ],
                text.clone(),
            ),
            Self::StrZfill => (vec![positional("width", Type::Int, true)], text.clone()),
            Self::StrExpandtabs => (
                vec![either("tabsize", ParameterType::Exact(Type::Int))],
                text.clone(),
            ),
            Self::StrEncode | Self::BytesDecode => (
                vec![
                    either("encoding", ParameterType::Codec),
                    either(
                        "errors",
                        ParameterType::Handler {
                            decoding: self == Self::BytesDecode,
                        },
                    ),
                ],
                if self == Self::StrEncode {
                    Type::Bytes
                } else {
                    Type::Str
                },
            ),
            Self::StrUpper
            | Self::StrLower
            | Self::StrCasefold
            | Self::StrSwapcase
            | Self::StrCapitalize
            | Self::StrTitle => (Vec::new(), text.clone()),
            Self::BytesHex => (Vec::new(), Type::Str),
            _ => (Vec::new(), Type::Bool),
        };
        MethodSignature { parameters, result }
    }
}

/// A parameter given only by position, of any parameter type.
fn positional_as(name: &'static str, ty: ParameterType, required: bool) -> Parameter {
    parameter(name, ty, ParameterKind::Positional, required)
}

/// An optional parameter given by position or by name.
fn either(name: &'static str, ty: ParameterType) -> Parameter {
    parameter(name, ty, ParameterKind::Either, false)
}

/// What a search looks for, then `start` and `end`.
fn searched(first: Parameter) -> Vec<Parameter> {
    std::iter::once(first).chain(bounds()).collect()
}
