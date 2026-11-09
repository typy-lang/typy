//! The header at the start of every heap object (NFR-12).

/// A reference count that marks an object as immortal: it is never counted or freed, and may live in read-only memory.
pub const IMMORTAL: u32 = u32::MAX;
/// The type ID of `str` objects.
pub const STR_TYPE_ID: u32 = 1;
/// The type ID of exception objects.
pub const EXCEPTION_TYPE_ID: u32 = 2;
/// The type ID of large integers.
pub const BIG_INT_TYPE_ID: u32 = 3;
/// The type ID of a `list`.
pub const LIST_TYPE_ID: u32 = 4;
/// The type ID of a `tuple[T, ...]`, a list object that is never changed after it is sealed.
pub const TUPLE_TYPE_ID: u32 = 5;
/// The type ID of a `bytes`: a `str` object of one byte per code point, each code point a byte.
pub const BYTES_TYPE_ID: u32 = 6;
/// The type ID of a `set`.
pub const SET_TYPE_ID: u32 = 7;
/// The type ID of a `dict`.
pub const DICT_TYPE_ID: u32 = 8;

/// The first 8 bytes of every heap object.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObjectHeader {
    /// The number of references, or `IMMORTAL`.
    pub reference_count: u32,
    /// The object's type ID.
    pub type_id: u32,
}

impl ObjectHeader {
    /// Returns the header of a new object with one reference.
    #[must_use]
    pub const fn new(type_id: u32) -> Self {
        Self {
            reference_count: 1,
            type_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ObjectHeader;

    #[test]
    fn the_header_takes_eight_bytes() {
        assert_eq!(core::mem::size_of::<ObjectHeader>(), 8);
        assert_eq!(ObjectHeader::new(1).reference_count, 1);
    }
}
