//! The builtin functions TyPy compiles (LIB-01, LIB-02).

/// A builtin function TyPy compiles (LIB-01, LIB-02). Each one is typed by the checker and lowered to runtime calls, because several take a variable number of arguments, which the 0.1 subset cannot declare (EXP-17).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[allow(
    missing_docs,
    reason = "each variant is the Python builtin of the same name"
)]
pub enum Builtin {
    Abs,
    All,
    Any,
    Bool,
    Bytes,
    /// `callable(x)`, whether `x` is a function value or a class (LIB-23).
    Callable,
    Chr,
    Dict,
    Enumerate,
    Float,
    Int,
    Input,
    IsInstance,
    /// `issubclass(class, classes)` (CLS-19).
    IsSubclass,
    Iter,
    Len,
    List,
    Max,
    Min,
    Next,
    Open,
    Ord,
    Print,
    Range,
    /// `slice(stop)` or `slice(start, stop[, step])` (CLS-23).
    Slice,
    Repr,
    Reversed,
    Round,
    Set,
    Sorted,
    Str,
    Sum,
    Tuple,
    Type,
    Zip,
}

impl Builtin {
    /// Returns the builtin with this name, if the release provides it.
    #[must_use]
    pub fn named(name: &str) -> Option<Self> {
        Some(match name {
            "abs" => Self::Abs,
            "all" => Self::All,
            "any" => Self::Any,
            "bool" => Self::Bool,
            "bytes" => Self::Bytes,
            "callable" => Self::Callable,
            "chr" => Self::Chr,
            "dict" => Self::Dict,
            "enumerate" => Self::Enumerate,
            "float" => Self::Float,
            "int" => Self::Int,
            "input" => Self::Input,
            "isinstance" => Self::IsInstance,
            "issubclass" => Self::IsSubclass,
            "iter" => Self::Iter,
            "len" => Self::Len,
            "list" => Self::List,
            "max" => Self::Max,
            "min" => Self::Min,
            "next" => Self::Next,
            "open" => Self::Open,
            "ord" => Self::Ord,
            "print" => Self::Print,
            "range" => Self::Range,
            "slice" => Self::Slice,
            "repr" => Self::Repr,
            "reversed" => Self::Reversed,
            "round" => Self::Round,
            "set" => Self::Set,
            "sorted" => Self::Sorted,
            "str" => Self::Str,
            "sum" => Self::Sum,
            "tuple" => Self::Tuple,
            "type" => Self::Type,
            "zip" => Self::Zip,
            _ => return None,
        })
    }

    /// Returns true for a built-in function, which CPython writes as `<built-in function
    /// NAME>`, and false for a built-in type, such as `int` or `zip`, which is a class.
    #[must_use]
    pub const fn is_function(self) -> bool {
        matches!(
            self,
            Self::Abs
                | Self::All
                | Self::Any
                | Self::Callable
                | Self::Chr
                | Self::Input
                | Self::IsInstance
                | Self::IsSubclass
                | Self::Iter
                | Self::Len
                | Self::Max
                | Self::Min
                | Self::Next
                | Self::Open
                | Self::Ord
                | Self::Print
                | Self::Repr
                | Self::Round
                | Self::Sorted
                | Self::Sum
        )
    }

    /// Returns true for a built-in type whose call TyPy types from its arguments, which is a
    /// value where a `Callable` is expected, as `key=int` (FUN-14).
    #[must_use]
    pub const fn is_callable_type(self) -> bool {
        matches!(
            self,
            Self::Int
                | Self::Float
                | Self::Str
                | Self::Bool
                | Self::Bytes
                | Self::List
                | Self::Tuple
                | Self::Set
                | Self::Dict
        )
    }

    /// Returns the builtin's Python name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Abs => "abs",
            Self::All => "all",
            Self::Any => "any",
            Self::Bool => "bool",
            Self::Bytes => "bytes",
            Self::Callable => "callable",
            Self::Chr => "chr",
            Self::Dict => "dict",
            Self::Enumerate => "enumerate",
            Self::Float => "float",
            Self::Int => "int",
            Self::Input => "input",
            Self::IsInstance => "isinstance",
            Self::IsSubclass => "issubclass",
            Self::Iter => "iter",
            Self::Len => "len",
            Self::List => "list",
            Self::Max => "max",
            Self::Min => "min",
            Self::Next => "next",
            Self::Open => "open",
            Self::Ord => "ord",
            Self::Print => "print",
            Self::Range => "range",
            Self::Slice => "slice",
            Self::Repr => "repr",
            Self::Reversed => "reversed",
            Self::Round => "round",
            Self::Set => "set",
            Self::Sorted => "sorted",
            Self::Str => "str",
            Self::Sum => "sum",
            Self::Tuple => "tuple",
            Self::Type => "type",
            Self::Zip => "zip",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Builtin;

    #[test]
    fn every_builtin_is_found_by_its_name() {
        for name in [
            "abs",
            "all",
            "any",
            "bool",
            "bytes",
            "callable",
            "chr",
            "dict",
            "enumerate",
            "float",
            "int",
            "isinstance",
            "issubclass",
            "len",
            "list",
            "max",
            "min",
            "ord",
            "print",
            "range",
            "repr",
            "reversed",
            "round",
            "set",
            "sorted",
            "str",
            "sum",
            "tuple",
            "zip",
        ] {
            let builtin = Builtin::named(name).unwrap();
            assert_eq!(builtin.name(), name);
        }
        assert_eq!(Builtin::named("eval"), None);
    }
}
