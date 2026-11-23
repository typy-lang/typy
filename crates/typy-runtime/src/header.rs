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
/// The type ID of a view of a dict: its keys, values or items.
pub const DICT_VIEW_TYPE_ID: u32 = 9;
/// The type ID of a type object (ADR-0012).
pub const TYPE_TYPE_ID: u32 = 10;
/// The type ID of an instance of a class, whose type object follows the header (ADR-0012).
pub const INSTANCE_TYPE_ID: u32 = 11;
/// The type ID of an iterator that `iter()` makes of a built-in iterable (ADR-0014).
pub const ITERATOR_TYPE_ID: u32 = 12;
/// The type ID of the traceback `__exit__` receives, which holds no frames (CST-10).
pub const TRACEBACK_TYPE_ID: u32 = 13;
/// The type ID of a `range` held as a value (DAT-17).
pub const RANGE_TYPE_ID: u32 = 14;
/// The type ID of a module object, which is immortal (MOD-10).
pub const MODULE_TYPE_ID: u32 = 15;
/// The type ID of a text file, `_io.TextIOWrapper` (LIB-04).
pub const TEXT_FILE_TYPE_ID: u32 = 17;
/// The type ID of a type alias object, `typing.TypeAliasType`, which is immortal (SMT-12).
pub const TYPE_ALIAS_TYPE_ID: u32 = 19;
/// The type ID of `sys.version_info`, one immortal object (LIB-16).
pub const VERSION_INFO_TYPE_ID: u32 = 20;
/// The type ID of `NotImplemented`, one immortal object (CLS-21).
pub const NOT_IMPLEMENTED_TYPE_ID: u32 = 21;
/// The type ID of a `slice` (CLS-23).
pub const SLICE_TYPE_ID: u32 = 22;
/// The type ID of a `ctypes` object: a scalar, a pointer or an array (FFI-03, ADR-0048).
pub const CDATA_TYPE_ID: u32 = 23;
/// The type ID of a function object: a function of the program as a value (FUN-05,
/// ADR-0060).
pub const FUNCTION_TYPE_ID: u32 = 24;
/// The type ID of a bound method: a function object whose one capture is the instance it
/// was read through (FUN-05, ADR-0060).
pub const METHOD_TYPE_ID: u32 = 25;
/// The type ID of a cell, a variable a nested function shares (FUN-04, ADR-0061).
pub const CELL_TYPE_ID: u32 = 26;
/// The type ID of a generator object, the state of a generator function's body between
/// steps (SMT-11, ADR-0063).
pub const GENERATOR_TYPE_ID: u32 = 27;
/// The type ID of the object of a built-in function used as a value, laid out as a function
/// object, which CPython writes as `<built-in function NAME>` (FUN-08, ADR-0077).
pub const BUILTIN_FUNCTION_TYPE_ID: u32 = 28;
/// The type ID of the object of a method of a built-in type read through the type, such as
/// `str.upper`, laid out as a function object (FUN-08, ADR-0077).
pub const METHOD_DESCRIPTOR_TYPE_ID: u32 = 29;
/// The type ID of a method of a built-in type read through a value, such as `words.append`,
/// laid out as a bound method (FUN-08, ADR-0077).
pub const BUILTIN_METHOD_TYPE_ID: u32 = 30;
/// The type ID of the object of a built-in type called as a function, such as `int` in
/// `key=int`, laid out as a function object and written as the type (FUN-14, ADR-0077).
pub const BUILTIN_TYPE_FUNCTION_TYPE_ID: u32 = 31;
/// The type ID of an `int` boxed where a value of type `object` holds it, laid out as a
/// `BoxedInt` (TYP-23, ADR-0090).
pub const BOXED_INT_TYPE_ID: u32 = 32;
/// The type ID of a `float` boxed where a value of type `object` holds it, laid out as a
/// `BoxedFloat` (TYP-23, ADR-0090).
pub const BOXED_FLOAT_TYPE_ID: u32 = 33;
/// The type ID of the immortal objects `True` and `False`, which a value of type `object`
/// holds (TYP-23, ADR-0090).
pub const BOOL_OBJECT_TYPE_ID: u32 = 34;
/// The type ID of the immortal object `None`, which a value of type `object` holds (TYP-23,
/// ADR-0090).
pub const NONE_OBJECT_TYPE_ID: u32 = 35;
/// The type ID of a typing value, such as `list[int]` or `int | None` read as a value, laid
/// out as a `TypingValueObject` (TYP-18, ADR-0087).
pub const TYPING_VALUE_TYPE_ID: u32 = 36;
/// The type ID of a `bytearray`, laid out as a `BytearrayObject` (LIB-28, ADR-0108).
pub const BYTEARRAY_TYPE_ID: u32 = 37;

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
mod tests;
