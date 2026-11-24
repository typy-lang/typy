//! Places a C call's integer arguments as Apple's arm64 convention does: in `x0` to `x7` while
//! they last, then on the stack, each at its own size (FFI-13, ADR-0058).

use inkwell::attributes::Attribute;
use inkwell::context::Context;
use inkwell::types::IntType;
use typy_tyir::CType;

use crate::structure_abi::Passing;

/// The general registers a call passes arguments in: `x0` to `x7`.
const GENERAL_REGISTERS: u32 = 8;

/// The general registers a call has left, as its arguments take them in order. A structure
/// result's address goes in `x8`, so it takes none.
pub(super) struct GeneralRegisters {
    left: u32,
}

impl GeneralRegisters {
    pub(super) const fn new() -> Self {
        Self {
            left: GENERAL_REGISTERS,
        }
    }

    /// Takes `count` registers and returns true when enough are left. Otherwise the argument
    /// goes on the stack and so does every later one, as AAPCS64 takes the rest.
    const fn take(&mut self, count: u32) -> bool {
        if count <= self.left {
            self.left -= count;
            return true;
        }
        self.left = 0;
        false
    }

    /// Takes the register a scalar of C type `ty` passes in, and returns whether it got one;
    /// a `float` or a `double` passes in a vector register and takes none of these.
    pub(super) const fn take_scalar(&mut self, ty: CType) -> bool {
        match ty {
            CType::Float | CType::Double => true,
            _ => self.take(1),
        }
    }

    /// Takes the registers a structure that passes as `passing` needs: none for one of
    /// `float`s or `double`s, which goes in vector registers, and one for each word otherwise,
    /// or for the address of a copy.
    pub(super) fn take_structure(&mut self, passing: &Passing) {
        let count = match passing {
            Passing::Registers(pieces) | Passing::Array(pieces)
                if !pieces.iter().any(|piece| piece.vector) =>
            {
                u32::try_from(pieces.len()).unwrap_or(u32::MAX)
            }
            Passing::Reference => 1,
            Passing::Nothing | Passing::Registers(_) | Passing::Array(_) | Passing::Memory => {
                return;
            }
        };
        self.take(count);
    }
}

/// Returns the type an integer or `c_bool` argument of C type `ty` takes on Apple's stack, its
/// C width, with the extension clang marks it with: `signext` for a signed type narrower than
/// 32 bits, and `zeroext` for an unsigned one or `c_bool`. Any other type gives `None`.
pub(super) fn stack_type(context: &Context, ty: CType) -> Option<(IntType<'_>, Option<Attribute>)> {
    let (ty, extension) = match (ty, ty.integer()) {
        (CType::Bool, _) => (context.bool_type(), Some("zeroext")),
        (_, Some((8, signed))) => (context.i8_type(), Some(extension(signed))),
        (_, Some((16, signed))) => (context.i16_type(), Some(extension(signed))),
        (_, Some((32, _))) => (context.i32_type(), None),
        _ => return None,
    };
    let attribute = extension
        .map(|kind| context.create_enum_attribute(Attribute::get_named_enum_kind_id(kind), 0));
    Some((ty, attribute))
}

const fn extension(signed: bool) -> &'static str {
    if signed { "signext" } else { "zeroext" }
}

#[cfg(test)]
mod tests;
