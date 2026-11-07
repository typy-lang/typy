//! Integers without a size limit (DAT-01): inline words, and large integers on the heap.

mod arithmetic;
mod bitwise;
mod limbs;
mod natural;
mod text;
mod value;

use crate::header::ObjectHeader;

/// The fixed part of a large integer; the limbs follow it, least significant first (detailed design section 10.2).
#[repr(C)]
#[derive(Debug)]
pub struct BigIntObject {
    /// The object header.
    pub header: ObjectHeader,
    /// The number of limbs, negated for a negative value.
    pub sign_and_length: i32,
    padding: u32,
}

impl BigIntObject {
    pub(crate) const fn new(header: ObjectHeader, sign_and_length: i32) -> Self {
        Self {
            header,
            sign_and_length,
            padding: 0,
        }
    }
}

/// Returns true when a large integer word is negative.
///
/// # Safety
///
/// `word` points to a live large integer.
pub(crate) unsafe fn is_negative(word: i64) -> bool {
    // SAFETY: the caller passes the address of a live large integer.
    unsafe { (*(word as *const BigIntObject)).sign_and_length < 0 }
}

#[cfg(test)]
mod tests {
    use super::BigIntObject;

    #[test]
    fn the_fixed_part_takes_sixteen_bytes() {
        assert_eq!(core::mem::size_of::<BigIntObject>(), 16);
    }
}
