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
    Chr,
    Dict,
    Enumerate,
    Float,
    Int,
    Len,
    List,
    Max,
    Min,
    Ord,
    Print,
    Range,
    Repr,
    Reversed,
    Round,
    Set,
    Sorted,
    Str,
    Sum,
    Tuple,
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
            "chr" => Self::Chr,
            "dict" => Self::Dict,
            "enumerate" => Self::Enumerate,
            "float" => Self::Float,
            "int" => Self::Int,
            "len" => Self::Len,
            "list" => Self::List,
            "max" => Self::Max,
            "min" => Self::Min,
            "ord" => Self::Ord,
            "print" => Self::Print,
            "range" => Self::Range,
            "repr" => Self::Repr,
            "reversed" => Self::Reversed,
            "round" => Self::Round,
            "set" => Self::Set,
            "sorted" => Self::Sorted,
            "str" => Self::Str,
            "sum" => Self::Sum,
            "tuple" => Self::Tuple,
            "zip" => Self::Zip,
            _ => return None,
        })
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
            Self::Chr => "chr",
            Self::Dict => "dict",
            Self::Enumerate => "enumerate",
            Self::Float => "float",
            Self::Int => "int",
            Self::Len => "len",
            Self::List => "list",
            Self::Max => "max",
            Self::Min => "min",
            Self::Ord => "ord",
            Self::Print => "print",
            Self::Range => "range",
            Self::Repr => "repr",
            Self::Reversed => "reversed",
            Self::Round => "round",
            Self::Set => "set",
            Self::Sorted => "sorted",
            Self::Str => "str",
            Self::Sum => "sum",
            Self::Tuple => "tuple",
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
            "chr",
            "dict",
            "enumerate",
            "float",
            "int",
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
