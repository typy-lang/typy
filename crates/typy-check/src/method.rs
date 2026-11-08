//! The methods of the built-in types that a release provides, under their Python names (LIB-03).

use crate::types::Type;

/// A method of a built-in type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Method {
    /// `list.append(item)`: adds an item at the end and returns `None`.
    ListAppend,
}

impl Method {
    /// Returns the method `name` of a value of type `receiver`, if the release provides it.
    #[must_use]
    pub fn of(receiver: &Type, name: &str) -> Option<Self> {
        match (receiver, name) {
            (Type::List(_), "append") => Some(Self::ListAppend),
            _ => None,
        }
    }

    /// Returns the method's Python name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::ListAppend => "append",
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
        assert_eq!(Method::ListAppend.name(), "append");
    }
}
