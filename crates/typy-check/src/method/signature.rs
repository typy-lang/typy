//! The parameters of the methods of built-in types: their names, how a call may give them,
//! and their types (LIB-03).

use super::Method;
use crate::types::Type;

/// How a call may give a parameter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParameterKind {
    /// Only by position, as most methods of built-in types take their arguments.
    Positional,
    /// By position or by its name.
    Either,
    /// Only by its name, as `reverse` of `list.sort`.
    Keyword,
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
        let item = receiver.sequence_item().cloned().unwrap_or(Type::Error);
        let (parameters, result) = match self {
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
            Self::ListClear | Self::ListReverse => (Vec::new(), Type::NoneType),
            Self::ListCopy => (Vec::new(), receiver.clone()),
            Self::ListSort => (
                vec![parameter(
                    "reverse",
                    ParameterType::Exact(Type::Bool),
                    ParameterKind::Keyword,
                    false,
                )],
                Type::NoneType,
            ),
            _ => return self.str_signature(),
        };
        MethodSignature { parameters, result }
    }

    /// Returns the signature of a method of `str`.
    fn str_signature(self) -> MethodSignature {
        let strings = Type::List(Box::new(Type::Str));
        let (parameters, result) = match self {
            Self::StrFind | Self::StrRfind | Self::StrIndex | Self::StrRindex | Self::StrCount => {
                (searched(positional("sub", Type::Str, true)), Type::Int)
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
                vec![positional("sep", Type::Str, true)],
                Type::Tuple(vec![Type::Str, Type::Str, Type::Str]),
            ),
            Self::StrJoin => (
                vec![positional_as("iterable", ParameterType::Strings, true)],
                Type::Str,
            ),
            Self::StrStrip | Self::StrLstrip | Self::StrRstrip => (
                vec![positional_as("chars", ParameterType::StrOrNone, false)],
                Type::Str,
            ),
            Self::StrRemoveprefix => (vec![positional("prefix", Type::Str, true)], Type::Str),
            Self::StrRemovesuffix => (vec![positional("suffix", Type::Str, true)], Type::Str),
            Self::StrReplace => (
                vec![
                    positional("old", Type::Str, true),
                    positional("new", Type::Str, true),
                    positional("count", Type::Int, false),
                ],
                Type::Str,
            ),
            Self::StrCenter | Self::StrLjust | Self::StrRjust => (
                vec![
                    positional("width", Type::Int, true),
                    positional("fillchar", Type::Str, false),
                ],
                Type::Str,
            ),
            Self::StrZfill => (vec![positional("width", Type::Int, true)], Type::Str),
            Self::StrExpandtabs => (
                vec![either("tabsize", ParameterType::Exact(Type::Int))],
                Type::Str,
            ),
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
