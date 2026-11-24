//! `Callable[[A, ...], R]`, the type of a function value (FUN-05, NFR-19, ADR-0060).

use crate::types::Type;

impl Type {
    /// Returns true when a function value of this `Callable` type may be held where the
    /// `Callable` type `target` is declared: the same number of parameters, each argument
    /// `target` passes taken by the function's parameter as it is, and the function's result
    /// taken as `target`'s as it is (NFR-19). A call through the value converts nothing, so
    /// each pair is held alike in memory and may differ only by the class of an object.
    #[must_use]
    pub(crate) fn fits_callable(&self, target: &Self) -> bool {
        let (
            Self::Callable {
                parameters,
                returns,
            },
            Self::Callable {
                parameters: declared,
                returns: declared_returns,
            },
        ) = (self, target)
        else {
            return false;
        };
        parameters.len() == declared.len()
            && declared
                .iter()
                .zip(parameters)
                .all(|(argument, parameter)| argument.passes_unchanged_as(parameter))
            && returns.passes_unchanged_as(declared_returns)
    }

    /// Returns true when a value of this type is a value of `target` as it is held: the same
    /// type, an instance or an exception of a class derived from `target`'s, either of them
    /// optional, or a function value that fits `target`. Every such object is one pointer,
    /// whatever its class, so nothing is converted.
    fn passes_unchanged_as(&self, target: &Self) -> bool {
        match (self, target) {
            (Self::Error, _) | (_, Self::Error) => true,
            // A protocol value is dispatched on by the class it holds (ADR-0041), so only the
            // same protocol takes it unchanged.
            (Self::Instance(class), Self::Instance(base)) if !class.protocol && !base.protocol => {
                class.derives_from(base)
            }
            (Self::Exception(class), Self::Exception(base)) => class.derives_from(base),
            (Self::Optional(inner), Self::Optional(base)) => inner.passes_unchanged_as(base),
            (Self::Callable { .. }, Self::Callable { .. }) => self.fits_callable(target),
            _ => self.matches(target),
        }
    }

    /// Returns the parameter types and the result type of a `Callable` type.
    #[must_use]
    pub fn callable_parts(&self) -> Option<(&[Self], &Self)> {
        match self {
            Self::Callable {
                parameters,
                returns,
            } => Some((parameters, returns)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests;
