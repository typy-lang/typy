//! The constants `typing.Literal` names as types (TYP-28, ADR-0110). A literal type is a
//! refinement of its base type: a value of `Literal['r']` is a `str`, held as one, so an
//! expression's type is never a literal; a declared type is, and the checker tests the
//! values written where one is declared.

use std::fmt::{self, Write as _};

use typy_source::syntax::{Constant, Expr, UnaryOperator};

use crate::typed::TypedConstant;
use crate::types::Type;

/// One constant of a literal type.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LiteralValue {
    /// An `int`, as decimal digits with a leading `-` when negative and no leading zeros.
    Int(String),
    /// `True` or `False`.
    Bool(bool),
    /// A `str` without surrogates.
    Str(String),
    /// A `str` that holds a surrogate, as its code points (LEX-10).
    StrPoints(Vec<u32>),
    /// A `bytes`.
    Bytes(Vec<u8>),
}

impl LiteralValue {
    /// Returns the literal value of a constant, or `None` for `None` and a float, which no
    /// literal value is.
    #[must_use]
    pub fn of_constant(constant: &TypedConstant) -> Option<Self> {
        Some(match constant {
            TypedConstant::Int(digits) => Self::Int(canonical(digits)),
            TypedConstant::Bool(value) => Self::Bool(*value),
            TypedConstant::Str(text) => Self::Str(text.clone()),
            TypedConstant::StrPoints(points) => Self::StrPoints(points.clone()),
            TypedConstant::Bytes(bytes) => Self::Bytes(bytes.clone()),
            TypedConstant::None | TypedConstant::Float(_) => return None,
        })
    }

    /// Returns the literal value an expression writes as a constant, such as `'r'` or `-1`,
    /// or `None` for any other expression, and for `None`, a float and a complex number.
    #[must_use]
    pub(crate) fn of_written(expression: &Expr) -> Option<Self> {
        let (constant, negated) = match expression {
            Expr::Constant { value, .. } => (value, false),
            // `-1` is a constant to `Literal`, as PEP 586 reads it.
            Expr::UnaryOp {
                operator: UnaryOperator::USub,
                operand,
                ..
            } => match operand.as_ref() {
                Expr::Constant {
                    value: value @ Constant::Int(_),
                    ..
                } => (value, true),
                _ => return None,
            },
            _ => return None,
        };
        let constant = match constant {
            Constant::Int(digits) => TypedConstant::Int(digits.clone()),
            Constant::Bool(value) => TypedConstant::Bool(*value),
            Constant::Str(text) => TypedConstant::Str(text.clone()),
            Constant::StrPoints(points) => TypedConstant::StrPoints(points.clone()),
            Constant::Bytes(bytes) => TypedConstant::Bytes(bytes.clone()),
            _ => return None,
        };
        let found = Self::of_constant(&constant)?;
        if negated {
            found.negated()
        } else {
            Some(found)
        }
    }

    /// Returns the negated value of an `int`, as `-1` reads, or `None` for any other value.
    #[must_use]
    pub fn negated(&self) -> Option<Self> {
        let Self::Int(digits) = self else {
            return None;
        };
        Some(Self::Int(match digits.strip_prefix('-') {
            Some(positive) => positive.to_owned(),
            None if digits == "0" => digits.clone(),
            None => format!("-{digits}"),
        }))
    }

    /// Returns the type every value of this one has: `int`, `bool`, `str` or `bytes`.
    #[must_use]
    pub fn base(&self) -> Type {
        match self {
            Self::Int(_) => Type::Int,
            Self::Bool(_) => Type::Bool,
            Self::Str(_) | Self::StrPoints(_) => Type::Str,
            Self::Bytes(_) => Type::Bytes,
        }
    }

    /// Returns whether a value equal to this one, by CPython's `==`, is this value: `True`
    /// equals `1` too, but has another base type.
    #[must_use]
    pub fn equals(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Bool(flag), Self::Int(digits)) | (Self::Int(digits), Self::Bool(flag)) => {
                digits == if *flag { "1" } else { "0" }
            }
            _ => self == other,
        }
    }
}

impl fmt::Display for LiteralValue {
    /// Writes the value as CPython's `repr` writes it.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int(digits) => formatter.write_str(digits),
            Self::Bool(true) => formatter.write_str("True"),
            Self::Bool(false) => formatter.write_str("False"),
            Self::Str(text) => formatter.write_str(&crate::subset::quoted(text.chars(), false)),
            Self::StrPoints(points) => {
                // A surrogate is no `char`, so it is written as the escape `repr` gives it.
                formatter.write_char('\'')?;
                for point in points {
                    match char::from_u32(*point) {
                        Some('\'') => formatter.write_str("\\'")?,
                        Some('\\') => formatter.write_str("\\\\")?,
                        Some('\n') => formatter.write_str("\\n")?,
                        Some('\r') => formatter.write_str("\\r")?,
                        Some('\t') => formatter.write_str("\\t")?,
                        Some(each) if each.is_control() => write!(formatter, "\\x{point:02x}")?,
                        Some(each) => formatter.write_char(each)?,
                        None => write!(formatter, "\\u{point:04x}")?,
                    }
                }
                formatter.write_char('\'')
            }
            Self::Bytes(bytes) => formatter.write_str(&crate::subset::quoted(
                bytes.iter().copied().map(char::from),
                true,
            )),
        }
    }
}

/// Returns the digits without leading zeros, and `0` without a sign.
fn canonical(digits: &str) -> String {
    let (sign, digits) = digits
        .strip_prefix('-')
        .map_or(("", digits), |rest| ("-", rest));
    let digits = digits.trim_start_matches('0');
    if digits.is_empty() {
        "0".to_owned()
    } else {
        format!("{sign}{digits}")
    }
}

impl Type {
    /// Returns the literal type of one value.
    #[must_use]
    pub fn literal(value: LiteralValue) -> Self {
        Self::Literal(Box::new(value))
    }

    /// Returns true when the type is, or holds, a literal type.
    #[must_use]
    pub fn has_literal(&self) -> bool {
        let mut found = false;
        self.visit(&mut |ty| found |= matches!(ty, Self::Literal(_)));
        found
    }

    /// Returns the type with each literal type in it replaced by its base type, the type of a
    /// value of it. An instance of a generic class keeps its type arguments, since they name
    /// the copy the instance is of.
    #[must_use]
    pub fn without_literal(&self) -> Self {
        if !self.has_literal() {
            return self.clone();
        }
        let inner = |item: &Self| Box::new(item.without_literal());
        match self {
            Self::Literal(value) => value.base(),
            Self::Abstract(collection) => collection.members.without_literal(),
            Self::List(item) => Self::List(inner(item)),
            Self::TupleOf(item) => Self::TupleOf(inner(item)),
            Self::Set(item) => Self::Set(inner(item)),
            Self::Iterator(item) => Self::Iterator(inner(item)),
            Self::Iterable(item) => Self::Iterable(inner(item)),
            Self::Optional(item) => Self::optional(*inner(item)),
            Self::Dict(key, value) => Self::Dict(inner(key), inner(value)),
            Self::DictView(view, key, value) => Self::DictView(*view, inner(key), inner(value)),
            Self::Tuple(items) => Self::Tuple(items.iter().map(|item| *inner(item)).collect()),
            Self::Union(items) => Self::union(items.iter().map(|item| *inner(item)).collect()),
            Self::Callable {
                parameters,
                returns,
            } => Self::Callable {
                parameters: parameters.iter().map(|item| *inner(item)).collect(),
                returns: inner(returns),
            },
            Self::Generator {
                yields,
                sends,
                returns,
            } => Self::Generator {
                yields: inner(yields),
                sends: inner(sends),
                returns: inner(returns),
            },
            other => other.clone(),
        }
    }

    /// Returns the type a value of this declared type has: each abstract collection as the
    /// built-in types it holds (LIB-20), each literal type as its base type (TYP-28), and
    /// `Self` as the method's class (TYP-30).
    #[must_use]
    pub fn value_type(&self) -> Self {
        self.without_abstract().without_declared_forms()
    }

    /// Returns the literal values of a type made of literal types, with whether `None` is
    /// among them, or `None` for a type that holds any other value.
    #[must_use]
    pub fn literal_values(&self) -> Option<(Vec<LiteralValue>, bool)> {
        match self {
            Self::Literal(value) => Some((vec![value.as_ref().clone()], false)),
            Self::Optional(inner) => {
                let (values, _) = inner.literal_values()?;
                Some((values, true))
            }
            Self::Union(members) => {
                let mut values = Vec::new();
                for member in members {
                    let Self::Literal(value) = member else {
                        return None;
                    };
                    values.push(value.as_ref().clone());
                }
                Some((values, false))
            }
            _ => None,
        }
    }

    /// Returns true when a value of type `self` passes where `target` is declared, with each
    /// literal type in `target` taking only its own values: a `str` that is not known to be
    /// one of them does not pass for `Literal['r']` (TYP-28).
    #[must_use]
    pub fn takes_literals_of(&self, target: &Self) -> bool {
        match (self, target) {
            (Self::Error, _) | (_, Self::Error) | (Self::NoneType, Self::Optional(_)) => true,
            (_, Self::Literal(_)) => {
                matches!(self, Self::Literal(_)) && self.is_assignable_to(target)
            }
            (Self::Union(members), _) => members
                .iter()
                .all(|member| member.takes_literals_of(target)),
            (Self::Optional(inner), Self::Optional(other)) => inner.takes_literals_of(other),
            (_, Self::Optional(inner)) => self.takes_literals_of(inner),
            (_, Self::Union(members)) => {
                members.iter().any(|member| self.takes_literals_of(member))
            }
            _ => self.is_assignable_to(target),
        }
    }
}

/// Returns the literal types both `one` and `other` allow, each a type made of literal types,
/// or `None` when either holds another value or none is allowed by both.
pub(crate) fn meet(one: &Type, other: &Type) -> Option<Type> {
    let (values, one_optional) = one.literal_values()?;
    let (others, other_optional) = other.literal_values()?;
    let both: Vec<Type> = values
        .into_iter()
        .filter(|value| others.contains(value))
        .map(Type::literal)
        .collect();
    if both.is_empty() {
        return None;
    }
    let met = Type::union(both);
    Some(if one_optional && other_optional {
        Type::optional(met)
    } else {
        met
    })
}

/// Writes literal values as `Literal[...]` writes them: `Literal['r', 'w']`.
pub(crate) fn write_literal(
    formatter: &mut fmt::Formatter<'_>,
    values: &[&LiteralValue],
) -> fmt::Result {
    formatter.write_str("Literal[")?;
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            formatter.write_str(", ")?;
        }
        write!(formatter, "{value}")?;
    }
    formatter.write_str("]")
}

#[cfg(test)]
mod tests;
