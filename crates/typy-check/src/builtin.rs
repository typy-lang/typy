//! The builtin functions TyPy 0.1 compiles (LIB-01).

/// A builtin function TyPy 0.1 compiles (LIB-01). Each one is typed by the checker and lowered to runtime calls, because several take a variable number of arguments, which the 0.1 subset cannot declare (EXP-17).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[allow(
    missing_docs,
    reason = "each variant is the Python builtin of the same name"
)]
pub enum Builtin {
    Abs,
    Bool,
    Chr,
    Float,
    Int,
    Len,
    Max,
    Min,
    Ord,
    Print,
    Range,
    Repr,
    Str,
}

impl Builtin {
    /// Returns the builtin with this name, if 0.1 provides it.
    #[must_use]
    pub fn named(name: &str) -> Option<Self> {
        Some(match name {
            "abs" => Self::Abs,
            "bool" => Self::Bool,
            "chr" => Self::Chr,
            "float" => Self::Float,
            "int" => Self::Int,
            "len" => Self::Len,
            "max" => Self::Max,
            "min" => Self::Min,
            "ord" => Self::Ord,
            "print" => Self::Print,
            "range" => Self::Range,
            "repr" => Self::Repr,
            "str" => Self::Str,
            _ => return None,
        })
    }

    /// Returns the builtin's Python name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Abs => "abs",
            Self::Bool => "bool",
            Self::Chr => "chr",
            Self::Float => "float",
            Self::Int => "int",
            Self::Len => "len",
            Self::Max => "max",
            Self::Min => "min",
            Self::Ord => "ord",
            Self::Print => "print",
            Self::Range => "range",
            Self::Repr => "repr",
            Self::Str => "str",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Builtin;

    #[test]
    fn every_builtin_of_0_1_is_found_by_its_name() {
        for name in [
            "abs", "bool", "chr", "float", "int", "len", "max", "min", "ord", "print", "range",
            "repr", "str",
        ] {
            let builtin = Builtin::named(name).unwrap();
            assert_eq!(builtin.name(), name);
        }
        assert_eq!(Builtin::named("eval"), None);
    }
}
